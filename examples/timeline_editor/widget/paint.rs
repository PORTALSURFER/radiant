use super::{
    ArrangementTimelineWidget, HEADER_WIDTH, LANE_COUNT, RESIZE_HANDLE_WIDTH, TOTAL_BEATS,
};
use radiant::gui::types::Rgba8;
use radiant::layout::{Point, Rect};
use radiant::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::TextWrap;

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

    if let Some(selection) = widget.selection.filter(|range| range.duration() > 0) {
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

    for clip in &widget.clips {
        let selected = widget.selected_clip == Some(clip.id);
        let rect = geometry.clip_rect(clip);
        let fill = match clip.lane {
            0 => theme.accent_mint,
            1 => theme.highlight_cyan,
            2 => theme.accent_copper,
            _ => theme.highlight_blue,
        };
        push_rect(
            primitives,
            widget.common.id,
            rect,
            if selected { fill } else { muted(fill) },
        );
        push_stroke(
            primitives,
            widget.common.id,
            rect,
            if selected {
                theme.text_primary
            } else {
                theme.border_emphasis
            },
            if selected { 2.0 } else { 1.0 },
        );
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(rect.min, Point::new(rect.min.x + 5.0, rect.max.y)),
            theme.surface_overlay,
        );
        push_text(
            primitives,
            widget.common.id,
            clip.name,
            Rect::from_min_max(
                Point::new(rect.min.x + 12.0, rect.min.y + 6.0),
                Point::new(rect.max.x - 8.0, rect.max.y),
            ),
            theme.text_primary,
            PaintTextAlign::Left,
        );
        if selected {
            push_resize_handles(primitives, widget.common.id, rect, theme.text_primary);
        }
    }
}

pub(super) fn push_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

pub(super) fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

pub(super) fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 13.0,
        baseline: Some(18.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
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

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn muted(color: Rgba8) -> Rgba8 {
    Rgba8 {
        r: ((color.r as u16 + 28) / 2) as u8,
        g: ((color.g as u16 + 28) / 2) as u8,
        b: ((color.b as u16 + 36) / 2) as u8,
        a: 230,
    }
}
