use crate::i18n::t;
use crate::{request, show_main_window};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter,
};

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let heading = MenuItem::with_id(
        app,
        "status",
        t("serviceLocalConnectorChecking"),
        false,
        None::<&str>,
    )?;
    let desktop = MenuItem::with_id(
        app,
        "desktop-status",
        t("serviceCodexChecking"),
        false,
        None::<&str>,
    )?;
    let verification = MenuItem::with_id(
        app,
        "verification",
        t("serviceChatgptChecking"),
        false,
        None::<&str>,
    )?;
    let connection = CheckMenuItem::with_id(
        app,
        "connection",
        t("serviceConnect"),
        false,
        false,
        None::<&str>,
    )?;
    let startup = CheckMenuItem::with_id(
        app,
        "startup",
        t("serviceConnectAtSystemSignIn"),
        false,
        false,
        None::<&str>,
    )?;
    let approval = CheckMenuItem::with_id(
        app,
        "approval",
        t("serviceTaskApprovalMode"),
        false,
        false,
        None::<&str>,
    )?;
    let tasks = MenuItem::with_id(app, "tasks", t("serviceTasks"), true, None::<&str>)?;
    let home = MenuItem::with_id(app, "overview", t("serviceOpenHome"), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", t("serviceSettings"), true, None::<&str>)?;
    let records = MenuItem::with_id(app, "logs", t("serviceRecords"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t("serviceQuitApp"), true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let separator3 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &heading,
            &desktop,
            &verification,
            &separator1,
            &connection,
            &startup,
            &approval,
            &separator2,
            &home,
            &tasks,
            &records,
            &settings,
            &separator3,
            &quit,
        ],
    )?;
    let busy = Arc::new(AtomicBool::new(false));
    let error = Arc::new(Mutex::new(None::<String>));
    let action_busy = busy.clone();
    let action_error = error.clone();
    let action_connection = connection.clone();
    let action_startup = startup.clone();
    let action_approval = approval.clone();
    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/tray-logo.png"))
        .icon_as_template(false)
        .tooltip("Local Connector")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            let action = event.id.as_ref();
            match action {
                "overview" | "settings" | "logs" | "tasks" => {
                    show_main_window(app);
                    let _ = app.emit("navigate", action);
                }
                "quit" => app.exit(0),
                "connection" | "startup" | "approval" => {
                    if action_busy.swap(true, Ordering::SeqCst) { return; }
                    let _ = action_connection.set_enabled(false);
                    let _ = action_startup.set_enabled(false);
                    let _ = action_approval.set_enabled(false);
                    let action = action.to_owned();
                    let app = app.clone();
                    let busy = action_busy.clone();
                    let error = action_error.clone();
                    tauri::async_runtime::spawn(async move {
                        let result: Result<Value, String> = async {
                            if action == "approval" {
                                let current = request(&app, "task-settings", "GET", json!({})).await?;
                                request(&app, "task-settings", "PUT", json!({"enabled": !current["enabled"].as_bool().unwrap_or(false)})).await
                            } else if action == "startup" {
                                let current = request(&app, "service", "GET", json!({})).await?;
                                request(&app, "service", "POST", json!({"enabled": !current["enabled"].as_bool().unwrap_or(false)})).await
                            } else {
                                let current = request(&app, "status", "GET", json!({})).await?;
                                let active = current["connection"]["running"].as_bool().unwrap_or(false);
                                request(&app, if active { "stop" } else { "start" }, "POST", json!({})).await
                            }
                        }.await;
                        *error.lock().unwrap() = result.err();
                        if let Some(message) = error.lock().unwrap().as_ref() {
                            let _ = app.emit("connection-error", message);
                            show_main_window(&app);
                        }
                        busy.store(false, Ordering::SeqCst);
                    });
                }
                _ => (),
            }
        })
        .build(app)?;
    let app = app.handle().clone();
    std::thread::spawn(move || loop {
        if !busy.load(Ordering::SeqCst) {
            let _ = home.set_text(t("serviceOpenHome"));
            let _ = settings.set_text(t("serviceSettings"));
            let _ = records.set_text(t("serviceRecords"));
            let _ = quit.set_text(t("serviceQuitApp"));
            let _ = startup.set_text(t("serviceConnectAtSystemSignIn"));
            let _ = approval.set_text(t("serviceTaskApprovalMode"));
            let status = tauri::async_runtime::block_on(request(&app, "status", "GET", json!({})));
            let service =
                tauri::async_runtime::block_on(request(&app, "service", "GET", json!({})));
            let task_records =
                tauri::async_runtime::block_on(request(&app, "tasks", "GET", json!({})));
            if !busy.load(Ordering::SeqCst) {
                let pending = task_records
                    .as_ref()
                    .ok()
                    .and_then(|v| v["records"].as_array())
                    .map(|records| {
                        records
                            .iter()
                            .filter(|r| r["state"] == "awaiting-approval")
                            .count()
                    });
                let _ = tasks.set_text(match pending {
                    Some(n) if n > 0 => {
                        t("serviceTasksValueAwaitingApproval").replace("{n}", &n.to_string())
                    }
                    _ => t("serviceTasks").to_owned(),
                });
                match status {
                    Ok(value) => {
                        let state = value["tunnel"]["state"].as_str().unwrap_or("unknown");
                        let active = value["connection"]["running"]
                            .as_bool()
                            .unwrap_or(matches!(state, "ready" | "starting" | "degraded"));
                        let codex_ready = value["core"]["desktop"]["state"] == "ready";
                        let verified = value["core"]["chatgpt"]["verifiedAt"].is_string();
                        let label = match state {
                            "ready" if !codex_ready => {
                                t("serviceConnectionNeedsAttentionCodexNotReady")
                            }
                            "ready" if verified => t("serviceConnected"),
                            "ready" => t("serviceTunnelReadyAwaitingChatgpt"),
                            "starting" => t("serviceConnecting"),
                            "stopping" => t("serviceDisconnecting"),
                            "error" | "degraded" => t("serviceConnectionNeedsAttention"),
                            _ => t("serviceConnectionClosed"),
                        };
                        let failure = error.lock().unwrap().take();
                        let _ = heading.set_text(
                            failure
                                .as_ref()
                                .map(|_| t("serviceActionIncompleteCheckTheApp"))
                                .unwrap_or(label),
                        );
                        let codex = match state {
                            "starting" | "stopping" => t("serviceConnecting"),
                            "error" | "degraded" => t("serviceConnectionError"),
                            "ready" => match value["core"]["desktop"]["state"].as_str() {
                                Some("ready") => t("serviceReady"),
                                Some("running") => t("serviceOpen"),
                                Some("connecting") => t("servicePreparing"),
                                Some("unavailable") => t("serviceUnavailable"),
                                Some("error") => t("serviceConnectionError"),
                                Some("disconnected") => t("serviceNotReady"),
                                _ => t("serviceCheckRequired"),
                            },
                            _ => t("serviceDisconnected"),
                        };
                        let _ = desktop.set_text(format!("Codex · {codex}"));
                        let _ = verification.set_text(if state != "ready" {
                            t("serviceChatgptDisconnected")
                        } else if verified {
                            t("serviceChatgptVerified")
                        } else {
                            t("serviceChatgptAwaitingVerification")
                        });
                        let _ = approval
                            .set_checked(value["taskApprovalEnabled"].as_bool().unwrap_or(false));
                        let _ = approval.set_enabled(!cfg!(debug_assertions));
                        let _ = connection.set_text(if active {
                            t("serviceDisconnect")
                        } else {
                            t("serviceConnect")
                        });
                        let _ = connection.set_checked(active);
                        let configured = value["config"]["configured"].as_bool().unwrap_or(false);
                        let _ = connection.set_enabled(
                            !cfg!(debug_assertions)
                                && configured
                                && !matches!(state, "starting" | "stopping"),
                        );
                    }
                    Err(_) => {
                        let _ = heading.set_text(t("serviceConnectionStatusErrorOpenTheApp"));
                        let _ = desktop.set_text(t("serviceCodexStatusUnknown"));
                        let _ = verification.set_text(t("serviceChatgptStatusUnknown"));
                        let _ = connection.set_checked(false);
                        let _ = connection.set_enabled(false);
                        let _ = approval.set_enabled(false);
                    }
                }
                if let Ok(value) = service {
                    let _ = startup.set_checked(value["enabled"].as_bool().unwrap_or(false));
                    let _ = startup.set_enabled(
                        !cfg!(debug_assertions) && value["supported"].as_bool().unwrap_or(false),
                    );
                } else {
                    let _ = startup.set_enabled(false);
                }
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    });
    Ok(())
}
