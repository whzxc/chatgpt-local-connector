#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(not(target_os = "windows"))]
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(not(target_os = "macos"))]
use tauri::Emitter;
use tauri::Manager;
#[cfg(target_os = "windows")]
pub use windows::install;

#[derive(Default)]
struct PanelInteraction(AtomicBool);
struct PanelHeight(AtomicU32);
#[cfg(not(target_os = "windows"))]
pub fn install(app: &tauri::App) -> tauri::Result<()> {
    app.manage(PanelInteraction::default());
    app.manage(PanelHeight(AtomicU32::new(100)));
    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/tray-logo.png"))
        .icon_as_template(false)
        .tooltip("Local Connector")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                position,
                rect,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = toggle(tray.app_handle(), position, rect) {
                    eprintln!("Tray panel: {error}");
                }
            }
        })
        .build(app)?;
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let app = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if app.state::<PanelInteraction>().0.load(Ordering::Relaxed) {
                    return;
                }
                #[cfg(target_os = "macos")]
                macos::retire_if_hidden(&app);
                #[cfg(not(target_os = "macos"))]
                if let Some(w) = app.get_webview_window("tray-panel") {
                    if w.is_visible() == Ok(false) {
                        let _ = w.destroy();
                    }
                }
            });
        }
    });
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn create_panel(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "tray-panel",
        tauri::WebviewUrl::App("tray-panel.html".into()),
    )
    .title("Local Connector — Usage")
    .inner_size(330., 100.)
    .visible(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .build()?;
    window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)))?;
    #[cfg(target_os = "macos")]
    macos::install(&window)?;
    let handle = window.clone();
    window.on_window_event(move |event| {
        if !cfg!(target_os = "macos")
            && matches!(event, tauri::WindowEvent::Focused(false))
            && !handle
                .app_handle()
                .state::<PanelInteraction>()
                .0
                .load(Ordering::Relaxed)
        {
            // Let the tray click own toggling while the pointer is over its icon.
            // Blur may arrive before or after that click; neither should consume a later click.
            if pointer_on_icon(&handle) {
                return;
            }
            let _ = hide(&handle);
        }
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide(&handle);
        }
    });
    Ok(window)
}
#[cfg(not(target_os = "windows"))]
fn pointer_on_icon(window: &tauri::WebviewWindow) -> bool {
    if let (Ok(point), Some(tray)) = (
        window.cursor_position(),
        window.app_handle().tray_by_id("main-tray"),
    ) {
        if let Ok(Some(rect)) = tray.rect() {
            let scale = window.scale_factor().unwrap_or(1.);
            let pos = rect.position.to_physical::<f64>(scale);
            let size = rect.size.to_physical::<f64>(scale);
            return point.x >= pos.x
                && point.x <= pos.x + size.width
                && point.y >= pos.y
                && point.y <= pos.y + size.height;
        }
    }
    false
}
fn hide(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::hide(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.hide()
    }
}
#[cfg(not(target_os = "windows"))]
fn toggle(
    app: &tauri::AppHandle,
    point: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) -> tauri::Result<()> {
    let window = match app.get_webview_window("tray-panel") {
        Some(window) => window,
        None => {
            create_panel(app)?;
            return Ok(());
        }
    };
    #[cfg(target_os = "macos")]
    {
        macos::toggle(&window, point, rect)
    }
    #[cfg(not(target_os = "macos"))]
    {
        if window.is_visible()? {
            return hide(&window);
        }
        let side = position_panel(&window, point, rect)?;
        window.emit("tray-panel:open", serde_json::json!({"side":side}))?;
        window.show()?;
        window.set_focus()
    }
}
fn position_panel(
    window: &tauri::WebviewWindow,
    point: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) -> tauri::Result<&'static str> {
    let monitors = window.available_monitors()?;
    let monitor = monitors.iter().find(|m| {
        let p = m.position();
        let s = m.size();
        point.x >= p.x as f64
            && point.x < p.x as f64 + s.width as f64
            && point.y >= p.y as f64
            && point.y < p.y as f64 + s.height as f64
    });
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        // Share the web popover with the browser; reserve a transparent side canvas.
        let margin = 14. * scale;
        let available_width = (area.size.width as f64 - 2. * margin).max(1.);
        let available_height = (area.size.height as f64 - 2. * margin).max(1.);
        let panel_width = (330. * scale).min(available_width);
        let width = (640. * scale).min(available_width);
        let height = (f64::from(
            window
                .app_handle()
                .state::<PanelHeight>()
                .0
                .load(Ordering::Relaxed),
        ) * scale)
            .max(448. * scale)
            .min(704. * scale)
            .min(available_height);
        let left = area.position.x as f64 + margin;
        let top = area.position.y as f64 + margin;
        let right = left + available_width;
        let side = if point.x > left + area.size.width as f64 / 2. {
            "left"
        } else {
            "right"
        };
        let panel_x = (point.x - panel_width / 2.).clamp(left, right - panel_width);
        let x = if side == "left" {
            panel_x + panel_width - width
        } else {
            panel_x
        }
        .clamp(left, right - width);
        let icon_pos = rect.position.to_physical::<f64>(scale);
        let icon_size = rect.size.to_physical::<f64>(scale);
        let y =
            (icon_pos.y + icon_size.height + margin).clamp(top, top + available_height - height);
        window.set_size(tauri::PhysicalSize::new(width as u32, height as u32))?;
        window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))?;
        return Ok(side);
    }
    Ok("left")
}
#[tauri::command]
pub async fn tray_panel_resize(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != "tray-panel" || !height.is_finite() || height <= 0. {
        return Err("invalid panel size".into());
    }
    window
        .app_handle()
        .state::<PanelHeight>()
        .0
        .store(height.ceil().clamp(1., 704.) as u32, Ordering::Relaxed);
    let tray = window
        .app_handle()
        .tray_by_id("main-tray")
        .ok_or("missing tray")?;
    if let Some(rect) = tray.rect().map_err(|e| e.to_string())? {
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let pos = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        position_panel(
            &window,
            tauri::PhysicalPosition::new(pos.x + size.width / 2., pos.y + size.height / 2.),
            rect,
        )
        .map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        macos::resize(&window).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub fn tray_action(window: tauri::WebviewWindow, action: String) -> Result<(), String> {
    if window.label() != "tray-panel" {
        return Err("invalid window".into());
    }
    let app = window.app_handle();
    match action.as_str() {
        "ready" => {
            #[cfg(not(target_os = "windows"))]
            if let Some(rect) = app
                .tray_by_id("main-tray")
                .and_then(|t| t.rect().ok().flatten())
            {
                let scale = window.scale_factor().map_err(|e| e.to_string())?;
                let point = rect.position.to_physical::<f64>(scale);
                toggle(app, point, rect).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        "menu-open" => {
            app.state::<PanelInteraction>()
                .0
                .store(true, Ordering::Relaxed);
            Ok(())
        }
        "menu-close" => {
            app.state::<PanelInteraction>()
                .0
                .store(false, Ordering::Relaxed);
            #[cfg(target_os = "macos")]
            macos::menu_closed(&window).map_err(|e| e.to_string())?;
            #[cfg(not(target_os = "macos"))]
            if !window.is_focused().map_err(|e| e.to_string())? {
                hide(&window).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        "updates" => {
            hide(&window).map_err(|e| e.to_string())?;
            crate::windows::show(app, Some(("updates:check".into(), serde_json::Value::Null)))
        }
        "hide" => hide(&window).map_err(|e| e.to_string()),
        "quit" => {
            app.exit(0);
            Ok(())
        }
        "overview" | "settings" | "logs" | "tasks" => {
            hide(&window).map_err(|e| e.to_string())?;
            crate::windows::show(app, Some(("navigate".into(), serde_json::json!(action))))
        }
        _ => Err("invalid action".into()),
    }
}
