use super::*;
use radiant::{
    layout::{Point, Vector2},
    runtime::Event,
};

#[test]
fn surface_runtime_skips_stable_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(16.0, 16.0),
        }),
        Some(10)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(20.0, 20.0),
        }),
        Some(10)
    );

    let probe = motion_probe(&runtime, 10, "motion probe");
    assert_eq!(probe.moves, 1);
    assert!(probe.common.state.hovered);
}

#[test]
fn surface_runtime_preserves_stable_pointer_motion_for_continuous_widgets() {
    let bridge = pointer_motion_bridge(true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(16.0, 16.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = motion_probe(&runtime, 10, "motion probe");
    assert_eq!(probe.moves, 2);
}

#[test]
fn surface_runtime_keeps_captured_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(primary_press(Point::new(16.0, 16.0)));
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(18.0, 18.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = motion_probe(&runtime, 10, "motion probe");
    assert_eq!(probe.moves, 2);
    assert!(probe.common.state.pressed);
}

#[test]
fn surface_runtime_stable_pointer_motion_still_respects_higher_overlapping_widgets() {
    let mut runtime = SurfaceRuntime::new(OverlappingPointerBridge, Vector2::new(140.0, 60.0));

    let first = runtime.dispatch_pointer_move_with_outcome(Point::new(100.0, 20.0));
    assert_eq!(first.target, Some(10));
    assert!(first.hover_changed);

    let second = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert_eq!(second.target, Some(20));
    assert!(second.hover_changed);

    let lower = motion_probe(&runtime, 10, "lower probe");
    let upper = motion_probe(&runtime, 20, "upper probe");
    assert_eq!(
        lower.moves, 2,
        "lower widget receives enter and leave motion"
    );
    assert_eq!(
        upper.moves, 1,
        "upper widget receives motion after becoming top target"
    );
}
