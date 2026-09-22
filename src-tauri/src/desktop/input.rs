//! A dedicated message thread owns the low-level hook. No DLL injection, no elevation.
//! Only a press on our fresh, visible sprite over the actual desktop is consumed.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};
struct Region {
    x: f64,
    y: f64,
    r: f64,
    updated: Instant,
}
static REGION: std::sync::LazyLock<Mutex<std::collections::HashMap<String, Region>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static DOWN: AtomicBool = AtomicBool::new(false);
static CAPTURED: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);
static RIGHT: AtomicBool = AtomicBool::new(false);
pub fn publish(owner: &str, x: f64, y: f64, r: f64) {
    if let Ok(mut hit) = REGION.lock() {
        hit.retain(|_, r| r.updated.elapsed() < Duration::from_secs(2));
        hit.insert(
            owner.to_owned(),
            Region {
                x,
                y,
                r,
                updated: Instant::now(),
            },
        );
    }
}
pub fn left() -> bool {
    DOWN.load(Ordering::Relaxed)
}
pub fn cancelled() -> bool {
    CANCEL.swap(false, Ordering::Relaxed)
}
unsafe extern "system" fn callback(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: Windows guarantees MSLLHOOKSTRUCT for HC_ACTION, valid only during callback.
    unsafe {
        if code == HC_ACTION as i32 {
            let m = &*(l.0 as *const MSLLHOOKSTRUCT);
            if m.flags & LLMHF_INJECTED == 0 {
                match w.0 as u32 {
                    WM_LBUTTONDOWN => {
                        DOWN.store(true, Ordering::Relaxed);
                        let hit = REGION
                            .try_lock()
                            .ok()
                            .map(|regions| {
                                regions.values().any(|r| {
                                    r.updated.elapsed() < Duration::from_millis(180)
                                        && ((m.pt.x as f64 - r.x).hypot(m.pt.y as f64 - r.y) < r.r)
                                })
                            })
                            .unwrap_or(false);
                        let under = WindowFromPoint(m.pt);
                        let mut name = [0u16; 64];
                        let n = GetClassNameW(under, &mut name);
                        let class = String::from_utf16_lossy(&name[..n as usize]);
                        let root = GetAncestor(under, GA_ROOT);
                        let mut root_name = [0u16; 64];
                        let count = GetClassNameW(root, &mut root_name);
                        let root_class = String::from_utf16_lossy(&root_name[..count as usize]);
                        if hit
                            && ["Progman", "WorkerW"].contains(&root_class.as_str())
                            && ["SysListView32", "WorkerW", "Progman"].contains(&class.as_str())
                        {
                            CAPTURED.store(true, Ordering::Relaxed);
                            return LRESULT(1);
                        }
                    }
                    WM_LBUTTONUP => {
                        DOWN.store(false, Ordering::Relaxed);
                        if CAPTURED.swap(false, Ordering::Relaxed) {
                            return LRESULT(1);
                        }
                    }
                    WM_RBUTTONDOWN if CAPTURED.load(Ordering::Relaxed) => {
                        CANCEL.store(true, Ordering::Relaxed);
                        RIGHT.store(true, Ordering::Relaxed);
                        return LRESULT(1);
                    }
                    WM_RBUTTONUP if RIGHT.swap(false, Ordering::Relaxed) => return LRESULT(1),

                    _ => {}
                }
            }
        }
        CallNextHookEx(None, code, w, l)
    }
}
pub struct Guard {
    thread: u32,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Guard {
    pub fn start() -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let join = std::thread::spawn(move || {
            // SAFETY: hook and message queue are created and destroyed on this dedicated thread.
            unsafe {
                let mut message = MSG::default();
                let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                let thread = GetCurrentThreadId();
                match SetWindowsHookExW(WH_MOUSE_LL, Some(callback), None, 0) {
                    Ok(hook) => {
                        let _ = tx.send(Ok(thread));
                        while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
                        let _ = UnhookWindowsHookEx(hook);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.code()));
                    }
                }
            }
        });
        match rx.recv() {
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
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}
