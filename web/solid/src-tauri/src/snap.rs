//! Windows 11 snap layouts for the page's own maximise button (ADR-0054).
//!
//! Windows shows the snap-layout flyout only to a window that answers `WM_NCHITTEST` with
//! `HTMAXBUTTON`, and the page cannot: WebView2's child window covers the whole client
//! area. So a small transparent child window of the main window is kept exactly over the
//! page's maximise button, above the webview. It answers `HTMAXBUTTON`, which is all
//! Windows needs to show the flyout, and because it covers the button it receives the
//! click too and maximises or restores the window itself.
//!
//! The page owns the button and tells us where it is (`set_snap_bounds`, physical pixels
//! relative to the window's client area). The overlay owns nothing but the hit test, so it
//! sends the page a `snap-hover` event (`true` on enter, `false` on leave): the button never
//! sees the mouse, so it cannot style its own `:hover`.
//!
//! The shape follows `tauri-plugin-snap-layout` (MIT, Hyph-M and others), read in
//! `references/`; the code here is our own. There is one window (ADR-0048), so the hover
//! flag and the app handle are plain statics rather than per-window state.

use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, OnceLock};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowExW, GetParent, IsZoomed, LoadCursorW, RegisterClassExW,
    SendMessageW, SetCursor, SetWindowPos, HTMAXBUTTON, HWND_TOP, IDC_ARROW, SC_MAXIMIZE, SC_RESTORE,
    SWP_NOACTIVATE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_SETCURSOR,
    WM_SYSCOMMAND, WNDCLASSEXW, WS_CHILD, WS_VISIBLE,
};

const CLASS_NAME: *const u16 = windows_sys::w!("SeulaSnapMaximize");

static REGISTER: Once = Once::new();
static HOVERING: AtomicBool = AtomicBool::new(false);
static APP: OnceLock<AppHandle> = OnceLock::new();

/// Place the overlay over the maximise button, creating it the first time.
pub fn set_bounds(window: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    let parent = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let _ = APP.set(window.app_handle().clone());
    // Window messages are delivered on the thread that created the window, so create it,
    // and move it, on the main thread.
    window
        .run_on_main_thread(move || unsafe {
            let parent = parent as HWND;
            register_class();
            let mut overlay = FindWindowExW(parent, null_mut(), CLASS_NAME, null());
            if overlay.is_null() {
                overlay = CreateWindowExW(
                    0, CLASS_NAME, null(), WS_CHILD | WS_VISIBLE, x, y, width, height, parent, null_mut(),
                    GetModuleHandleW(null()), null(),
                );
            }
            if !overlay.is_null() {
                // The webview is a sibling and may have been raised since; stay above it.
                SetWindowPos(overlay, HWND_TOP, x, y, width, height, SWP_NOACTIVATE);
            }
        })
        .map_err(|e| e.to_string())
}

unsafe fn register_class() {
    REGISTER.call_once(|| {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(overlay_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null()),
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: CLASS_NAME,
            hIconSm: null_mut(),
        };
        RegisterClassExW(&class);
    });
}

fn emit_hover(hovering: bool) {
    if let Some(app) = APP.get() {
        let _ = app.emit("snap-hover", hovering);
    }
}

unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            if !HOVERING.swap(true, Ordering::Relaxed) {
                emit_hover(true);
                // Ask for WM_NCMOUSELEAVE, which is how we learn the cursor left.
                let mut track = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE | TME_NONCLIENT,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut track);
            }
            HTMAXBUTTON as LRESULT
        }
        WM_NCMOUSELEAVE => {
            if HOVERING.swap(false, Ordering::Relaxed) {
                emit_hover(false);
            }
            0
        }
        // Swallow the press so Windows does not start its own caption-button tracking.
        WM_NCLBUTTONDOWN if wparam == HTMAXBUTTON as usize => 0,
        WM_NCLBUTTONUP if wparam == HTMAXBUTTON as usize => {
            let window = GetParent(hwnd);
            let command = if IsZoomed(window) != 0 { SC_RESTORE } else { SC_MAXIMIZE };
            SendMessageW(window, WM_SYSCOMMAND, command as WPARAM, 0);
            0
        }
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(null_mut(), IDC_ARROW));
            1
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
