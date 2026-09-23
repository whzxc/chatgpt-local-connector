use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

#[derive(Default)]
struct PanelInteraction(AtomicBool);
pub fn install(app: &tauri::App) -> tauri::Result<()> {
    app.manage(PanelInteraction::default());
    crate::tray_detail::install(app)?;
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "tray-panel",
        tauri::WebviewUrl::App("tray-panel.html".into()),
    )
    .title("Local Connector — Usage")
    .inner_size(380., 704.)
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
    let skip_click = Arc::new(AtomicBool::new(false));
    let focus_skip = skip_click.clone();
    let handle = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Focused(false))
            && !crate::tray_detail::detail_visible()
            && !handle
                .app_handle()
                .state::<PanelInteraction>()
                .0
                .load(Ordering::Relaxed)
        {
            // Clicking the tray icon can blur the panel before its mouse-up event.
            if let (Ok(point), Some(tray)) = (
                handle.cursor_position(),
                handle.app_handle().tray_by_id("main-tray"),
            ) {
                if let Ok(Some(rect)) = tray.rect() {
                    let scale = handle.scale_factor().unwrap_or(1.);
                    let pos = rect.position.to_physical::<f64>(scale);
                    let size = rect.size.to_physical::<f64>(scale);
                    focus_skip.store(
                        point.x >= pos.x
                            && point.x <= pos.x + size.width
                            && point.y >= pos.y
                            && point.y <= pos.y + size.height,
                        Ordering::Relaxed,
                    );
                }
            }
            crate::tray_detail::dismiss(handle.app_handle());
            let _ = handle.hide();
        }
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            crate::tray_detail::dismiss(handle.app_handle());
            let _ = handle.hide();
        }
    });
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
                if skip_click.swap(false, Ordering::Relaxed) {
                    return;
                }
                if let Err(error) = toggle(tray.app_handle(), position, rect) {
                    eprintln!("Tray panel: {error}");
                }
            }
        })
        .build(app)?;
    Ok(())
}
fn toggle(
    app: &tauri::AppHandle,
    point: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("tray-panel") else {
        return Ok(());
    };
    if window.is_visible()? {
        crate::tray_detail::dismiss(app);
        return window.hide();
    }
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
        let width = (380. * scale).min(area.size.width as f64);
        let height = (704. * scale).min(area.size.height as f64);
        let left = area.position.x as f64;
        let top = area.position.y as f64;
        let right = left + area.size.width as f64;
        let side = if point.x > left + area.size.width as f64 / 2. {
            "left"
        } else {
            "right"
        };
        let x = (point.x - width / 2.).clamp(left, right - width);
        let icon_pos = rect.position.to_physical::<f64>(scale);
        let icon_size = rect.size.to_physical::<f64>(scale);
        let y = (icon_pos.y + icon_size.height).clamp(top, top + area.size.height as f64 - height);
        window.set_size(tauri::PhysicalSize::new(width as u32, height as u32))?;
        window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))?;
        window.emit("tray-panel:open", serde_json::json!({"side":side}))?;
    }
    window.show()?;
    window.set_focus()
}
#[tauri::command]
pub fn tray_action(window: tauri::WebviewWindow, action: String) -> Result<(), String> {
    if window.label() != "tray-panel" {
        return Err("invalid window".into());
    }
    let app = window.app_handle();
    crate::tray_detail::dismiss(app);
    match action.as_str() {
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
            if !window.is_focused().map_err(|e| e.to_string())? {
                window.hide().map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        "updates" => {
            window.hide().map_err(|e| e.to_string())?;
            crate::show_main_window(app);
            app.emit_to("main", "updates:check", ())
                .map_err(|e| e.to_string())
        }
        "hide" => window.hide().map_err(|e| e.to_string()),
        "quit" => {
            app.exit(0);
            Ok(())
        }
        "overview" | "settings" | "logs" | "tasks" => {
            window.hide().map_err(|e| e.to_string())?;
            crate::show_main_window(app);
            app.emit_to("main", "navigate", action)
                .map_err(|e| e.to_string())
        }
        _ => Err("invalid action".into()),
    }
}
