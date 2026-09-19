use crate::request;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::{watch, Mutex};

pub struct UpdateState {
    install: Mutex<()>,
    cancel: watch::Sender<bool>,
    downloading: AtomicBool,
    pub installing: AtomicBool,
}
impl Default for UpdateState {
    fn default() -> Self {
        Self {
            install: Mutex::new(()),
            cancel: watch::channel(false).0,
            downloading: AtomicBool::new(false),
            installing: AtomicBool::new(false),
        }
    }
}
fn resume_path() -> std::path::PathBuf {
    connector_core::root().join("update-resume.json")
}
pub fn take_resume() -> Option<bool> {
    let path = resume_path();
    let Ok(value) = connector_core::load(&path) else {
        return None;
    };
    // A marker from a failed/interrupted install must not alter ordinary startup.
    let resume =
        (value["version"] == env!("CARGO_PKG_VERSION")).then(|| value["connected"] == true);
    let _ = std::fs::remove_file(path);
    resume
}
fn updater(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    if cfg!(debug_assertions) {
        return Err("开发预览不检查或安装更新。".into());
    }
    app.updater_builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<Value, String> {
    let update = updater(&app)?.check().await.map_err(|e| e.to_string())?;
    Ok(match update {
        Some(u) => {
            json!({"available":true,"version":u.version,"notes":u.body,"date":u.date.map(|d|d.to_string())})
        }
        None => json!({"available":false}),
    })
}
#[tauri::command]
pub fn cancel_update(app: tauri::AppHandle) {
    let state = app.state::<UpdateState>();
    if state.downloading.load(Ordering::SeqCst) {
        state.cancel.send_replace(true);
    }
}
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, version: String) -> Result<Value, String> {
    let state = app.state::<UpdateState>();
    let _guard = state.install.try_lock().map_err(|_| "已有更新正在进行")?;
    let mut cancel = state.cancel.subscribe();
    state.cancel.send_replace(false);
    cancel.borrow_and_update();
    state.downloading.store(true, Ordering::SeqCst);
    let result = tokio::select! {
        result = async {
            let Some(update) = updater(&app)?.check().await.map_err(|e| e.to_string())? else {
                return Ok(None);
            };
            if update.version != version {
                return Err("可用版本已变化，请重新检查更新。".to_owned());
            }
            let mut downloaded = 0usize;
            let bytes = update.download(|chunk, total| {
                downloaded += chunk;
                let _ = app.emit("update-progress", json!({"downloaded":downloaded,"total":total,"phase":"downloading"}));
            }, || {}).await.map_err(|e|e.to_string())?;
            Ok(Some((update, bytes)))
        } => result,
        _ = cancel.changed() => Err("UPDATE_CANCELLED".into()),
    };
    state.downloading.store(false, Ordering::SeqCst);
    let Some((update, bytes)) = result? else {
        return Ok(json!({"available":false}));
    };
    state.installing.store(true, Ordering::SeqCst);
    struct Reset<'a>(&'a AtomicBool);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _reset = Reset(&state.installing);
    let _ = app.emit("update-progress", json!({"phase":"installing"}));
    let service = app
        .state::<std::sync::Arc<connector_core::service::Service>>()
        .inner()
        .clone();
    // Download and signature verification finish before any connection is stopped.
    let current = request(&app, "status", "GET", json!({})).await?;
    let connected = current["connection"]["running"].as_bool().unwrap_or(false);
    connector_core::save(
        &resume_path(),
        &json!({"version":version,"connected":connected}),
    )?;
    if connected {
        if let Err(error) = service.request("stop", "POST", json!({})).await {
            let _ = std::fs::remove_file(resume_path());
            return Err(error);
        }
    }
    if let Err(error) = update.install(bytes) {
        let _ = std::fs::remove_file(resume_path());
        let restored = if connected {
            service.request("start", "POST", json!({})).await
        } else {
            Ok(json!({}))
        };
        return Err(match restored {
            Ok(_) => format!("安装失败：{error}。可重试或手动下载。"),
            Err(restore) => format!("安装失败：{error}；恢复连接失败：{restore}"),
        });
    }
    app.restart();
}
