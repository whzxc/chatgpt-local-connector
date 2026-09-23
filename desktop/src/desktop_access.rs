//! Fixed local desktop operations, shared by the frontend and authenticated CLI.
use connector_core::transport::{DesktopAccess, DesktopRequest};
use serde_json::{json, Value};
use tauri::{Emitter, Manager};

pub struct Owner(pub tauri::AppHandle);
pub fn check_write(app: &tauri::AppHandle) -> Result<(), String> {
    if app
        .state::<crate::updates::UpdateState>()
        .installing
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Err("正在安装更新，请等待应用重启。".into());
    }
    Ok(())
}
impl DesktopAccess for Owner {
    fn check_write(&self) -> Result<(), String> {
        check_write(&self.0)
    }
    fn request(
        &self,
        operation: DesktopRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>> {
        let app = self.0.clone();
        Box::pin(async move { request(&app, operation).await })
    }
}

pub async fn request(app: &tauri::AppHandle, operation: DesktopRequest) -> Result<Value, String> {
    // Validation reads the existing in-memory registry snapshot, never credentials.
    if let DesktopRequest::SubscriptionsOpen(body) = &operation {
        if !body.is_object() || body.as_object().unwrap().keys().any(|k| k != "providerId") {
            return Err("invalid subscriptions open request".into());
        }
        if let Some(id) = body.get("providerId") {
            let id = id
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("unknown subscription provider")?;
            let snapshot = if cfg!(debug_assertions) {
                connector_core::transport::forward_request("subscriptions", "GET", json!({}))
                    .await?
            } else {
                let service = app
                    .state::<std::sync::Arc<connector_core::service::Service>>()
                    .inner()
                    .clone();
                service.subscriptions.snapshot().await
            };
            if !snapshot["providers"]
                .as_array()
                .is_some_and(|rows| rows.iter().any(|p| p["providerId"] == id))
            {
                return Err("unknown subscription provider".into());
            }
        }
    }
    let handle = app.clone();
    let (sent, received) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        // A timed-out queued write must not run later when the UI becomes free.
        if sent.is_closed() {
            return;
        }
        let result = (|| {
            if !matches!(&operation, DesktopRequest::PanelGet) {
                check_write(&handle)?;
            }
            match operation {
                DesktopRequest::PanelGet => Ok(crate::usage_panel::snapshot()),
                DesktopRequest::PanelSet(body) => {
                    crate::usage_panel::configure(&handle, body)?;
                    let mut result = crate::usage_panel::snapshot();
                    result["saved"] = json!(true);
                    result["application"] =
                        json!(if result["runtime"]["pendingPreferences"] == true {
                            "deferred"
                        } else if result["runtime"]["status"] == "not-created"
                            || result["runtime"]["status"] == "unsupported"
                        {
                            "saved"
                        } else {
                            "controller-applied"
                        });
                    Ok(result)
                }
                DesktopRequest::SubscriptionsOpen(body) => {
                    open(&handle, body["providerId"].as_str().unwrap_or(""))
                }
            }
        })();
        let _ = sent.send(result);
    })
    .map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(5), received)
        .await
        .map_err(|_| {
            "desktop request timed out; read current state before retrying a write".to_owned()
        })?
        .map_err(|_| "desktop owner unavailable".to_owned())?
}

pub fn open(app: &tauri::AppHandle, provider: &str) -> Result<Value, String> {
    check_write(app)?;
    let window = app
        .get_webview_window("main")
        .ok_or("main window unavailable")?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window
        .emit_to("main", "subscriptions:open", provider)
        .map_err(|e| e.to_string())?;
    Ok(json!({"opened":true,"providerId":provider,"pid":std::process::id()}))
}

pub fn open_settings(app: &tauri::AppHandle) -> Result<(), String> {
    check_write(app)?;
    let window = app
        .get_webview_window("main")
        .ok_or("main window unavailable")?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window
        .emit_to("main", "agents:settings", ())
        .map_err(|e| e.to_string())
}
