use super::TimelineGeometry;
use radiant::layout::{Point, Rect};
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;

use super::paint::{push_rect, push_resize_handles, push_stroke, push_text};
use super::{ArrangementTimelineWidget, RESIZE_HANDLE_WIDTH, TimelineDrag};
use radiant::runtime::PaintTextAlign;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TimelineCursorOverlay {
    hover: Option<TimelineHoverCursor>,
}

#[derive(Clone, Copy, Debug)]
struct TimelineHoverCursor {
    beat: u32,
    x: f32,
}

impl TimelineCursorOverlay {
    pub(super) fn set_hover(&mut self, geometry: TimelineGeometry, position: Point) -> Option<u32> {
        let beat = geometry.beat_at(position)?;
        let x = geometry.cursor_x_at(position)?;
        self.hover = Some(TimelineHoverCursor { beat, x });
        Some(beat)
    }

    pub(super) fn clear_hover(&mut self) {
        self.hover = None;
    }

    pub(super) fn hover_beat(self) -> Option<u32> {
        self.hover.map(|cursor| cursor.beat)
    }

    pub(super) fn active_beat(self, playhead_beat: u32) -> u32 {
        self.hover_beat().unwrap_or(playhead_beat)
    }

    pub(super) fn append_paint(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: u64,
        geometry: TimelineGeometry,
        bounds: Rect,
        playhead_beat: u32,
        theme: &ThemeTokens,
    ) {
        let indicator_x = self
            .hover
            .map(|cursor| cursor.x)
            .unwrap_or_else(|| geometry.x_for_beat(playhead_beat));
        let indicator_color = if self.hover.is_some() {
            theme.highlight_orange_soft
        } else {
            theme.highlight_orange
        };
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(
                Point::new(indicator_x - 1.5, geometry.ruler.min.y),
                Point::new(indicator_x + 1.5, bounds.max.y),
            ),
            indicator_color,
        );
    }
}

pub(super) fn append_runtime_timeline_overlay(
    widget: &ArrangementTimelineWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let geometry = widget.geometry(bounds);
    if let Some(clip_id) = widget
        .hover_clip_id
        .filter(|clip_id| widget.selected_clip != Some(*clip_id) && widget.drag.is_none())
        && let Some(clip) = widget.clips.iter().find(|clip| clip.id == clip_id)
    {
        let rect = geometry.clip_rect(clip);
        push_stroke(primitives, widget.common.id, rect, theme.text_primary, 2.0);
        push_rect(
            primitives,
            widget.common.id,
            rect.top_edge_strip(4.0),
            theme.highlight_orange_soft,
        );
        if rect.width() >= RESIZE_HANDLE_WIDTH {
            push_resize_handles(primitives, widget.common.id, rect, theme.highlight_orange);
        }
    }
    if let Some(TimelineDrag::Selecting { lane, .. }) = widget.drag
        && let Some(range) = widget.selection.filter(|range| range.duration() > 0)
    {
        let rect = geometry.clip_rect_for_range(lane, range);
        push_rect(
            primitives,
            widget.common.id,
            rect,
            preview_fill(theme.accent_mint),
        );
        push_stroke(primitives, widget.common.id, rect, theme.text_primary, 2.0);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(rect.min, Point::new(rect.min.x + 5.0, rect.max.y)),
            theme.surface_overlay,
        );
        push_text(
            primitives,
            widget.common.id,
            "New clip",
            Rect::from_min_max(
                Point::new(rect.min.x + 12.0, rect.min.y + 6.0),
                Point::new(rect.max.x - 8.0, rect.max.y),
            ),
            theme.text_primary,
            PaintTextAlign::Left,
        );
    }
    widget.cursor.append_paint(
        primitives,
        widget.common.id,
        geometry,
        bounds,
        widget.playhead_beat,
        theme,
    );
}

fn preview_fill(mut color: radiant::gui::types::Rgba8) -> radiant::gui::types::Rgba8 {
    color.a = 210;
    color
}
