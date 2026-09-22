use tauri::{Emitter, Manager};
#[derive(serde::Serialize)]
pub struct Display {
    pub id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub x: i32,
    pub y: i32,
}
pub fn list(app: &tauri::AppHandle) -> Result<Vec<Display>, String> {
    Ok(app
        .available_monitors()
        .map_err(|_| "monitors")?
        .iter()
        .enumerate()
        .map(|(i, m)| Display {
            id: m.name().cloned().unwrap_or_else(|| format!("display-{i}")),
            label: format!(
                "显示器 {} · {} × {} · {}%",
                i + 1,
                m.size().width,
                m.size().height,
                (m.scale_factor() * 100.).round()
            ),
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
            x: m.position().x,
            y: m.position().y,
        })
        .collect())
}
pub fn signature(app: &tauri::AppHandle) -> String {
    let monitors = serde_json::to_string(&list(app).unwrap_or_default()).unwrap_or_default();
    // SAFETY: querying Explorer identity only; borrowed handle is not modified.
    let pid = unsafe {
        let mut pid = 0;
        if let Ok(p) =
            windows::Win32::UI::WindowsAndMessaging::FindWindowW(windows::core::w!("Progman"), None)
        {
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(p, Some(&mut pid));
        }
        pid
    };
    format!("{pid}:{}:{monitors}", super::host_signature())
}
pub fn rebuild(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    state.operations.cancel();
    let _ = app.emit("cancelled", ());
    let selected = state.config.lock().map_err(|_| "state")?.monitor.clone();
    for (label, w) in app.webview_windows() {
        if label == "overlay" || label.starts_with("overlay-") {
            w.destroy().map_err(|_| "overlay_destroy")?;
        }
    }
    let monitors = app.available_monitors().map_err(|_| "monitors")?;
    let primary = app.primary_monitor().map_err(|_| "primary_monitor")?;
    let mut chosen: Vec<_> = monitors
        .iter()
        .filter(|m| {
            selected == "all"
                || if selected == "primary" {
                    primary
                        .as_ref()
                        .is_some_and(|p| p.position() == m.position())
                } else {
                    m.name().is_some_and(|n| n == &selected)
                }
        })
        .collect();
    if chosen.is_empty() {
        if let Some(m) = monitors.first() {
            chosen.push(m);
        }
    }
    for (i, m) in chosen.into_iter().enumerate() {
        let label = if i == 0 {
            "overlay".to_owned()
        } else {
            format!("overlay-{i}")
        };
        let layer = tauri::WebviewWindowBuilder::new(
            app,
            label,
            tauri::WebviewUrl::App("index.html?overlay".into()),
        )
        .title("Pulsar desktop layer")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|_| "overlay_create")?;
        layer
            .set_ignore_cursor_events(true)
            .map_err(|_| "click_through")?;
        let hwnd = windows::Win32::Foundation::HWND(layer.hwnd().map_err(|_| "hwnd")?.0);
        let attach_mode = super::attach(hwnd).ok();
        let attached = attach_mode.is_some();
        let mut point = windows::Win32::Foundation::POINT {
            x: m.position().x,
            y: m.position().y,
        };
        // SAFETY: newly-created app window is live. Attached child position is converted from screen physical coordinates to its parent's client coordinates.
        unsafe {
            if attached {
                let parent = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd)
                    .map_err(|_| "parent")?;
                windows::Win32::Graphics::Gdi::ScreenToClient(parent, &mut point)
                    .ok()
                    .map_err(|_| "desktop_origin")?;
            }
            windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                Some(if attached {
                    windows::Win32::UI::WindowsAndMessaging::HWND_TOP
                } else {
                    windows::Win32::UI::WindowsAndMessaging::HWND_BOTTOM
                }),
                point.x,
                point.y,
                m.size().width as i32,
                m.size().height as i32,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
            )
            .map_err(|_| "overlay_layout")?;
        }
        if !attached {
            super::bottom(hwnd).map_err(|_| "fallback")?;
        }
        layer.show().map_err(|_| "overlay_show")?;
        log::info!(
            "overlay attach_mode={} display={i}",
            attach_mode.unwrap_or("fallback")
        );
    }
    Ok(())
}
pub fn maintain(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label != "overlay" && !label.starts_with("overlay-") {
            continue;
        }
        let Ok(raw) = window.hwnd() else { continue };
        let hwnd = windows::Win32::Foundation::HWND(raw.0);
        // SAFETY: the handle belongs to this app. Parent and iconic state are read only.
        unsafe {
            if windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd)
                .map(|parent| parent.is_invalid())
                .unwrap_or(true)
            {
                let _ = super::bottom(hwnd);
            }
        }
    }
}
