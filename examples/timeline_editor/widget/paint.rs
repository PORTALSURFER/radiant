use super::{
    ArrangementTimelineWidget, HEADER_WIDTH, LANE_COUNT, RESIZE_HANDLE_WIDTH, TOTAL_BEATS,
    TimelineDrag,
};
#[path = "paint/clips.rs"]
mod clips;

use radiant::gui::types::Rgba8;
use radiant::layout::{Point, Rect};
use radiant::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextMetrics, push_text_run_with_metrics,
};
use radiant::theme::ThemeTokens;

pub(super) use clips::clip_fill_for_lane;
pub(super) use radiant::runtime::{push_fill_rect as push_rect, push_stroke_rect as push_stroke};

pub(super) fn append_timeline_paint(
    widget: &ArrangementTimelineWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let geometry = widget.geometry(bounds);
    push_rect(primitives, widget.common.id, bounds, theme.bg_secondary);
    push_rect(
        primitives,
        widget.common.id,
        geometry.ruler,
        theme.surface_raised,
    );
    push_rect(
        primitives,
        widget.common.id,
        geometry.header,
        theme.surface_base,
    );
    push_stroke(
        primitives,
        widget.common.id,
        bounds,
        theme.border_emphasis,
        1.0,
    );

    for lane in 0..LANE_COUNT {
        let rect = geometry.lane_rect(lane);
        let fill = if lane % 2 == 0 {
            theme.surface_base
        } else {
            theme.bg_tertiary
        };
        push_rect(primitives, widget.common.id, rect, fill);
        push_text(
            primitives,
            widget.common.id,
            format!("Track {}", lane + 1),
            Rect::from_min_max(
                Point::new(bounds.min.x + 14.0, rect.min.y + 11.0),
                Point::new(bounds.min.x + HEADER_WIDTH - 10.0, rect.max.y),
            ),
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }

    for beat in (0..=TOTAL_BEATS).step_by(4) {
        let x = geometry.x_for_beat(beat);
        let strong = beat % 16 == 0;
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(x, geometry.ruler.min.y),
                Point::new(x + 1.0, bounds.max.y),
            ),
            if strong {
                theme.grid_strong
            } else {
                theme.grid_soft
            },
        );
        if strong {
            push_text(
                primitives,
                widget.common.id,
                format!("{}", beat / 4 + 1),
                Rect::from_min_max(
                    Point::new(x + 5.0, geometry.ruler.min.y + 6.0),
                    Point::new(x + 52.0, geometry.ruler.max.y),
                ),
                theme.text_muted,
                PaintTextAlign::Left,
            );
        }
    }

    if let Some(selection) = widget
        .selection
        .filter(|range| range.duration() > 0)
        .filter(|_| !matches!(widget.drag, Some(TimelineDrag::Selecting { .. })))
    {
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(geometry.x_for_beat(selection.start), geometry.lanes.min.y),
                Point::new(geometry.x_for_beat(selection.end), geometry.lanes.max.y),
            ),
            translucent(theme.highlight_blue, 64),
        );
    }

    clips::append_clip_paint(widget, primitives, geometry, theme);
}

pub(super) fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    push_text_run_with_metrics(
        primitives,
        widget_id,
        text,
        rect,
        color,
        align,
        PaintTextMetrics::new(13.0, Some(18.0)),
    );
}

pub(super) fn push_resize_handles(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
) {
    let width = RESIZE_HANDLE_WIDTH.min((rect.width() * 0.5).max(0.0));
    if width <= 0.0 {
        return;
    }
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(rect.min, Point::new(rect.min.x + width, rect.max.y)),
        color,
    );
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(Point::new(rect.max.x - width, rect.min.y), rect.max),
        color,
    );
}

fn translucent(color: Rgba8, alpha: u8) -> Rgba8 {
    color.with_alpha(alpha)
}
