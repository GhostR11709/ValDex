#![cfg_attr(windows, windows_subsystem = "windows")]

mod update;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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
    let player = root.join(player_name());
    let node = root.join(node_name());
    let server = root.join("server");
    if !player.is_file() {
        return Err("The ValDex player is missing from this install.".into());
    }
    let mut started = if port_open() {
        None
    } else {
        Some(start_server(&node, &server)?)
    };
    if !wait_for_port(Duration::from_secs(90)) {
        if let Some(child) = started.as_mut() {
            stop_server(child);
        }
        let log = app_data().map(|dir| dir.join("server.log")).ok();
        let notes = log.map(|path| path.display().to_string()).unwrap_or_else(|| "the ValDex log".into());
        return Err(format!("ValDex did not start. The notes are in {notes}."));
    }
    let _ = Command::new(&player).current_dir(&root).status();
    if let Some(child) = started.as_mut() {
        stop_server(child);
    }
    Ok(())
}

fn player_name() -> &'static str {
    if cfg!(windows) { "vivi-player.exe" } else { "vivi-player" }
}

fn node_name() -> &'static str {
    if cfg!(windows) { "node.exe" } else { "node" }
}

fn start_server(node: &Path, server: &Path) -> Result<Child, String> {
    if !node.is_file() {
        return Err(format!("This ValDex install is missing {}.", node_name()));
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
    let mut command = Command::new(node);
    command
        .arg("server.js")
        .current_dir(server)
        .env("PORT", PORT.to_string())
        .env("HOSTNAME", "127.0.0.1")
        .env("NODE_ENV", "production")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    hide(&mut command).spawn().map_err(|error| format!("The catalog server did not start. {error}"))
}

fn stop_server(child: &mut Child) {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &child.id().to_string(), "/T", "/F"]);
        let _ = hide(&mut command).status();
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }
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
    #[cfg(windows)]
    {
        let root = std::env::var_os("APPDATA").ok_or_else(|| "Windows did not provide an AppData folder.".to_string())?;
        return Ok(PathBuf::from(root).join("Vivi"));
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| "macOS did not provide a home folder.".to_string())?;
        return Ok(PathBuf::from(home).join("Library/Application Support/Vivi"));
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("ValDex does not know where to keep its files on this system.".into())
    }
}

#[cfg(windows)]
fn hide(command: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000)
}

#[cfg(not(windows))]
fn hide(command: &mut Command) -> &mut Command {
    command
}

fn ask(title: &str, text: &str) -> bool {
    #[cfg(windows)]
    {
        let title = wide(title);
        let text = wide(text);
        return unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x00000024) == 6 };
    }
    #[cfg(not(windows))]
    {
        let script = format!("display dialog {text} with title {title} buttons {{\"Later\", \"Install\"}} default button \"Install\"", text = apple(text), title = apple(title));
        let Ok(output) = Command::new("osascript").args(["-e", &script]).output() else { return false };
        String::from_utf8_lossy(&output.stdout).contains("Install")
    }
}

fn alert(title: &str, text: &str) {
    #[cfg(windows)]
    {
        let title = wide(title);
        let text = wide(text);
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x00000010);
        }
    }
    #[cfg(not(windows))]
    {
        let script = format!("display alert {title} message {text}", title = apple(title), text = apple(text));
        let _ = Command::new("osascript").args(["-e", &script]).status();
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(not(windows))]
fn apple(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}
