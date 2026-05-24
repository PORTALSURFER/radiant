use super::model::CHANNEL_COUNT;
use super::paint::{push_rect, push_text};
use super::panel::MixerPanelWidget;
use radiant::prelude::*;

#[path = "panel_paint/fader.rs"]
mod fader;
#[path = "panel_paint/meter.rs"]
mod meter;
#[path = "panel_paint/overlay.rs"]
mod overlay;
#[path = "panel_paint/strip.rs"]
mod strip;

impl MixerPanelWidget {
    pub(super) fn append_console_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        for channel in 0..CHANNEL_COUNT {
            strip::append_strip(self, primitives, bounds, channel, theme);
        }
        push_text(
            primitives,
            self.common.id,
            format!("frame {}", self.frame),
            Rect::from_min_max(
                Point::new(bounds.max.x - 120.0, bounds.min.y + 12.0),
                Point::new(bounds.max.x - 20.0, bounds.min.y + 32.0),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }

    pub(super) fn append_fader_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        overlay::append_fader_drag_overlay(self, primitives, bounds, source_channel, theme);
    }

    pub(super) fn append_send_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        send: usize,
        theme: &ThemeTokens,
    ) {
        overlay::append_send_drag_overlay(self, primitives, bounds, channel_index, send, theme);
    }

    pub(super) fn append_reorder_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        overlay::append_reorder_drag_overlay(self, primitives, bounds, source_channel, theme);
    }
}
