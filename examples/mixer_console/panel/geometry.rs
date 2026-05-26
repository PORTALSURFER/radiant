use super::super::model::{CHANNEL_COUNT, SEND_COUNT};
use super::MixerPanelWidget;
use radiant::prelude::*;

impl MixerPanelWidget {
    pub(super) fn console_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, bounds.min.y + 12.0),
            Point::new(bounds.max.x - 12.0, bounds.max.y - 12.0),
        )
    }

    pub(crate) fn strip_rect(&self, bounds: Rect, channel: usize) -> Rect {
        let console = self.console_rect(bounds);
        let gap = 4.0;
        let strip_width =
            (console.width() - gap * (CHANNEL_COUNT - 1) as f32) / CHANNEL_COUNT as f32;
        let x = console.min.x + channel as f32 * (strip_width + gap);
        Rect::from_min_size(
            Point::new(x, console.min.y),
            Vector2::new(strip_width.max(1.0), console.height()),
        )
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
        (0..CHANNEL_COUNT).find(|channel| self.strip_rect(bounds, *channel).contains(position))
    }

    pub(crate) fn insertion_index_at(&self, bounds: Rect, position: Point) -> usize {
        let console = self.console_rect(bounds);
        if position.x <= console.min.x {
            return 0;
        }
        if position.x >= console.max.x {
            return CHANNEL_COUNT;
        }
        for channel in 0..CHANNEL_COUNT {
            if position.x < self.strip_rect(bounds, channel).center().x {
                return channel;
            }
        }
        CHANNEL_COUNT
    }

    pub(crate) fn insertion_line_rect(&self, bounds: Rect, insert: usize) -> Rect {
        let console = self.console_rect(bounds);
        let insert = insert.min(CHANNEL_COUNT);
        let x = if insert == 0 {
            self.strip_rect(bounds, 0).min.x - 2.0
        } else if insert == CHANNEL_COUNT {
            self.strip_rect(bounds, CHANNEL_COUNT - 1).max.x + 2.0
        } else {
            let left = self.strip_rect(bounds, insert - 1);
            let right = self.strip_rect(bounds, insert);
            (left.max.x + right.min.x) * 0.5
        };
        Rect::from_min_max(
            Point::new(x - 2.0, console.min.y + 4.0),
            Point::new(x + 2.0, console.max.y - 4.0),
        )
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
}
