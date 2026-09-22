//! Desktop hit testing uses a ListView index and the *same* Shell view's PIDL.
//! No display text, OCR, filename joining, shortcut resolution or process injection.
use super::{NativeError, Result};
use serde::Serialize;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{Com::*, Diagnostics::Debug::*, Memory::*, Threading::*, Variant::VARIANT},
        UI::{Controls::*, Shell::*, WindowsAndMessaging::*},
    },
};

struct Apartment;
impl Apartment {
    fn enter() -> windows::core::Result<Self> {
        // SAFETY: this function runs on a dedicated short-lived thread, with no existing apartment.
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: balances successful initialization on this thread.
        unsafe { CoUninitialize() }
    }
}
struct Pidl(*mut Common::ITEMIDLIST);
impl Drop for Pidl {
    fn drop(&mut self) {
        // SAFETY: Shell allocated PIDLs use the COM task allocator.
        unsafe { CoTaskMemFree(Some(self.0.cast())) }
    }
}
struct Remote {
    process: HANDLE,
    address: *mut core::ffi::c_void,
}
impl Drop for Remote {
    fn drop(&mut self) {
        // SAFETY: allocation and handle belong to this RAII object; no message remains pending after successful synchronous send.
        unsafe {
            let _ = VirtualFreeEx(self.process, self.address, 0, MEM_RELEASE);
            let _ = CloseHandle(self.process);
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub path: String,
    pub name: String,
    pub kind: String,
}

static UNHEALTHY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn hit(x: i32, y: i32) -> Result<Option<Target>> {
    if UNHEALTHY.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(NativeError {
            stage: "shell_disabled_after_timeout",
            code: "restart_required".into(),
        });
    }
    resolve(x, y).map_err(|e| NativeError::at("desktop_hit", e))
}
fn resolve(x: i32, y: i32) -> windows::core::Result<Option<Target>> {
    let _apartment = Apartment::enter()?;
    // SAFETY: all COM interfaces remain within this apartment. Out parameters and remote
    // LVHITTESTINFO storage have documented sizes; pointer lifetimes cover each synchronous call.
    unsafe {
        let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
        let mut unused = 0;
        let dispatch = shell.FindWindowSW(
            &VARIANT::from(CSIDL_DESKTOP as i32),
            &VARIANT::default(),
            SWC_DESKTOP,
            &mut unused,
            SWFO_NEEDDISPATCH,
        )?;
        let provider: IServiceProvider = dispatch.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;
        let folder: IFolderView = view.cast()?;
        let host = view.GetWindow()?;
        let list = FindWindowExW(Some(host), None, w!("SysListView32"), None)?;
        let screen = POINT { x, y };
        // A foreground app, menu, or taskbar covering the icon must never target the desktop beneath it.
        let actual = WindowFromPoint(screen);
        if actual != list {
            return Ok(None);
        }
        let mut local = screen;
        windows::Win32::Graphics::Gdi::ScreenToClient(list, &mut local).ok()?;
        let mut pid = 0;
        GetWindowThreadProcessId(list, Some(&mut pid));
        let process = OpenProcess(
            PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )?;
        let address = VirtualAllocEx(
            process,
            None,
            std::mem::size_of::<LVHITTESTINFO>(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if address.is_null() {
            let _ = CloseHandle(process);
            return Err(Error::from_win32());
        }
        let memory = Remote { process, address };
        let mut info = LVHITTESTINFO {
            pt: local,
            ..Default::default()
        };
        WriteProcessMemory(
            process,
            address,
            (&info as *const LVHITTESTINFO).cast(),
            std::mem::size_of::<LVHITTESTINFO>(),
            None,
        )?;
        let mut index = 0usize;
        if SendMessageTimeoutW(
            list,
            LVM_HITTEST,
            WPARAM(0),
            LPARAM(address as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            300,
            Some(&mut index),
        )
        .0 == 0
        {
            // A timed-out receiver could still use the buffer. Deliberately retain this tiny remote
            // allocation until Explorer exits; closing the process handle is safe. No retry loop.
            UNHEALTHY.store(true, std::sync::atomic::Ordering::Relaxed);
            std::mem::forget(memory);
            let _ = CloseHandle(process);
            return Err(Error::from_hresult(HRESULT(0x800705B4u32 as i32)));
        }
        ReadProcessMemory(
            process,
            address,
            (&mut info as *mut LVHITTESTINFO).cast(),
            std::mem::size_of::<LVHITTESTINFO>(),
            None,
        )?;
        if info.iItem < 0 || (info.flags & (LVHT_ONITEMICON | LVHT_ONITEMLABEL)).0 == 0 {
            return Ok(None);
        }
        let pidl = Pidl(folder.Item(info.iItem)?);
        let parent: IShellFolder = folder.GetFolder()?;
        let item: IShellItem = SHCreateItemWithParent(None, &parent, pidl.0)?;
        let raw = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path_result = raw.to_string();
        CoTaskMemFree(Some(raw.0.cast()));
        let path = path_result?;
        // Reject a changing view: identity at this exact index must still match.
        let second = Pidl(folder.Item(info.iItem)?);
        if !ILIsEqual(pidl.0, second.0).as_bool() {
            return Ok(None);
        }
        let p = std::path::Path::new(&path);
        let name = p
            .file_name()
            .ok_or(Error::from_hresult(E_FAIL))?
            .to_string_lossy()
            .into_owned();
        let kind = if p.is_dir() {
            "文件夹"
        } else if p.extension().is_some_and(|s| s.eq_ignore_ascii_case("lnk")) {
            "快捷方式"
        } else {
            "文件"
        }
        .to_owned();
        Ok(Some(Target { path, name, kind }))
    }
}
#[derive(Serialize)]
pub struct Diagnostic {
    pub desktop_items: i32,
    pub list_view_found: bool,
    pub filesystem_items: u32,
}

pub fn diagnose() -> Result<Diagnostic> {
    let _apt = Apartment::enter().map_err(|e| NativeError::at("com_init", e))?;
    // SAFETY: local STA, output pointers remain owned and are freed immediately after use.
    unsafe {
        let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)
            .map_err(|e| NativeError::at("shell_windows", e))?;
        let mut unused = 0;
        let dispatch = shell
            .FindWindowSW(
                &VARIANT::from(CSIDL_DESKTOP as i32),
                &VARIANT::default(),
                SWC_DESKTOP,
                &mut unused,
                SWFO_NEEDDISPATCH,
            )
            .map_err(|e| NativeError::at("find_desktop", e))?;
        let provider: IServiceProvider = dispatch
            .cast()
            .map_err(|e| NativeError::at("service_provider", e))?;
        let browser: IShellBrowser = provider
            .QueryService(&SID_STopLevelBrowser)
            .map_err(|e| NativeError::at("shell_browser", e))?;
        let view = browser
            .QueryActiveShellView()
            .map_err(|e| NativeError::at("active_view", e))?;
        let folder: IFolderView = view.cast().map_err(|e| NativeError::at("folder_view", e))?;
        let host = view
            .GetWindow()
            .map_err(|e| NativeError::at("view_hwnd", e))?;
        let list_view_found = FindWindowExW(Some(host), None, w!("SysListView32"), None).is_ok();
        let desktop_items = folder
            .ItemCount(SVGIO_ALLVIEW)
            .map_err(|e| NativeError::at("item_count", e))?;
        let parent: IShellFolder = folder
            .GetFolder()
            .map_err(|e| NativeError::at("shell_folder", e))?;
        let mut filesystem_items = 0;
        for i in 0..desktop_items {
            let pidl = Pidl(
                folder
                    .Item(i)
                    .map_err(|e| NativeError::at("item_pidl", e))?,
            );
            let item: IShellItem = SHCreateItemWithParent(None, &parent, pidl.0)
                .map_err(|e| NativeError::at("shell_item", e))?;
            if let Ok(raw) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                filesystem_items += 1;
                CoTaskMemFree(Some(raw.0.cast()));
            }
        }
        Ok(Diagnostic {
            desktop_items,
            list_view_found,
            filesystem_items,
        })
    }
}
