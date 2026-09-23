use objc2::{define_class, rc::Retained, runtime::ProtocolObject, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAutoresizingMaskOptions, NSPopover, NSPopoverBehavior, NSPopoverDelegate,
    NSView, NSViewController, NSWindow,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRectEdge,
    NSSize, NSString,
};
use std::cell::RefCell;
use tauri::Manager;

pub struct DelegateState {
    app: tauri::AppHandle,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateState]
    struct PopoverDelegate;
    unsafe impl NSObjectProtocol for PopoverDelegate {}
    unsafe impl NSPopoverDelegate for PopoverDelegate {
        #[unsafe(method(popoverDidClose:))]
        fn did_close(&self, _notification: &NSNotification) {
            super::closed(&self.ivars().app);
            if let Some(primary) = self.ivars().app.get_webview_window("tray-panel") {
                if !primary.is_focused().unwrap_or(false) {
                    let _ = primary.hide();
                }
            }
        }
    }
);
struct Presentation {
    popover: Retained<NSPopover>,
    _delegate: Retained<PopoverDelegate>,
}
thread_local! { static PRESENTATION: RefCell<Option<Presentation>> = const { RefCell::new(None) }; }

pub fn install(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    window.with_webview(move |view| unsafe {
        let mtm = MainThreadMarker::new().expect("AppKit main thread");
        let original = &*view.ns_window().cast::<NSWindow>();
        let content = original.contentView().expect("detail content");
        let placeholder = NSView::initWithFrame(NSView::alloc(mtm), content.frame());
        original.setContentView(Some(&placeholder));
        content.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let controller = NSViewController::new(mtm);
        controller.setView(&content);
        let popover = NSPopover::new(mtm);
        popover.setBehavior(NSPopoverBehavior::Transient);
        popover.setAnimates(true);
        popover.setContentViewController(Some(&controller));
        let delegate = PopoverDelegate::alloc(mtm).set_ivars(DelegateState { app });
        let delegate: Retained<PopoverDelegate> = objc2::msg_send![super(delegate), init];
        popover.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        original.orderOut(None);
        PRESENTATION.with(|state| {
            *state.borrow_mut() = Some(Presentation {
                popover,
                _delegate: delegate,
            })
        });
    })
}

pub fn shown() -> bool {
    PRESENTATION.with(|state| state.borrow().as_ref().is_some_and(|p| p.popover.isShown()))
}
pub fn close(app: &tauri::AppHandle) {
    let _ = app.run_on_main_thread(|| {
        let popover = PRESENTATION.with(|state| state.borrow().as_ref().map(|p| p.popover.clone()));
        if let Some(popover) = popover {
            if popover.isShown() {
                unsafe {
                    popover.performClose(None);
                }
            }
        }
    });
}
pub fn show(
    primary: &tauri::WebviewWindow,
    anchor: serde_json::Value,
    height: f64,
    dark: bool,
    animate: bool,
) -> Result<(), String> {
    primary
        .with_webview(move |view| unsafe {
            let native = &*view.ns_window().cast::<NSWindow>();
            let Some(content) = native.contentView() else {
                return;
            };
            let number = |key: &str| anchor[key].as_f64().filter(|n| n.is_finite()).unwrap_or(0.);
            let y = if content.isFlipped() {
                number("y")
            } else {
                content.bounds().size.height - number("y") - number("height")
            };
            let rect = NSRect::new(
                NSPoint::new(number("x"), y),
                NSSize::new(number("width"), number("height")),
            );
            let popover =
                PRESENTATION.with(|state| state.borrow().as_ref().map(|p| p.popover.clone()));
            if let Some(popover) = popover {
                popover.setAnimates(animate);
                let appearance = NSAppearance::appearanceNamed(&NSString::from_str(if dark {
                    "NSAppearanceNameDarkAqua"
                } else {
                    "NSAppearanceNameAqua"
                }));
                popover.setAppearance(appearance.as_deref());
                popover.setContentSize(NSSize::new(250., height.clamp(40., 432.)));
                popover.showRelativeToRect_ofView_preferredEdge(rect, &content, NSRectEdge::MinY);
            }
        })
        .map_err(|e| e.to_string())
}
