#[cfg(target_os = "macos")]
mod macos;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Default)]
struct DetailState(Mutex<Option<serde_json::Value>>);

pub fn install(app: &tauri::App, _primary: &tauri::WebviewWindow) -> tauri::Result<()> {
    app.manage(DetailState::default());
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        "tray-detail",
        tauri::WebviewUrl::App("tray-detail.html".into()),
    )
    .title("Local Connector — Detail")
    .inner_size(250., 396.)
    .visible(false)
    .focused(false)
    .focusable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible_on_all_workspaces(true);
    // An owned Windows popup stays above its panel without taking keyboard focus.
    #[cfg(target_os = "windows")]
    let builder = builder.owner(_primary)?;
    let window = builder.build()?;
    #[cfg(target_os = "macos")]
    macos::install(&window)?;
    #[cfg(not(target_os = "macos"))]
    let _ = window;
    Ok(())
}

pub fn dismiss(app: &tauri::AppHandle) {
    closed(app);
    #[cfg(target_os = "macos")]
    macos::close(app);
}

pub fn detail_visible() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::shown()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

fn closed(app: &tauri::AppHandle) {
    let previous = app.state::<DetailState>().0.lock().unwrap().take();
    if let Some(window) = app.get_webview_window("tray-detail") {
        let _ = window.hide();
    }
    if let Some(value) = previous {
        let _ = app.emit_to(
            "tray-panel",
            "tray-detail:hover",
            serde_json::json!({"owner":value["owner"],"inside":false,"dismiss":true}),
        );
    }
}

#[tauri::command]
pub fn tray_detail(
    window: tauri::WebviewWindow,
    action: String,
    mut payload: serde_json::Value,
) -> Result<(), String> {
    let app = window.app_handle();
    let primary = app
        .get_webview_window("tray-panel")
        .ok_or("missing panel")?;
    let detail = app
        .get_webview_window("tray-detail")
        .ok_or("missing detail")?;
    let state = app.state::<DetailState>();
    match (window.label(), action.as_str()) {
        ("tray-panel", "show") => {
            payload["nativeChrome"] = serde_json::json!(cfg!(target_os = "macos"));
            if !primary.is_visible().map_err(|e| e.to_string())? {
                return Ok(());
            }
            let mut current = state.0.lock().unwrap();
            if let Some(old) = current
                .as_ref()
                .filter(|old| old["owner"] != payload["owner"])
            {
                app.emit_to(
                    "tray-panel",
                    "tray-detail:hover",
                    serde_json::json!({"owner":old["owner"],"inside":false,"dismiss":true}),
                )
                .map_err(|e| e.to_string())?;
            }
            *current = Some(payload.clone());
            detail
                .emit("tray-detail:content", payload)
                .map_err(|e| e.to_string())
        }
        ("tray-panel", "hide") => {
            let owned = state.0.lock().unwrap().as_ref().is_some_and(|v| {
                v["owner"] == payload["owner"]
                    && (payload["request"].is_null() || v["request"] == payload["request"])
            });
            if owned {
                dismiss(app);
            }
            Ok(())
        }
        ("tray-detail", "hover") => {
            if let Some(current) = state.0.lock().unwrap().as_ref() {
                app.emit_to(
                    "tray-panel",
                    "tray-detail:hover",
                    serde_json::json!({"owner":current["owner"],"inside":payload["inside"]}),
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        ("tray-detail", "ready") => {
            let current = state.0.lock().unwrap().clone();
            let Some(data) = current.as_ref() else {
                return Ok(());
            };
            if data["request"] != payload["request"]
                || !primary.is_visible().map_err(|e| e.to_string())?
            {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            {
                macos::show(
                    &primary,
                    data["anchor"].clone(),
                    payload["height"].as_f64().unwrap_or(396.),
                    payload["dark"].as_bool().unwrap_or(false),
                    payload["animate"].as_bool().unwrap_or(true),
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                let monitor = primary
                    .current_monitor()
                    .map_err(|e| e.to_string())?
                    .ok_or("missing monitor")?;
                let scale = monitor.scale_factor();
                let area = monitor.work_area();
                let pos = primary.inner_position().map_err(|e| e.to_string())?;
                let a = &data["anchor"];
                let number = |key: &str| a[key].as_f64().filter(|n| n.is_finite()).unwrap_or(0.);
                let ax = pos.x as f64 + number("x") * scale;
                let ay = pos.y as f64 + (number("y") + number("height") / 2.) * scale;
                let w = (250. * scale).min(area.size.width as f64);
                let h = (payload["height"].as_f64().unwrap_or(396.).clamp(50., 432.) * scale)
                    .min(area.size.height as f64);
                let left = area.position.x as f64;
                let top = area.position.y as f64;
                let right = left + area.size.width as f64;
                let x = (ax + number("width") * scale / 2. - w / 2.).clamp(left, right - w);
                let y = (ay - number("height") * scale / 2. - h - 8. * scale)
                    .clamp(top, top + area.size.height as f64 - h);
                detail
                    .set_size(tauri::PhysicalSize::new(w as u32, h as u32))
                    .map_err(|e| e.to_string())?;
                detail
                    .set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
                    .map_err(|e| e.to_string())?;
                detail.show().map_err(|e| e.to_string())
            }
        }
        _ => Err("invalid detail action".into()),
    }
}
