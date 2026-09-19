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
        "Local Connector · 正在检查",
        false,
        None::<&str>,
    )?;
    let desktop = MenuItem::with_id(
        app,
        "desktop-status",
        "Codex · 正在检查",
        false,
        None::<&str>,
    )?;
    let verification = MenuItem::with_id(
        app,
        "verification",
        "ChatGPT · 正在检查",
        false,
        None::<&str>,
    )?;
    let connection =
        CheckMenuItem::with_id(app, "connection", "开启连接", false, false, None::<&str>)?;
    let startup =
        CheckMenuItem::with_id(app, "startup", "登录系统时启动", false, false, None::<&str>)?;
    let home = MenuItem::with_id(app, "overview", "打开首页", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let records = MenuItem::with_id(app, "logs", "连接记录…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出应用并关闭连接", true, None::<&str>)?;
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
            &separator2,
            &home,
            &settings,
            &records,
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
    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/tray.png"))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Local Connector")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            let action = event.id.as_ref();
            match action {
                "overview" | "settings" | "logs" => {
                    show_main_window(app);
                    let _ = app.emit("navigate", action);
                }
                "quit" => app.exit(0),
                "connection" | "startup" => {
                    if action_busy.swap(true, Ordering::SeqCst) { return; }
                    let _ = action_connection.set_enabled(false);
                    let _ = action_startup.set_enabled(false);
                    let action = action.to_owned();
                    let app = app.clone();
                    let busy = action_busy.clone();
                    let error = action_error.clone();
                    tauri::async_runtime::spawn(async move {
                        let result: Result<Value, String> = async {
                            if action == "startup" {
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
            let status = tauri::async_runtime::block_on(request(&app, "status", "GET", json!({})));
            let service =
                tauri::async_runtime::block_on(request(&app, "service", "GET", json!({})));
            if !busy.load(Ordering::SeqCst) {
                match status {
                    Ok(value) => {
                        let state = value["tunnel"]["state"].as_str().unwrap_or("unknown");
                        let active = value["connection"]["running"]
                            .as_bool()
                            .unwrap_or(matches!(state, "ready" | "starting" | "degraded"));
                        let label = match state {
                            "ready" => "通道已开启",
                            "starting" => "正在连接",
                            "stopping" => "正在关闭",
                            "error" | "degraded" => "连接需要处理",
                            _ => "连接已关闭",
                        };
                        let failure = error.lock().unwrap().clone();
                        let _ = heading.set_text(
                            failure
                                .as_ref()
                                .map(|_| "操作未完成 · 请查看应用")
                                .unwrap_or(label),
                        );
                        let codex = match value["core"]["desktop"]["state"].as_str() {
                            Some("ready") => "已就绪",
                            Some("running") => "已打开",
                            Some("connecting") => "连接中",
                            Some("disconnected") => "未就绪",
                            _ => "需检查",
                        };
                        let _ = desktop.set_text(format!("Codex · {codex}"));
                        let _ = verification.set_text(
                            if value["core"]["chatgpt"]["verifiedAt"].is_string() {
                                "ChatGPT · 已有连通记录"
                            } else {
                                "ChatGPT · 尚未验证"
                            },
                        );
                        let _ = connection.set_text(if active {
                            "关闭连接"
                        } else {
                            "开启连接"
                        });
                        let _ = connection.set_checked(active);
                        let configured = value["config"]["hasApiKey"].as_bool().unwrap_or(false)
                            && value["config"]["tunnelId"]
                                .as_str()
                                .is_some_and(|s| !s.is_empty());
                        let _ = connection.set_enabled(
                            !cfg!(debug_assertions)
                                && configured
                                && !matches!(state, "starting" | "stopping"),
                        );
                    }
                    Err(_) => {
                        let _ = heading.set_text("连接状态异常 · 请打开应用");
                        let _ = desktop.set_text("Codex · 状态未知");
                        let _ = verification.set_text("ChatGPT · 状态未知");
                        let _ = connection.set_checked(false);
                        let _ = connection.set_enabled(false);
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
