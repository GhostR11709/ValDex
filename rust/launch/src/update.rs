#[cfg(windows)]
use std::io::Write;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::Deserialize;

const REPO: &str = "GhostR11709/ValDex";
const ASSET: &str = "ValDex-Setup.exe";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct Update {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn available() -> Option<Update> {
    #[cfg(windows)]
    {
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(8)).user_agent("ValDex").build().ok()?;
        let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
        let response = client.get(url).header("Accept", "application/vnd.github+json").send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        return choose(env!("CARGO_PKG_VERSION"), &response.json().ok()?);
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn install(update: &Update) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = update;
        return Err("This Mac build does not install the Windows setup.".into());
    }
    #[cfg(windows)]
    {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .user_agent("ValDex")
        .build()
        .map_err(|error| error.to_string())?;
    let response = client.get(&update.url).send().map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub returned {}.", response.status()));
    }
    let bytes = response.bytes().map_err(|error| error.to_string())?;
    if bytes.len() < 100_000 || bytes.get(0..2) != Some(b"MZ") {
        return Err("The download was not the ValDex installer.".into());
    }
    let path = installer_path()?;
    let mut file = std::fs::File::create(&path).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    schedule(&path)
    }
}

fn choose(current: &str, release: &Release) -> Option<Update> {
    let version = release.tag_name.trim().trim_start_matches('v').to_string();
    if !newer(current, &version) {
        return None;
    }
    let asset = release.assets.iter().find(|asset| asset.name.eq_ignore_ascii_case(ASSET))?;
    Some(Update { version, url: asset.browser_download_url.clone() })
}

fn newer(current: &str, latest: &str) -> bool {
    parts(latest) > parts(current)
}

fn parts(version: &str) -> [u64; 3] {
    let mut numbers = [0; 3];
    for (index, piece) in version.split('.').take(3).enumerate() {
        let digits: String = piece.chars().take_while(|char| char.is_ascii_digit()).collect();
        numbers[index] = digits.parse().unwrap_or(0);
    }
    numbers
}

#[cfg(windows)]
fn installer_path() -> Result<PathBuf, String> {
    let root = std::env::temp_dir().join("ValDex");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    Ok(root.join(ASSET))
}

#[cfg(windows)]
fn schedule(path: &Path) -> Result<(), String> {
    let shown = path.display().to_string().replace('"', "");
    let script = format!("timeout /t 2 /nobreak >nul & start \"\" \"{shown}\"");
    Command::new("cmd")
        .args(["/C", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, asset: &str) -> Release {
        Release {
            tag_name: tag.into(),
            assets: vec![Asset { name: asset.into(), browser_download_url: "https://example.test/ValDex-Setup.exe".into() }],
        }
    }

    #[test]
    fn a_higher_release_is_offered() {
        let update = choose("0.1.0", &release("v0.1.1", ASSET)).expect("update");
        assert_eq!(update.version, "0.1.1");
    }

    #[test]
    fn the_same_or_older_release_is_ignored() {
        assert!(choose("0.1.0", &release("v0.1.0", ASSET)).is_none());
        assert!(choose("0.2.0", &release("v0.1.9", ASSET)).is_none());
    }

    #[test]
    fn a_release_without_the_installer_is_ignored() {
        assert!(choose("0.1.0", &release("v0.2.0", "notes.txt")).is_none());
    }
}
