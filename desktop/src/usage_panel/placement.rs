use super::macos::metric;
use objc2_app_kit::NSScreen;
use objc2_foundation::{MainThreadMarker, NSNumber, NSPoint, NSRect, NSSize, NSString};
use serde_json::{json, Value};
use std::ffi::{c_char, c_void};
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFUUIDCreateString(allocator: *const c_void, uuid: *const c_void) -> *const c_void;
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut c_char,
        size: isize,
        encoding: u32,
    ) -> bool;
    fn CFRelease(value: *const c_void);
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenGeometry {
    pub id: String,
    pub frame: NSRect,
    pub visible: NSRect,
    pub scale: f64,
    pub notch: Option<NSRect>,
}
pub fn screens(mtm: MainThreadMarker) -> Vec<ScreenGeometry> {
    NSScreen::screens(mtm)
        .iter()
        .filter_map(|s| {
            let number = s
                .deviceDescription()
                .objectForKey(&NSString::from_str("NSScreenNumber"))?;
            let number = number.downcast_ref::<NSNumber>()?.unsignedIntValue();
            let id = unsafe {
                let uuid = CGDisplayCreateUUIDFromDisplayID(number);
                if uuid.is_null() {
                    return None;
                }
                let string = CFUUIDCreateString(std::ptr::null(), uuid);
                let mut bytes = [0i8; 128];
                let ok = !string.is_null()
                    && CFStringGetCString(string, bytes.as_mut_ptr(), 128, 0x08000100);
                if !string.is_null() {
                    CFRelease(string)
                }
                CFRelease(uuid);
                if !ok {
                    return None;
                }
                std::ffi::CStr::from_ptr(bytes.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            let frame = s.frame();
            // These selectors arrived with notched Macs. Older macOS keeps an
            // ordinary physical-top berth without querying unavailable APIs.
            let supports: bool =
                unsafe { objc2::msg_send![&*s,respondsToSelector:objc2::sel!(safeAreaInsets)] };
            let inset = if supports { s.safeAreaInsets().top } else { 0. };
            let l = if supports {
                s.auxiliaryTopLeftArea()
            } else {
                rect(0., 0., 0., 0.)
            };
            let r = if supports {
                s.auxiliaryTopRightArea()
            } else {
                rect(0., 0., 0., 0.)
            };
            let notch = if inset > 0.
                && l.size.width > 0.
                && r.size.width > 0.
                && r.origin.x > l.origin.x + l.size.width
            {
                Some(rect(
                    l.origin.x + l.size.width,
                    frame.origin.y + frame.size.height - inset,
                    r.origin.x - l.origin.x - l.size.width,
                    inset,
                ))
            } else {
                None
            };
            Some(ScreenGeometry {
                id,
                frame,
                visible: s.visibleFrame(),
                scale: s.backingScaleFactor(),
                notch,
            })
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub display: String,
    pub dock: String,
    pub x: f64,
    pub y: f64,
}
impl Placement {
    pub fn load(v: &Value) -> Self {
        Self {
            display: v["display"].as_str().unwrap_or("").into(),
            dock: v["dock"].as_str().unwrap_or("right").into(),
            x: v["x"].as_f64().unwrap_or(1.),
            y: v["y"].as_f64().unwrap_or(0.5),
        }
    }
    pub fn value(&self) -> Value {
        json!({"display":self.display,"dock":self.dock,"x":self.x,"y":self.y})
    }
    pub fn horizontal(&self) -> bool {
        match self.dock.as_str() {
            "top" | "bottom" => true,
            "floating" => false,
            _ => false,
        }
    }
}
pub fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
pub fn contains(r: NSRect, p: NSPoint) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.width
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.height
}
pub fn scale(p: &Value) -> f64 {
    match p["size"].as_str() {
        Some("small") => metric("smallScale"),
        Some("large") => metric("largeScale"),
        _ => 1.,
    }
}
pub fn metrics(p: &Value, count: usize, placement: &Placement) -> Value {
    let s = scale(p);
    let horizontal = placement.horizontal();
    let round = true;
    let docked = placement.dock != "floating";
    let percentage = if horizontal {
        p["horizontalPercentages"] == true
    } else {
        true
    };
    let flare = 32.;
    let pad = 54. - if docked { 0. } else { flare };
    let item = if horizontal {
        if percentage {
            38.
        } else {
            36.
        }
    } else {
        metric("groupHeight")
    };
    let gap = metric("groupGap")
        * match p["spacing"].as_str() {
            Some("compact") => 0.6,
            Some("roomy") => 1.4,
            _ => 1.,
        };
    let length = (pad * 2. + item * count as f64 + count.saturating_sub(1) as f64 * gap) * s;
    let thickness = if horizontal && percentage {
        78.
    } else {
        metric("railWidth")
    } * s;
    json!({"scale":s,"length":length,"thickness":thickness,"padding":pad*s,"pitch":(item+gap)*s,"item":item*s,"percentages":percentage,"round":round,"horizontal":horizontal})
}
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub frame: NSRect,
    pub rail: NSRect,
    pub edge: String,
    pub notch: Option<NSRect>,
    pub metrics: Value,
}
impl Layout {
    pub fn value(&self) -> Value {
        json!({"screenX":self.frame.origin.x,"screenY":-self.frame.origin.y-self.frame.size.height,"width":self.frame.size.width,"height":self.frame.size.height,"rail":{"x":self.rail.origin.x,"y":self.frame.size.height-self.rail.origin.y-self.rail.size.height,"width":self.rail.size.width,"height":self.rail.size.height},"edge":self.edge,"notch":self.notch.map(|n|json!({"width":n.size.width,"height":n.size.height})),"metrics":self.metrics})
    }
}
// Ratios always describe the rail's available travel, never the transparent window.
pub fn area(screen: &ScreenGeometry, placement: &Placement) -> NSRect {
    let mut a = screen.visible;
    if placement.dock == "top" {
        a.size.height = screen.frame.origin.y + screen.frame.size.height - a.origin.y;
    }
    a
}
fn fitted_metrics(
    p: &Value,
    count: usize,
    placement: &Placement,
    screen: &ScreenGeometry,
) -> Value {
    let mut m = metrics(p, count, placement);
    let s = scale(p);
    let a = area(screen, placement);
    let available = if placement.horizontal() {
        a.size.width
            - if placement.dock == "top" && screen.notch.is_some() {
                2. * metric("flareWidth") * s
            } else {
                0.
            }
    } else {
        a.size.height
    };
    let item = m["item"].as_f64().unwrap();
    let mut pad = m["padding"].as_f64().unwrap();
    let mut gap = m["pitch"].as_f64().unwrap() - item;
    let n = count.max(1) as f64;
    if m["length"].as_f64().unwrap() > available {
        if count > 1 {
            gap = gap.min(((available - 2. * pad - item * n) / (n - 1.)).max(0.));
        }
        let minimum = if placement.dock == "floating" {
            12.
        } else {
            36.
        } * s;
        pad = pad.min(((available - item * n - gap * (n - 1.)) / 2.).max(minimum));
    }
    let mut visible = count.max(1);
    let mut footer = 0.;
    if 2. * pad + item * n + gap * (n - 1.) > available {
        footer = 24. * s;
        visible = ((available - 2. * pad - footer) / (item + gap))
            .floor()
            .max(1.) as usize;
    }
    let length = 2. * pad + item * visible as f64 + gap * visible.saturating_sub(1) as f64 + footer;
    m["length"] = json!(length);
    m["padding"] = json!(pad);
    m["pitch"] = json!(item + gap);
    m["visibleCount"] = json!(visible);
    m["footer"] = json!(footer);
    m
}
pub fn rail_size(
    p: &Value,
    count: usize,
    placement: &Placement,
    screen: &ScreenGeometry,
) -> NSSize {
    let m = fitted_metrics(p, count, placement, screen);
    let along = m["length"].as_f64().unwrap();
    let across = m["thickness"].as_f64().unwrap();
    if placement.horizontal() {
        let notch = if placement.dock == "top" && p["notchFusion"] != false {
            screen.notch
        } else {
            None
        };
        NSSize::new(
            along.max(notch.map_or(0., |n| n.size.width)),
            across + notch.map_or(0., |n| n.size.height),
        )
    } else {
        NSSize::new(across, along)
    }
}
pub fn origin(placement: &Placement, screen: &ScreenGeometry, size: NSSize) -> NSPoint {
    let a = area(screen, placement);
    let mut x = a.origin.x + (a.size.width - size.width).max(0.) * placement.x;
    let mut y = a.origin.y + (a.size.height - size.height).max(0.) * placement.y;
    match placement.dock.as_str() {
        "right" => x = a.origin.x + a.size.width - size.width,
        "left" => x = a.origin.x,
        "top" => {
            y = a.origin.y + a.size.height - size.height;
            if let Some(n) = screen.notch {
                x = n.origin.x + n.size.width / 2. - size.width / 2.
            }
        }
        "bottom" => y = a.origin.y,
        _ => {}
    }
    NSPoint::new(x, y)
}
pub fn record(placement: &mut Placement, screen: &ScreenGeometry, r: NSRect) {
    let a = area(screen, placement);
    placement.display = screen.id.clone();
    if !(placement.dock == "top" && screen.notch.is_some()) {
        placement.x =
            ((r.origin.x - a.origin.x) / (a.size.width - r.size.width).max(1.)).clamp(0., 1.);
    }
    placement.y =
        ((r.origin.y - a.origin.y) / (a.size.height - r.size.height).max(1.)).clamp(0., 1.);
}
pub fn layout(
    p: &Value,
    count: usize,
    placement: &Placement,
    screen: &ScreenGeometry,
    at: NSPoint,
) -> Layout {
    let metrics = fitted_metrics(p, count, placement, screen);
    let s = scale(p);
    let size = rail_size(p, count, placement, screen);
    let a = area(screen, placement);
    let edge = if placement.dock == "floating" {
        if placement.horizontal() {
            if at.y + size.height / 2. > a.origin.y + a.size.height / 2. {
                "top"
            } else {
                "bottom"
            }
        } else if at.x + size.width / 2. > a.origin.x + a.size.width / 2. {
            "right"
        } else {
            "left"
        }
    } else {
        &placement.dock
    }
    .to_string();
    let pad = metric("windowPadding") * s;
    let card = metric("cardWidth") * s;
    let gap = (metric("pointerWidth") + metric("cardGap")) * s;
    // Keep enough transparent budget for the short turn between axes. This is
    // decided once per layout change, never resized on animation frames.
    let mut vertical = placement.clone();
    vertical.dock = "right".into();
    let turn = metrics_for_turn(p, count, &vertical).min(a.size.width.min(a.size.height));
    let (w, h) = if placement.horizontal() {
        (
            (size.width
                + if placement.dock == "top" && p["notchFusion"] != false && screen.notch.is_some()
                {
                    64. * s
                } else {
                    0.
                })
            .max(size.width + 2. * (card + gap) + 2. * pad)
            .min(a.size.width)
            .max(turn),
            (size.height + gap + metric("cardBudgetHeight") * s + pad)
                .max(turn)
                .min(a.size.height),
        )
    } else {
        (
            (size.width + 2. * gap + 2. * card.max(metric("cardWidth")) + 2. * pad)
                .max(turn)
                .min(a.size.width),
            size.height
                .max(metric("cardBudgetHeight") * s + 2. * pad)
                .min(a.size.height),
        )
    };
    let x = if placement.horizontal() {
        at.x + (size.width - w) / 2.
    } else if edge == "right" {
        at.x + size.width - w
    } else {
        at.x
    };
    let y = if !placement.horizontal() {
        at.y + (size.height - h) / 2.
    } else if edge == "top" {
        at.y + size.height - h
    } else {
        at.y
    };
    let frame = rect(
        x.clamp(a.origin.x, (a.origin.x + a.size.width - w).max(a.origin.x)),
        y.clamp(a.origin.y, (a.origin.y + a.size.height - h).max(a.origin.y)),
        w,
        h,
    );
    Layout {
        frame,
        rail: rect(
            at.x - frame.origin.x,
            at.y - frame.origin.y,
            size.width,
            size.height,
        ),
        edge,
        notch: if placement.dock == "top" && p["notchFusion"] != false {
            screen.notch
        } else {
            None
        },
        metrics,
    }
}

fn metrics_for_turn(p: &Value, count: usize, placement: &Placement) -> f64 {
    metrics(p, count, placement)["length"].as_f64().unwrap()
}
