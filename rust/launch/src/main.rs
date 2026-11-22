#![windows_subsystem = "windows"]

mod update;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const PORT: u16 = 4731;

fn main() {
    if let Err(error) = run() {
        alert("ValDex", &error);
    }
}

fn run() -> Result<(), String> {
    if let Some(update) = update::available() {
        let question = format!("Version {} is ready.\n\nInstall it now?", update.version);
        if ask("ValDex", &question) {
            match update::install(&update) {
                Ok(()) => return Ok(()),
                Err(error) => alert("ValDex", &format!("The update did not download. ValDex will open this version.\n\n{error}")),
            }
        }
    }
    let root = exe_dir()?;
    let player = root.join("vivi-player.exe");
    let node = root.join("node.exe");
    let server = root.join("server");
    if !player.is_file() {
        return Err("The ValDex player is missing from this install.".into());
    }
    let started = if port_open() {
        None
    } else {
        Some(start_server(&node, &server)?)
    };
    if !wait_for_port(Duration::from_secs(90)) {
        if let Some(child) = &started {
            stop_server(child);
        }
        return Err("ValDex did not start. The notes are in %APPDATA%\\Vivi\\server.log.".into());
    }
    let _ = Command::new(&player).current_dir(&root).status();
    if let Some(child) = &started {
        stop_server(child);
    }
    Ok(())
}

fn start_server(node: &Path, server: &Path) -> Result<Child, String> {
    if !node.is_file() {
        return Err("This ValDex install is missing node.exe.".into());
    }
    if !server.join("server.js").is_file() {
        return Err("This ValDex install is missing its catalog server.".into());
    }
    let log_path = app_data()?.join("server.log");
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut log = OpenOptions::new().create(true).append(true).open(&log_path).map_err(|error| error.to_string())?;
    let _ = writeln!(log, "---- starting ValDex server ----");
    let log_err = log.try_clone().map_err(|error| error.to_string())?;
    Command::new(node)
        .arg("server.js")
        .current_dir(server)
        .env("PORT", PORT.to_string())
        .env("HOSTNAME", "127.0.0.1")
        .env("NODE_ENV", "production")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("The catalog server did not start. {error}"))
}

fn stop_server(child: &Child) {
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

fn port_open() -> bool {
    TcpStream::connect(("127.0.0.1", PORT)).is_ok()
}

fn wait_for_port(limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if port_open() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    port_open()
}

fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    exe.parent().map(Path::to_path_buf).ok_or_else(|| "ValDex could not find its folder.".into())
}

fn app_data() -> Result<PathBuf, String> {
    let root = std::env::var_os("APPDATA").ok_or_else(|| "Windows did not provide an AppData folder.".to_string())?;
    Ok(PathBuf::from(root).join("Vivi"))
}

fn ask(title: &str, text: &str) -> bool {
    let title = wide(title);
    let text = wide(text);
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x00000024) == 6 }
}

fn alert(title: &str, text: &str) {
    let title = wide(title);
    let text = wide(text);
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x00000010);
    }
}

fn wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}
