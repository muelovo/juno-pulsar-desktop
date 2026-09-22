use serde::Serialize;
use windows::{
    core::*,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

#[derive(Debug, Serialize)]
pub struct NativeError {
    pub stage: &'static str,
    pub code: String,
}
impl NativeError {
    pub fn at(stage: &'static str, e: windows::core::Error) -> Self {
        log::warn!("native stage={stage} code={:?}", e.code());
        Self {
            stage,
            code: format!("{:?}", e.code()),
        }
    }
}
pub type Result<T> = std::result::Result<T, NativeError>;

pub fn attach(hwnd: HWND) -> Result<&'static str> {
    // SAFETY: hwnd is a live window owned by this process. Explorer windows are only borrowed.
    unsafe {
        let progman =
            FindWindowW(w!("Progman"), None).map_err(|e| NativeError::at("find_progman", e))?;
        let mut response = 0;
        let sent = SendMessageTimeoutW(
            progman,
            0x052c,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            800,
            Some(&mut response),
        );
        if sent.0 == 0 {
            return Err(NativeError::at("workerw_timeout", Error::from_win32()));
        }
        let mut desktop_host = HWND::default();
        EnumWindows(
            Some(find_desktop_host),
            LPARAM((&mut desktop_host as *mut HWND) as isize),
        )
        .map_err(|e| NativeError::at("enum_windows", e))?;
        let (parent, mode) = if desktop_host.is_invalid() {
            (progman, "progman")
        } else {
            (desktop_host, "desktop_host")
        };
        SetParent(hwnd, Some(parent)).map_err(|e| NativeError::at("set_parent", e))?;
        let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetLastError(WIN32_ERROR(0));
        let old_extended = SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            extended | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize,
        );
        if old_extended == 0 && GetLastError().0 != 0 {
            let _ = SetParent(hwnd, None);
            return Err(NativeError::at("desktop_ex_style", Error::from_win32()));
        }
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetLastError(WIN32_ERROR(0));
        let old = SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (style & !(WS_POPUP.0 as isize)) | WS_CHILD.0 as isize,
        );
        if old == 0 && GetLastError().0 != 0 {
            let _ = SetParent(hwnd, None);
            return Err(NativeError::at("child_style", Error::from_win32()));
        }
        Ok(mode)
    }
}
unsafe extern "system" fn find_desktop_host(window: HWND, context: LPARAM) -> BOOL {
    // SAFETY: EnumWindows invokes synchronously; context points to the caller's valid HWND.
    unsafe {
        if !(*(context.0 as *mut HWND)).is_invalid() {
            return TRUE;
        }
        if FindWindowExW(Some(window), None, w!("SHELLDLL_DefView"), None).is_ok() {
            *(context.0 as *mut HWND) = window;
        }
    }
    TRUE
}
pub fn host_signature() -> isize {
    let mut desktop_host = HWND::default();
    // SAFETY: EnumWindows is synchronous and the callback only stores a borrowed HWND value.
    unsafe {
        let _ = EnumWindows(
            Some(find_desktop_host),
            LPARAM((&mut desktop_host as *mut HWND) as isize),
        );
    }
    desktop_host.0 as isize
}
pub fn bottom(hwnd: HWND) -> Result<()> {
    // SAFETY: only positioning our live window; no external window is modified.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .map_err(|e| NativeError::at("bottom_fallback", e))
    }
}

#[derive(Serialize)]
pub struct Pointer {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub cancel: bool,
}
pub fn pointer() -> Result<Pointer> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let mut p = POINT::default();
    // SAFETY: p is writable; GetAsyncKeyState receives documented virtual keys and installs no hooks.
    unsafe {
        GetCursorPos(&mut p).map_err(|e| NativeError::at("cursor", e))?;
        Ok(Pointer {
            x: p.x,
            y: p.y,
            left: input::left(),
            cancel: input::cancelled()
                || GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0
                || GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0,
        })
    }
}
pub mod input;
pub mod shell;

pub mod monitors;
