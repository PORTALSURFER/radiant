//! Native hit qualification for application-local drags.
//!
//! Translate an admitted source sample through native view/client coordinates
//! and ask the platform which window actually receives input at that point.
//! Geometry overlap alone cannot authorize a hidden background receiver.
use crate::gui::types::Point;
use winit::window::{Window, WindowId};

/// Unsupported backends keep ordinary surface-local routing. A supported
/// backend returning no hit instead means the sample has no admitted receiver.
pub(super) const SUPPORTED: bool = cfg!(any(target_os = "macos", target_os = "windows"));

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeDragHit {
    pub(super) window: WindowId,
    pub(super) position: Point,
}

/// Frozen native screen location of one admitted pointer sample. Keeping this
/// independent of window-local geometry prevents callbacks that move a window
/// from silently moving the original input sample too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeDragLocation {
    x: f64,
    y: f64,
}

#[cfg(test)]
pub(super) const fn test_drag_location() -> NativeDragLocation {
    NativeDragLocation { x: 0.0, y: 0.0 }
}

pub(super) fn capture_drag_location(
    source: &Window,
    position: Point,
) -> Option<NativeDragLocation> {
    if !position.is_finite() {
        return None;
    }
    platform::capture(source, position)
}

pub(super) fn hit_test_drag_location(
    location: NativeDragLocation,
    candidates: &[&Window],
) -> Option<NativeDragHit> {
    if !location.x.is_finite() || !location.y.is_finite() || candidates.len() > 64 {
        return None;
    }
    platform::hit(location, candidates)
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn checked_logical_point(x: f64, y: f64) -> Option<Point> {
    let point = Point::new(x as f32, y as f32);
    (x.is_finite() && y.is_finite() && point.is_finite()).then_some(point)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2_app_kit::{NSView, NSWindow};
    use objc2_foundation::{MainThreadMarker, NSPoint};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub(super) fn capture(source: &Window, position: Point) -> Option<NativeDragLocation> {
        let _main_thread = MainThreadMarker::new()?;
        let handle = source.window_handle().ok()?;
        let RawWindowHandle::AppKit(raw) = handle.as_raw() else {
            return None;
        };
        // The borrowed winit window handle keeps its NSView alive. AppKit work
        // is restricted to the main thread and no native reference escapes.
        let view = unsafe { raw.ns_view.cast::<NSView>().as_ref() };
        let window = view.window()?;
        let bounds = view.bounds();
        let local = NSPoint::new(
            bounds.origin.x + f64::from(position.x),
            if view.isFlipped() {
                bounds.origin.y + f64::from(position.y)
            } else {
                bounds.origin.y + bounds.size.height - f64::from(position.y)
            },
        );
        let screen = unsafe { window.convertPointToScreen(view.convertPoint_toView(local, None)) };
        if !screen.x.is_finite() || !screen.y.is_finite() {
            return None;
        }
        Some(NativeDragLocation {
            x: screen.x,
            y: screen.y,
        })
    }

    pub(super) fn hit(
        location: NativeDragLocation,
        candidates: &[&Window],
    ) -> Option<NativeDragHit> {
        let main_thread = MainThreadMarker::new()?;
        let screen = NSPoint::new(location.x, location.y);
        // Uses mouse-down hit rules, including foreign windows, transparent
        // regions and windows that ignore mouse events. Never guess z-order.
        let number = unsafe {
            NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(screen, 0, main_thread)
        };
        if number <= 0 {
            return None;
        }
        for candidate in candidates {
            let handle = candidate.window_handle().ok()?;
            let RawWindowHandle::AppKit(raw) = handle.as_raw() else {
                continue;
            };
            let view = unsafe { raw.ns_view.cast::<NSView>().as_ref() };
            let Some(window) = view.window() else {
                continue;
            };
            if unsafe { window.windowNumber() } != number {
                continue;
            }
            let local = view.convertPoint_fromView(window.convertPointFromScreen(screen), None);
            let bounds = view.bounds();
            let y = if view.isFlipped() {
                local.y - bounds.origin.y
            } else {
                bounds.origin.y + bounds.size.height - local.y
            };
            return Some(NativeDragHit {
                window: candidate.id(),
                position: checked_logical_point(local.x - bounds.origin.x, y)?,
            });
        }
        None
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
        UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, WindowFromPoint},
    };

    pub(super) fn capture(source: &Window, position: Point) -> Option<NativeDragLocation> {
        let handle = source.window_handle().ok()?;
        let RawWindowHandle::Win32(raw) = handle.as_raw() else {
            return None;
        };
        let scale = source.scale_factor();
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let pixel = |value: f32| {
            let value = (f64::from(value) * scale).round();
            (value.is_finite() && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX))
                .then_some(value as i32)
        };
        let mut screen = POINT {
            x: pixel(position.x)?,
            y: pixel(position.y)?,
        };
        if unsafe { ClientToScreen(raw.hwnd.get() as _, &mut screen) } == 0 {
            return None;
        }
        Some(NativeDragLocation {
            x: f64::from(screen.x),
            y: f64::from(screen.y),
        })
    }

    pub(super) fn hit(
        location: NativeDragLocation,
        candidates: &[&Window],
    ) -> Option<NativeDragHit> {
        let screen = POINT {
            x: location.x as i32,
            y: location.y as i32,
        };
        let hit = unsafe { WindowFromPoint(screen) };
        if hit.is_null() {
            return None;
        }
        let root = unsafe { GetAncestor(hit, GA_ROOT) };
        if root.is_null() {
            return None;
        }
        for candidate in candidates {
            let handle = candidate.window_handle().ok()?;
            let RawWindowHandle::Win32(raw) = handle.as_raw() else {
                continue;
            };
            let hwnd = raw.hwnd.get() as _;
            if hwnd != root {
                continue;
            }
            let mut local = screen;
            if unsafe { ScreenToClient(hwnd, &mut local) } == 0 {
                return None;
            }
            let scale = candidate.scale_factor();
            if !scale.is_finite() || scale <= 0.0 {
                return None;
            }
            return Some(NativeDragHit {
                window: candidate.id(),
                position: checked_logical_point(
                    f64::from(local.x) / scale,
                    f64::from(local.y) / scale,
                )?,
            });
        }
        None
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;
    pub(super) fn capture(_: &Window, _: Point) -> Option<NativeDragLocation> {
        None
    }
    pub(super) fn hit(_: NativeDragLocation, _: &[&Window]) -> Option<NativeDragHit> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_points_reject_overflow_and_nonfinite_coordinates() {
        assert_eq!(
            checked_logical_point(-12.5, 40.0),
            Some(Point::new(-12.5, 40.0))
        );
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
            assert!(checked_logical_point(value, 0.0).is_none());
            assert!(checked_logical_point(0.0, value).is_none());
        }
    }
}
