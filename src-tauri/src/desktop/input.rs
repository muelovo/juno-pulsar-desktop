//! A dedicated message thread owns the low-level hook. No DLL injection, no elevation.
//! Only a press on a fresh sprite hit region over the desktop/our click-through layer is consumed.
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        LazyLock, Mutex,
    },
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy)]
pub struct HitRegion {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

struct PublishedRegions {
    overlay: HWND,
    sprites: Vec<HitRegion>,
    updated: Instant,
}

#[derive(Clone)]
struct Capture {
    owner: String,
    index: u32,
}

// HWND is an opaque value. Access remains synchronized and the handle is only queried while
// its owning overlay publishes fresh regions.
unsafe impl Send for PublishedRegions {}

static REGIONS: LazyLock<Mutex<HashMap<String, PublishedRegions>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CAPTURE: LazyLock<Mutex<Option<Capture>>> = LazyLock::new(|| Mutex::new(None));
static DOWN: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);
static RIGHT: AtomicBool = AtomicBool::new(false);
static ACCEPTED: AtomicU64 = AtomicU64::new(0);
static SURFACE_REJECTED: AtomicU64 = AtomicU64::new(0);

pub fn publish(owner: &str, overlay: HWND, sprites: Vec<HitRegion>) {
    if let Ok(mut regions) = REGIONS.lock() {
        regions.retain(|_, entry| entry.updated.elapsed() < Duration::from_secs(2));
        regions.insert(
            owner.to_owned(),
            PublishedRegions {
                overlay,
                sprites,
                updated: Instant::now(),
            },
        );
    }
}

pub fn left() -> bool {
    DOWN.load(Ordering::Relaxed)
}

pub fn captured(owner: &str) -> Option<u32> {
    CAPTURE
        .lock()
        .ok()?
        .as_ref()
        .filter(|capture| capture.owner == owner)
        .map(|capture| capture.index)
}

pub fn cancelled() -> bool {
    CANCEL.swap(false, Ordering::Relaxed)
}

pub fn drain_diagnostics() -> (u64, u64) {
    (
        ACCEPTED.swap(0, Ordering::AcqRel),
        SURFACE_REJECTED.swap(0, Ordering::AcqRel),
    )
}

fn class_name(window: HWND) -> String {
    let mut name = [0u16; 128];
    // SAFETY: the stack buffer is valid for the duration of the read-only window query.
    let count = unsafe { GetClassNameW(window, &mut name) };
    String::from_utf16_lossy(&name[..count as usize])
}

fn is_desktop_surface(point: POINT, overlay: HWND) -> bool {
    // SAFETY: all calls are read-only HWND/geometry queries. Stale handles simply fail closed.
    unsafe {
        let under = WindowFromPoint(point);
        if under == overlay || IsChild(overlay, under).as_bool() {
            return true;
        }

        let root = GetAncestor(under, GA_ROOT);
        let root_class = class_name(root);
        if ["CabinetWClass", "ExploreWClass", "Shell_TrayWnd"].contains(&root_class.as_str()) {
            return false;
        }

        let progman = match FindWindowW(w!("Progman"), None) {
            Ok(window) => window,
            Err(_) => return false,
        };
        let mut desktop = HWND::default();
        let _ = EnumWindows(
            Some(find_desktop_list),
            LPARAM((&mut desktop as *mut HWND) as isize),
        );
        if desktop.is_invalid() {
            desktop = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None)
                .ok()
                .and_then(|view| FindWindowExW(Some(view), None, w!("SysListView32"), None).ok())
                .unwrap_or_default();
        }
        if desktop.is_invalid() {
            return false;
        }

        let mut rect = RECT::default();
        if GetWindowRect(desktop, &mut rect).is_err()
            || !windows::Win32::Graphics::Gdi::PtInRect(
                &raw const rect,
                POINT {
                    x: point.x,
                    y: point.y,
                },
            )
            .as_bool()
        {
            return false;
        }

        let mut desktop_pid = 0;
        let mut under_pid = 0;
        GetWindowThreadProcessId(desktop, Some(&mut desktop_pid));
        GetWindowThreadProcessId(under, Some(&mut under_pid));
        desktop_pid != 0 && desktop_pid == under_pid
    }
}

unsafe extern "system" fn find_desktop_list(window: HWND, context: LPARAM) -> BOOL {
    // SAFETY: EnumWindows invokes synchronously and context points to a live caller-owned HWND.
    unsafe {
        if let Ok(view) = FindWindowExW(Some(window), None, w!("SHELLDLL_DefView"), None) {
            if let Ok(list) = FindWindowExW(Some(view), None, w!("SysListView32"), None) {
                *(context.0 as *mut HWND) = list;
                return FALSE;
            }
        }
    }
    TRUE
}

fn hit(point: POINT) -> Option<(String, u32, HWND)> {
    let regions = REGIONS.try_lock().ok()?;
    regions
        .iter()
        .filter(|(_, published)| published.updated.elapsed() < Duration::from_millis(250))
        .flat_map(|(owner, published)| {
            published.sprites.iter().filter_map(move |sprite| {
                let distance = (point.x as f64 - sprite.x).hypot(point.y as f64 - sprite.y);
                (distance < sprite.radius).then_some((
                    distance,
                    owner.clone(),
                    sprite.index,
                    published.overlay,
                ))
            })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, owner, index, overlay)| (owner, index, overlay))
}

unsafe extern "system" fn callback(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    // SAFETY: Windows guarantees MSLLHOOKSTRUCT for HC_ACTION, valid only during callback.
    unsafe {
        if code == HC_ACTION as i32 {
            let mouse = &*(data.0 as *const MSLLHOOKSTRUCT);
            if mouse.flags & LLMHF_INJECTED == 0 {
                match message.0 as u32 {
                    WM_LBUTTONDOWN => {
                        DOWN.store(true, Ordering::Release);
                        if let Some((owner, index, overlay)) = hit(mouse.pt) {
                            if is_desktop_surface(mouse.pt, overlay) {
                                if let Ok(mut capture) = CAPTURE.try_lock() {
                                    *capture = Some(Capture { owner, index });
                                    ACCEPTED.fetch_add(1, Ordering::Relaxed);
                                    return LRESULT(1);
                                }
                            } else {
                                SURFACE_REJECTED.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    WM_LBUTTONUP => {
                        DOWN.store(false, Ordering::Release);
                        if CAPTURE
                            .try_lock()
                            .ok()
                            .and_then(|mut capture| capture.take())
                            .is_some()
                        {
                            return LRESULT(1);
                        }
                    }
                    WM_RBUTTONDOWN
                        if CAPTURE
                            .try_lock()
                            .ok()
                            .is_some_and(|capture| capture.is_some()) =>
                    {
                        CANCEL.store(true, Ordering::Release);
                        RIGHT.store(true, Ordering::Release);
                        return LRESULT(1);
                    }
                    WM_RBUTTONUP if RIGHT.swap(false, Ordering::AcqRel) => return LRESULT(1),
                    _ => {}
                }
            }
        }
        CallNextHookEx(None, code, message, data)
    }
}

pub struct Guard {
    thread: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Guard {
    pub fn start() -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let join = std::thread::spawn(move || {
            // SAFETY: hook and message queue are created and destroyed on this dedicated thread.
            unsafe {
                let mut message = MSG::default();
                let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                let thread = GetCurrentThreadId();
                match SetWindowsHookExW(WH_MOUSE_LL, Some(callback), None, 0) {
                    Ok(hook) => {
                        let _ = sender.send(Ok(thread));
                        while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
                        let _ = UnhookWindowsHookEx(hook);
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error.code()));
                    }
                }
            }
        });
        match receiver.recv() {
            Ok(Ok(thread)) => Ok(Self {
                thread,
                join: Some(join),
            }),
            Ok(Err(code)) => {
                let _ = join.join();
                Err(Error::from_hresult(code))
            }
            Err(_) => Err(Error::from_hresult(E_FAIL)),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: this id belongs to the live owned message thread.
        unsafe {
            let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_the_specific_sprite_that_was_pressed() {
        let owner = "input-region-test";
        publish(
            owner,
            HWND(std::ptr::dangling_mut()),
            vec![
                HitRegion {
                    index: 0,
                    x: 100.0,
                    y: 100.0,
                    radius: 30.0,
                },
                HitRegion {
                    index: 1,
                    x: 300.0,
                    y: 200.0,
                    radius: 30.0,
                },
                HitRegion {
                    index: 2,
                    x: 500.0,
                    y: 400.0,
                    radius: 30.0,
                },
            ],
        );
        assert_eq!(hit(POINT { x: 300, y: 200 }).map(|hit| hit.1), Some(1));
        assert_eq!(hit(POINT { x: 500, y: 400 }).map(|hit| hit.1), Some(2));
        assert!(hit(POINT { x: 700, y: 700 }).is_none());
    }
}
