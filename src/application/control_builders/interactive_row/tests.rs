use super::*;
use crate::{
    application::{IntoView, ViewNode, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, UiSurface},
    widgets::{InteractiveRowMessage, WidgetOutput},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Activate,
    ActivateKey(String),
    ActivateWithModifiers(crate::widgets::PointerModifiers),
    ActivateWithModifiersKey(String, crate::widgets::PointerModifiers),
    DoubleActivate,
    DoubleActivateKey(String),
    DragKey(String, crate::widgets::DragHandleMessage),
    Drop,
    DropKey(String),
    HoverDrop(Point),
    HoverDropKey(String, Point),
    Secondary(Point),
    SecondaryKey(String, Point),
}

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

#[test]
fn interactive_row_actions_route_common_row_messages() {
    fn action_row() -> ViewNode<DemoMessage> {
        interactive_row_underlay(text("Collection"))
            .input_id(771)
            .actions(
                InteractiveRowActions::new()
                    .activate(|| DemoMessage::Activate)
                    .double_activate(|| DemoMessage::DoubleActivate)
                    .drop(|| DemoMessage::Drop)
                    .hover_drop(DemoMessage::HoverDrop)
                    .secondary(DemoMessage::Secondary),
            )
            .size(140.0, 22.0)
    }

    let hover = Point::new(4.0, 9.0);
    let secondary = Point::new(10.0, 12.0);

    assert_eq!(
        action_row()
            .view_dispatch_widget_output(771, WidgetOutput::typed(InteractiveRowMessage::Drop),),
        Some(DemoMessage::Drop)
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::HoverDropTarget { position: hover }),
        ),
        Some(DemoMessage::HoverDrop(hover))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::SecondaryActivate {
                position: secondary,
            }),
        ),
        Some(DemoMessage::Secondary(secondary))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::DoubleActivate),
        ),
        Some(DemoMessage::DoubleActivate)
    );
}

#[test]
fn interactive_row_actions_route_modifier_activation_for_embedded_rows() {
    let modifiers = crate::widgets::PointerModifiers {
        shift: true,
        command: true,
        ..crate::widgets::PointerModifiers::default()
    };
    let actions = InteractiveRowActions::new()
        .activate(|| DemoMessage::Activate)
        .activate_with_modifiers(DemoMessage::ActivateWithModifiers);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(DemoMessage::ActivateWithModifiers(
            crate::widgets::PointerModifiers::default()
        ))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(DemoMessage::ActivateWithModifiers(modifiers))
    );
}

#[test]
fn interactive_row_actions_route_keyed_activation_and_secondary_actions() {
    let actions = InteractiveRowActions::new()
        .activate_key(String::from("target-a"), DemoMessage::ActivateKey)
        .double_activate_key(String::from("target-b"), DemoMessage::DoubleActivateKey)
        .secondary_key(String::from("target-c"), DemoMessage::SecondaryKey);
    let secondary = Point::new(8.0, 14.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(DemoMessage::ActivateKey(String::from("target-a")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some(DemoMessage::DoubleActivateKey(String::from("target-b")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate {
            position: secondary
        }),
        Some(DemoMessage::SecondaryKey(
            String::from("target-c"),
            secondary
        ))
    );
}

#[test]
fn interactive_row_actions_route_keyed_modifier_activation_and_drag() {
    let modifiers = crate::widgets::PointerModifiers {
        alt: true,
        ..crate::widgets::PointerModifiers::default()
    };
    let drag = crate::widgets::DragHandleMessage::moved(Point::new(4.0, 6.0));
    let actions = InteractiveRowActions::new()
        .activate_with_modifiers_key(
            String::from("target-a"),
            DemoMessage::ActivateWithModifiersKey,
        )
        .drag_key(String::from("target-b"), DemoMessage::DragKey);

    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(DemoMessage::ActivateWithModifiersKey(
            String::from("target-a"),
            modifiers
        ))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::Drag(drag)),
        Some(DemoMessage::DragKey(String::from("target-b"), drag))
    );
}

#[test]
fn interactive_row_actions_route_keyed_drop_targets() {
    let target = String::from("target-a");
    let actions = row_actions()
        .drop_key(target.clone(), DemoMessage::DropKey)
        .hover_drop_key(target, DemoMessage::HoverDropKey);
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

#[test]
fn interactive_row_underlay_paints_visible_content() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let paints_label = frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Collection"),
    );

    assert!(
        paints_label,
        "interactive row underlay should paint visible content"
    );
}

#[test]
fn interactive_row_underlay_dense_chrome_paints_selected_state() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .selected(true)
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(120)),
        "dense underlay should paint the selected row fill"
    );
    assert!(
        frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Collection"),
        ),
        "dense underlay should keep app-owned visible content above the row chrome"
    );
}

#[test]
fn interactive_row_underlay_dense_chrome_paints_active_target_state() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .tracked_drop_target(true, true)
            .dense_chrome()
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(220)),
        "dense underlay should paint the active drop-target fill"
    );
}
