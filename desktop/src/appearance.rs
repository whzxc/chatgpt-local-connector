//! macOS application entry-point visibility, owned by the desktop process.
use connector_core::{load, root, save};
use serde_json::{json, Value};
use tauri::Manager;
use tokio::sync::Mutex;

struct Preferences(Mutex<Value>);

pub fn install(app: &mut tauri::App) -> Result<(), String> {
    let file = root().join("desktop-appearance.json");
    let mut settings = if file.exists() {
        load(&file)?
    } else {
        json!({"showMenuBar":true,"showDock":true})
    };
    // The three display locations always retain at least one application entry point.
    if settings["showMenuBar"] == false && settings["showDock"] == false {
        settings["showMenuBar"] = json!(true);
        settings["showDock"] = json!(true);
        save(&file, &settings)?;
    }
    validate(&settings)?;
    update_dock_icon(app.handle())?;
    app.set_dock_visibility(settings["showDock"].as_bool().unwrap());
    app.tray_by_id("main-tray")
        .ok_or("菜单栏图标尚未就绪")?
        .set_visible(settings["showMenuBar"].as_bool().unwrap())
        .map_err(|e| e.to_string())?;
    app.manage(Preferences(Mutex::new(settings)));
    Ok(())
}
fn validate(settings: &Value) -> Result<(), String> {
    if settings.as_object().is_none_or(|o| o.len() != 2)
        || !settings["showMenuBar"].is_boolean()
        || !settings["showDock"].is_boolean()
        || (settings["showMenuBar"] == false && settings["showDock"] == false)
    {
        return Err("显示设置无效".into());
    }
    Ok(())
}
fn apply(app: &tauri::AppHandle, settings: &Value) -> Result<(), String> {
    app.tray_by_id("main-tray")
        .ok_or("菜单栏图标尚未就绪")?
        .set_visible(settings["showMenuBar"].as_bool().unwrap())
        .map_err(|e| e.to_string())?;
    app.set_dock_visibility(settings["showDock"].as_bool().unwrap())
        .map_err(|e| e.to_string())
}
pub async fn request(app: &tauri::AppHandle, method: &str, body: Value) -> Result<Value, String> {
    let state = app.state::<Preferences>();
    let mut current = state.0.lock().await;
    if method == "GET" {
        return Ok(current.clone());
    }
    if method != "PUT" {
        return Err("不支持的显示设置操作".into());
    }
    validate(&body)?;
    if let Err(error) = apply(app, &body) {
        let _ = apply(app, &current);
        return Err(error);
    }
    if let Err(error) = save(&root().join("desktop-appearance.json"), &body) {
        let _ = apply(app, &current);
        return Err(error);
    }
    *current = body;
    Ok(current.clone())
}

/// The Dock follows the native system appearance, independently of WebView themes.
pub fn update_dock_icon(app: &tauri::AppHandle) -> Result<(), String> {
    use objc2::{runtime::Bool, AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSBezierPath, NSColor, NSImage};
    use objc2_foundation::{NSData, NSPoint, NSRect, NSSize};

    let mtm = MainThreadMarker::new().ok_or("Dock 图标必须在主线程更新")?;
    let application = NSApplication::sharedApplication(mtm);
    let dark = app
        .get_webview_window("main")
        .ok_or("主窗口尚未就绪")?
        .theme()
        .map_err(|e| e.to_string())?
        == tauri::Theme::Dark;
    if !dark {
        // nil restores the bundle icon, including the development-build identity.
        unsafe { application.setApplicationIconImage(None) };
        return Ok(());
    }
    let data = NSData::with_bytes(include_bytes!("../../ui/assets/local-connector-head.png"));
    let head = NSImage::initWithData(NSImage::alloc(), &data).ok_or("无法加载 Dock 头像")?;
    let drawing = block2::RcBlock::new(move |_rect: NSRect| {
        let tile = NSRect::new(NSPoint::new(50., 50.), NSSize::new(412., 412.));
        NSColor::colorWithSRGBRed_green_blue_alpha(0.09, 0.10, 0.11, 1.).setFill();
        let background = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(tile, 92., 92.);
        background.fill();
        NSColor::colorWithSRGBRed_green_blue_alpha(1., 1., 1., 0.12).setStroke();
        background.setLineWidth(2.);
        background.stroke();
        head.drawInRect(NSRect::new(NSPoint::new(60., 60.), NSSize::new(392., 392.)));
        Bool::YES
    });
    let icon =
        NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(512., 512.), false, &drawing);
    unsafe { application.setApplicationIconImage(Some(&icon)) };
    Ok(())
}
