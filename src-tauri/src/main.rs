#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod desktop;
mod logging;
mod operations;
mod skins;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
struct AppState {
    config: Mutex<config::Config>,
    operations: operations::Operations,
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
async fn probe(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<Option<desktop::shell::Target>, String> {
    only(&window, "overlay")?;
    let p = desktop::pointer().map_err(|_| "pointer_unavailable")?;
    let target = tauri::async_runtime::spawn_blocking(move || desktop::shell::hit(p.x, p.y))
        .await
        .map_err(|_| "shell_worker")?
        .map_err(|e| format!("{}:{}", e.stage, e.code))?;
    state.operations.observe(target.as_ref(), p.left);
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
async fn prepare(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<operations::Proposal, String> {
    only(&window, "overlay")?;
    if !state
        .config
        .lock()
        .map_err(|_| "state_unavailable")?
        .deletion
    {
        return Err("deletion_disabled".into());
    }
    let p = desktop::pointer().map_err(|_| "pointer_unavailable")?;
    if p.left || p.cancel {
        return Err("release_required".into());
    }
    let target = tauri::async_runtime::spawn_blocking(move || desktop::shell::hit(p.x, p.y))
        .await
        .map_err(|_| "shell_worker")?
        .map_err(|_| "target_unresolved")?
        .ok_or("no_target")?;
    let proposal = state.operations.prepare(target)?;
    if let Some(w) = app.get_webview_window("confirm") {
        w.show().map_err(|_| "confirm_show")?;
        w.set_focus().map_err(|_| "confirm_focus")?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "confirm",
            tauri::WebviewUrl::App("index.html?confirm".into()),
        )
        .title("确认移入回收站")
        .inner_size(540., 420.)
        .resizable(false)
        .always_on_top(true)
        .build()
        .map_err(|_| "confirm_create")?;
    }
    app.emit_to("confirm", "proposal", &proposal)
        .map_err(|_| "confirm_event")?;
    Ok(proposal)
}
#[tauri::command]
fn pending(
    window: tauri::WebviewWindow,
    state: tauri::State<AppState>,
) -> Result<Option<operations::Proposal>, String> {
    only(&window, "confirm")?;
    Ok(state.operations.current())
}
#[tauri::command]
async fn recycle(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    token: String,
) -> Result<operations::Outcome, String> {
    only(&window, "confirm")?;
    let result = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            let state = app.state::<AppState>();
            if !state.config.lock().map(|c| c.deletion).unwrap_or(false) {
                state.operations.cancel();
            }
            state.operations.execute(&token)
        }
    })
    .await
    .map_err(|_| "worker_failed")?;
    let _ = app.emit("operation-result", &result);
    Ok(result)
}
#[tauri::command]
fn cancel(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    state.operations.cancel();
    if window.label() == "confirm" {
        window.hide().map_err(|_| "hide_failed")?;
    }
    app.emit("cancelled", ()).map_err(|_| "event_failed".into())
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
                        if let Some(w) = app.get_webview_window("overlay") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
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
                if w.label() == "settings" || w.label() == "confirm" {
                    api.prevent_close();
                    let _ = w.hide();
                    if w.label() == "confirm" {
                        w.state::<AppState>().operations.cancel();
                        let _ = w.emit("cancelled", ());
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_displays,
            list_skins,
            import_skin,
            load_skin,
            publish_sprite,
            input_frame,
            pointer,
            probe,
            get_config,
            save_config,
            prepare,
            pending,
            recycle,
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
    origin_x: i32,
    origin_y: i32,
    scale: f64,
    fullscreen: bool,
}
#[tauri::command]
fn input_frame(window: tauri::WebviewWindow) -> Result<Frame, String> {
    only(&window, "overlay")?;
    let p = desktop::pointer().map_err(|_| "pointer")?;
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
        origin_x: origin.x,
        origin_y: origin.y,
        scale: window.scale_factor().map_err(|_| "dpi")?,
        fullscreen,
    })
}

#[tauri::command]
fn publish_sprite(window: tauri::WebviewWindow, x: f64, y: f64, r: f64) -> Result<(), String> {
    only(&window, "overlay")?;
    if !x.is_finite() || !y.is_finite() || !r.is_finite() || !(0.0..=512.).contains(&r) {
        return Err("invalid_hit_region".into());
    }
    if window.is_visible().unwrap_or(false) {
        desktop::input::publish(window.label(), x, y, r);
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
