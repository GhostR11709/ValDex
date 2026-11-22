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
        let path = download(update)?;
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

fn progress_text(done: u64, total: u64) -> String {
    if total == 0 {
        return format!("{} so far", size_label(done));
    }
    format!("{} of {}", size_label(done), size_label(total))
}

fn size_label(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{} KB", bytes / 1_000)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(windows)]
struct Gate {
    cancelled: std::sync::atomic::AtomicBool,
    downloaded: std::sync::atomic::AtomicU64,
    total: std::sync::atomic::AtomicU64,
    finished: std::sync::atomic::AtomicBool,
    status: std::sync::Mutex<String>,
    result: std::sync::Mutex<Option<Result<PathBuf, String>>>,
}

#[cfg(windows)]
fn download(update: &Update) -> Result<PathBuf, String> {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    let gate = Arc::new(Gate {
        cancelled: std::sync::atomic::AtomicBool::new(false),
        downloaded: std::sync::atomic::AtomicU64::new(0),
        total: std::sync::atomic::AtomicU64::new(0),
        finished: std::sync::atomic::AtomicBool::new(false),
        status: std::sync::Mutex::new("Connecting…".into()),
        result: std::sync::Mutex::new(None),
    });
    let worker_gate = Arc::clone(&gate);
    let url = update.url.clone();
    let worker = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fetch(&url, &worker_gate)));
        let outcome = match outcome {
            Ok(result) => result,
            Err(_) => Err("The download stopped.".into()),
        };
        *worker_gate.result.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(outcome);
        worker_gate.finished.store(true, Ordering::Release);
    });
    let mut window = DownloadWindow::open(&format!("Downloading ValDex {}", update.version));
    while !gate.finished.load(Ordering::Acquire) {
        if window.dismissed() {
            gate.cancelled.store(true, Ordering::Relaxed);
        }
        let status = gate.status.lock().unwrap_or_else(|poison| poison.into_inner()).clone();
        let note = if gate.cancelled.load(Ordering::Relaxed) { "Cancelling…".to_string() } else { status };
        window.show(gate.downloaded.load(Ordering::Relaxed), gate.total.load(Ordering::Relaxed), &note);
        window.pump();
        std::thread::sleep(Duration::from_millis(30));
    }
    let _ = worker.join();
    let outcome = gate.result.lock().unwrap_or_else(|poison| poison.into_inner()).take().unwrap_or_else(|| Err("The download stopped.".into()));
    if outcome.is_ok() {
        let total = gate.total.load(Ordering::Relaxed).max(gate.downloaded.load(Ordering::Relaxed));
        window.show(total, total, "Opening the installer…");
        window.pump();
        std::thread::sleep(Duration::from_millis(400));
        window.pump();
    }
    window.close();
    outcome
}

#[cfg(windows)]
fn fetch(url: &str, gate: &std::sync::Arc<Gate>) -> Result<PathBuf, String> {
    use std::io::{Read, Write};
    use std::sync::atomic::Ordering;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(900))
        .user_agent("ValDex")
        .build()
        .map_err(|error| error.to_string())?;
    let mut response = client.get(url).send().map_err(|error| error.to_string())?;
    if gate.cancelled.load(Ordering::Relaxed) {
        return Err("The download was cancelled.".into());
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned {}.", response.status()));
    }
    let total = response.content_length().unwrap_or(0);
    gate.total.store(total, Ordering::Relaxed);
    *gate.status.lock().unwrap_or_else(|poison| poison.into_inner()) = progress_text(0, total);
    let path = installer_path()?;
    let partial = path.with_extension("part");
    let mut file = std::fs::File::create(&partial).map_err(|error| error.to_string())?;
    let mut buf = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    loop {
        if gate.cancelled.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&partial);
            return Err("The download was cancelled.".into());
        }
        let read = response.read(&mut buf).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buf[..read]).map_err(|error| error.to_string())?;
        downloaded += read as u64;
        gate.downloaded.store(downloaded, Ordering::Relaxed);
        *gate.status.lock().unwrap_or_else(|poison| poison.into_inner()) = progress_text(downloaded, total);
    }
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    if downloaded < 100_000 || !starts_with_mz(&partial) {
        let _ = std::fs::remove_file(&partial);
        return Err("The download was not the ValDex installer.".into());
    }
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    std::fs::rename(&partial, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(windows)]
fn starts_with_mz(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else { return false };
    use std::io::Read;
    let mut mark = [0_u8; 2];
    Read::read_exact(&mut file, &mut mark).is_ok() && mark == *b"MZ"
}

#[cfg(windows)]
struct DownloadWindow {
    window: *mut std::ffi::c_void,
    detail: *mut std::ffi::c_void,
    bar: *mut std::ffi::c_void,
    pulse: u32,
}

#[cfg(windows)]
impl DownloadWindow {
    fn open(title: &str) -> Self {
        let blank = Self { window: std::ptr::null_mut(), detail: std::ptr::null_mut(), bar: std::ptr::null_mut(), pulse: 0 };
        unsafe {
            let mut common = InitCommon { size: 8, icc: 0x20 };
            InitCommonControlsEx(&mut common);
            let class = wide("ValDexDownload");
            let instance = GetModuleHandleW(std::ptr::null());
            let window_class = WndClass {
                style: 0,
                wnd_proc: wnd_proc,
                cls_extra: 0,
                wnd_extra: 0,
                instance,
                icon: std::ptr::null_mut(),
                cursor: LoadCursorW(std::ptr::null_mut(), 32512 as *const u16),
                background: 16 as *mut std::ffi::c_void,
                menu_name: std::ptr::null(),
                class_name: class.as_ptr(),
            };
            RegisterClassW(&window_class);
            let width = 460;
            let height = 168;
            let x = (GetSystemMetrics(0) - width) / 2;
            let y = (GetSystemMetrics(1) - height) / 2;
            let name = wide("ValDex");
            let window = CreateWindowExW(0x00040000, class.as_ptr(), name.as_ptr(), 0x00C00000 | 0x00080000 | 0x10000000, x, y, width, height, std::ptr::null_mut(), std::ptr::null_mut(), instance, std::ptr::null_mut());
            if window.is_null() {
                return blank;
            }
            let heading = wide(title);
            let detail_text = wide("Connecting…");
            let label = CreateWindowExW(0, wide("STATIC").as_ptr(), heading.as_ptr(), 0x40000000 | 0x10000000, 24, 18, 400, 22, window, std::ptr::null_mut(), instance, std::ptr::null_mut());
            let detail = CreateWindowExW(0, wide("STATIC").as_ptr(), detail_text.as_ptr(), 0x40000000 | 0x10000000, 24, 46, 400, 22, window, std::ptr::null_mut(), instance, std::ptr::null_mut());
            let bar = CreateWindowExW(0, wide("msctls_progress32").as_ptr(), std::ptr::null(), 0x40000000 | 0x10000000 | 0x01, 24, 84, 400, 22, window, std::ptr::null_mut(), instance, std::ptr::null_mut());
            let font = GetStockObject(17);
            SendMessageW(label, 0x0030, font as usize, 1);
            SendMessageW(detail, 0x0030, font as usize, 1);
            SendMessageW(bar, 0x0406, 0, 1000);
            ShowWindow(window, 5);
            UpdateWindow(window);
            SetForegroundWindow(window);
            let view = Self { window, detail, bar, pulse: 0 };
            view.pump();
            view
        }
    }

    fn show(&mut self, done: u64, total: u64, note: &str) {
        if self.window.is_null() {
            return;
        }
        let text = wide(note);
        unsafe {
            SetWindowTextW(self.detail, text.as_ptr());
            let pos = if total == 0 {
                self.pulse = (self.pulse + 18) % 1000;
                self.pulse
            } else {
                (done.saturating_mul(1000) / total).min(1000) as u32
            };
            SendMessageW(self.bar, 0x0402, pos as usize, 0);
        }
    }

    fn pump(&self) {
        if self.window.is_null() {
            return;
        }
        unsafe {
            let mut message = Msg { hwnd: std::ptr::null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, x: 0, y: 0 };
            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, 1) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn dismissed(&self) -> bool {
        self.window.is_null() || unsafe { IsWindow(self.window) == 0 } || CANCELLED.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn close(self) {
        if !self.window.is_null() && unsafe { IsWindow(self.window) != 0 } {
            unsafe { DestroyWindow(self.window) };
        }
        CANCELLED.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn wnd_proc(hwnd: *mut std::ffi::c_void, message: u32, wparam: usize, lparam: isize) -> isize {
    if message == 0x0010 {
        CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed);
        DestroyWindow(hwnd);
        return 0;
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(windows)]
#[repr(C)]
struct InitCommon {
    size: u32,
    icc: u32,
}

#[cfg(windows)]
#[repr(C)]
struct WndClass {
    style: u32,
    wnd_proc: unsafe extern "system" fn(*mut std::ffi::c_void, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: *mut std::ffi::c_void,
    icon: *mut std::ffi::c_void,
    cursor: *mut std::ffi::c_void,
    background: *mut std::ffi::c_void,
    menu_name: *const u16,
    class_name: *const u16,
}

#[cfg(windows)]
#[repr(C)]
struct Msg {
    hwnd: *mut std::ffi::c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
}

#[cfg(windows)]
#[link(name = "comctl32")]
unsafe extern "system" {
    fn InitCommonControlsEx(init: *mut InitCommon) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetStockObject(object: i32) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const WndClass) -> u16;
    fn CreateWindowExW(ex_style: u32, class: *const u16, window: *const u16, style: u32, x: i32, y: i32, width: i32, height: i32, parent: *mut std::ffi::c_void, menu: *mut std::ffi::c_void, instance: *mut std::ffi::c_void, param: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn DefWindowProcW(hwnd: *mut std::ffi::c_void, message: u32, wparam: usize, lparam: isize) -> isize;
    fn DestroyWindow(hwnd: *mut std::ffi::c_void) -> i32;
    fn DispatchMessageW(message: *const Msg) -> isize;
    fn GetModuleHandleW(name: *const u16) -> *mut std::ffi::c_void;
    fn GetSystemMetrics(index: i32) -> i32;
    fn IsWindow(hwnd: *mut std::ffi::c_void) -> i32;
    fn LoadCursorW(instance: *mut std::ffi::c_void, name: *const u16) -> *mut std::ffi::c_void;
    fn PeekMessageW(message: *mut Msg, hwnd: *mut std::ffi::c_void, min: u32, max: u32, remove: u32) -> i32;
    fn SendMessageW(hwnd: *mut std::ffi::c_void, message: u32, wparam: usize, lparam: isize) -> isize;
    fn SetForegroundWindow(hwnd: *mut std::ffi::c_void) -> i32;
    fn SetWindowTextW(hwnd: *mut std::ffi::c_void, text: *const u16) -> i32;
    fn ShowWindow(hwnd: *mut std::ffi::c_void, command: i32) -> i32;
    fn TranslateMessage(message: *const Msg) -> i32;
    fn UpdateWindow(hwnd: *mut std::ffi::c_void) -> i32;
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

    #[test]
    fn the_download_bar_names_how_much_has_arrived() {
        assert_eq!(progress_text(12_400_000, 36_000_000), "12.4 MB of 36.0 MB");
        assert_eq!(progress_text(0, 0), "0 B so far");
    }
}
