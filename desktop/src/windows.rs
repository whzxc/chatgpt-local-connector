//! Main WebView lifetime and navigation while its frontend is being recreated.
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct MainState(Mutex<Main>);
#[derive(Default)]
struct Main {
    ready: bool,
    pending: Option<(String, Value)>,
}

pub fn show(app: &tauri::AppHandle, action: Option<(String, Value)>) -> Result<(), String> {
    let window = ensure(app)?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    let state = app.state::<MainState>();
    let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((event, payload)) = action {
        if state.ready {
            window.emit(&event, payload).map_err(|e| e.to_string())?;
        } else {
            state.pending = Some((event, payload));
        }
    }
    Ok(())
}

fn ensure(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .ok_or("missing main window configuration")?;
            let window = tauri::WebviewWindowBuilder::from_config(app, config)
                .map_err(|e| e.to_string())?
                .visible(false)
                .build()
                .map_err(|e| e.to_string())?;
            window
                .set_background_color(Some(tauri::window::Color(0, 0, 0, 0)))
                .map_err(|e| e.to_string())?;
            #[cfg(target_os = "windows")]
            crate::windows_frame::install(&window).map_err(|e| e.to_string())?;
            window
        }
    };
    Ok(window)
}

pub fn install(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            let handle = app.clone();
            // The existing updater owns its preferences, download and announcement
            // state in the main frontend. Wake it invisibly at its hourly cadence;
            // it retires again when idle, without duplicating updater ownership.
            let _ = app.run_on_main_thread(move || {
                if handle.get_webview_window("main").is_none()
                    && crate::desktop_access::check_write(&handle).is_ok()
                {
                    let _ = ensure(&handle);
                }
            });
        }
    });
}

#[tauri::command]
pub fn main_window_ready(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("invalid window".into());
    }
    let state = window.app_handle().state::<MainState>();
    let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
    state.ready = true;
    if let Some((event, payload)) = state.pending.take() {
        window.emit(&event, payload).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn release_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("invalid window".into());
    }
    let app = window.app_handle().clone();
    // The visibility check and destruction must be ordered with Dock/tray reopen.
    window
        .run_on_main_thread(move || {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(true)
                    || crate::desktop_access::check_write(&app).is_err()
                {
                    return;
                }
                if window.destroy().is_ok() {
                    *app.state::<MainState>()
                        .0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Main::default();
                }
            }
        })
        .map_err(|e| e.to_string())
}
