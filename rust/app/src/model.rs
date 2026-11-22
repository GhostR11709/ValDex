use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Youtube,
    Soundcloud,
    Audius,
    Archive,
}

impl Source {
    pub const ALL: [Source; 4] = [Source::Youtube, Source::Soundcloud, Source::Audius, Source::Archive];

    pub fn label(self) -> &'static str {
        match self {
            Source::Youtube => "YouTube Music",
            Source::Soundcloud => "SoundCloud",
            Source::Audius => "Audius",
            Source::Archive => "Internet Archive",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Source::Youtube => "Shelves come from YouTube Music. Songs you save stay on this PC.",
            Source::Soundcloud => "Music from SoundCloud. No account, and playback is not capped in this app.",
            Source::Audius => "Free music uploaded by artists. No account, and playback is not capped in this app.",
            Source::Archive => "Free netlabel releases and old recordings. No account.",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Source::Youtube => "youtube",
            Source::Soundcloud => "soundcloud",
            Source::Audius => "audius",
            Source::Archive => "archive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Song,
    Video,
    Album,
    Playlist,
    Artist,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Song => "Song",
            Kind::Video => "Video",
            Kind::Album => "Album",
            Kind::Playlist => "Playlist",
            Kind::Artist => "Artist",
        }
    }

    pub fn api(self) -> Option<&'static str> {
        match self {
            Kind::Album => Some("album"),
            Kind::Artist => Some("artist"),
            Kind::Playlist => Some("playlist"),
            Kind::Song | Kind::Video => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Album {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    #[serde(default)]
    pub source: Option<Source>,
    pub title: String,
    #[serde(default)]
    pub artists: Vec<Person>,
    #[serde(default)]
    pub album: Option<Album>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub thumbnail: String,
}

impl Track {
    pub fn source(&self) -> Source {
        self.source.unwrap_or(Source::Youtube)
    }

    pub fn artist(&self) -> String {
        let names: Vec<&str> = self.artists.iter().map(|artist| artist.name.as_str()).filter(|name| !name.is_empty()).collect();
        if names.is_empty() { "Unknown artist".into() } else { names.join(", ") }
    }

    pub fn same(&self, other: &Track) -> bool {
        self.id == other.id && self.source() == other.source()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Card {
    #[serde(default)]
    pub kind: Kind,
    pub id: String,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub track: Option<Track>,
}

impl Card {
    pub fn source(&self) -> Source {
        self.source.or_else(|| self.track.as_ref().map(Track::source)).unwrap_or(Source::Youtube)
    }

    pub fn art(&self) -> &str {
        if !self.thumbnail.is_empty() { &self.thumbnail } else { self.track.as_ref().map(|track| track.thumbnail.as_str()).unwrap_or("") }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Shelf {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub items: Vec<Card>,
}

impl Shelf {
    pub fn songs(&self) -> Vec<Track> {
        self.items.iter().filter(|item| matches!(item.kind, Kind::Song | Kind::Video)).filter_map(|item| item.track.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Home {
    #[serde(default)]
    pub filters: Vec<String>,
    #[serde(default)]
    pub shelves: Vec<Shelf>,
}

impl Home {
    #[allow(dead_code)]
    pub fn tracks(&self) -> Vec<Track> {
        let mut tracks = Vec::new();
        for shelf in &self.shelves {
            for track in shelf.songs() {
                if !tracks.iter().any(|have: &Track| have.same(&track)) {
                    tracks.push(track);
                }
            }
        }
        tracks
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Search {
    #[serde(default)]
    pub songs: Vec<Track>,
    #[serde(default)]
    pub videos: Vec<Track>,
    #[serde(default)]
    pub albums: Vec<Card>,
    #[serde(default)]
    pub artists: Vec<Card>,
    #[serde(default)]
    pub playlists: Vec<Card>,
}

impl Search {
    pub fn tracks(&self) -> Vec<Track> {
        self.songs.iter().chain(self.videos.iter()).cloned().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Collection {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub shelves: Vec<Shelf>,
}

impl Collection {
    pub fn playable(&self) -> Vec<Track> {
        if !self.tracks.is_empty() {
            return self.tracks.clone();
        }
        let mut tracks = Vec::new();
        for shelf in &self.shelves {
            for track in shelf.songs() {
                if !tracks.iter().any(|have: &Track| have.same(&track)) {
                    tracks.push(track);
                }
            }
        }
        tracks
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Playback {
    pub mode: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Look {
    #[default]
    Night,
    Light,
    Purple,
    Rose,
    Ocean,
    Forest,
    Sunset,
    Ink,
    Neon,
}

impl Look {
    pub const ALL: [Look; 9] = [Look::Night, Look::Light, Look::Purple, Look::Rose, Look::Ocean, Look::Forest, Look::Sunset, Look::Ink, Look::Neon];

    pub fn label(self) -> &'static str {
        match self {
            Look::Night => "Night",
            Look::Light => "Light",
            Look::Purple => "Purple",
            Look::Rose => "Rose",
            Look::Ocean => "Ocean",
            Look::Forest => "Forest",
            Look::Sunset => "Sunset",
            Look::Ink => "Ink",
            Look::Neon => "Neon",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Look::Night => "The original pink and black.",
            Look::Light => "A light page, same pink mark.",
            Look::Purple => "Violet instead of pink.",
            Look::Rose => "A warmer rose on black.",
            Look::Ocean => "Deep water and a bright mark.",
            Look::Forest => "Dark green and a leaf mark.",
            Look::Sunset => "Orange over a late evening.",
            Look::Ink => "Almost black, with a white mark.",
            Look::Neon => "Black, with a neon green mark.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume {
    pub queue: Vec<Track>,
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub position: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Library {
    #[serde(default)]
    pub likes: Vec<Track>,
    #[serde(default)]
    pub recents: Vec<Track>,
    #[serde(default)]
    pub searches: Vec<String>,
    #[serde(default)]
    pub downloads: Vec<Track>,
    #[serde(default)]
    pub catalog: Option<Source>,
    #[serde(default)]
    pub look: Look,
    #[serde(default)]
    pub resume: Option<Resume>,
    #[serde(default = "sidebar_open")]
    pub drawer: bool,
    #[serde(default)]
    pub above: bool,
    #[serde(default = "usual_volume")]
    pub volume: f32,
}

fn sidebar_open() -> bool {
    true
}

fn usual_volume() -> f32 {
    0.8
}

pub fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{size:.1} {}", units[unit]) }
}

pub fn format_duration(seconds: Option<u64>) -> String {
    let Some(seconds) = seconds else { return String::new() };
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let remain = seconds % 60;
    if hours > 0 { format!("{hours}:{minutes:02}:{remain:02}") } else { format!("{minutes}:{remain:02}") }
}

pub fn format_clock(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".into();
    }
    let text = format_duration(Some(seconds.floor() as u64));
    if text.is_empty() { "0:00".into() } else { text }
}

pub fn greeting(hour: u32) -> &'static str {
    if hour < 5 {
        "Still up"
    } else if hour < 12 {
        "Good morning"
    } else if hour < 17 {
        "Good afternoon"
    } else if hour < 21 {
        "Good evening"
    } else {
        "Late night"
    }
}

pub fn greeting_now() -> &'static str {
    greeting(local_hour())
}

fn local_hour() -> u32 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct SystemTime {
            _year: u16,
            _month: u16,
            _day_of_week: u16,
            _day: u16,
            hour: u16,
            _minute: u16,
            _second: u16,
            _milliseconds: u16,
        }
        extern "system" {
            fn GetLocalTime(time: *mut SystemTime);
        }
        unsafe {
            let mut time = std::mem::zeroed();
            GetLocalTime(&mut time);
            time.hour as u32
        }
    }
    #[cfg(not(windows))]
    {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0);
        ((secs / 3600) % 24) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_library_stays_on_night_with_nothing_to_resume() {
        let library: Library = serde_json::from_str(r#"{"likes":[]}"#).expect("library");
        assert_eq!(library.look, Look::Night);
        assert!(library.resume.is_none());
        assert!(library.drawer);
        assert!(!library.above);
        assert_eq!(library.volume, 0.8);
    }

    #[test]
    fn home_keeps_song_cards_in_order() {
        let home: Home = serde_json::from_str(
            r#"{"filters":["Pop"],"shelves":[{"title":"Trending","items":[
                {"kind":"song","id":"a","title":"One","subtitle":"","thumbnail":"","track":{"id":"a","source":"audius","title":"One","artists":[{"name":"Ada"}],"duration":90,"thumbnail":""}},
                {"kind":"playlist","id":"p","title":"Mix","subtitle":"","thumbnail":""}
            ]}]}"#,
        )
        .unwrap();
        let tracks = home.tracks();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].source(), Source::Audius);
        assert_eq!(tracks[0].artist(), "Ada");
        assert_eq!(format_duration(Some(90)), "1:30");
        assert_eq!(format_duration(Some(3723)), "1:02:03");
        assert_eq!(greeting(9), "Good morning");
        assert_eq!(greeting(22), "Late night");
    }
}
