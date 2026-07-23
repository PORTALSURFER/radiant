use super::*;
use crate::{
    application::{IntoView, app, column, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, SurfaceRuntime, UiSurface},
    widgets::{InteractiveRowActions, PointerButton, PointerModifiers, WidgetInput},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Activate,
    ClearDrop,
    Drag,
    Secondary,
}

#[derive(Default)]
struct DemoState {
    status: &'static str,
}

#[test]
fn interactive_badge_routes_row_interactions_through_badge_visual() {
    let bridge = app(DemoState::default())
        .view(|state| {
            column([
                interactive_badge("Tag")
                    .filter_mapped(|message| {
                        if message.secondary_position().is_some() {
                            return Some(DemoMessage::Secondary);
                        }
                        if message.is_single_activation() {
                            return Some(DemoMessage::Activate);
                        }
                        None
                    })
                    .width(80.0)
                    .height(22.0),
                text(state.status).id(330).height(22.0),
            ])
            .spacing(0.0)
        })
        .update(|state, message| {
            state.status = match message {
                DemoMessage::Activate => "activated",
                DemoMessage::ClearDrop => "cleared",
                DemoMessage::Drag => "dragged",
                DemoMessage::Secondary => "secondary",
            };
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 44.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        runtime
            .surface()
            .find_widget(330)
            .and_then(|widget| widget
                .widget_object()
                .as_any()
                .downcast_ref::<crate::widgets::TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("secondary")
    );
}

#[test]
fn interactive_badge_routes_common_row_actions() {
    let bridge = app(DemoState::default())
        .view(|state| {
            column([
                interactive_badge("Tag")
                    .actions(InteractiveRowActions::new().activate(|| DemoMessage::Activate))
                    .width(80.0)
                    .height(22.0),
                text(state.status).id(331).height(22.0),
            ])
            .spacing(0.0)
        })
        .update(|state, message| {
            state.status = match message {
                DemoMessage::Activate => "activated",
                DemoMessage::ClearDrop => "cleared",
                DemoMessage::Drag => "dragged",
                DemoMessage::Secondary => "secondary",
            };
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 44.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        runtime
            .surface()
            .find_widget(331)
            .and_then(|widget| widget
                .widget_object()
                .as_any()
                .downcast_ref::<crate::widgets::TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("activated")
    );
}

#[test]
fn interactive_badge_tracked_drag_source_with_motion_routes_source_moves() {
    let bridge = app(DemoState::default())
        .view(|state| {
            column([
                interactive_badge("Tag")
                    .tracked_drag_source_with_motion(true, true)
                    .actions(
                        InteractiveRowActions::new()
                            .drag(|_| DemoMessage::Drag)
                            .activate(|| DemoMessage::Activate),
                    )
                    .width(80.0)
                    .height(22.0),
                text(state.status).id(332).height(22.0),
            ])
            .spacing(0.0)
        })
        .update(|state, message| {
            state.status = match message {
                DemoMessage::Activate => "activated",
                DemoMessage::ClearDrop => "cleared",
                DemoMessage::Drag => "dragged",
                DemoMessage::Secondary => "secondary",
            };
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 44.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(position, WidgetInput::PointerMove { position });

    assert_eq!(
        runtime
            .surface()
            .find_widget(332)
            .and_then(|widget| widget
                .widget_object()
                .as_any()
                .downcast_ref::<crate::widgets::TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("dragged")
    );
}

#[test]
fn interactive_badge_tracked_drop_candidate_routes_stale_target_clear() {
    let bridge = app(DemoState::default())
        .view(|state| {
            column([
                interactive_badge("Tag")
                    .tracked_drop_candidate(true, false, false, true)
                    .actions(InteractiveRowActions::new().clear_drop(|_| DemoMessage::ClearDrop))
                    .width(80.0)
                    .height(22.0),
                text(state.status).id(333).height(22.0),
            ])
            .spacing(0.0)
        })
        .update(|state, message| {
            state.status = match message {
                DemoMessage::Activate => "activated",
                DemoMessage::ClearDrop => "cleared",
                DemoMessage::Drag => "dragged",
                DemoMessage::Secondary => "secondary",
            };
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 44.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(position, WidgetInput::PointerMove { position });

    assert_eq!(
        runtime
            .surface()
            .find_widget(333)
            .and_then(|widget| widget
                .widget_object()
                .as_any()
                .downcast_ref::<crate::widgets::TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("cleared")
    );
}

#[test]
fn interactive_badge_paints_badge_content() {
    let frame = UiSurface::new(
        interactive_badge("Project")
            .primary()
            .active(true)
            .mapped(|_| ())
            .size(100.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 22.0)),
        &Default::default(),
    );

    let paints_label =
        frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Project"),
        );

    assert!(
        paints_label,
        "interactive badge should paint its badge label"
    );
}

#[test]
fn badge_builder_passive_paints_without_host_message() {
    let frame = UiSurface::new(
        badge("Passive")
            .subtle()
            .passive::<()>()
            .size(100.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 22.0)),
        &Default::default(),
    );

    let paints_label =
        frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Passive"),
        );

    assert!(paints_label, "passive badge should paint its label");
}

#[test]
fn outlined_badge_keeps_background_fill_and_paints_accent_border() {
    let theme = crate::theme::ThemeTokens::default();
    let frame = UiSurface::new(
        badge("ONE-SHOT ×")
            .primary()
            .active(true)
            .outline()
            .passive::<()>()
            .size(100.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 22.0)),
        &theme,
    );

    assert!(frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == theme.bg_primary)
    ));
    assert!(frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::StrokeRect(stroke) if stroke.color != theme.bg_primary && stroke.width == 1.0)
    ));
    assert!(frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "ONE-SHOT ×" && text.color == theme.text_primary)
    ));
}
