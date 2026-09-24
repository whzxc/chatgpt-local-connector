#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use connector_core::service::Service;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::Manager;
#[cfg(target_os = "macos")]
mod appearance;
mod desktop_access;
mod i18n;
mod tray;
mod updates;
mod usage_panel;
#[cfg(target_os = "windows")]
mod windows_frame;

async fn request(
    app: &tauri::AppHandle,
    route: &str,
    method: &str,
    body: Value,
) -> Result<Value, String> {
    if method != "GET" {
        desktop_access::check_write(app)?;
    }
    #[cfg(target_os = "macos")]
    if route == "appearance" {
        return appearance::request(app, method, body).await;
    }
    if route == "usage-panel/geometry" && method == "POST" {
        usage_panel::geometry(app, body).await?;
        return Ok(json!({"accepted":true}));
    }
    if route == "usage-panel/ready" && method == "POST" {
        usage_panel::ready(app);
        return Ok(json!({"ready":true}));
    }
    if route == "usage-panel" && ["GET", "PUT"].contains(&method) {
        let operation = if method == "GET" {
            connector_core::transport::DesktopRequest::PanelGet
        } else {
            connector_core::transport::DesktopRequest::PanelSet(body)
        };
        return desktop_access::request(app, operation).await;
    }
    if route == "subscriptions/open" && method == "POST" {
        return desktop_access::request(
            app,
            connector_core::transport::DesktopRequest::SubscriptionsOpen(body),
        )
        .await;
    }
    if cfg!(debug_assertions) {
        return connector_core::transport::forward_request(route, method, body).await;
    }
    app.state::<Arc<Service>>()
        .inner()
        .clone()
        .request(route, method, body)
        .await
}
#[tauri::command]
async fn service_request(
    app: tauri::AppHandle,
    route: String,
    method: String,
    body: Value,
) -> Result<Value, String> {
    request(&app, &route, &method, body).await
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
fn main() {
    if let Some(state_dir) = std::env::args()
        .skip_while(|arg| arg != "--state-dir")
        .nth(1)
    {
        std::env::set_var("CLC_STATE_DIR", state_dir);
    }
    if std::env::args().nth(1).as_deref() == Some("cli") {
        let runtime = tokio::runtime::Runtime::new().expect("CLI runtime");
        std::process::exit(
            runtime.block_on(connector_core::cli::run(std::env::args().skip(2).collect())),
        );
    }
    if std::env::args().nth(1).as_deref() == Some("stdio") {
        let runtime = tokio::runtime::Runtime::new().expect("MCP runtime");
        if let Err(error) = runtime.block_on(connector_core::transport::stdio()) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .manage(updates::UpdateState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["usage-rail", "tray-panel"])
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        & !(tauri_plugin_window_state::StateFlags::VISIBLE
                            | tauri_plugin_window_state::StateFlags::DECORATIONS),
                )
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            tray::tray_action,
            tray::tray_panel_resize,
            service_request,
            i18n::set_ui_locale,
            updates::check_update,
            updates::install_update,
            updates::download_update,
            updates::cancel_update,
            #[cfg(target_os = "windows")]
            windows_frame::set_windows_appearance
        ])
        .setup(|app| {
            if !cfg!(debug_assertions) {
                let service = Service::new().map_err(std::io::Error::other)?;
                app.manage(service.clone());
                tauri::async_runtime::block_on(connector_core::transport::listen(
                    service.clone(),
                    Some(Arc::new(desktop_access::Owner(app.handle().clone()))),
                ))
                .map_err(std::io::Error::other)?;
                let resume = updates::take_resume();
                if resume.is_some() || std::env::args().any(|arg| arg == "--autostart") {
                    tauri::async_runtime::spawn(async move {
                        if let Some(ids) = resume {
                            for id in ids {
                                if let Err(error) = service
                                    .request(&format!("ingress/{id}/start"), "POST", json!({}))
                                    .await
                                {
                                    service.log("ERROR", &error).await;
                                }
                            }
                        } else if let Err(error) = service
                            .request("ingress/start-all", "POST", json!({}))
                            .await
                        {
                            service.log("ERROR", &error).await;
                        }
                    });
                }
            }
            if let Some(window) = app.get_webview_window("main") {
                // Clear the live WebView backing as well as the window configuration.
                // On macOS this disables WKWebView's own opaque background.
                window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)))?;
            }
            usage_panel::install(app.handle());
            tray::install(app)?;
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                windows_frame::install(&window)?;
            }
            #[cfg(target_os = "macos")]
            appearance::install(app).map_err(std::io::Error::other)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::WindowEvent::ThemeChanged(_)) {
                if let Err(error) = appearance::update_dock_icon(window.app_handle()) {
                    eprintln!("{error}");
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !cfg!(debug_assertions)
                    || window
                        .app_handle()
                        .get_webview_window("usage-rail")
                        .is_some()
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Local Connector 启动失败")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                usage_panel::close();
                // The updater already drained the service before replacing the
                // app. Do not block the event loop on a second async shutdown.
                if !app
                    .state::<updates::UpdateState>()
                    .restarting
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    if let Some(service) = app.try_state::<Arc<Service>>() {
                        tauri::async_runtime::block_on(service.stop()).ok();
                    }
                }
                let metadata = connector_core::root().join("web/native.json");
                if connector_core::load(&metadata).is_ok_and(|v| v["pid"] == std::process::id()) {
                    let _ = std::fs::remove_file(metadata);
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
