use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::{
        Dwm::{
            DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
            DwmSetWindowAttribute, DWMWA_CAPTION_BUTTON_BOUNDS, DWMWA_CAPTION_COLOR,
            DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
        },
        Gdi::*,
    },
    UI::{
        Controls::MARGINS,
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

#[tauri::command]
pub async fn set_windows_appearance(
    window: tauri::WebviewWindow,
    dark: bool,
) -> Result<(), String> {
    if window.label() != "main" {
        return Ok(());
    }
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    // Subclass data belongs to the window thread, including theme updates from IPC.
    window
        .run_on_main_thread(move || unsafe {
            let hwnd = hwnd as HWND;
            let mut previous = 0;
            let result = if GetWindowSubclass(hwnd, Some(frame_proc), 1, &mut previous) == 0 {
                Err("Windows frame is not installed".to_string())
            } else if SetWindowSubclass(hwnd, Some(frame_proc), 1, usize::from(dark)) == 0 {
                Err(std::io::Error::last_os_error().to_string())
            } else {
                let result = apply_appearance(hwnd, dark);
                clip_webview(hwnd);
                RedrawWindow(
                    hwnd,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    RDW_FRAME | RDW_INVALIDATE,
                );
                result
            };
            let _ = sender.send(result);
        })
        .map_err(|e| e.to_string())?;
    receiver.await.map_err(|e| e.to_string())?
}

unsafe fn apply_appearance(hwnd: HWND, dark: bool) -> Result<(), String> {
    // DWM keeps ownership of the glyphs, hover states and Snap Layouts.
    let caption: u32 = if dark { 0x000000 } else { 0xffffff };
    let text: u32 = if dark { 0xffffff } else { 0x000000 };
    for (attribute, value) in [
        (DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark)),
        (DWMWA_CAPTION_COLOR, caption),
        (DWMWA_TEXT_COLOR, text),
    ] {
        let result = DwmSetWindowAttribute(
            hwnd,
            attribute as u32,
            &value as *const _ as _,
            std::mem::size_of_val(&value) as u32,
        );
        if result < 0 {
            return Err(format!(
                "DwmSetWindowAttribute({attribute}) failed: 0x{:08X}",
                result as u32
            ));
        }
    }
    Ok(())
}

pub fn install(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?.0 as HWND;
    let dark = window.theme()? == tauri::Theme::Dark;
    unsafe {
        if SetWindowSubclass(hwnd, Some(frame_proc), 1, usize::from(dark)) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // Older Windows versions may not support caption colors; keep the window usable.
        if let Err(error) = apply_appearance(hwnd, dark) {
            eprintln!("{error}");
        }
        extend_frame(hwnd);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let child = GetWindow(hwnd, GW_CHILD);
        if !child.is_null() {
            if SetWindowSubclass(child, Some(webview_proc), 1, hwnd as usize) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            clip_webview(hwnd);
        }
    }
    Ok(())
}

unsafe extern "system" fn webview_proc(
    child: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    parent: usize,
) -> LRESULT {
    let result = DefSubclassProc(child, msg, wparam, lparam);
    if msg == WM_SIZE {
        clip_webview(parent as HWND);
    } else if msg == WM_NCDESTROY {
        RemoveWindowSubclass(child, Some(webview_proc), id);
    }
    result
}

// The single Wry child covers the client area. Leave DWM's caption rectangle
// outside its region so both painting and pointer events reach the real buttons.
unsafe fn clip_webview(hwnd: HWND) {
    let child = GetWindow(hwnd, GW_CHILD);
    if child.is_null() {
        return;
    }
    let mut buttons: RECT = std::mem::zeroed();
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CAPTION_BUTTON_BOUNDS as u32,
        &mut buttons as *mut _ as _,
        std::mem::size_of::<RECT>() as u32,
    ) < 0
    {
        return;
    }
    let mut outer: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut outer);
    let mut child_rect: RECT = std::mem::zeroed();
    GetWindowRect(child, &mut child_rect);
    let region = CreateRectRgn(
        0,
        0,
        child_rect.right - child_rect.left,
        child_rect.bottom - child_rect.top,
    );
    let hole = CreateRectRgn(
        outer.left + buttons.left - child_rect.left,
        0,
        outer.left + buttons.right - child_rect.left,
        outer.top + buttons.bottom - child_rect.top,
    );
    CombineRgn(region, region, hole, RGN_DIFF);
    DeleteObject(hole);
    if SetWindowRgn(child, region, 1) == 0 {
        DeleteObject(region);
    }
}

unsafe fn extend_frame(hwnd: HWND) {
    let margins = MARGINS {
        cxLeftWidth: 0,
        cxRightWidth: 0,
        cyTopHeight: (38 * GetDpiForWindow(hwnd) / 96) as i32,
        cyBottomHeight: 0,
    };
    DwmExtendFrameIntoClientArea(hwnd, &margins);
}

unsafe extern "system" fn frame_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    dark: usize,
) -> LRESULT {
    if msg == WM_NCCALCSIZE && wparam != 0 {
        let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
        let top = params.rgrc[0].top;
        DefSubclassProc(hwnd, msg, wparam, lparam);
        params.rgrc[0].top = top;
        if IsZoomed(hwnd) != 0 {
            let dpi = GetDpiForWindow(hwnd);
            params.rgrc[0].top += GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
        }
        return 0;
    }
    let mut result = 0;
    let handled = DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut result) != 0;
    if matches!(
        msg,
        WM_ACTIVATE
            | WM_THEMECHANGED
            | WM_SETTINGCHANGE
            | WM_DWMCOLORIZATIONCOLORCHANGED
            | WM_DWMCOMPOSITIONCHANGED
            | WM_DPICHANGED
    ) {
        if !handled {
            result = DefSubclassProc(hwnd, msg, wparam, lparam);
        }
        // Apply after default processing so activation/system accent changes cannot
        // leave the exposed native caption buttons using the system accent color.
        if let Err(error) = apply_appearance(hwnd, dark != 0) {
            eprintln!("{error}");
        }
        extend_frame(hwnd);
        clip_webview(hwnd);
        RedrawWindow(
            hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_FRAME | RDW_INVALIDATE,
        );
        return result;
    }
    if handled {
        return result;
    }
    match msg {
        WM_NCHITTEST => {
            // DWM does not hit-test caption buttons on maximized custom frames.
            // Keep their real bounds and return non-client hits so Windows still
            // owns activation, hover feedback and the maximize Snap menu.
            if IsZoomed(hwnd) != 0 {
                let mut titlebar = TITLEBARINFOEX {
                    cbSize: std::mem::size_of::<TITLEBARINFOEX>() as u32,
                    ..Default::default()
                };
                SendMessageW(
                    hwnd,
                    WM_GETTITLEBARINFOEX,
                    0,
                    &mut titlebar as *mut _ as isize,
                );
                let x = (lparam as i16) as i32;
                let y = ((lparam >> 16) as i16) as i32;
                for (index, hit) in [(2, HTMINBUTTON), (3, HTMAXBUTTON), (5, HTCLOSE)] {
                    let rect = titlebar.rgrect[index];
                    if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
                        return hit as isize;
                    }
                }
            }
            result = DefSubclassProc(hwnd, msg, wparam, lparam);
            if result == HTCLIENT as isize && IsZoomed(hwnd) == 0 {
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                let y = ((lparam >> 16) as i16) as i32;
                let dpi = GetDpiForWindow(hwnd);
                if y < rect.top
                    + GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                    + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
                {
                    return HTTOP as isize;
                }
            }
            return result;
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(frame_proc), id);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
