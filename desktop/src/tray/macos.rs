//! The tray is a transient nonactivating panel, not another application window.
use block2::RcBlock;
use objc2::{define_class, rc::Retained, runtime::AnyObject, MainThreadOnly};
use objc2_app_kit::*;
use objc2_foundation::MainThreadMarker;
use std::{cell::RefCell, ptr::NonNull};
use tauri::{Emitter, Manager};

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    struct TrayPanel;
    impl TrayPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { true }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool { false }
    }
);

struct Panel {
    native: Retained<TrayPanel>,
    original: Retained<NSWindow>,
    _monitors: Vec<Retained<AnyObject>>,
}
thread_local! { static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) }; }

pub fn install(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let handle = window.clone();
    window.with_webview(move |view| unsafe {
        let mtm = MainThreadMarker::new().expect("AppKit main thread");
        let original = &*view.ns_window().cast::<NSWindow>();
        let panel: Retained<TrayPanel> = objc2::msg_send![TrayPanel::alloc(mtm),
            initWithContentRect: original.frame(),
            styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            backing: NSBackingStoreType::Buffered, defer: false];
        panel.setReleasedWhenClosed(false);
        panel.setTitle(&original.title());
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setLevel(NSFloatingWindowLevel);
        panel.setFloatingPanel(true);
        panel.setHidesOnDeactivate(false);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setAcceptsMouseMovedEvents(true);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::MoveToActiveSpace
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        let content = original.contentView().expect("tray canvas");
        // Tao still consults its original content view for backing-scale changes.
        let placeholder = NSView::initWithFrame(NSView::alloc(mtm), content.frame());
        original.setContentView(Some(&placeholder));
        panel.setContentView(Some(&content));
        original.orderOut(None);

        let mask =
            NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;
        let global_handle = handle.clone();
        let global = RcBlock::new(move |_: NonNull<NSEvent>| outside(&global_handle, None));
        let local = RcBlock::new(move |event: NonNull<NSEvent>| {
            outside(
                &handle,
                event
                    .as_ref()
                    .window(MainThreadMarker::new().expect("AppKit main thread"))
                    .as_deref(),
            );
            event.as_ptr()
        });
        let monitors = [
            NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global),
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local),
        ]
        .into_iter()
        .flatten()
        .collect();
        PANEL.with(|cell| {
            *cell.borrow_mut() = Some(Panel {
                native: panel,
                original: Retained::retain(original as *const NSWindow as *mut NSWindow)
                    .expect("tray window"),
                _monitors: monitors,
            })
        });
    })
}

fn outside(window: &tauri::WebviewWindow, target: Option<&NSWindow>) {
    if window
        .app_handle()
        .state::<super::PanelInteraction>()
        .0
        .load(std::sync::atomic::Ordering::Relaxed)
        || super::pointer_on_icon(window)
    {
        return;
    }
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow().as_ref() {
            if p.native.isVisible()
                && target.map(|w| w.windowNumber()) != Some(p.native.windowNumber())
            {
                p.native.orderOut(None);
            }
        }
    });
}

pub fn toggle(
    window: &tauri::WebviewWindow,
    point: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) -> tauri::Result<()> {
    let handle = window.clone();
    // Do not use with_webview here: it holds the WebView lock, while emit
    // needs that same lock to dispatch JavaScript listeners.
    window.run_on_main_thread(move || {
        let visible =
            PANEL.with(|cell| cell.borrow().as_ref().is_some_and(|p| p.native.isVisible()));
        if visible {
            hide_now();
            return;
        }
        match super::position_panel(&handle, point, rect) {
            Ok(side) => {
                let _ = handle.emit("tray-panel:open", serde_json::json!({"side": side}));
                PANEL.with(|cell| {
                    if let Some(p) = cell.borrow().as_ref() {
                        p.native.setFrame_display(p.original.frame(), true);
                        // Unlike Tauri set_focus(), this does not activate the app.
                        p.native.makeKeyAndOrderFront(None);
                    }
                });
            }
            Err(error) => eprintln!("Tray panel: {error}"),
        }
    })
}

fn hide_now() {
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow().as_ref() {
            p.native.orderOut(None);
        }
    });
}

pub fn hide(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.run_on_main_thread(hide_now)
}

pub fn resize(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.run_on_main_thread(|| {
        PANEL.with(|cell| {
            if let Some(p) = cell.borrow().as_ref() {
                p.native.setFrame_display(p.original.frame(), true);
            }
        });
    })
}

pub fn menu_closed(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.run_on_main_thread(|| {
        PANEL.with(|cell| {
            if let Some(p) = cell.borrow().as_ref() {
                if !p.native.isKeyWindow() {
                    p.native.orderOut(None);
                }
            }
        });
    })
}
