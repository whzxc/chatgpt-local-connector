use crate::i18n::t;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter,
};

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", status_text("unknown"), false, None::<&str>)?;
    let actions = [
        ("overview", "trayOpen"),
        ("tasks", "trayTasks"),
        ("logs", "trayLogs"),
        ("settings", "traySettings"),
        ("quit", "trayQuit"),
    ];
    let menu = Menu::new(app)?;
    menu.append(&status)?;
    let mut items = Vec::new();
    for (id, key) in actions {
        let item = MenuItem::with_id(app, id, t(key), true, None::<&str>)?;
        menu.append(&item)?;
        items.push((item, key));
    }
    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/tray-logo.png"))
        .icon_as_template(false)
        .tooltip("Local Connector")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                crate::show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "overview" | "tasks" | "logs" | "settings" => {
                crate::show_main_window(app);
                if let Err(error) = app.emit_to("main", "navigate", event.id.as_ref()) {
                    eprintln!("Tray navigation: {error}");
                }
            }
            _ => {}
        })
        .build(app)?;

    // Keep the native menu current even while the main WebView is hidden.
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let result = tokio::time::timeout(
                Duration::from_secs(4),
                crate::request(&handle, "status", "GET", serde_json::json!({})),
            )
            .await;
            let key = match result {
                Ok(Ok(value)) => match value["connection"]["running"].as_bool() {
                    Some(true) => "trayConnected",
                    Some(false) => "trayDisconnected",
                    None => "unknown",
                },
                _ => "unknown",
            };
            let _ = status.set_text(status_text(key));
            for (item, key) in &items {
                let _ = item.set_text(t(key));
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    Ok(())
}

fn status_text(key: &str) -> String {
    format!("{}: {}", t("trayConnectionStatus"), t(key))
}
