//! A non-key NSPanel owns input and placement. Vue reports the presented shape;
//! no second native animation attempts to predict where its pixels are.
use super::placement::{self, Layout, Placement, ScreenGeometry};
use block2::RcBlock;
use objc2::{define_class, rc::Retained, runtime::AnyObject, MainThreadOnly};
use objc2_app_kit::*;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use serde_json::{json, Value};
use std::{cell::RefCell, ptr::NonNull, time::Instant};
use tauri::{Emitter, Manager};
define_class!(
 #[unsafe(super(NSPanel))]
 #[thread_kind = MainThreadOnly]
 struct UsagePanel;
 impl UsagePanel {
  #[unsafe(method(canBecomeKeyWindow))]
  fn can_become_key(&self)->bool{false}
  #[unsafe(method(canBecomeMainWindow))]
  fn can_become_main(&self)->bool{false}
  #[unsafe(method(sendEvent:))]
  fn send_event(&self,event:&NSEvent){if !input(event){unsafe{let _:()=objc2::msg_send![super(self),sendEvent:event];}}}
  #[unsafe(method(panelAction:))]
  fn panel_action(&self,item:&NSMenuItem){menu_action(item.tag());}
 }
);
struct Interaction {
    start: NSPoint,
    grab: NSPoint,
    down_ring: Option<String>,
    dragged: bool,
}
struct Panel {
    native: Retained<UsagePanel>,
    app: tauri::AppHandle,
    count: usize,
    preferences: Value,
    placement: Placement,
    screen: Option<ScreenGeometry>,
    layout: Option<Layout>,
    generation: u64,
    expanded: bool,
    slot: Option<usize>,
    provider_id: Option<String>,
    left: Option<Instant>,
    geometry: Value,
    interaction: Option<Interaction>,
    menu_open: bool,
    pending_preferences: Option<Value>,
    monitors: Vec<Retained<AnyObject>>,
}
thread_local! {static PANEL:RefCell<Option<Panel>>=const{RefCell::new(None)};}
pub fn snapshot() -> Value {
    debug_assert!(MainThreadMarker::new().is_some());
    PANEL.with(|cell| {
        let state = cell.borrow();
        let Some(p) = state.as_ref() else {
            return json!({"status":"not-created","observedAt":connector_core::now()});
        };
        let frame = p.native.frame();
        json!({
            "status":if p.native.isVisible() {"visible"} else {"hidden"},
            "observedAt":connector_core::now(),"placement":p.placement.value(),
            "visible":p.native.isVisible(),"expanded":p.expanded,
            "interaction":match &p.interaction {None=>"idle",Some(v) if v.dragged=>"dragging",Some(_)=>"pressed"},
            "providerId":p.provider_id,"menuOpen":p.menu_open,
            "frame":{"x":frame.origin.x,"y":frame.origin.y,"width":frame.size.width,"height":frame.size.height,"coordinates":"appkit-screen-points"},
            "pendingPreferences":p.pending_preferences.is_some(),"generation":p.generation,
            "acceptedGeometry":{"generation":p.geometry["generation"],"matches":p.geometry["generation"] == p.generation && p.geometry["display"] == p.placement.display && p.geometry["dock"] == p.placement.dock}
        })
    })
}
pub fn install(window: &tauri::WebviewWindow, count: usize) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    window.with_webview(move |view| unsafe {
        let mtm = MainThreadMarker::new().expect("AppKit main thread");
        let original = &*view.ns_window().cast::<NSWindow>();
        let panel: Retained<UsagePanel> = objc2::msg_send![UsagePanel::alloc(mtm),
            initWithContentRect: NSRect::new(NSPoint::new(0.,0.),NSSize::new(width(),height(count))),
            styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            backing: NSBackingStoreType::Buffered, defer: false];
        panel.setReleasedWhenClosed(false);
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setLevel(NSFloatingWindowLevel);
        panel.setHidesOnDeactivate(false);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setAcceptsMouseMovedEvents(true);
        panel.setIgnoresMouseEvents(true);
        panel.setCollectionBehavior(NSWindowCollectionBehavior::MoveToActiveSpace | NSWindowCollectionBehavior::FullScreenNone);
        if let Some(content) = original.contentView() {
            // Tao still reads the original window's contentView when AppKit
            // changes its backing scale, even after the window is hidden.
            // Keep a view there so a display change cannot panic in Tao.
            let placeholder = NSView::initWithFrame(NSView::alloc(mtm), content.frame());
            original.setContentView(Some(&placeholder));
            panel.setContentView(Some(&content));
        }
        original.orderOut(None);
        // Mouse-only monitors don't request keyboard/accessibility permission.
        // Global receives other apps' motion even while this panel ignores input;
        // local receives our own. They observe, never swallow or synthesize input.
        let mask = NSEventMask::MouseMoved | NSEventMask::LeftMouseDragged | NSEventMask::RightMouseDragged;
        let global_app = app.clone();
        let global = RcBlock::new(move |_: NonNull<NSEvent>| pointer(&global_app));
        let local_app = app.clone();
        let local = RcBlock::new(move |event: NonNull<NSEvent>| { pointer(&local_app); event.as_ptr() });
        let monitors = [NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global),
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local)].into_iter().flatten().collect();
        PANEL.with(|cell| *cell.borrow_mut() = Some(Panel {native:panel,app:app.clone(),count,preferences:super::preferences(),placement:Placement::load(&super::preferences()["placement"]),layout:None,screen:None,generation:0,expanded:false,slot:None,provider_id:None,left:None,geometry:Value::Null,monitors,interaction:None,menu_open:false,pending_preferences:None}));
        tick(&app,count,super::preferences()["autoCollapse"]!=false);
    })
}

pub fn width() -> f64 {
    354.
}
pub fn height(count: usize) -> f64 {
    (2. * metric("railPadding")
        + count as f64 * metric("groupHeight")
        + count.saturating_sub(1) as f64 * metric("groupGap"))
    .max(metric("cardBudgetHeight"))
        + 24.
}
fn local(p: &Panel, mouse: NSPoint) -> NSPoint {
    let f = p.native.frame();
    NSPoint::new(mouse.x - f.origin.x, f.size.height - (mouse.y - f.origin.y))
}
fn ring(p: &Panel, point: NSPoint) -> Option<(usize, String)> {
    p.geometry["rings"].as_array()?.iter().find_map(|r| {
        let dx = point.x - r["x"].as_f64()?;
        let dy = point.y - r["y"].as_f64()?;
        (dx * dx + dy * dy <= r["radius"].as_f64()?.powi(2)).then(|| {
            (
                r["slot"].as_u64().unwrap_or(0) as usize,
                r["providerId"].as_str().unwrap_or("").into(),
            )
        })
    })
}
fn place(p: &mut Panel, mut screen: ScreenGeometry, at: Option<NSPoint>) {
    if p.preferences["notchFusion"] == false {
        screen.notch = None;
    }
    let size = placement::rail_size(&p.preferences, p.count, &p.placement, &screen);
    let origin = at.unwrap_or_else(|| placement::origin(&p.placement, &screen, size));
    let mut layout = placement::layout(&p.preferences, p.count, &p.placement, &screen, origin);
    p.native.setLevel(if p.placement.dock == "top" {
        NSStatusWindowLevel
    } else {
        NSFloatingWindowLevel
    });
    if p.native.frame() != layout.frame {
        p.native.setFrame_display(layout.frame, true);
    }
    p.native.orderFrontRegardless();
    // AppKit may constrain on orderFront, not just setFrame. Compensate with
    // the local rail offset against the frame actually granted by the server.
    let actual = p.native.frame();
    layout.rail.origin.x += layout.frame.origin.x - actual.origin.x;
    layout.rail.origin.y += layout.frame.origin.y - actual.origin.y;
    layout.frame = actual;
    let changed = p.layout.as_ref() != Some(&layout) || p.screen.as_ref() != Some(&screen);
    if changed {
        p.generation += 1;
        p.geometry = Value::Null;
    }
    p.layout = Some(layout);
    p.screen = Some(screen);
    if changed {
        emit(&p.app, p);
    }
}
pub fn configure(app: &tauri::AppHandle, preferences: Value) {
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().as_mut() {
            p.pending_preferences = Some(preferences);
        }
    });
    tick(app, 0, true);
}
pub fn tick(app: &tauri::AppHandle, count: usize, _auto_collapse: bool) {
    // App deactivation, sleep or a lost capture can omit mouse-up. Cancel the
    // click and release ownership once the physical button is no longer held.
    let released = PANEL.with(|cell| {
        let mut state = cell.borrow_mut();
        let p = state.as_mut()?;
        if p.interaction.is_some() && NSEvent::pressedMouseButtons() & 1 == 0 {
            let held = p.interaction.take()?;
            p.slot = None;
            p.provider_id = None;
            p.left = None;
            if held.dragged {
                p.preferences["placement"] = p.placement.value();
                return Some(p.placement.value());
            }
        }
        None
    });
    if let Some(value) = released {
        super::save_placement(app, value);
    }

    let mut remembered = None;
    PANEL.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(p) = state.as_mut() else { return };
        if p.interaction.is_some() || p.menu_open {
            return;
        }
        let reconfigured = p.pending_preferences.is_some();
        if let Some(prefs) = p.pending_preferences.take() {
            p.placement = Placement::load(&prefs["placement"]);
            p.preferences = prefs;
            p.layout = None;
        }
        if p.preferences["visible"] == false {
            p.native.orderOut(None);
            p.slot = None;
            p.provider_id = None;
            return;
        }
        if count > 0 && count != p.count {
            p.count = count;
            p.layout = None;
            p.slot = None;
            p.provider_id = None;
        }
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let screens = placement::screens(mtm);
        let Some(mut screen) = screens
            .iter()
            .find(|s| s.id == p.placement.display)
            .or_else(|| screens.first())
            .cloned()
        else {
            return;
        };
        let display_changed = p.placement.display != screen.id;
        if p.preferences["notchFusion"] == false {
            screen.notch = None;
        }
        if p.layout
            .as_ref()
            .is_none_or(|l| l.frame != p.native.frame())
            || p.screen.as_ref() != Some(&screen)
        {
            p.placement.display = screen.id.clone();
            place(p, screen, None);
        }
        if reconfigured || display_changed {
            p.preferences["placement"] = p.placement.value();
            remembered = Some(p.placement.value());
        }
    });
    if let Some(value) = remembered {
        super::save_placement(app, value);
    }
    pointer(app);
}
fn wake(p: &Panel, point: NSPoint) -> bool {
    let Some(l) = &p.layout else { return false };
    let r = l.rail;
    let y = l.frame.size.height - r.origin.y - r.size.height;
    if let Some(n) = l.notch {
        return point.x >= r.origin.x + (r.size.width - n.size.width) / 2.
            && point.x <= r.origin.x + (r.size.width + n.size.width) / 2.
            && point.y >= y
            && point.y <= y + n.size.height;
    }
    let s = placement::scale(&p.preferences);
    let length = metric("collapsedLength") * s;
    let width = metric("wakeWidth") * s;
    let rect = match p.placement.dock.as_str() {
        "right" => placement::rect(
            r.origin.x + r.size.width - width,
            y + (r.size.height - length) / 2.,
            width,
            length,
        ),
        "left" => placement::rect(r.origin.x, y + (r.size.height - length) / 2., width, length),
        "top" => placement::rect(r.origin.x + (r.size.width - length) / 2., y, length, width),
        "bottom" => placement::rect(
            r.origin.x + (r.size.width - length) / 2.,
            y + r.size.height - width,
            length,
            width,
        ),
        _ => return false,
    };
    placement::contains(rect, point)
}
fn pointer(app: &tauri::AppHandle) {
    PANEL.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            return;
        };
        let Some(p) = state.as_mut() else { return };
        if p.preferences["visible"] == false {
            p.native.setIgnoresMouseEvents(true);
            return;
        }
        if p.interaction.is_some() || p.menu_open {
            p.native.setIgnoresMouseEvents(false);
            return;
        }
        let old = (p.expanded, p.slot, p.provider_id.clone());
        let point = local(p, NSEvent::mouseLocation());
        let (x, y) = (point.x, point.y);
        // A non-key WKWebView does not reliably receive DOM hover events.
        // Forward observed pointer coordinates without activating the panel.
        if p.slot.is_some() {
            let _ = app.emit_to("usage-rail", "usage-panel:hover", json!({"x": x, "y": y}));
        }
        let rail = hit(&p.geometry["rail"], x, y);
        let detail = hit(&p.geometry["detail"], x, y);
        let corridor = p.slot.is_some() && hit(&p.geometry["corridor"], x, y);
        if rail || detail || corridor || wake(p, point) {
            p.expanded = true;
            p.left = None;
            if let Some((slot, id)) = ring(p, point) {
                p.provider_id = Some(id);
                p.slot = Some(slot);
            }
        } else {
            if p.left.is_none() {
                p.left = Some(Instant::now());
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(metric("leaveMs") as u64))
                        .await;
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || pointer(&handle));
                });
            }
            if p.left
                .is_some_and(|v| v.elapsed().as_millis() as f64 >= metric("leaveMs"))
            {
                p.expanded = false;
                p.slot = None;
                p.provider_id = None;
            }
        }
        if p.preferences["autoCollapse"] == false || p.placement.dock == "floating" {
            p.expanded = true;
        }
        // Wake zones/corridors observe only; unpainted pixels still click through.
        p.native.setIgnoresMouseEvents(!(rail || detail));
        if old != (p.expanded, p.slot, p.provider_id.clone()) {
            emit(app, p);
        }
    });
}
fn input(event: &NSEvent) -> bool {
    let kind = event.r#type();
    if kind == NSEventType::RightMouseDown
        || (kind == NSEventType::LeftMouseDown
            && event
                .modifierFlags()
                .contains(NSEventModifierFlags::Control))
    {
        show_menu();
        return true;
    }
    let mut open = None;
    let mut save = None;
    let consumed = PANEL.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(p) = state.as_mut() else {
            return false;
        };
        let mouse = NSEvent::mouseLocation();
        let point = local(p, mouse);
        if kind == NSEventType::LeftMouseDown {
            if hit(&p.geometry["controls"], point.x, point.y)
                || !hit(&p.geometry["rail"], point.x, point.y)
            {
                return false;
            }
            let Some(l) = &p.layout else { return false };
            let f = p.native.frame();
            p.interaction = Some(Interaction {
                start: mouse,
                grab: NSPoint::new(
                    mouse.x - f.origin.x - l.rail.origin.x,
                    mouse.y - f.origin.y - l.rail.origin.y,
                ),
                down_ring: ring(p, point).map(|v| v.1),
                dragged: false,
            });
            p.expanded = true;
            p.slot = None;
            p.provider_id = None;
            p.left = None;
            p.native.setIgnoresMouseEvents(false);
            emit(&p.app, p);
            return true;
        }
        if kind == NSEventType::LeftMouseDragged {
            let Some(mut held) = p.interaction.take() else {
                return false;
            };
            if !held.dragged
                && (mouse.x - held.start.x).hypot(mouse.y - held.start.y) < metric("dragThreshold")
            {
                p.interaction = Some(held);
                return true;
            }
            held.dragged = true;
            let Some(mtm) = MainThreadMarker::new() else {
                p.interaction = Some(held);
                return true;
            };
            let screens = placement::screens(mtm);
            let Some(mut screen) = screens
                .iter()
                .find(|s| placement::contains(s.frame, mouse))
                .or(p.screen.as_ref())
                .cloned()
            else {
                p.interaction = Some(held);
                return true;
            };
            if p.preferences["notchFusion"] == false {
                screen.notch = None;
            }
            let previous = p.placement.clone();
            let old_size = placement::rail_size(&p.preferences, p.count, &previous, &screen);
            let desired = NSPoint::new(mouse.x - held.grab.x, mouse.y - held.grab.y);
            let a = screen.visible;
            let threshold = |dock: &str| {
                if previous.dock == dock {
                    metric("undockDistance")
                } else {
                    metric("dockDistance")
                }
            };
            let dock = if screen.frame.origin.y + screen.frame.size.height - mouse.y
                < threshold("top")
            {
                "top"
            } else if mouse.y - a.origin.y < threshold("bottom") {
                "bottom"
            } else if desired.x - a.origin.x < threshold("left") {
                "left"
            } else if a.origin.x + a.size.width - desired.x - old_size.width < threshold("right") {
                "right"
            } else {
                "floating"
            };
            p.placement.dock = dock.into();
            p.placement.display = screen.id.clone();
            let size = placement::rail_size(&p.preferences, p.count, &p.placement, &screen);
            if previous.horizontal() != p.placement.horizontal() {
                held.grab = NSPoint::new(size.width / 2., size.height / 2.);
            } else {
                held.grab.x += (size.width - old_size.width) / 2.;
                held.grab.y += (size.height - old_size.height) / 2.;
            }
            let a = placement::area(&screen, &p.placement);
            let mut at = NSPoint::new(
                (mouse.x - held.grab.x).clamp(
                    a.origin.x,
                    (a.origin.x + a.size.width - size.width).max(a.origin.x),
                ),
                (mouse.y - held.grab.y).clamp(
                    a.origin.y,
                    (a.origin.y + a.size.height - size.height).max(a.origin.y),
                ),
            );
            placement::record(&mut p.placement, &screen, NSRect::new(at, size));
            if dock != "floating" {
                at = placement::origin(&p.placement, &screen, size);
            }
            p.interaction = Some(held);
            place(p, screen, Some(at));
            return true;
        }
        if kind == NSEventType::LeftMouseUp {
            if let Some(held) = p.interaction.take() {
                if held.dragged {
                    save = Some(p.placement.value());
                    p.preferences["placement"] = p.placement.value();
                } else if let Some(id) = held.down_ring {
                    if ring(p, point).is_some_and(|v| v.1 == id) {
                        open = Some((p.app.clone(), id));
                    }
                }
                p.left = None;
                emit(&p.app, p);
                return true;
            }
        }
        false
    });
    if let Some(value) = save {
        PANEL.with(|cell| {
            if let Some(p) = cell.borrow().as_ref() {
                super::save_placement(&p.app, value)
            }
        })
    }
    if let Some((app, id)) = open {
        open_main(&app, &id)
    }
    consumed
}
fn open_main(app: &tauri::AppHandle, id: &str) {
    let _ = crate::desktop_access::open(app, id);
}
fn show_menu() {
    let data = PANEL.with(|cell| {
        let mut state = cell.borrow_mut();
        let p = state.as_mut()?;
        p.menu_open = true;
        p.expanded = true;
        p.slot = None;
        p.provider_id = None;
        emit(&p.app, p);
        Some(p.native.clone())
    });
    let Some(panel) = data else { return };
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let menu = NSMenu::new(mtm);
    for (tag, key) in ["settings", "usageRestoreDefaults", "usageHide"]
        .iter()
        .enumerate()
    {
        unsafe {
            let item = menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str(crate::i18n::t(key)),
                Some(objc2::sel!(panelAction:)),
                &NSString::from_str(""),
            );
            item.setTag(tag as isize);
            item.setTarget(Some(&panel));
        }
    }
    menu.popUpMenuPositioningItem_atLocation_inView(None, NSEvent::mouseLocation(), None);
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().as_mut() {
            p.menu_open = false;
            p.left = None;
        }
    });
}
fn menu_action(tag: isize) {
    let app = PANEL.with(|cell| cell.borrow().as_ref().map(|p| p.app.clone()));
    let Some(app) = app else { return };
    if tag == 0 {
        let _ = crate::desktop_access::open_settings(&app);
    } else if tag == 1 {
        let _ = super::configure(&app, json!({"resetDefaults":true}));
    } else if tag == 2 {
        let _ = super::configure(&app, json!({"visible":false}));
    }
}
fn emit(app: &tauri::AppHandle, p: &Panel) {
    let _=app.emit_to("usage-rail","usage-panel:pointer",json!({"expanded":p.expanded,"slot":p.slot,"providerId":p.provider_id,"generation":p.generation,"display":p.placement.display,"dock":p.placement.dock,"pressed":p.interaction.is_some(),"layout":p.layout.as_ref().map(Layout::value),"preferences":p.preferences}));
}
fn hit(value: &Value, x: f64, y: f64) -> bool {
    let Some(points) = value.as_array() else {
        return false;
    };
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = &points[points.len() - 1];
    for point in points {
        let ax = point[0].as_f64().unwrap_or(0.);
        let ay = point[1].as_f64().unwrap_or(0.);
        let bx = previous[0].as_f64().unwrap_or(0.);
        let by = previous[1].as_f64().unwrap_or(0.);
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
        previous = point;
    }
    inside
}

pub fn geometry(app: &tauri::AppHandle, body: Value) {
    if ["rail", "detail", "corridor", "controls"]
        .iter()
        .any(|key| {
            body[*key].as_array().is_none_or(|v| {
                v.len() > 512
                    || v.iter().any(|p| {
                        p.as_array().is_none_or(|a| {
                            a.len() != 2
                                || a.iter().any(|v| v.as_f64().is_none_or(|n| !n.is_finite()))
                        })
                    })
            })
        })
    {
        return;
    }
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().as_mut() {
            let s = p.native.frame().size;
            if body["generation"] == p.generation
                && body["display"] == p.placement.display
                && body["dock"] == p.placement.dock
                && body["width"].as_f64() == Some(s.width)
                && body["height"].as_f64() == Some(s.height)
            {
                p.geometry = body;
            }
        }
    });
    pointer(app);
}
pub fn ready(app: &tauri::AppHandle) {
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow().as_ref() {
            emit(app, p);
        }
    });
}
pub fn close() {
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().take() {
            unsafe {
                for monitor in &p.monitors {
                    NSEvent::removeMonitor(monitor);
                }
            }
            p.native.orderOut(None);
            p.native.setContentView(None);
            p.native.close();
        }
    });
}
pub(super) fn metric(name: &str) -> f64 {
    static METRICS: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    METRICS.get_or_init(|| {
        serde_json::from_str(include_str!("../../../shared/usage-panel.json"))
            .expect("panel metrics")
    })[name]
        .as_f64()
        .expect("panel metric")
}
