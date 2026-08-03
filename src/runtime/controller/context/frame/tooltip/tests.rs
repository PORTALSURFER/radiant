use super::{
    TOOLTIP_FONT_SIZE, TOOLTIP_HORIZONTAL_PADDING, TOOLTIP_LINE_HEIGHT, TOOLTIP_MAX_WIDTH,
    TOOLTIP_OVERLAY_ID, tooltip_character_advance, tooltip_layout,
};
use crate::{
    gui::types::{Point, Rect},
    layout::Vector2,
    prelude::{IntoView, button, text},
    runtime::{
        Command, DeclarativeOwnedRuntimeBridge, Event, PaintText, SurfaceChild, SurfaceNode,
        SurfaceRuntime, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, DragHandleWidget, WidgetSizing},
};

#[test]
fn runtime_frame_with_default_theme_projects_paint_plan() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| crate::runtime::UiSurface::new(text("Ready").into_node()),
        |_, _: ()| {},
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Ready")
    );
}

#[derive(Default)]
struct TooltipDemoState {
    clicked: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum TooltipDemoMessage {
    Click,
}

#[test]
fn hovered_widget_tooltip_paints_without_intercepting_activation() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        TooltipDemoState::default(),
        |state| {
            crate::runtime::UiSurface::new(
                button(if state.clicked { "Clicked" } else { "Idle" })
                    .message(TooltipDemoMessage::Click)
                    .tooltip("Audition volume")
                    .id(301)
                    .size(80.0, 24.0)
                    .into_node(),
            )
        },
        |state, message| match message {
            TooltipDemoMessage::Click => state.clicked = true,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });

    assert!(
        !runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Audition volume")
    );
    let deadline = runtime
        .timed_repaint_deadline()
        .expect("hover should arm tooltip deadline");
    assert!(runtime.advance_timed_repaints(deadline));

    let frame = runtime.frame_with_default_theme();
    let tooltip = frame
        .paint_plan
        .first_text_run("Audition volume")
        .expect("hover should paint tooltip text");
    assert_eq!(tooltip.font_size, TOOLTIP_FONT_SIZE);

    let tooltip_panel = frame
        .paint_plan
        .visible_fill_rects_for_widget(TOOLTIP_OVERLAY_ID)
        .find(|fill| fill.rect.height() == TOOLTIP_LINE_HEIGHT + 8.0)
        .expect("hover should paint tooltip panel fill");
    assert_ne!(
        tooltip_panel.color,
        ThemeTokens::default().accent_copper,
        "tooltips should not reuse loud accent overlay fills"
    );

    runtime.dispatch_primary_click(Point::new(8.0, 8.0));

    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Clicked")
    );
}

#[test]
fn tooltip_hover_intent_does_not_restart_for_same_target_motion() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| {
            crate::runtime::UiSurface::new(
                button("Idle")
                    .message(())
                    .tooltip("Audition volume")
                    .id(301)
                    .size(80.0, 24.0)
                    .into_node(),
            )
        },
        |_, _: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });
    let first_deadline = runtime
        .timed_repaint_deadline()
        .expect("hover should arm tooltip deadline");
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(9.0, 9.0),
    });
    assert_eq!(runtime.timed_repaint_deadline(), Some(first_deadline));
}

#[test]
fn tooltip_hover_intent_resets_on_leave_focus_loss_and_capture_release_rearms() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| {
            crate::runtime::UiSurface::new(
                button("Idle")
                    .message(())
                    .tooltip("Audition volume")
                    .id(301)
                    .size(80.0, 24.0)
                    .into_node(),
            )
        },
        |_, _: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    let inside = Point::new(8.0, 8.0);

    runtime.dispatch_event(Event::PointerMove { position: inside });
    assert!(runtime.timed_repaint_deadline().is_some());
    runtime.clear_pointer_hover();
    assert_eq!(runtime.timed_repaint_deadline(), None);

    runtime.dispatch_event(Event::PointerMove { position: inside });
    assert!(runtime.timed_repaint_deadline().is_some());
    runtime.dispatch_primary_click(inside);
    assert!(runtime.timed_repaint_deadline().is_some());
    let deadline = runtime.timed_repaint_deadline().unwrap();
    assert!(runtime.advance_timed_repaints(deadline));
    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Audition volume")
    );
}

#[test]
fn tooltip_release_reconciles_hover_target_after_exclusive_capture() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| {
            let mut capture =
                DragHandleWidget::new(401, WidgetSizing::fixed(Vector2::new(80.0, 32.0)));
            capture.common.tooltip = Some(String::from("Capture A"));
            let mut target = ButtonWidget::new(
                402,
                PaintText::from("B"),
                WidgetSizing::fixed(Vector2::new(80.0, 32.0)),
            );
            target.common.tooltip = Some(String::from("Target B"));
            crate::runtime::UiSurface::new(SurfaceNode::row(
                400,
                0.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(capture, WidgetMessageMapper::none())),
                    SurfaceChild::fill(SurfaceNode::widget(target, WidgetMessageMapper::none())),
                ],
            ))
        },
        |_, _: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 40.0));
    let capture_position = Point::new(8.0, 8.0);
    let target_position = Point::new(120.0, 8.0);

    runtime.dispatch_event(Event::PointerMove {
        position: capture_position,
    });
    runtime.dispatch_event(Event::PointerPress {
        position: capture_position,
        button: crate::widgets::PointerButton::Primary,
        modifiers: crate::widgets::PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(401));
    runtime.dispatch_event(Event::PointerMove {
        position: target_position,
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: target_position,
        button: crate::widgets::PointerButton::Primary,
        modifiers: crate::widgets::PointerModifiers::default(),
        timestamp: None,
    });

    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.hovered_widget(), Some(402));
    assert!(
        runtime
            .surface()
            .find_widget(402)
            .expect("reconciled hover widget")
            .widget()
            .common()
            .state
            .hovered
    );
    let deadline = runtime
        .timed_repaint_deadline()
        .expect("release over B should arm B tooltip deadline");
    assert!(!runtime.advance_timed_repaints(deadline - std::time::Duration::from_millis(1)));
    let before_deadline = runtime.frame_with_default_theme().paint_plan;
    assert!(!before_deadline.contains_text("Capture A"));
    assert!(!before_deadline.contains_text("Target B"));

    assert!(runtime.advance_timed_repaints(deadline));
    let at_deadline = runtime.frame_with_default_theme().paint_plan;
    assert!(!at_deadline.contains_text("Capture A"));
    assert!(at_deadline.contains_text("Target B"));
}

#[test]
fn tooltip_hover_intent_is_fenced_when_runtime_closes() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| {
            crate::runtime::UiSurface::new(
                button("Idle")
                    .message(())
                    .tooltip("Audition volume")
                    .id(301)
                    .size(80.0, 24.0)
                    .into_node(),
            )
        },
        |_, _: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });
    assert!(runtime.timed_repaint_deadline().is_some());

    assert!(runtime.execute_command(Command::Exit).exit_requested);
    assert_eq!(runtime.timed_repaint_deadline(), None);
}

#[test]
fn tooltip_if_false_skips_hover_tooltip() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        (),
        |_| {
            crate::runtime::UiSurface::new(
                button("Idle")
                    .message(TooltipDemoMessage::Click)
                    .tooltip_if(false, "Audition volume")
                    .id(301)
                    .size(80.0, 24.0)
                    .into_node(),
            )
        },
        |_, _: TooltipDemoMessage| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });

    assert!(
        !runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Audition volume")
    );
}

#[test]
fn tooltip_rect_allows_long_compact_help_text_to_fit() {
    let layout = tooltip_layout(
        Rect::from_min_size(Point::new(240.0, 300.0), Vector2::new(40.0, 18.0)),
        "Sample row: select, double-click to load, drag to copy, right-click for actions.",
        Vector2::new(1280.0, 720.0),
    );

    assert!(layout.lines.len() > 1);
    assert!(layout.rect.height() > TOOLTIP_LINE_HEIGHT + 8.0);
    assert!(layout.rect.width() <= TOOLTIP_MAX_WIDTH);
}

#[test]
fn tooltip_layout_respects_author_supplied_line_breaks() {
    let layout = tooltip_layout(
        Rect::from_min_size(Point::new(20.0, 20.0), Vector2::new(40.0, 18.0)),
        "Random section playback\nClick: play a random section now.\nCommand-click: make Space use random sections.",
        Vector2::new(360.0, 240.0),
    );

    assert_eq!(
        layout.lines.first().map(String::as_str),
        Some("Random section playback")
    );
    assert!(
        layout
            .lines
            .iter()
            .any(|line| line.contains("Command-click"))
    );
    assert!(layout.rect.height() >= 3.0 * TOOLTIP_LINE_HEIGHT + 8.0);
}

#[test]
fn tooltip_layout_reserves_rendered_bitmap_width_for_short_toolbar_help() {
    let tooltip = "Loop preview playback.";
    let layout = tooltip_layout(
        Rect::from_min_size(Point::new(144.0, 72.0), Vector2::new(28.0, 24.0)),
        tooltip,
        Vector2::new(572.0, 344.0),
    );
    let text_width = layout.rect.width() - TOOLTIP_HORIZONTAL_PADDING;
    let required_width = tooltip.chars().count() as f32 * tooltip_character_advance();

    assert_eq!(layout.lines, vec![String::from(tooltip)]);
    assert!(
        text_width >= required_width,
        "tooltip should reserve enough text width: {text_width} >= {required_width}"
    );
}
