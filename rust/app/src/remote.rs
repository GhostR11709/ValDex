use crate::model::{Collection, Home, Library, Playback, Search, Source, Track};

const BASE: &str = "http://127.0.0.1:4731";

#[derive(serde::Deserialize)]
struct ApiError {
    error: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().build().map_err(|error| error.to_string())
}

async fn get_json<T: serde::de::DeserializeOwned>(path: &str, query: &[(&str, &str)]) -> Result<T, String> {
    let response = client()?.get(format!("{BASE}{path}")).query(query).send().await.map_err(|error| {
        if error.is_connect() {
            "The current ValDex server is not running. Start it, then press Retry.".into()
        } else {
            error.to_string()
        }
    })?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        let message = serde_json::from_slice::<ApiError>(&bytes).ok().and_then(|body| body.error);
        return Err(message.unwrap_or_else(|| format!("The catalog answered {status}.")));
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

pub async fn home(source: Source, mood: &str) -> Result<Home, String> {
    let mut query = vec![("source", source.as_str())];
    if !mood.is_empty() {
        query.push(("mood", mood));
    }
    get_json("/api/home", &query).await
}

pub async fn search(source: Source, query: &str) -> Result<Search, String> {
    get_json("/api/search", &[("source", source.as_str()), ("q", query)]).await
}

pub async fn collection(kind: &str, source: Source, id: &str) -> Result<Collection, String> {
    get_json(&format!("/api/{kind}"), &[("source", source.as_str()), ("id", id)]).await
}

pub fn absolute_media(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.starts_with("https://") || raw.starts_with("http://") {
        Some(raw.to_string())
    } else if raw.starts_with('/') {
        Some(format!("{BASE}{raw}"))
    } else {
        None
    }
}

pub async fn image(url: &str) -> Result<Vec<u8>, String> {
    let response = client()?.get(url).timeout(std::time::Duration::from_secs(8)).send().await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err("Artwork didn't load.".into());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() < 32 || bytes.len() > 4_000_000 || !is_image(&bytes) {
        return Err("Artwork wasn't a picture.".into());
    }
    Ok(bytes.to_vec())
}

fn is_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF]) || bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"GIF8") || bytes.starts_with(b"RIFF")
}

pub async fn audio(track: &Track) -> Result<Vec<u8>, String> {
    let source = track.source().as_str();
    let playback: Playback = get_json("/api/playback", &[("source", source), ("id", track.id.as_str())]).await?;
    if playback.mode == "embed" {
        return Err(playback.reason.unwrap_or_else(|| "Direct audio isn't available for this song.".into()));
    }
    let pack = playback.mode == "hls";
    match load_audio(source, &track.id, pack).await {
        Ok(bytes) => crate::media::playable(bytes),
        Err(error) if retriable(&error) => load_audio(source, &track.id, pack).await.and_then(crate::media::playable),
        Err(error) => Err(error),
    }
}

fn retriable(error: &str) -> bool {
    !error.contains("cannot decode") && !error.contains("too large")
}

async fn load_audio(source: &str, id: &str, pack: bool) -> Result<Vec<u8>, String> {
    let mut query = vec![("source", source), ("id", id)];
    if pack {
        query.push(("pack", "1"));
    }
    let response = client()?
        .get(format!("{BASE}/api/stream"))
        .query(&query)
        .send()
        .await
        .map_err(|error| if error.is_connect() { "The current ValDex server is not running. Start it, then press Retry.".into() } else { error.to_string() })?;
    let status = response.status();
    if !status.is_success() {
        let bytes = response.bytes().await.unwrap_or_default();
        let message = serde_json::from_slice::<ApiError>(&bytes).ok().and_then(|body| body.error);
        return Err(message.unwrap_or_else(|| format!("The recording came back as {status}.")));
    }
    let kind = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("").to_string();
    if kind.contains("mpegurl") {
        return Err("This song is a stream the native player cannot decode yet.".into());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() > 80_000_000 {
        return Err("That recording is too large to play in this build.".into());
    }
    if bytes.starts_with(b"#EXTM3U") {
        return Err("This song is a stream the native player cannot decode yet.".into());
    }
    if bytes.is_empty() {
        return Err("The audio stream was empty.".into());
    }
    Ok(bytes.to_vec())
}

pub fn library_path() -> std::path::PathBuf {
    dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default()).join("Vivi").join("library.json")
}

pub fn load_library() -> Library {
    let Ok(text) = std::fs::read_to_string(library_path()) else { return Library::default() };
    serde_json::from_str(&text).unwrap_or_default()
}

#[cfg(test)]
mod live {
    use super::*;
    use rodio::Source as AudioSource;

    #[test]
    #[ignore = "needs the Next.js server on port 4731"]
    fn audius_file_decodes() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let tracks = home(Source::Audius, "").await.expect("home").tracks();
            let track = tracks.into_iter().find(|track| track.duration.unwrap_or(0) > 5).expect("a song");
            let bytes = audio(&track).await.expect("audio");
            assert_plays(bytes, &track.title);
        });
    }

    #[test]
    #[ignore = "needs the Next.js server on port 4731"]
    fn soundcloud_hls_decodes() {
        play_known(Source::Soundcloud, "2100018744", "AllmyViBes");
    }

    #[test]
    #[ignore = "needs the Next.js server on port 4731"]
    fn youtube_file_decodes() {
        play_known(Source::Youtube, "wB1vdj37z3Y", "All My Vibes");
    }

    fn play_known(source: Source, id: &str, title: &str) {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let track = Track { id: id.into(), source: Some(source), title: title.into(), artists: vec![], album: None, duration: None, thumbnail: String::new() };
            let bytes = audio(&track).await.unwrap_or_else(|error| panic!("{title}: {error}"));
            assert_plays(bytes, title);
        });
    }

    #[test]
    fn download_name_keeps_the_catalog_and_id() {
        let track = Track { id: "wB1vdj37z3Y".into(), source: Some(Source::Youtube), title: "All My Vibes".into(), artists: vec![], album: None, duration: None, thumbnail: String::new() };
        assert_eq!(download_name(&track), "youtube-wB1vdj37z3Y");
    }

    fn assert_plays(bytes: Vec<u8>, title: &str) {
        let decoder = rodio::Decoder::try_from(std::io::Cursor::new(bytes)).unwrap_or_else(|error| panic!("{title} did not decode: {error}"));
        let need = u64::from(decoder.sample_rate().get()) * u64::from(decoder.channels().get()) * 12;
        let count = decoder.take(need as usize + 1).count();
        assert!(count as u64 > need, "{title} decoded {count} samples, wanted more than {need}");
    }
}

pub fn download_name(track: &Track) -> String {
    let id: String = track.id.chars().filter(|symbol| symbol.is_ascii_alphanumeric() || *symbol == '-' || *symbol == '_').take(80).collect();
    format!("{}-{id}", track.source().as_str())
}

fn downloads_dir() -> std::path::PathBuf {
    library_path().parent().unwrap_or(std::path::Path::new(".")).join("downloads")
}

pub fn downloads_folder() -> std::path::PathBuf {
    downloads_dir()
}

pub fn open_downloads() -> Result<(), String> {
    let dir = downloads_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let opener = if cfg!(windows) { "explorer" } else { "open" };
    std::process::Command::new(opener).arg(&dir).spawn().map_err(|error| error.to_string())?;
    Ok(())
}

fn download_candidates(track: &Track) -> Vec<std::path::PathBuf> {
    let base = downloads_dir().join(download_name(track));
    let mut paths = Vec::new();
    for extension in ["m4a", "mp3", "flac", "ogg", "opus", "wav", "aac", "audio"] {
        paths.push(base.with_extension(extension));
    }
    paths.push(base);
    paths
}

fn locate_download(track: &Track) -> Option<std::path::PathBuf> {
    download_candidates(track).into_iter().find(|path| std::fs::metadata(path).is_ok_and(|meta| meta.len() > 0))
}

pub fn has_download(track: &Track) -> bool {
    locate_download(track).is_some()
}

pub fn download_len(track: &Track) -> u64 {
    locate_download(track).and_then(|path| std::fs::metadata(path).ok()).map(|meta| meta.len()).unwrap_or(0)
}

pub fn read_download(track: &Track) -> Option<Vec<u8>> {
    let bytes = std::fs::read(locate_download(track)?).ok()?;
    if bytes.is_empty() { None } else { Some(bytes) }
}

pub fn save_download(track: &Track, bytes: &[u8], cover: Option<&[u8]>) -> Result<(), String> {
    let stored = crate::songfile::store(bytes, track, cover);
    let path = write_download(track, &stored.extension, &stored.bytes)?;
    if !file_opens(&path) && file_bytes_open(bytes) {
        let plain = crate::songfile::plain(bytes);
        let fallback = write_download(track, &plain.extension, &plain.bytes)?;
        if fallback != path {
            let _ = std::fs::remove_file(path);
        }
        remove_other_downloads(track, &fallback);
        return Ok(());
    }
    remove_other_downloads(track, &path);
    Ok(())
}

/// Older downloads were saved without an extension or tags. Rewrite those once, and leave the old file if the new one will not play.
pub fn upgrade_download(track: &Track) -> Result<(), String> {
    let Some(path) = locate_download(track) else { return Ok(()) };
    if path.extension().is_some() {
        return Ok(());
    }
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    let cover = read_cover(track);
    save_download(track, &bytes, cover.as_deref())
}

fn write_download(track: &Track, extension: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    let dir = downloads_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let name = download_name(track);
    let path = dir.join(format!("{name}.{extension}"));
    let temp = dir.join(format!("{name}.{extension}.part"));
    std::fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn remove_other_downloads(track: &Track, keep: &std::path::Path) {
    for path in download_candidates(track) {
        if path != keep {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn file_opens(path: &std::path::Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else { return false };
    rodio::Decoder::try_from(file).is_ok_and(|decoder| decoder.take(32).count() >= 8)
}

fn file_bytes_open(bytes: &[u8]) -> bool {
    rodio::Decoder::try_from(std::io::Cursor::new(bytes.to_vec())).is_ok_and(|decoder| decoder.take(32).count() >= 8)
}

pub fn delete_download(track: &Track) {
    for path in download_candidates(track) {
        let _ = std::fs::remove_file(path);
    }
    delete_cover(track);
}

pub fn cover_key(track: &Track) -> String {
    format!("cover:{}", download_name(track))
}

pub fn cover_path(track: &Track) -> std::path::PathBuf {
    library_path().parent().unwrap_or(std::path::Path::new(".")).join("covers").join(format!("{}.img", download_name(track)))
}

pub fn read_cover(track: &Track) -> Option<Vec<u8>> {
    let bytes = std::fs::read(cover_path(track)).ok()?;
    if bytes.len() < 32 { None } else { Some(bytes) }
}

pub fn save_cover(track: &Track, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 32 {
        return Err("Artwork wasn't a picture.".into());
    }
    let path = cover_path(track);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}

pub fn delete_cover(track: &Track) {
    let _ = std::fs::remove_file(cover_path(track));
}

pub fn save_library(library: &Library) -> Result<(), String> {
    let path = library_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(library).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}
