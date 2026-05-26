use super::super::model::{CHANNEL_COUNT, SEND_COUNT};
use super::MixerPanelWidget;
use radiant::prelude::*;

impl MixerPanelWidget {
    pub(super) fn console_rect(&self, bounds: Rect) -> Rect {
        bounds.inset(12.0, 12.0, 12.0, 12.0)
    }

    pub(crate) fn strip_rect(&self, bounds: Rect, channel: usize) -> Rect {
        self.strip_layout(bounds)
            .strip_rect(channel)
            .unwrap_or_else(|| self.console_rect(bounds).empty_at_min())
    }

    pub(crate) fn fader_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.x_for_ratio(0.58), strip.min.y + 56.0),
            Point::new(strip.x_for_ratio(0.86), strip.max.y - 150.0),
        )
    }

    pub(crate) fn meter_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.x_for_ratio(0.14), strip.min.y + 50.0),
            Point::new(strip.x_for_ratio(0.42), strip.max.y - 150.0),
        )
    }

    pub(crate) fn send_rect(&self, strip: Rect, send: usize) -> Rect {
        let y = strip.max.y - 136.0 + send as f32 * 18.0;
        Rect::from_min_size(
            Point::new(strip.min.x + 5.0, y),
            Vector2::new(strip.width() - 10.0, 12.0),
        )
    }

    pub(crate) fn button_rect(&self, strip: Rect, index: usize) -> Rect {
        let width = (strip.width() - 10.0) / 3.0;
        let x = strip.min.x + 4.0 + index as f32 * (width + 1.0);
        Rect::from_min_size(
            Point::new(x, strip.max.y - 72.0),
            Vector2::new(width.max(1.0), 22.0),
        )
    }

    pub(super) fn channel_at(&self, bounds: Rect, position: Point) -> Option<usize> {
        self.strip_layout(bounds).strip_at_position(position)
    }

    pub(crate) fn insertion_index_at(&self, bounds: Rect, position: Point) -> usize {
        self.strip_layout(bounds).insertion_index_at(position)
    }

    pub(crate) fn insertion_line_rect(&self, bounds: Rect, insert: usize) -> Rect {
        self.strip_layout(bounds)
            .insertion_line_rect(insert, 4.0, 4.0)
            .unwrap_or_else(|| self.console_rect(bounds).empty_at_min())
    }

    pub(super) fn fader_ratio_at(&self, strip: Rect, position: Point) -> f32 {
        vertical_value_at_point(self.fader_rect(strip), position)
    }

    pub(super) fn send_ratio_at(&self, strip: Rect, send: usize, position: Point) -> f32 {
        let send = self.send_rect(strip, send);
        send.ratio_for_x(position.x)
    }

    pub(super) fn send_at(&self, strip: Rect, position: Point) -> Option<usize> {
        (0..SEND_COUNT).find(|send| self.send_rect(strip, *send).contains(position))
    }

    fn strip_layout(&self, bounds: Rect) -> HorizontalStripLayout {
        HorizontalStripLayout::new(self.console_rect(bounds), CHANNEL_COUNT, 4.0)
    }
}
