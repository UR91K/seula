//! The Tauri shell (ADR-0048): a window around the web page, plus the OS calls.
//!
//! The Tauri side holds no state and never touches the API. Try the command from the
//! webview's devtools console:
//!
//! ```js
//! __TAURI__.core.invoke('reveal_in_explorer', { path: 'C:\Windows\notepad.exe' })
//! ```
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![reveal_in_explorer])
        .run(tauri::generate_context!())
        .expect("error while running the Seula shell");
}
