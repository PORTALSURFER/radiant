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
        strip
            .horizontal_ratio_span(0.58, 0.86)
            .inset_vertical(56.0, 150.0)
    }

    pub(crate) fn meter_rect(&self, strip: Rect) -> Rect {
        strip
            .horizontal_ratio_span(0.14, 0.42)
            .inset_vertical(50.0, 150.0)
    }

    pub(crate) fn send_rect(&self, strip: Rect, send: usize) -> Rect {
        self.send_layout(strip)
            .slot_rect(send)
            .unwrap_or_else(|| strip.empty_at_min())
    }

    pub(crate) fn button_rect(&self, strip: Rect, index: usize) -> Rect {
        self.button_layout(strip)
            .strip_rect(index)
            .unwrap_or_else(|| strip.empty_at_min())
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
        HorizontalValueAxis::normalized(send).value_for_x(position.x)
    }

    pub(super) fn send_at(&self, strip: Rect, position: Point) -> Option<usize> {
        (0..SEND_COUNT).find(|send| self.send_rect(strip, *send).contains(position))
    }

    fn strip_layout(&self, bounds: Rect) -> HorizontalStripLayout {
        HorizontalStripLayout::new(self.console_rect(bounds), CHANNEL_COUNT, 4.0)
    }

    fn send_layout(&self, strip: Rect) -> VerticalStripStackLayout {
        let control_rect = Rect::from_min_max(
            Point::new(strip.min.x + 5.0, strip.max.y - 136.0),
            Point::new(strip.max.x - 5.0, strip.max.y),
        );
        VerticalStripStackLayout::new(control_rect, SEND_COUNT, 12.0, 6.0)
    }

    fn button_layout(&self, strip: Rect) -> HorizontalStripLayout {
        HorizontalStripLayout::new(
            Rect::from_min_size(
                Point::new(strip.min.x + 4.0, strip.max.y - 72.0),
                Vector2::new(strip.width() - 8.0, 22.0),
            ),
            3,
            1.0,
        )
    }
}
