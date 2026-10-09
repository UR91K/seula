//! The Tauri shell (ADR-0048): a window around the web page, plus the OS calls.
//!
//! The Tauri side holds no state and never proxies the API. Its one request is `/health`,
//! to start the daemon when nothing answers (ADR-0069). Try the command from the
//! webview's devtools console:
//!
//! ```js
//! __TAURI__.core.invoke('reveal_in_explorer', { path: 'C:\Windows\notepad.exe' })
//! ```
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod snap;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Reveal `path` selected in Explorer.
#[tauri::command]
fn reveal_in_explorer(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("{} does not exist", path.display()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `/select,<path>` is one argument and must not be quoted by std's escaping.
        // explorer.exe returns a nonzero exit code even on success, so only spawn.
        Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        let dir = path.parent().unwrap_or(&path);
        Command::new("xdg-open").arg(dir).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Keep the snap-layout overlay over the page's maximise button (ADR-0057). The rectangle is
/// in physical pixels, relative to the window's client area. A no-op off Windows.
#[tauri::command]
fn set_snap_bounds(window: tauri::WebviewWindow, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    #[cfg(windows)]
    return snap::set_bounds(&window, x, y, width, height);
    #[cfg(not(windows))]
    {
        let _ = (window, x, y, width, height);
        Ok(())
    }
}

/// The daemon's default `http_port` (src/config/defaults.rs), which the page also assumes
/// (web/shared/api.ts).
const DAEMON_PORT: u16 = 50052;
const DAEMON_BIN: &str = if cfg!(windows) { "seula.exe" } else { "seula" };
const DAEMON_START_TIMEOUT: Duration = Duration::from_secs(15);

/// Whether a Seula daemon answers `/health` on the default port. An HTTP request rather
/// than a port test, so another program on the port is not mistaken for the daemon.
fn daemon_is_up() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], DAEMON_PORT));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    if stream
        .write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut head = [0u8; 12];
    stream.read_exact(&mut head).is_ok() && head.starts_with(b"HTTP/1.") && &head[8..12] == b" 200"
}

/// Start the daemon from this executable's folder unless one already answers (ADR-0069).
/// In tray mode, so the user can see it running and quit it, and detached, so it outlives
/// this window (ADR-0048). Failures are logged and left for the page to show, as "Cannot
/// reach the daemon".
fn ensure_daemon() {
    if daemon_is_up() {
        return;
    }
    let Some(daemon) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(DAEMON_BIN)))
    else {
        eprintln!("seula-shell: cannot locate the daemon next to this executable");
        return;
    };
    if !daemon.is_file() {
        eprintln!("seula-shell: no daemon at {}", daemon.display());
        return;
    }
    let mut cmd = Command::new(&daemon);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    if let Err(e) = cmd.spawn() {
        eprintln!("seula-shell: could not start {}: {e}", daemon.display());
        return;
    }
    let deadline = Instant::now() + DAEMON_START_TIMEOUT;
    while Instant::now() < deadline {
        if daemon_is_up() {
            return;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    eprintln!("seula-shell: the daemon did not answer within {DAEMON_START_TIMEOUT:?}");
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            ensure_daemon();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![reveal_in_explorer, set_snap_bounds])
        .run(tauri::generate_context!())
        .expect("error while running the Seula shell");
}
