use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

// The WebView resolves Auto and persists the preference with other UI settings.
// Only tray presentation uses this locale; the service and MCP never read it.
static CHINESE: AtomicBool = AtomicBool::new(false);
static EN: OnceLock<Value> = OnceLock::new();
static ZH: OnceLock<Value> = OnceLock::new();

#[tauri::command]
pub fn set_ui_locale(locale: String) {
    CHINESE.store(locale == "zh-CN", Ordering::Relaxed);
}

pub fn t(key: &str) -> &str {
    let en = EN.get_or_init(|| {
        serde_json::from_str(include_str!("../../ui/locales/en.json"))
            .expect("English UI resources")
    });
    let translated = if CHINESE.load(Ordering::Relaxed) {
        let zh = ZH.get_or_init(|| {
            serde_json::from_str(include_str!("../../ui/locales/zh-CN.json"))
                .expect("Chinese UI resources")
        });
        zh[key].as_str()
    } else {
        None
    };
    translated.or_else(|| en[key].as_str()).unwrap_or(key)
}
