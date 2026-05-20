#[path = "overlay/previews.rs"]
mod previews;

use super::TimelineGeometry;
use radiant::layout::{Point, Rect};
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;

use super::ArrangementTimelineWidget;
use super::paint::push_rect;

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
    previews::append_clip_overlays(widget, primitives, geometry, theme);
    widget.cursor.append_paint(
        primitives,
        widget.common.id,
        geometry,
        bounds,
        widget.playhead_beat,
        theme,
    );
}
