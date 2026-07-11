use super::*;

#[test]
fn interactive_row_underlay_configures_tracked_drop_target() {
    let surface = interactive_row_underlay(text("Collection"))
        .tracked_drop_target(true, true)
        .input_id(772)
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();
    let _ = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let row = surface
        .find_widget(772)
        .and_then(|widget| {
            widget
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::InteractiveRowWidget>()
        })
        .expect("underlay should preserve the configured input row");

    assert!(row.props.droppable);
    assert!(row.props.drag_active);
    assert!(!row.props.drop_hover);
    assert!(row.props.pointer_motion_active);
}

#[test]
fn interactive_row_underlay_configures_tracked_drop_candidate_clear() {
    let surface = interactive_row_underlay(text("Collection"))
        .tracked_drop_candidate(true, false, false, true)
        .input_id(775)
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();
    let _ = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let row = surface
        .find_widget(775)
        .and_then(|widget| {
            widget
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::InteractiveRowWidget>()
        })
        .expect("underlay should preserve the configured input row");

    assert!(row.props.droppable);
    assert!(row.props.drag_active);
    assert!(row.props.drop_hover);
    assert!(row.props.clear_drop_on_hover);
    assert!(row.props.pointer_motion_active);
}

#[test]
fn interactive_row_actions_route_keyed_drop_targets() {
    let target = String::from("target-a");
    let actions =
        row_actions().drop_target_key(target, DemoMessage::DropKey, DemoMessage::HoverDropKey);
    let hover = Point::new(6.0, 12.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Drop),
        Some(DemoMessage::DropKey(String::from("target-a")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::HoverDropTarget { position: hover }),
        Some(DemoMessage::HoverDropKey(String::from("target-a"), hover))
    );
}

#[test]
fn tracked_drop_target_accepts_drop_without_repeating_hover_for_active_target() {
    let view = interactive_row().tracked_drop_target(true, true).widget();

    assert!(view.props.droppable);
    assert!(view.props.drag_active);
    assert!(!view.props.drop_hover);
    assert!(view.props.pointer_motion_active);
    assert_eq!(
        view.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
}

#[test]
fn tracked_drop_target_emits_hover_for_candidate_target() {
    let view = interactive_row().tracked_drop_target(true, false).widget();

    assert!(view.props.droppable);
    assert!(view.props.drag_active);
    assert!(view.props.drop_hover);
    assert!(!view.props.pointer_motion_active);
    assert_eq!(
        view.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
}

#[test]
fn tracked_drop_candidate_suppresses_hover_for_current_target() {
    let view = interactive_row()
        .tracked_drop_candidate(true, true, true, true)
        .widget();

    assert!(view.props.droppable);
    assert!(view.props.drag_active);
    assert!(!view.props.drop_hover);
    assert!(!view.props.clear_drop_on_hover);
    assert!(view.props.pointer_motion_active);
    assert_eq!(
        view.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
}

#[test]
fn tracked_drop_candidate_reports_hover_for_new_candidate() {
    let view = interactive_row()
        .tracked_drop_candidate(true, false, true, false)
        .widget();

    assert!(view.props.droppable);
    assert!(view.props.drag_active);
    assert!(view.props.drop_hover);
    assert!(!view.props.clear_drop_on_hover);
    assert!(!view.props.pointer_motion_active);
}

#[test]
fn tracked_drop_candidate_reports_hover_to_clear_active_target() {
    let view = interactive_row()
        .tracked_drop_candidate(true, false, false, true)
        .widget();

    assert!(view.props.droppable);
    assert!(view.props.drag_active);
    assert!(view.props.drop_hover);
    assert!(view.props.clear_drop_on_hover);
    assert!(view.props.pointer_motion_active);
}

#[test]
fn tracked_drag_source_sets_drag_and_motion_policy() {
    let view = interactive_row().tracked_drag_source(true, true).widget();

    assert!(view.props.draggable);
    assert!(view.props.drag_active);
    assert!(view.props.drag_source);
    assert_eq!(
        view.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
}

#[test]
fn tracked_drag_source_with_motion_emits_retained_source_motion() {
    let view = interactive_row()
        .tracked_drag_source_with_motion(true, true)
        .widget();

    assert!(view.props.draggable);
    assert!(view.props.drag_active);
    assert!(view.props.drag_source);
    assert!(view.props.drag_source_motion);
    assert_eq!(
        view.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
}
