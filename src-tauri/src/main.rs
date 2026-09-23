#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod desktop;
mod logging;
mod operations;
mod skins;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};
struct AppState {
    config: Mutex<config::Config>,
    operations: operations::Operations,
    overlay_visible: AtomicBool,
}
fn only(window: &tauri::WebviewWindow, label: &str) -> Result<(), String> {
    if window.label() == label || (label == "overlay" && window.label().starts_with("overlay-")) {
        Ok(())
    } else {
        Err("window_not_authorized".into())
    }
}
#[tauri::command]
fn pointer() -> desktop::Result<desktop::Pointer> {
    desktop::pointer()
}
#[tauri::command]
async fn probe(window: tauri::WebviewWindow) -> Result<Option<desktop::shell::Target>, String> {
    only(&window, "overlay")?;
    let p = desktop::pointer().map_err(|_| "pointer_unavailable")?;
    let target = tauri::async_runtime::spawn_blocking(move || desktop::shell::hit(p.x, p.y))
        .await
        .map_err(|_| "shell_worker")?
        .map_err(|e| format!("{}:{}", e.stage, e.code))?;
    Ok(target)
}
#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<config::Config, String> {
    state
        .config
        .lock()
        .map(|c| c.clone())
        .map_err(|_| "state_unavailable".into())
}
#[tauri::command]
fn save_config(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    value: config::Config,
) -> Result<(), String> {
    only(&window, "settings")?;
    value.validate()?;
    let dir = app.path().app_data_dir().map_err(|_| "appdata")?;
    let mut config = state.config.lock().map_err(|_| "state_unavailable")?;
    let display_changed = config.monitor != value.monitor;
    config::autostart(value.autostart)?;
    config::save(&dir, &value)?;
    *config = value.clone();
    drop(config);
    if display_changed {
        let handle = app.clone();
        app.run_on_main_thread(move || {
            if let Err(code) = desktop::monitors::rebuild(&handle) {
                log::error!("display_rebuild {code}");
            }
        })
        .map_err(|_| "layout_schedule")?;
    }
    state.operations.cancel();
    app.emit("config", value).map_err(|_| "event_failed")?;
    Ok(())
}
#[tauri::command]
fn cancel(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    state.operations.cancel();
    app.emit("cancelled", ()).map_err(|_| "event_failed".into())
}
#[tauri::command]
async fn drop_recycle(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> Result<operations::Outcome, String> {
    only(&window, "overlay")?;
    if !app
        .state::<AppState>()
        .config
        .lock()
        .map(|config| config.deletion)
        .unwrap_or(false)
    {
        return Err("deletion_disabled".into());
    }
    let pointer = desktop::pointer().map_err(|_| "pointer_unavailable")?;
    if pointer.left || pointer.cancel {
        return Err("release_required".into());
    }
    let target =
        tauri::async_runtime::spawn_blocking(move || desktop::shell::hit(pointer.x, pointer.y))
            .await
            .map_err(|_| "shell_worker")?
            .map_err(|_| "target_unresolved")?
            .ok_or("no_target")?;
    let result = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || app.state::<AppState>().operations.recycle_target(target)
    })
    .await
    .map_err(|_| "worker_failed")?;
    let _ = app.emit("operation-result", &result);
    Ok(result)
}
fn main() {
    if std::env::args().any(|a| a == "--diagnose") {
        match desktop::shell::diagnose() {
            Ok(d) => println!("{}", serde_json::to_string(&d).unwrap()),
            Err(e) => {
                eprintln!("diagnostic failed: {e:?}");
                std::process::exit(1)
            }
        }
        return;
    }

    let _input_guard = desktop::input::Guard::start().expect("input hook unavailable");
    tauri::Builder::default()
        .setup(|app| {
            if std::env::args().any(|a| a == "--smoke") {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(8));
                    app.exit(0);
                });
            }
            let dir = app.path().app_data_dir()?;
            logging::init(&dir)?;
            app.manage(AppState {
                config: Mutex::new(config::load(&dir)),
                operations: Default::default(),
                overlay_visible: AtomicBool::new(true),
            });
            desktop::monitors::rebuild(app.handle()).map_err(std::io::Error::other)?;
            let lifecycle = app.handle().clone();
            std::thread::spawn(move || {
                let mut signature = desktop::monitors::signature(&lifecycle);
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let next = desktop::monitors::signature(&lifecycle);
                    if next != signature {
                        signature = next;
                        let handle = lifecycle.clone();
                        let _ = lifecycle.run_on_main_thread(move || {
                            if let Err(code) = desktop::monitors::rebuild(&handle) {
                                log::error!("lifecycle_rebuild {code}");
                            }
                        });
                    } else {
                        let handle = lifecycle.clone();
                        let _ = lifecycle
                            .run_on_main_thread(move || desktop::monitors::maintain(&handle));
                    }
                }
            });
            use tauri::menu::{Menu, MenuItem};
            let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "暂停 / 继续", true, None::<&str>)?;
            let visible = MenuItem::with_id(app, "visible", "显示 / 隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &pause, &visible, &quit])?;
            let pixels = [115u8, 239, 255, 255].repeat(32 * 32);
            tauri::tray::TrayIconBuilder::new()
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("Juno Pulsar Desktop")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        if let Ok(mut c) = app.state::<AppState>().config.lock() {
                            c.paused = !c.paused;
                            let _ = app.emit("config", c.clone());
                        }
                    }
                    "visible" => {
                        let visible = !app
                            .state::<AppState>()
                            .overlay_visible
                            .fetch_xor(true, Ordering::SeqCst);
                        for (label, window) in app.webview_windows() {
                            if label == "overlay" || label.starts_with("overlay-") {
                                if visible {
                                    let _ = window.show();
                                } else {
                                    let _ = window.hide();
                                }
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            if std::env::args().any(|s| s == "--background") {
                if let Some(w) = app.get_webview_window("settings") {
                    w.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if w.label() == "settings" {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_displays,
            list_skins,
            import_skin,
            load_skin,
            publish_sprites,
            input_frame,
            pointer,
            probe,
            get_config,
            save_config,
            drop_recycle,
            cancel
        ])
        .run(tauri::generate_context!())
        .expect("application runtime failed");
}
#[derive(serde::Serialize)]
struct Frame {
    x: i32,
    y: i32,
    left: bool,
    cancel: bool,
    captured: Option<u32>,
    origin_x: i32,
    origin_y: i32,
    scale: f64,
    fullscreen: bool,
}
#[tauri::command]
fn input_frame(window: tauri::WebviewWindow) -> Result<Frame, String> {
    only(&window, "overlay")?;
    let p = desktop::pointer().map_err(|_| "pointer")?;
    let (accepted, rejected) = desktop::input::drain_diagnostics();
    if accepted > 0 {
        log::info!("input capture accepted count={accepted}");
    }
    if rejected > 0 {
        log::warn!("input capture surface_rejected count={rejected}");
    }
    let hwnd = windows::Win32::Foundation::HWND(window.hwnd().map_err(|_| "hwnd")?.0);
    let mut origin = windows::Win32::Foundation::POINT::default();
    let mut fullscreen = false;
    // SAFETY: live app hwnd and writable point/rect; foreground handle is only queried.
    unsafe {
        windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut origin)
            .ok()
            .map_err(|_| "origin")?;
        let fg = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        let mut rect = windows::Win32::Foundation::RECT::default();
        if let Ok(Some(m)) = window.current_monitor() {
            if windows::Win32::UI::WindowsAndMessaging::GetWindowRect(fg, &mut rect).is_ok() {
                fullscreen = rect.left <= m.position().x
                    && rect.top <= m.position().y
                    && rect.right >= m.position().x + m.size().width as i32
                    && rect.bottom >= m.position().y + m.size().height as i32;
                let mut class = [0u16; 128];
                let n = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(fg, &mut class);
                let class = String::from_utf16_lossy(&class[..n as usize]);
                if class == "Progman" || class == "WorkerW" {
                    fullscreen = false;
                }
            }
        }
    }
    Ok(Frame {
        x: p.x,
        y: p.y,
        left: p.left,
        cancel: p.cancel,
        captured: desktop::input::captured(window.label()),
        origin_x: origin.x,
        origin_y: origin.y,
        scale: window.scale_factor().map_err(|_| "dpi")?,
        fullscreen,
    })
}

#[derive(serde::Deserialize)]
struct SpriteRegion {
    index: u32,
    x: f64,
    y: f64,
    radius: f64,
}

#[tauri::command]
fn publish_sprites(window: tauri::WebviewWindow, regions: Vec<SpriteRegion>) -> Result<(), String> {
    only(&window, "overlay")?;
    if regions.len() > 10
        || regions.iter().any(|region| {
            region.index >= 10
                || !region.x.is_finite()
                || !region.y.is_finite()
                || !region.radius.is_finite()
                || !(0.0..=512.).contains(&region.radius)
        })
    {
        return Err("invalid_hit_region".into());
    }
    if window.is_visible().unwrap_or(false) {
        let hwnd = windows::Win32::Foundation::HWND(window.hwnd().map_err(|_| "hwnd")?.0);
        desktop::input::publish(
            window.label(),
            hwnd,
            regions
                .into_iter()
                .map(|region| desktop::input::HitRegion {
                    index: region.index,
                    x: region.x,
                    y: region.y,
                    radius: region.radius,
                })
                .collect(),
        );
    }
    Ok(())
}

#[tauri::command]
fn list_displays(app: tauri::AppHandle) -> Result<Vec<desktop::monitors::Display>, String> {
    desktop::monitors::list(&app)
}
#[tauri::command]
fn list_skins(app: tauri::AppHandle) -> Result<Vec<skins::Manifest>, String> {
    Ok(skins::list(
        &app.path()
            .app_data_dir()
            .map_err(|_| "appdata")?
            .join("skins"),
    ))
}
#[tauri::command]
async fn import_skin(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    folder: bool,
) -> Result<Option<skins::Manifest>, String> {
    only(&window, "settings")?;
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| "appdata")?
        .join("skins");
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = rfd::FileDialog::new().set_title("导入本地皮肤");
        let selected = if folder {
            dialog.pick_folder()
        } else {
            dialog
                .add_filter("Juno Pulsar Skin", &["jpskin"])
                .pick_file()
        };
        selected.map(|p| skins::import(&p, &root)).transpose()
    })
    .await
    .map_err(|_| "skin_worker")?
}
#[tauri::command]
async fn load_skin(app: tauri::AppHandle, id: String) -> Result<skins::Bundle, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| "appdata")?
        .join("skins");
    tauri::async_runtime::spawn_blocking(move || skins::load(&root, &id))
        .await
        .map_err(|_| "skin_worker")?
}
