use super::*;

#[path = "paint/previews.rs"]
mod previews;

#[test]
fn timeline_widget_paints_one_vertical_cursor_indicator() {
    let state = TimelineEditorState::default();
    let mut widget = ArrangementTimelineWidget::new(&state);
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let handled = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(24), bounds.center().y),
        },
    );
    assert!(handled.is_none());
    let mut primitives = Vec::new();

    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let indicator_lines = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                    if rect.width() <= 3.0
                        && rect.height() >= bounds.height() - RULER_HEIGHT
                        && (*color == theme.highlight_orange
                            || *color == theme.highlight_orange_soft)
            )
        })
        .count();
    assert_eq!(indicator_lines, 1);
}

#[test]
fn timeline_cursor_overlay_tracks_unsnapped_pointer_position() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let pointer_x = geometry.x_for_beat(24) + 3.25;

    let handled = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(pointer_x, geometry.lane_rect(1).center().y),
        },
    );
    assert!(handled.is_none());

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let cursor_rect = primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                if *color == theme.highlight_orange_soft
                    && rect.width() <= 3.0
                    && rect.height() >= bounds.height() - RULER_HEIGHT =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .expect("hover cursor line should be painted");
    assert!((cursor_rect.center().x - pointer_x).abs() < 0.01);
}

#[test]
fn timeline_widget_highlights_hovered_clip() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let theme = ThemeTokens::default();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);

    let handled = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y),
        },
    );
    assert!(handled.is_none());
    assert_eq!(widget.hover_clip_id, Some(1));

    let mut primitives = Vec::new();
    widget.append_runtime_overlay_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

    let hover_rect = geometry.clip_rect(&widget.clips[0]);
    let hover_border = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::StrokeRect(PaintStrokeRect {
                rect,
                color,
                width,
                ..
            }) if *rect == hover_rect && *color == theme.text_primary && *width == 2.0
        )
    });
    let hover_strip = primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                if *rect == hover_rect.top_edge_strip(4.0)
                    && *color == theme.highlight_orange_soft
        )
    });

    assert!(hover_border);
    assert!(hover_strip);
}
