use super::TimelineGeometry;
use radiant::layout::{Point, Rect};
use radiant::runtime::{PaintPrimitive, PaintTextAlign};
use radiant::theme::ThemeTokens;

use super::paint::{push_rect, push_text};

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
        let indicator_beat = self.active_beat(playhead_beat);
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
        if self.hover.is_some() {
            push_text(
                primitives,
                widget_id,
                format!("beat {indicator_beat}"),
                Rect::from_min_max(
                    Point::new(
                        (indicator_x + 8.0).min(bounds.max.x - 82.0),
                        geometry.ruler.min.y + 6.0,
                    ),
                    Point::new(
                        (indicator_x + 82.0).min(bounds.max.x - 4.0),
                        geometry.ruler.max.y,
                    ),
                ),
                theme.text_primary,
                PaintTextAlign::Left,
            );
        }
    }
}
