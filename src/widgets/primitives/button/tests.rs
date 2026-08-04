use crate::gui::svg::{IconName, SvgIcon};
use crate::gui::{
    input::{InputSequence, InputSequenceRange, InputTimestamp},
    types::{Point, Vector2},
};
use crate::widgets::contract::WidgetState;
use crate::widgets::interaction::{
    DragHandleMessage, DragHandleMetadata, PointerButton, PointerModifiers, WidgetInput, WidgetKey,
};
use std::sync::Arc;

use super::*;

#[test]
fn button_releases_inside_bounds_emit_activation() {
    let mut button = ButtonWidget::new(5, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(80.0, 28.0));

    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert!(button.common.state.pressed);

    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(24.0, 32.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        Some(ButtonMessage::Activate)
    );
    assert!(!button.common.state.pressed);
}

#[test]
fn focused_button_space_emits_activation() {
    let mut button = ButtonWidget::new(6, "Stop", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

    let _ = button.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

    assert_eq!(
        button.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Space)),
        Some(ButtonMessage::Activate)
    );
}

#[test]
fn secondary_click_only_emits_when_enabled() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut default_button =
        ButtonWidget::new(7, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let mut context_button =
        ButtonWidget::new(8, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
            .with_secondary_click();

    let secondary_press = WidgetInput::PointerPress {
        position: Point::new(10.0, 10.0),
        button: PointerButton::Secondary,
        modifiers: Default::default(),
        timestamp: None,
    };

    assert_eq!(
        default_button.handle_input(bounds, secondary_press.clone()),
        None
    );
    assert_eq!(
        context_button.handle_input(bounds, secondary_press),
        Some(ButtonMessage::SecondaryActivate {
            position: Point::new(10.0, 10.0),
        })
    );
}

#[test]
fn draggable_button_emits_drag_lifecycle_instead_of_click_when_moved() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(9, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();

    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(Point::new(12.0, 14.0)),),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: Point::new(10.0, 10.0),
            position: Point::new(12.0, 14.0),
            metadata: DragHandleMetadata::empty(),
        }))
    );
    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(20.0, 22.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        Some(ButtonMessage::Drag(DragHandleMessage::Ended {
            position: Point::new(20.0, 22.0),
            metadata: DragHandleMetadata::empty(),
        }))
    );
}

#[test]
fn draggable_button_sanitizes_threshold_start_sequence_range_but_preserves_moved_range() {
    let bounds = Rect::from_size(80.0, 28.0);
    let mut button =
        ButtonWidget::new(18, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let origin = Point::new(10.0, 10.0);
    let threshold = Point::new(14.0, 10.0);
    let moved = Point::new(18.0, 12.0);
    let modifiers = PointerModifiers {
        command: true,
        shift: false,
        alt: true,
    };
    let timestamp = InputTimestamp::capture();
    let sequence_range = InputSequenceRange::singleton(InputSequence::from_runtime_value(42));

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(origin)),
        None
    );

    let started = button
        .handle_input(
            bounds,
            WidgetInput::pointer_move_with_metadata(
                threshold,
                modifiers,
                Some(timestamp),
                Some(sequence_range),
            ),
        )
        .expect("threshold-crossing move should start a drag")
        .drag_message()
        .expect("button output should contain a drag message");
    assert!(started.is_started());
    assert_eq!(
        started.input_metadata(),
        DragHandleMetadata {
            modifiers,
            timestamp: Some(timestamp),
            sequence_range: None,
        }
    );

    let moved = button
        .handle_input(
            bounds,
            WidgetInput::pointer_move_with_metadata(
                moved,
                modifiers,
                Some(timestamp),
                Some(sequence_range),
            ),
        )
        .expect("subsequent move should remain active")
        .drag_message()
        .expect("button output should contain a drag message");
    assert!(moved.is_moved());
    assert_eq!(
        moved.input_metadata(),
        DragHandleMetadata {
            modifiers,
            timestamp: Some(timestamp),
            sequence_range: Some(sequence_range),
        }
    );
}

#[test]
fn draggable_button_ignores_tiny_pointer_jitter_before_click_release() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(17, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let jitter_point = Point::new(12.0, 11.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(jitter_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_release(jitter_point)),
        Some(ButtonMessage::Activate)
    );
}

#[test]
fn draggable_button_release_after_capture_state_restore_ends_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(15, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let move_point = Point::new(100.0, 10.0);
    let release_point = Point::new(140.0, 10.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(move_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: press_point,
            position: move_point,
            metadata: DragHandleMetadata::empty(),
        }))
    );

    let restored_common_state = button.common.state;
    let mut refreshed =
        ButtonWidget::new(15, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    refreshed.common.state = restored_common_state;

    assert_eq!(
        refreshed.handle_input(bounds, WidgetInput::primary_release(release_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Ended {
            position: release_point,
            metadata: DragHandleMetadata::empty(),
        }))
    );
    assert!(!refreshed.common.state.active);
}

#[test]
fn draggable_button_focus_loss_cancels_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(16, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let move_point = Point::new(30.0, 10.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(move_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: press_point,
            position: move_point,
            metadata: DragHandleMetadata::empty(),
        }))
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::FocusChanged(false)),
        Some(ButtonMessage::Drag(DragHandleMessage::Cancelled {
            position: press_point
        }))
    );
    assert!(!button.common.state.pressed);
    assert!(!button.common.state.active);
    assert!(!button.state.dragged);
}

#[test]
fn button_message_helpers_classify_common_outputs() {
    let secondary_position = Point::new(10.0, 12.0);
    let drag_position = Point::new(18.0, 20.0);
    let drag = DragHandleMessage::Moved {
        position: drag_position,
        metadata: DragHandleMetadata::empty(),
    };

    assert!(ButtonMessage::Activate.is_activate());
    assert_eq!(ButtonMessage::Activate.secondary_position(), None);
    assert_eq!(ButtonMessage::Activate.drag_message(), None);

    let secondary = ButtonMessage::SecondaryActivate {
        position: secondary_position,
    };
    assert!(!secondary.is_activate());
    assert_eq!(secondary.secondary_position(), Some(secondary_position));
    assert_eq!(secondary.drag_message(), None);

    let drag_message = ButtonMessage::Drag(drag);
    assert!(!drag_message.is_activate());
    assert_eq!(drag_message.secondary_position(), None);
    assert_eq!(drag_message.drag_message(), Some(drag));
}

#[test]
fn button_chrome_shares_fill_and_stroke_point_storage() {
    let button = ButtonWidget::new(10, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let fill_points = primitives.iter().find_map(|primitive| match primitive {
        PaintPrimitive::FillPolygon(fill) => Some(&fill.points),
        _ => None,
    });
    let stroke_points = primitives.iter().find_map(|primitive| match primitive {
        PaintPrimitive::StrokePolygon(stroke) => Some(&stroke.points),
        _ => None,
    });

    assert!(
        fill_points
            .zip(stroke_points)
            .is_some_and(|(fill, stroke)| Arc::ptr_eq(fill, stroke))
    );
}

#[test]
fn input_only_button_does_not_paint_chrome_or_text() {
    let mut button = ButtonWidget::new(12, "", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    button.common.paint.paints_state_layers = false;
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.is_empty());
}

#[test]
fn hover_chrome_only_button_paints_only_when_hovered() {
    let mut button = ButtonWidget::new(13, "", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
        .with_hover_chrome_only();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(primitives.is_empty());

    let _ = button.handle_input(bounds, WidgetInput::pointer_move(Point::new(10.0, 10.0)));
    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_)))
    );
}

#[test]
fn hover_chrome_only_button_keeps_idle_selected_and_active_chrome() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(80.0, 28.0));
    let neutral = ButtonWidget::new(22, "", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
        .with_hover_chrome_only();
    let mut primitives = Vec::new();
    neutral.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(primitives.is_empty());

    for state in ["selected", "active"] {
        let mut button = neutral.clone();
        if state == "selected" {
            button.common.state.selected = true;
        } else {
            button.common.state.active = true;
        }
        let mut primitives = Vec::new();
        button.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_))),
            "idle {state} button should retain shared chrome"
        );
    }
}

#[test]
fn button_opts_into_state_synchronization() {
    let button = ButtonWidget::new(14, "Drag", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

    assert!(button.needs_state_synchronization());
}

#[test]
fn button_state_synchronization_separates_runtime_and_declarative_state() {
    let mut previous = ButtonWidget::new(
        23,
        "Previous",
        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
    );
    previous.common.state = WidgetState {
        hovered: true,
        pressed: true,
        focused: true,
        selected: false,
        active: false,
        disabled: false,
        read_only: false,
        automation_active: true,
    };
    previous.state = ButtonState {
        armed: true,
        dragged: true,
        press_position: Some(Point::new(4.0, 5.0)),
    };

    let mut current =
        ButtonWidget::new(23, "Current", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    current.common.state.selected = true;
    current.common.state.active = true;
    current.common.state.disabled = true;
    current.common.state.read_only = true;
    current.common.state.automation_active = false;
    current.props.text_align = TextAlign::Left;

    current.synchronize_from_previous(&previous);

    assert!(current.common.state.hovered);
    assert!(current.common.state.pressed);
    assert!(current.common.state.focused);
    assert!(current.common.state.selected);
    assert!(current.common.state.active);
    assert!(current.common.state.disabled);
    assert!(current.common.state.read_only);
    assert!(!current.common.state.automation_active);
    assert_eq!(current.state, previous.state);
    assert_eq!(current.props.label, "Current");
    assert_eq!(current.props.text_align, TextAlign::Left);
}

#[test]
fn button_text_alignment_can_be_overridden() {
    let mut button =
        ButtonWidget::new(11, "Folder", WidgetSizing::fixed(Vector2::new(120.0, 24.0)));

    assert_eq!(button.props.text_align, TextAlign::Center);
    assert!(button.set_text_align(TextAlign::Left));
    assert_eq!(button.props.text_align, TextAlign::Left);
}

#[test]
fn catalog_trailing_icon_follows_enabled_and_disabled_foreground() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let enabled = ButtonWidget::new(18, "Menu", WidgetSizing::fixed(Vector2::new(96.0, 28.0)))
        .with_trailing_icon_tint_cache(IconName::ChevronDown.tint_cache());
    let mut disabled = enabled.clone();
    disabled.common.state.disabled = true;
    let mut enabled_primitives = Vec::new();
    let mut disabled_primitives = Vec::new();

    enabled.append_paint(
        &mut enabled_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    disabled.append_paint(
        &mut disabled_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let enabled_svg = enabled_primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Svg(svg) => Some(svg),
            _ => None,
        })
        .expect("enabled catalog icon should paint");
    let disabled_svg = disabled_primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Svg(svg) => Some(svg),
            _ => None,
        })
        .expect("disabled catalog icon should paint");
    assert_ne!(enabled_svg.document, disabled_svg.document);
}

#[test]
fn caller_owned_trailing_svg_remains_untinted() {
    let raw = SvgIcon::from_svg(
        r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg"><path fill="#f00" d="M1 1h14v14H1z"/></svg>"##,
    )
    .expect("valid caller-owned icon");
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let enabled = ButtonWidget::new(19, "Menu", WidgetSizing::fixed(Vector2::new(96.0, 28.0)))
        .with_trailing_icon(raw.clone());
    let mut disabled = enabled.clone();
    disabled.common.state.disabled = true;
    let mut enabled_primitives = Vec::new();
    let mut disabled_primitives = Vec::new();
    enabled.append_paint(
        &mut enabled_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    disabled.append_paint(
        &mut disabled_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let enabled_svg = enabled_primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Svg(svg) => Some(svg),
            _ => None,
        })
        .expect("enabled caller-owned icon should paint");
    let disabled_svg = disabled_primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Svg(svg) => Some(svg),
            _ => None,
        })
        .expect("disabled caller-owned icon should paint");
    assert_eq!(enabled_svg.document, disabled_svg.document);
}

#[test]
fn automation_marker_is_added_only_when_button_state_is_active() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let button = ButtonWidget::new(20, "Menu", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let mut active = button.clone();
    active.common.state.automation_active = true;
    let mut passive_primitives = Vec::new();
    let mut active_primitives = Vec::new();
    button.append_paint(
        &mut passive_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    active.append_paint(
        &mut active_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert_eq!(active_primitives.len(), passive_primitives.len() + 1);
    assert!(
        active_primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokePolyline(_)))
    );
}

#[test]
fn selected_button_has_leading_marker_and_combined_states_keep_both_cues() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(96.0, 28.0));
    let theme = ThemeTokens::default();
    let mut selected = ButtonWidget::new(21, "Menu", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    selected.common.state.selected = true;
    let mut selected_primitives = Vec::new();
    selected.append_paint(
        &mut selected_primitives,
        bounds,
        &LayoutOutput::default(),
        &theme,
    );
    assert!(selected_primitives.iter().any(|primitive| {
        matches!(primitive, PaintPrimitive::StrokePolyline(marker)
            if marker.points.len() == 2
                && (marker.points[0].x - marker.points[1].x).abs() < f32::EPSILON
                && (marker.points[0].x - 2.0).abs() < f32::EPSILON
                && (marker.width - 2.0).abs() < f32::EPSILON)
    }));

    selected.common.state.focused = true;
    selected.common.state.automation_active = true;
    let mut combined = Vec::new();
    selected.append_paint(&mut combined, bounds, &LayoutOutput::default(), &theme);
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        &theme,
        selected.common.style,
        selected.common.state,
    );
    let expected_focus_points =
        crate::runtime::diagonal_cut_rect_points(crate::runtime::inset_rect(bounds, 1.0, 1.0));
    assert_eq!(
        combined
            .iter()
            .filter(|primitive| {
                matches!(primitive, PaintPrimitive::StrokePolygon(stroke)
                    if stroke.points == expected_focus_points
                        && stroke.color == tokens.foreground
                        && (stroke.width - 2.0).abs() < f32::EPSILON)
            })
            .count(),
        1,
        "focused button should paint one in-bounds contrasting focus polygon"
    );
    let vertical_markers = combined
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::StrokePolyline(marker)
                if marker.points.len() == 2
                    && (marker.points[0].x - marker.points[1].x).abs() < f32::EPSILON
                    && marker.color == tokens.foreground
                    && (marker.width - 2.0).abs() < f32::EPSILON =>
            {
                Some(marker.points[0].x)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(vertical_markers.contains(&2.0));
    assert!(vertical_markers.contains(&(bounds.max.x - 2.0)));

    selected.common.state.disabled = true;
    let mut disabled = Vec::new();
    selected.append_paint(&mut disabled, bounds, &LayoutOutput::default(), &theme);
    assert!(!disabled.iter().any(|primitive| {
        matches!(primitive, PaintPrimitive::StrokePolyline(marker)
            if marker.points.len() == 2
                && (marker.width - 2.0).abs() < f32::EPSILON
                && ((marker.points[0].x - 2.0).abs() < f32::EPSILON
                    || (marker.points[0].x - (bounds.max.x - 2.0)).abs() < f32::EPSILON))
    }));
}
