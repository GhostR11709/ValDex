use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "1574097507875815608";
const HANDSHAKE: u32 = 0;
const FRAME: u32 = 1;
const CLOSE: u32 = 2;
const PING: u32 = 3;
const PONG: u32 = 4;

pub struct Presence {
    tx: Sender<Note>,
}

enum Note {
    Id(String),
    Song(Song),
    Clear,
}

#[derive(Clone)]
struct Song {
    title: String,
    artist: String,
    image: String,
    playing: bool,
    start_ms: i64,
    end_ms: i64,
}

impl Presence {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Note::Id(CLIENT_ID.to_string()));
        thread::Builder::new().name("valdex-discord".into()).spawn(move || serve(rx)).ok();
        Self { tx }
    }

    pub fn show(&self, title: &str, artist: &str, image: &str, playing: bool, position: f32, duration: f32) {
        let position = if position.is_finite() { position.max(0.0) } else { 0.0 };
        let duration = if duration.is_finite() { duration.max(0.0) } else { 0.0 };
        let now = unix_ms();
        let start_ms = now.saturating_sub((position * 1000.0) as i64);
        let end_ms = start_ms.saturating_add((duration * 1000.0) as i64);
        let title = clip(title);
        let _ = self.tx.send(Note::Song(Song {
            title: if title.is_empty() { "ValDex".into() } else { title },
            artist: clip(artist),
            image: public_art(image),
            playing,
            start_ms,
            end_ms,
        }));
    }

    pub fn clear(&self) {
        let _ = self.tx.send(Note::Clear);
    }
}

pub fn valid_id(value: &str) -> bool {
    let count = value.len();
    (17..=20).contains(&count) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn clip(value: &str) -> String {
    value.trim().chars().take(128).collect()
}

fn public_art(url: &str) -> String {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") && (1..=300).contains(&url.len()) && !lower.contains("localhost") && !lower.contains("127.0.0.1") {
        url.to_string()
    } else {
        String::new()
    }
}

fn unix_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_millis() as i64).unwrap_or(0)
}

fn serve(rx: Receiver<Note>) {
    let mut id = String::new();
    let mut song: Option<Song> = None;
    let mut sent: Option<Option<Song>> = None;
    let mut pipe: Option<File> = None;
    let mut nonce = 1u64;
    let mut next_try = Instant::now();
    let pid = std::process::id();
    loop {
        let mut reset = false;
        match rx.recv_timeout(Duration::from_millis(400)) {
            Ok(note) => {
                apply(&mut id, &mut song, &mut reset, note);
                while let Ok(extra) = rx.try_recv() {
                    apply(&mut id, &mut song, &mut reset, extra);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if reset {
            pipe = None;
            sent = None;
            next_try = Instant::now();
        }
        if !valid_id(&id) {
            pipe = None;
            sent = None;
            continue;
        }
        if pipe.is_none() {
            if Instant::now() < next_try {
                continue;
            }
            match connect(&id) {
                Ok(file) => {
                    pipe = Some(file);
                    sent = None;
                }
                Err(_) => {
                    next_try = Instant::now() + Duration::from_secs(2);
                    continue;
                }
            }
        }
        let alive = match pipe.as_mut() {
            Some(file) => pump(file).is_ok(),
            None => false,
        };
        if !alive {
            pipe = None;
            sent = None;
            next_try = Instant::now() + Duration::from_secs(2);
            continue;
        }
        if song.is_none() && sent.is_none() {
            sent = Some(None);
            continue;
        }
        if unchanged(&sent, &song) {
            continue;
        }
        let payload = match &song {
            Some(current) => activity_json(current, nonce, pid),
            None => clear_json(nonce, pid),
        };
        nonce = nonce.wrapping_add(1);
        let wrote = match pipe.as_mut() {
            Some(file) => write_packet(file, FRAME, payload.as_bytes()).is_ok(),
            None => false,
        };
        if !wrote {
            pipe = None;
            sent = None;
            next_try = Instant::now() + Duration::from_secs(2);
            continue;
        }
        sent = Some(song.clone());
    }
}

fn apply(id: &mut String, song: &mut Option<Song>, reset: &mut bool, note: Note) {
    match note {
        Note::Id(next) => {
            if next != *id {
                *id = next;
                *reset = true;
            }
        }
        Note::Song(next) => *song = Some(next),
        Note::Clear => *song = None,
    }
}

fn unchanged(sent: &Option<Option<Song>>, song: &Option<Song>) -> bool {
    match sent {
        None => false,
        Some(previous) => match (previous, song) {
            (None, None) => true,
            (Some(before), Some(after)) => {
                before.title == after.title
                    && before.artist == after.artist
                    && before.playing == after.playing
                    && before.image == after.image
                    && (before.start_ms - after.start_ms).abs() < 2_500
                    && (before.end_ms - after.end_ms).abs() < 2_500
            }
            _ => false,
        },
    }
}

// type 2 is Listening. status_display_type 2 puts the song in "Listening to …".
fn activity_json(song: &Song, nonce: u64, pid: u32) -> String {
    let mut activity = serde_json::json!({
        "type": 2,
        "name": "ValDex",
        "details": song.title,
        "status_display_type": 2,
        "instance": false,
    });
    if !song.artist.is_empty() {
        activity["state"] = serde_json::Value::String(song.artist.clone());
    }
    if !song.image.is_empty() {
        activity["assets"] = serde_json::json!({
            "large_image": song.image,
            "large_text": song.title,
        });
    }
    if song.playing && song.start_ms > 0 {
        if song.end_ms > song.start_ms {
            activity["timestamps"] = serde_json::json!({ "start": song.start_ms, "end": song.end_ms });
        } else {
            activity["timestamps"] = serde_json::json!({ "start": song.start_ms });
        }
    }
    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "nonce": nonce.to_string(),
        "args": { "pid": pid, "activity": activity }
    })
    .to_string()
}

fn clear_json(nonce: u64, pid: u32) -> String {
    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "nonce": nonce.to_string(),
        "args": { "pid": pid, "activity": null }
    })
    .to_string()
}

fn connect(id: &str) -> io::Result<File> {
    let handshake = serde_json::json!({ "v": 1, "client_id": id }).to_string();
    let mut last = io::Error::new(io::ErrorKind::NotFound, "discord");
    for index in 0..10 {
        let mut file = match open_pipe(index) {
            Ok(file) => file,
            Err(error) => {
                last = error;
                continue;
            }
        };
        if write_packet(&mut file, HANDSHAKE, handshake.as_bytes()).is_err() {
            continue;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut ready = false;
        while Instant::now() < deadline {
            match read_one(&mut file, Duration::from_millis(200)) {
                Ok(Some((FRAME, body))) if body.windows(5).any(|window| window == b"READY") => {
                    ready = true;
                    break;
                }
                Ok(Some((PING, body))) => {
                    if write_packet(&mut file, PONG, &body).is_err() {
                        break;
                    }
                }
                Ok(Some((CLOSE, _))) | Err(_) => break,
                Ok(Some(_)) => {}
                Ok(None) => {}
            }
        }
        if ready {
            return Ok(file);
        }
    }
    Err(last)
}

fn open_pipe(index: u32) -> io::Result<File> {
    std::fs::OpenOptions::new().read(true).write(true).share_mode(0x1 | 0x2).open(format!(r"\\.\pipe\discord-ipc-{index}"))
}

fn pump(file: &mut File) -> io::Result<()> {
    loop {
        match read_one(file, Duration::from_millis(1))? {
            Some((PING, body)) => write_packet(file, PONG, &body)?,
            Some((CLOSE, _)) => return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "discord closed")),
            Some(_) => {}
            None => return Ok(()),
        }
    }
}

fn write_packet(file: &mut File, op: u32, body: &[u8]) -> io::Result<()> {
    file.write_all(&encode(op, body))?;
    file.flush()
}

fn encode(op: u32, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn read_one(file: &mut File, wait: Duration) -> io::Result<Option<(u32, Vec<u8>)>> {
    let deadline = Instant::now() + wait;
    loop {
        let available = peek_available(file)?;
        if available < 8 {
            if Instant::now() >= deadline {
                return Ok(None);
            }
            pause(deadline);
            continue;
        }
        let mut header = [0u8; 8];
        peek_into(file, &mut header)?;
        let op = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        if len > 1_048_576 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "discord frame"));
        }
        if available < (8 + len) as u32 {
            if Instant::now() >= deadline {
                return Ok(None);
            }
            pause(deadline);
            continue;
        }
        file.read_exact(&mut header)?;
        let mut body = vec![0u8; len];
        if len > 0 {
            file.read_exact(&mut body)?;
        }
        return Ok(Some((op, body)));
    }
}

fn pause(deadline: Instant) {
    let left = deadline.saturating_duration_since(Instant::now());
    if !left.is_zero() {
        thread::sleep(left.min(Duration::from_millis(20)));
    }
}

fn peek_available(file: &File) -> io::Result<u32> {
    let mut available = 0u32;
    let ok = unsafe { PeekNamedPipe(file.as_raw_handle(), std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut available, std::ptr::null_mut()) };
    if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(available) }
}

fn peek_into(file: &File, buffer: &mut [u8]) -> io::Result<()> {
    let mut read = 0u32;
    let ok = unsafe { PeekNamedPipe(file.as_raw_handle(), buffer.as_mut_ptr(), buffer.len() as u32, &mut read, std::ptr::null_mut(), std::ptr::null_mut()) };
    if ok == 0 || read < buffer.len() as u32 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn PeekNamedPipe(pipe: *mut std::ffi::c_void, buffer: *mut u8, size: u32, read: *mut u32, available: *mut u32, left: *mut u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn night(playing: bool) -> Song {
        Song { title: "Night".into(), artist: "Ada".into(), image: String::new(), playing, start_ms: 1_000, end_ms: 91_000 }
    }

    #[test]
    fn listening_names_the_song_and_keeps_a_clock_while_playing() {
        let value: serde_json::Value = serde_json::from_str(&activity_json(&night(true), 7, 42)).unwrap();
        assert_eq!(value["cmd"], "SET_ACTIVITY");
        assert_eq!(value["args"]["pid"], 42);
        let activity = &value["args"]["activity"];
        assert_eq!(activity["type"], 2);
        assert_eq!(activity["status_display_type"], 2);
        assert_eq!(activity["name"], "ValDex");
        assert_eq!(activity["details"], "Night");
        assert_eq!(activity["state"], "Ada");
        assert_eq!(activity["timestamps"]["start"], 1_000);
        assert_eq!(activity["timestamps"]["end"], 91_000);
        assert_eq!(activity["instance"], false);
    }

    #[test]
    fn paused_listening_hides_the_clock() {
        let value: serde_json::Value = serde_json::from_str(&activity_json(&night(false), 1, 1)).unwrap();
        assert!(value["args"]["activity"].get("timestamps").is_none());
        assert_eq!(value["args"]["activity"]["details"], "Night");
    }

    #[test]
    fn quotes_in_a_title_stay_inside_the_string() {
        let song = Song { title: "Say \"hi\"\\".into(), artist: "A\nB".into(), image: String::new(), playing: true, start_ms: 5, end_ms: 0 };
        let value: serde_json::Value = serde_json::from_str(&activity_json(&song, 1, 1)).unwrap();
        assert_eq!(value["args"]["activity"]["details"], "Say \"hi\"\\");
        assert_eq!(value["args"]["activity"]["state"], "A\nB");
        assert!(value["args"]["activity"]["timestamps"].get("end").is_none());
        assert_eq!(value["args"]["activity"]["timestamps"]["start"], 5);
    }

    #[test]
    fn idle_clears_the_activity() {
        let value: serde_json::Value = serde_json::from_str(&clear_json(3, 9)).unwrap();
        assert!(value["args"]["activity"].is_null());
        assert_eq!(value["args"]["pid"], 9);
    }

    #[test]
    fn a_frame_carries_its_opcode_and_length() {
        let bytes = encode(FRAME, br#"{"ok":true}"#);
        assert_eq!(&bytes[0..4], &FRAME.to_le_bytes());
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 11);
        assert_eq!(&bytes[8..], br#"{"ok":true}"#);
    }

    #[test]
    fn the_cover_is_the_discord_image() {
        let mut song = night(true);
        song.image = "https://example.com/cover.jpg".into();
        let value: serde_json::Value = serde_json::from_str(&activity_json(&song, 1, 1)).unwrap();
        assert_eq!(value["args"]["activity"]["assets"]["large_image"], "https://example.com/cover.jpg");
        assert_eq!(value["args"]["activity"]["assets"]["large_text"], "Night");
        assert!(public_art("http://127.0.0.1:4731/cover.jpg").is_empty());
        assert_eq!(public_art("https://example.com/cover.jpg"), "https://example.com/cover.jpg");
        assert!(public_art(&format!("https://example.com/{}", "a".repeat(290))).is_empty());
    }

    #[test]
    fn valdex_uses_its_own_application() {
        assert!(valid_id(CLIENT_ID));
    }

    #[test]
    fn an_application_id_is_a_long_number() {
        assert!(valid_id("12345678901234567"));
        assert!(!valid_id("123"));
        assert!(!valid_id("12345678901234567a"));
        assert!(!valid_id(""));
    }

    #[test]
    fn a_small_clock_drift_is_the_same_song() {
        let playing = night(true);
        let mut drifted = playing.clone();
        drifted.start_ms += 1_000;
        drifted.end_ms += 1_000;
        let sent = Some(Some(playing));
        assert!(unchanged(&sent, &Some(drifted)));
        let mut sought = night(true);
        sought.start_ms += 8_000;
        sought.end_ms += 8_000;
        assert!(!unchanged(&sent, &Some(sought)));
        assert!(!unchanged(&sent, &Some(night(false))));
        assert!(!unchanged(&None, &Some(night(true))));
    }
}
