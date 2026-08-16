//! Deterministic runtime-owned split-pane divider interaction sandbox.

use radiant::gui::types::{Point, Vector2};
use radiant::prelude::*;
use radiant::runtime::{Event, SurfaceRuntime, declarative_owned_runtime_bridge};
use radiant::widgets::PointerButton;

#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeSplitReport {
    initial_divider: Option<radiant::layout::Rect>,
    moved_first_width: f32,
    capture_after_press: bool,
    capture_after_release: bool,
}

fn run_runtime_split_demo() -> RuntimeSplitReport {
    let bridge = declarative_owned_runtime_bridge(
        (),
        |_state: &mut ()| {
            split_pane(text("First pane"), text("Second pane"))
                .initial_ratio(0.25)
                .divider_extent(8.0)
                .runtime_owned_ratio()
                .into_surface()
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
    let initial_divider = runtime
        .layout_target_at(Point::new(52.0, 40.0))
        .map(|target| target.bounds);
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    let capture_after_press = runtime.layout_pointer_capture().is_some();
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    let moved_first_width = runtime
        .layout()
        .rects
        .values()
        .copied()
        .filter(|rect| rect.min.x == 0.0 && rect.width() > 0.0 && rect.width() < 200.0)
        .map(|rect| rect.width())
        .next()
        .unwrap_or_default();
    runtime.dispatch_event(Event::pointer_release(
        Point::new(130.0, 100.0),
        PointerButton::Primary,
        Default::default(),
    ));

    RuntimeSplitReport {
        initial_divider,
        moved_first_width,
        capture_after_press,
        capture_after_release: runtime.layout_pointer_capture().is_some(),
    }
}

fn main() {
    let report = run_runtime_split_demo();
    println!(
        "runtime split divider={:?} moved_first_width={:.0} capture_press={} capture_release={}",
        report.initial_divider,
        report.moved_first_width,
        report.capture_after_press,
        report.capture_after_release,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_split_example_reports_capture_and_live_ratio() {
        let report = run_runtime_split_demo();
        assert_eq!(
            report.initial_divider,
            Some(radiant::layout::Rect::from_xy_size(48.0, 0.0, 8.0, 80.0))
        );
        assert_eq!(report.moved_first_width, 130.0);
        assert!(report.capture_after_press);
        assert!(!report.capture_after_release);
    }
}
