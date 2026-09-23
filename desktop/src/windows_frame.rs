use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::{
        Dwm::{
            DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
            DwmSetWindowAttribute, DWMWA_CAPTION_BUTTON_BOUNDS, DWMWA_CAPTION_COLOR,
            DWMWA_COLOR_NONE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        },
        Gdi::*,
    },
    UI::{
        Controls::MARGINS,
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

#[tauri::command]
pub fn set_windows_appearance(window: tauri::WebviewWindow, dark: bool) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    // DWM keeps ownership of the glyphs, hover states and Snap Layouts.
    let caption: u32 = if dark { 0x000000 } else { 0xffffff };
    let dark = i32::from(dark);
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &caption as *const _ as _,
            4,
        );
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark as *const _ as _,
            4,
        );
        clip_webview(hwnd);
    }
    Ok(())
}

pub fn install(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?.0 as HWND;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &DWMWA_COLOR_NONE as *const _ as _,
            4,
        );
        if SetWindowSubclass(hwnd, Some(frame_proc), 1, 0) == 0 {
            return Err(std::io::Error::last_os_error().into());
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
    _: usize,
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
    if DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut result) != 0 {
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
        WM_DWMCOMPOSITIONCHANGED | WM_DPICHANGED => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            extend_frame(hwnd);
            clip_webview(hwnd);
            return result;
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(frame_proc), id);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
