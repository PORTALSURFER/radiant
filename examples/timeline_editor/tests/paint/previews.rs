use super::super::*;

#[test]
fn timeline_widget_paints_new_clip_preview_while_selecting() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(56), geometry.lane_rect(0).center().y),
        },
    );

    let mut primitives = Vec::new();
    widget.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let preview_rect = geometry.clip_rect_for_range(0, BeatRange { start: 48, end: 56 });
    let preview_fill = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                if *rect == preview_rect && *color == {
                    let mut color = theme.accent_mint;
                    color.a = 210;
                    color
                }
        )
    });
    let preview_stroke = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::StrokeRect(PaintStrokeRect { rect, color, width, .. })
                if *rect == preview_rect && *color == theme.text_primary && *width == 2.0
        )
    });

    assert!(preview_fill);
    assert!(preview_stroke);
}

#[test]
fn timeline_widget_paints_clip_preview_while_moving() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(20), geometry.lane_rect(2).center().y),
        },
    );

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let preview_rect = geometry.clip_rect_for_range(2, BeatRange { start: 16, end: 32 });
    assert_clip_preview(
        &primitives,
        preview_rect,
        {
            let mut color = theme.accent_copper;
            color.a = 210;
            color
        },
        "Kick loop",
        &theme,
    );
}

#[test]
fn timeline_widget_keeps_move_preview_from_captured_drag_state() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(20), geometry.lane_rect(2).center().y),
        },
    );
    widget.clips.retain(|clip| clip.id != 1);

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    assert_clip_preview(
        &primitives,
        geometry.clip_rect_for_range(2, BeatRange { start: 16, end: 32 }),
        {
            let mut color = theme.accent_copper;
            color.a = 210;
            color
        },
        "Kick loop",
        &theme,
    );
}

#[test]
fn timeline_widget_paints_clip_preview_while_resizing() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let clip_rect = geometry.clip_rect(&widget.clips[0]);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(clip_rect.max.x - 2.0, clip_rect.center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(22), clip_rect.center().y),
        },
    );

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let preview_rect = geometry.clip_rect_for_range(0, BeatRange { start: 0, end: 22 });
    assert_clip_preview(
        &primitives,
        preview_rect,
        {
            let mut color = theme.accent_mint;
            color.a = 210;
            color
        },
        "Kick loop",
        &theme,
    );
}

#[test]
fn timeline_widget_keeps_resize_preview_from_captured_drag_state() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let clip_rect = geometry.clip_rect(&widget.clips[0]);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(clip_rect.max.x - 2.0, clip_rect.center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(22), clip_rect.center().y),
        },
    );
    widget.clips.retain(|clip| clip.id != 1);

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    assert_clip_preview(
        &primitives,
        geometry.clip_rect_for_range(0, BeatRange { start: 0, end: 22 }),
        {
            let mut color = theme.accent_mint;
            color.a = 210;
            color
        },
        "Kick loop",
        &theme,
    );
}
