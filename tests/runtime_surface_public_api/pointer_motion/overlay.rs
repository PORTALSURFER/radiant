use super::*;
use radiant::layout::{Point, Vector2};

#[test]
fn surface_runtime_reports_paint_only_pointer_overlay_outcomes() {
    let bridge = pointer_motion_bridge_with_policy(true, true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let first = runtime.dispatch_pointer_move_with_outcome(Point::new(16.0, 16.0));
    assert!(first.routed());
    assert!(first.hover_changed);
    assert!(first.needs_scene_rebuild());

    let second = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert!(second.routed());
    assert!(!second.hover_changed);
    assert!(!second.pointer_captured);
    assert!(second.paint_only_requested);
    assert!(!second.repaint_requested);
    assert!(!second.needs_scene_rebuild());
    assert!(second.needs_redraw());

    let probe = motion_probe(&runtime, 10, "motion probe");
    assert_eq!(probe.moves, 2);
}

#[test]
fn surface_runtime_reports_captured_paint_only_pointer_overlay_outcomes() {
    let bridge = pointer_motion_bridge_with_policy(true, true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(primary_press(Point::new(16.0, 16.0)));
    let enter_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(18.0, 18.0));
    assert!(enter_drag.routed());
    assert!(enter_drag.pointer_captured);
    assert!(enter_drag.needs_scene_rebuild());

    let preview_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));
    assert!(preview_drag.routed());
    assert!(preview_drag.pointer_captured);
    assert!(preview_drag.paint_only_requested);
    assert!(!preview_drag.repaint_requested);
    assert!(!preview_drag.needs_scene_rebuild());
    assert!(preview_drag.needs_redraw());
}

#[test]
fn surface_runtime_keeps_captured_non_overlay_pointer_motion_on_scene_rebuild_path() {
    let bridge = pointer_motion_bridge_with_policy(true, false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(primary_press(Point::new(16.0, 16.0)));
    let _ = runtime.dispatch_pointer_move_with_outcome(Point::new(18.0, 18.0));
    let preview_drag = runtime.dispatch_pointer_move_with_outcome(Point::new(20.0, 20.0));

    assert!(preview_drag.routed());
    assert!(preview_drag.pointer_captured);
    assert!(!preview_drag.paint_only_requested);
    assert!(preview_drag.repaint_requested);
    assert!(preview_drag.needs_scene_rebuild());
}
