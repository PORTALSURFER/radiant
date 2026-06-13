use super::{fixtures::*, shared::*};

#[test]
fn exclusive_capture_refresh_clears_retained_hover_on_other_widgets() {
    let mut core =
        GenericNativeRuntimeCore::new(ExclusiveCaptureHoverBridge, Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 91, "drag handle");
    let target_point = widget_point(&core, 92, "hover target");

    let _ = core.route_pointer_move(target_point);
    assert!(hover_fill_visible(&core, 92));

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_move(target_point);
    core.runtime.refresh();

    assert!(!hover_fill_visible(&core, 92));
    assert_eq!(core.runtime.pointer_capture(), Some(91));
}
