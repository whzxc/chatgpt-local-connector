#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod placement;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tauri::{Emitter, Manager};
#[derive(Default)]
struct Runtime {
    count: AtomicUsize,
    auto_collapse: AtomicBool,
    wake: tokio::sync::Notify,
}
fn defaults() -> Value {
    json!({"visible":true,"autoCollapse":true,"size":"standard","spacing":"standard","horizontalPercentages":false,"alertColor":true,"notchFusion":true,"placement":{"dock":"right","display":"","x":1.,"y":0.5}})
}
pub fn preferences() -> Value {
    let mut value = defaults();
    if let Ok(saved) = connector_core::load(&connector_core::root().join("web/usage-panel.json")) {
        if let Some(fields) = saved.as_object() {
            for (k, v) in fields {
                if value.get(k).is_some() {
                    value[k] = v.clone();
                }
            }
        }
    }
    value
}
pub(super) fn save_placement(app: &tauri::AppHandle, value: Value) {
    let mut p = preferences();
    p["placement"] = value;
    let _ = connector_core::save(&connector_core::root().join("web/usage-panel.json"), &p);
    let _ = app.emit_to("main", "usage-panel:preferences", &p);
}
pub fn configure(app: &tauri::AppHandle, body: Value) -> Result<Value, String> {
    crate::desktop_access::check_write(app)?;
    let fields = body.as_object().ok_or("invalid usage panel settings")?;
    let mut p = if body["resetDefaults"] == true {
        defaults()
    } else {
        preferences()
    };
    for (k, v) in fields {
        let valid = match k.as_str() {
            "visible" | "autoCollapse" | "horizontalPercentages" | "alertColor" | "notchFusion" => {
                v.is_boolean()
            }
            "size" => ["small", "standard", "large"].contains(&v.as_str().unwrap_or("")),
            "spacing" => ["compact", "standard", "roomy"].contains(&v.as_str().unwrap_or("")),
            "dock" => {
                ["left", "right", "top", "bottom", "floating"].contains(&v.as_str().unwrap_or(""))
            }
            "resetPosition" | "resetDefaults" => v == true,
            _ => false,
        };
        if !valid {
            return Err("invalid usage panel settings".into());
        }
        match k.as_str() {
            "resetPosition" | "resetDefaults" => (),
            "dock" => {
                p["placement"]["dock"] = v.clone();
                if v == "floating" {
                    p["placement"]["x"] = json!(0.5);
                    p["placement"]["y"] = json!(0.5);
                }
            }
            _ => p[k] = v.clone(),
        }
    }
    if body["resetPosition"] == true {
        p["placement"] = defaults()["placement"].clone();
    }
    connector_core::save(&connector_core::root().join("web/usage-panel.json"), &p)?;
    app.state::<Arc<Runtime>>()
        .auto_collapse
        .store(p["autoCollapse"] == true, Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    {
        macos::configure(app, p.clone());
        sync_window(
            app,
            app.state::<Arc<Runtime>>().count.load(Ordering::Relaxed),
        );
    }
    let saved = preferences();
    let _ = app.emit_to("main", "usage-panel:preferences", &saved);
    Ok(saved)
}
// Called only on the AppHandle owner's main thread; it never creates a panel.
pub fn snapshot() -> Value {
    #[cfg(target_os = "macos")]
    let runtime = macos::snapshot();
    #[cfg(not(target_os = "macos"))]
    let runtime = json!({"status":"unsupported","observedAt":connector_core::now()});
    json!({"preferences":preferences(),"runtime":runtime})
}
fn apply(app: &tauri::AppHandle, snapshot: Value, state: &Arc<Runtime>) {
    for label in ["main", "usage-rail"] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.emit_to(label, "subscriptions:changed", &snapshot);
        }
    }
    let count = if snapshot["settings"]["enabled"] == true {
        snapshot["providers"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|p| p["selected"] == true && p["eligible"] == true)
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };
    let previous = state.count.swap(count, Ordering::Relaxed);
    if previous != count {
        #[cfg(target_os = "macos")]
        sync_window(app, count);
        state.wake.notify_one();
    }
}
#[cfg(target_os = "macos")]
fn sync_window(app: &tauri::AppHandle, count: usize) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if count == 0 || preferences()["visible"] == false {
            macos::close();
            if let Some(w) = handle.get_webview_window("usage-rail") {
                let _ = w.destroy();
            }
        } else if handle.get_webview_window("usage-rail").is_none() {
            if let Ok(w) = tauri::WebviewWindowBuilder::new(
                &handle,
                "usage-rail",
                tauri::WebviewUrl::App("usage-rail.html".into()),
            )
            .title("Subscription usage")
            .inner_size(macos::width(), macos::height(count))
            .visible(false)
            .transparent(true)
            .decorations(false)
            .focused(false)
            .focusable(false)
            .skip_taskbar(true)
            .build()
            {
                let _ = w.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                let _ = macos::install(&w, count);
            }
        }
    });
}
pub fn install(app: &tauri::AppHandle) {
    let state = Arc::new(Runtime::default());
    state
        .auto_collapse
        .store(preferences()["autoCollapse"] != false, Ordering::Relaxed);
    app.manage(state.clone());
    let app = app.clone();
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        let state = state.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let count = state.count.load(Ordering::Relaxed);
                if count == 0 {
                    state.wake.notified().await;
                    continue;
                }
                let auto_collapse = state.auto_collapse.load(Ordering::Relaxed);
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || macos::tick(&handle, count, auto_collapse));
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
        });
    }
    tauri::async_runtime::spawn(async move {
        let service = app
            .state::<Arc<connector_core::runtime::Client>>()
            .inner()
            .clone();
        let mut previous = serde_json::Value::Null;
        loop {
            if let Ok(snapshot) = service.subscriptions().await {
                if snapshot != previous {
                    apply(&app, snapshot.clone(), &state);
                    previous = snapshot;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
pub fn close() {
    #[cfg(target_os = "macos")]
    macos::close();
}

pub fn ready(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || macos::ready(&handle));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub async fn geometry(app: &tauri::AppHandle, body: Value) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        let (sent, received) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            macos::geometry(&handle, body);
            let _ = sent.send(());
        })
        .map_err(|e| e.to_string())?;
        received.await.map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, body);
    Ok(())
}
