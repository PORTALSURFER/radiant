use super::*;

#[test]
fn interactive_row_underlay_preserves_input_widget_identity() {
    let view = interactive_row_underlay(text("Collection"))
        .input_id(770)
        .filter_mapped(|message| {
            message
                .is_single_activation()
                .then_some(DemoMessage::Activate)
        })
        .size(140.0, 22.0);

    assert_eq!(
        view.view_dispatch_widget_output(770, WidgetOutput::typed(InteractiveRowMessage::Activate)),
        Some(DemoMessage::Activate)
    );
}

#[test]
fn interactive_row_underlay_exposes_common_input_presets() {
    let surface = interactive_row_underlay(text("Sample"))
        .custom_paint_hit_target()
        .activation_modifiers()
        .tracked_drag_source(true, true)
        .input_id(774)
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();
    let _ = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let row = surface
        .find_widget(774)
        .and_then(|widget| {
            widget
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::InteractiveRowWidget>()
        })
        .expect("underlay should preserve the configured input row");

    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.activation_modifiers);
    assert_eq!(
        row.props.pointer_motion,
        crate::widgets::InteractiveRowPointerMotion::DuringInteraction
    );
    assert_eq!(row.common.focus, crate::widgets::FocusBehavior::None);
    assert!(!row.common.paint.paints_state_layers);
}

#[test]
fn interactive_row_underlay_applies_dense_row_policy_input_bundle() {
    let surface = interactive_row_underlay(text("Sample"))
        .dense_row_policy(
            DenseRowPolicy::new()
                .custom_paint_hit_target()
                .activation_modifiers()
                .drag_session_motion(true)
                .tracked_drag_source_with_motion(true, true),
        )
        .input_id(776)
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();
    let _ = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let row = surface
        .find_widget(776)
        .and_then(|widget| {
            widget
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::InteractiveRowWidget>()
        })
        .expect("underlay should preserve the configured input row");

    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.drag_source_motion);
    assert!(row.props.pointer_motion_active);
    assert!(row.props.activation_modifiers);
    assert_eq!(row.common.focus, crate::widgets::FocusBehavior::None);
    assert!(!row.common.paint.paints_state_layers);
}

#[test]
fn interactive_row_underlay_derives_stable_text_input_id() {
    let view = interactive_row_underlay(text("Source"))
        .stable_input_id(42, "source-a")
        .mapped(|_| DemoMessage::Activate)
        .size(140.0, 22.0);
    let input_id = crate::widgets::stable_widget_id(42, "source-a");

    assert_eq!(
        view.view_dispatch_widget_output(
            input_id,
            WidgetOutput::typed(InteractiveRowMessage::Activate),
        ),
        Some(DemoMessage::Activate)
    );
}

#[test]
fn interactive_row_underlay_stable_row_identity_keys_row_and_input() {
    let row_key = "source-row-source-a";
    fn keyed_row(row_key: &'static str) -> ViewNode<DemoMessage> {
        interactive_row_underlay(text("Source"))
            .stable_row_identity(42, row_key)
            .mapped(|_| DemoMessage::Activate)
            .size(140.0, 22.0)
    }
    let input_id = crate::widgets::stable_widget_id(42, row_key);

    assert_eq!(
        keyed_row(row_key).view_dispatch_widget_output(
            input_id,
            WidgetOutput::typed(InteractiveRowMessage::Activate),
        ),
        Some(DemoMessage::Activate)
    );
    let layout = keyed_row(row_key).view_layout_at_size(Vector2::new(140.0, 22.0));
    let root_id = crate::application::scoped_key_id(crate::application::ROOT_KEY_SCOPE, row_key);

    assert!(
        layout.rects.contains_key(&root_id),
        "stable row identity should key the composed row subtree"
    );
}

#[test]
fn interactive_row_underlay_derives_stable_numeric_input_id() {
    let view = interactive_row_underlay(text("Collection"))
        .stable_u64_input_id(43, 7)
        .mapped(|_| DemoMessage::Activate)
        .size(140.0, 22.0);
    let input_id = crate::widgets::stable_widget_id_u64(43, 7);

    assert_eq!(
        view.view_dispatch_widget_output(
            input_id,
            WidgetOutput::typed(InteractiveRowMessage::Activate),
        ),
        Some(DemoMessage::Activate)
    );
}
