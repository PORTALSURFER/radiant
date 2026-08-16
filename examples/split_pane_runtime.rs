//! Deterministic runtime-owned split-pane divider interaction sandbox.

use radiant::gui::types::{Point, Vector2};
use radiant::layout::SplitPaneCollapsePolicy;
use radiant::prelude::*;
use radiant::runtime::{Event, SurfaceRuntime, declarative_owned_runtime_bridge};
use radiant::widgets::PointerButton;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
enum RuntimeSplitMessage {
    RatioSettled(f32),
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeSplitReport {
    initial_divider: Option<radiant::layout::Rect>,
    moved_first_width: f32,
    collapsed_first_width: f32,
    restored_first_width: f32,
    capture_after_press: bool,
    capture_after_release: bool,
    settled_ratios: Vec<f32>,
}

fn run_runtime_split_demo() -> RuntimeSplitReport {
    let settled_ratios = Rc::new(RefCell::new(Vec::new()));
    let reduced_settled_ratios = Rc::clone(&settled_ratios);
    let bridge = declarative_owned_runtime_bridge(
        (),
        |_state: &mut ()| {
            split_pane::<RuntimeSplitMessage>(text("First pane").id(2), text("Second pane").id(3))
                .initial_ratio(0.25)
                .divider_extent(8.0)
                .runtime_owned_ratio()
                .collapse_policy(SplitPaneCollapsePolicy::FirstPane)
                .on_ratio_settled(RuntimeSplitMessage::RatioSettled)
                .into_surface()
        },
        move |_state: &mut (), message| match message {
            RuntimeSplitMessage::RatioSettled(ratio) => {
                // The application owns the settled value/history; Radiant only
                // reports the discrete runtime interaction result.
                reduced_settled_ratios.borrow_mut().push(ratio);
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
    let initial_divider = runtime
        .layout_target_at(Point::new(52.0, 40.0))
        .map(|target| target.bounds);
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    let capture_after_press = runtime.layout_pointer_capture().is_some();
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    let moved_first_width = runtime.layout().rects[&2].width();
    runtime.dispatch_event(Event::pointer_release(
        Point::new(130.0, 100.0),
        PointerButton::Primary,
        Default::default(),
    ));
    runtime.dispatch_event(Event::primary_double_click(Point::new(134.0, 40.0)));
    let collapsed_first_width = runtime.layout().rects[&2].width();
    runtime.dispatch_event(Event::primary_double_click(Point::new(4.0, 40.0)));
    let restored_first_width = runtime.layout().rects[&2].width();

    RuntimeSplitReport {
        initial_divider,
        moved_first_width,
        collapsed_first_width,
        restored_first_width,
        capture_after_press,
        capture_after_release: runtime.layout_pointer_capture().is_some(),
        settled_ratios: settled_ratios.borrow().clone(),
    }
}

fn main() {
    let report = run_runtime_split_demo();
    println!(
        "runtime split divider={:?} moved_first_width={:.0} collapsed_first_width={:.0} restored_first_width={:.0} capture_press={} capture_release={} settled_ratios={:?}",
        report.initial_divider,
        report.moved_first_width,
        report.collapsed_first_width,
        report.restored_first_width,
        report.capture_after_press,
        report.capture_after_release,
        report.settled_ratios,
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
        assert_eq!(report.collapsed_first_width, 0.0);
        assert_eq!(report.restored_first_width, 130.0);
        assert!(report.capture_after_press);
        assert!(!report.capture_after_release);
        assert_eq!(
            report.settled_ratios,
            vec![130.0_f32 / 192.0_f32, 0.0, 130.0_f32 / 192.0_f32,]
        );
    }
}
