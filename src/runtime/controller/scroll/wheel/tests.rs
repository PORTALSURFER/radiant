use super::{ScrollUpdateMetadata, WheelWidgetDispatch};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface},
    widgets::{PointerModifiers, WheelDelta, WheelPhase, WheelSample, WidgetSizing},
};
use std::{cell::Cell, sync::Arc, time::Duration};

struct WheelTestBridge;

impl RuntimeBridge<()> for WheelTestBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::scroll_area(
            1,
            SurfaceNode::text(
                2,
                "scroll",
                crate::widgets::WidgetSizing::fixed(Vector2::new(120.0, 400.0)),
            ),
        )))
    }
}

#[derive(Clone, Copy)]
enum SettlementMessage {
    A,
    B,
}

struct SettlementTraceBridge {
    settled_a: Arc<Cell<usize>>,
    settled_b: Arc<Cell<usize>>,
}

impl RuntimeBridge<SettlementMessage> for SettlementTraceBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<SettlementMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(
                    SurfaceNode::scroll_area(
                        31,
                        SurfaceNode::text(32, "A", WidgetSizing::fixed(Vector2::new(120.0, 400.0))),
                    )
                    .on_offset_settled(move |_| SettlementMessage::A),
                ),
                SurfaceChild::fill(
                    SurfaceNode::scroll_area(
                        41,
                        SurfaceNode::text(42, "B", WidgetSizing::fixed(Vector2::new(120.0, 400.0))),
                    )
                    .on_offset_settled(move |_| SettlementMessage::B),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, message: SettlementMessage) {
        match message {
            SettlementMessage::A => self.settled_a.set(self.settled_a.get() + 1),
            SettlementMessage::B => self.settled_b.set(self.settled_b.get() + 1),
        }
    }
}

#[test]
fn retained_no_output_is_distinct_from_unhandled_fallback() {
    assert!(WheelWidgetDispatch::RetainedNoOutput.retained());
    assert!(!WheelWidgetDispatch::Unhandled.retained());
    assert!(!WheelWidgetDispatch::Handled { retained: false }.retained());
    assert!(WheelWidgetDispatch::Handled { retained: true }.retained());
    assert_ne!(
        WheelWidgetDispatch::RetainedNoOutput,
        WheelWidgetDispatch::Unhandled
    );
}

#[test]
fn exact_sample_preserves_units_phase_and_scroll_metadata() {
    let modifiers = PointerModifiers {
        command: true,
        shift: false,
        alt: true,
    };
    let sample = WheelSample::new_with_metadata(
        WheelDelta::Lines(Vector2::new(0.25, -2.0)),
        Some(WheelPhase::Changed),
        modifiers,
        None,
        None,
    )
    .expect("finite line sample");

    assert_eq!(sample.delta(), WheelDelta::Lines(Vector2::new(0.25, -2.0)));
    assert_eq!(sample.phase(), Some(WheelPhase::Changed));
    assert_eq!(sample.modifiers(), modifiers);
    assert_eq!(
        sample.delta().to_logical_pixels(),
        Some(Vector2::new(10.0, -80.0))
    );
    assert_eq!(
        ScrollUpdateMetadata::from(sample),
        ScrollUpdateMetadata {
            modifiers,
            timestamp: None,
            sequence_range: None,
        }
    );
}

#[test]
fn all_explicit_phases_and_phase_less_samples_are_constructible() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel sample");
    for phase in [
        None,
        Some(WheelPhase::Started),
        Some(WheelPhase::Changed),
        Some(WheelPhase::Ended),
        Some(WheelPhase::Cancelled),
        Some(WheelPhase::Discrete),
    ] {
        assert_eq!(
            WheelSample::new(delta, phase, PointerModifiers::default())
                .expect("phase should be accepted")
                .phase(),
            phase
        );
    }
}

#[test]
fn invalid_non_started_samples_are_rejected_before_scroll_routing() {
    let mut runtime = SurfaceRuntime::new(WheelTestBridge, Vector2::new(120.0, 80.0));
    let point = crate::gui::types::Point::new(10.0, 10.0);
    let valid = WheelSample::phase_less(
        WheelDelta::Pixels(Vector2::new(0.0, 8.0)),
        PointerModifiers::default(),
    )
    .expect("finite phase-less sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point, valid));
    let before = runtime.layout().rects[&2];

    let invalid = WheelSample::from_parts(
        WheelDelta::Pixels(Vector2::new(f32::NAN, 8.0)),
        Some(WheelPhase::Changed),
        PointerModifiers::default(),
        None,
        None,
    );
    assert!(!runtime.wheel_or_scroll_at_with_sample(point, invalid));
    assert_eq!(runtime.layout().rects[&2], before);
}

#[test]
fn replacement_started_settles_a_before_old_deadline_and_keeps_b_pending() {
    let settled_a = Arc::new(Cell::new(0));
    let settled_b = Arc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        SettlementTraceBridge {
            settled_a: Arc::clone(&settled_a),
            settled_b: Arc::clone(&settled_b),
        },
        Vector2::new(120.0, 80.0),
    );
    let delta = WheelDelta::Pixels(Vector2::new(0.0, 8.0));
    let phase_less = WheelSample::phase_less(delta, PointerModifiers::default())
        .expect("finite phase-less sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(Point::new(10.0, 10.0), phase_less));

    let started =
        WheelSample::started(delta, PointerModifiers::default()).expect("finite started sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(Point::new(10.0, 50.0), started));
    assert_eq!(settled_a.get(), 1);
    assert_eq!(settled_b.get(), 0);

    assert!(runtime.advance_timed_repaints(std::time::Instant::now() + Duration::from_secs(1)));
    assert_eq!(
        settled_a.get(),
        1,
        "A's old deadline cannot settle it again"
    );
    assert_eq!(
        settled_b.get(),
        0,
        "B remains pending while its sequence is active"
    );

    let changed =
        WheelSample::changed(delta, PointerModifiers::default()).expect("finite changed sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(Point::new(10.0, 50.0), changed));
    assert_eq!(settled_b.get(), 0);
    let ended =
        WheelSample::ended(delta, PointerModifiers::default()).expect("finite ended sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(Point::new(10.0, 50.0), ended));
    assert_eq!(settled_a.get(), 1);
    assert_eq!(settled_b.get(), 1);
}
