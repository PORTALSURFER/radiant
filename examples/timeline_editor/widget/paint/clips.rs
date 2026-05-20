use super::super::{ArrangementTimelineWidget, TimelineGeometry};
use super::{push_rect, push_resize_handles, push_stroke, push_text};
use radiant::gui::types::Rgba8;
use radiant::layout::{Point, Rect};
use radiant::runtime::{PaintPrimitive, PaintTextAlign};
use radiant::theme::ThemeTokens;

pub(super) fn append_clip_paint(
    widget: &ArrangementTimelineWidget,
    primitives: &mut Vec<PaintPrimitive>,
    geometry: TimelineGeometry,
    theme: &ThemeTokens,
) {
    for clip in &widget.clips {
        let selected = widget.selected_clip == Some(clip.id);
        let rect = geometry.clip_rect(clip);
        let fill = clip_fill_for_lane(clip.lane, theme);
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

pub(in super::super) fn clip_fill_for_lane(lane: usize, theme: &ThemeTokens) -> Rgba8 {
    match lane {
        0 => theme.accent_mint,
        1 => theme.highlight_cyan,
        2 => theme.accent_copper,
        _ => theme.highlight_blue,
    }
}

fn muted(color: Rgba8) -> Rgba8 {
    Rgba8 {
        r: ((color.r as u16 + 28) / 2) as u8,
        g: ((color.g as u16 + 28) / 2) as u8,
        b: ((color.b as u16 + 36) / 2) as u8,
        a: 230,
    }
}
