//! Host-controlled frame sandbox for custom event loops and embedded renderers.

use radiant::prelude::*;
use radiant::{
    gui::types::{Point, Vector2},
    runtime::{Event, SurfaceRuntime, declarative_runtime_bridge},
    theme::ThemeTokens,
    widgets::PointerButton,
};
use std::sync::Arc;

fn main() {
    let report = run_host_surface_frame_demo();
    println!(
        "radiant_host_surface_frame target={:?} clicks={} viewport=({:.0},{:.0}) primitives={} fills={} text={}",
        report.target_widget,
        report.clicks,
        report.viewport.x,
        report.viewport.y,
        report.primitive_total,
        report.fills,
        report.text,
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct HostState {
    clicks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostMessage {
    Increment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HostFrameReport {
    target_widget: Option<u64>,
    clicks: usize,
    viewport: Vector2,
    primitive_total: usize,
    fills: usize,
    text: usize,
}

fn run_host_surface_frame_demo() -> HostFrameReport {
    let bridge = declarative_runtime_bridge(
        HostState::default(),
        |state: &mut HostState| {
            Arc::new(
                column([
                    text(format!("Host frames: {}", state.clicks))
                        .id(10)
                        .height(24.0)
                        .fill_width(),
                    button("Render")
                        .message(HostMessage::Increment)
                        .id(20)
                        .size(120.0, 32.0),
                ])
                .id(1)
                .padding(8.0)
                .spacing(8.0)
                .into_surface(),
            )
        },
        |state: &mut HostState, message| match message {
            HostMessage::Increment => state.clicks += 1,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let theme = ThemeTokens::default();
    let mut paint_plan = SurfacePaintPlan::empty(&theme);
    let point = Point::new(24.0, 48.0);

    runtime.dispatch_event(Event::PointerMove { position: point });
    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let target_widget = runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let frame = runtime.borrowed_frame_into(&theme, &mut paint_plan);
    let stats = frame.paint_plan.stats();

    HostFrameReport {
        target_widget,
        clicks: runtime.bridge().state().clicks,
        viewport: runtime.viewport(),
        primitive_total: stats.total,
        fills: stats.fills,
        text: stats.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_surface_frame_demo_dispatches_events_and_reports_frame_stats() {
        let report = run_host_surface_frame_demo();

        assert_eq!(report.target_widget, Some(20));
        assert_eq!(report.clicks, 1);
        assert_eq!(report.viewport, Vector2::new(220.0, 96.0));
        assert!(report.primitive_total > 0);
        assert!(report.fills > 0);
        assert!(report.text > 0);
    }
}
