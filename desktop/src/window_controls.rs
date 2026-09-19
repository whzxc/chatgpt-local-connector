use objc2_app_kit::{NSView, NSWindow, NSWindowButton};

/// Keep AppKit's real controls and their tracking areas inside the 64pt app header.
/// Configure the entire titlebar, rather than only offsetting its buttons.
pub fn align(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        let window = &*webview.ns_window().cast::<NSWindow>();
        // AppKit owns the full-screen titlebar and its reveal animation.
        if window
            .styleMask()
            .contains(objc2_app_kit::NSWindowStyleMask::FullScreen)
        {
            return;
        }
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
            return;
        };
        let Some(titlebar) = close.superview() else {
            return;
        };
        let Some(container) = titlebar.superview() else {
            return;
        };
        let mut frame = container.frame();
        frame.size.height = 64.0;
        frame.origin.y = window.frame().size.height - frame.size.height;
        container.setFrame(frame);
        let mut frame = titlebar.frame();
        frame.size.height = 64.0;
        frame.origin.y = 0.0;
        titlebar.setFrame(frame);
        for (index, kind) in [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(button) = window.standardWindowButton(kind) {
                let mut frame = NSView::frame(&button);
                frame.origin.x = 20.0 + index as f64 * 23.0;
                frame.origin.y = (64.0 - frame.size.height) / 2.0;
                button.setFrame(frame);
                button.updateTrackingAreas();
            }
        }
        titlebar.updateTrackingAreas();
        container.updateTrackingAreas();
        titlebar.setNeedsDisplay(true);
    });
}
