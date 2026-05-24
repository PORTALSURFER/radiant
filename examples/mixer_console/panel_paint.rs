#[path = "panel_paint/fader.rs"]
mod fader;
#[path = "panel_paint/meter.rs"]
mod meter;

use self::{fader::append_fader, meter::append_meter};
use super::model::{CHANNEL_COUNT, MixerChannel};
use super::paint::{push_rect, push_stroke, push_text, strip_fill_color, translucent};
use super::panel::MixerPanelWidget;
use radiant::prelude::*;

impl MixerPanelWidget {
    pub(super) fn append_console_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        for channel in 0..CHANNEL_COUNT {
            self.append_strip(primitives, bounds, channel, theme);
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

    fn append_strip(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        theme: &ThemeTokens,
    ) {
        let channel = self.channels[channel_index];
        let strip = self.strip_rect(bounds, channel_index);
        let selected = channel_index == self.selected_channel;
        let solo_active = self.channels.iter().any(|channel| channel.solo);
        let solo_dimmed = channel.is_visually_dimmed_by_solo(solo_active);

        push_rect(
            primitives,
            self.common.id,
            strip,
            strip_fill_color(selected, solo_dimmed, theme),
        );
        push_stroke(primitives, self.common.id, strip, theme.border, 1.0);
        push_text(
            primitives,
            self.common.id,
            channel.label,
            Rect::from_min_size(
                Point::new(strip.min.x + 8.0, strip.min.y + 10.0),
                Vector2::new(strip.width() - 16.0, 22.0),
            ),
            if solo_dimmed {
                theme.text_muted
            } else {
                theme.text_primary
            },
            PaintTextAlign::Center,
        );
        append_meter(self, primitives, channel, strip, solo_dimmed, theme);
        append_fader(self, primitives, channel, strip, solo_dimmed, theme);
        self.append_channel_buttons(primitives, channel, strip, theme);
        self.append_strip_footer(primitives, channel, strip, solo_dimmed, theme);
    }

    fn append_channel_buttons(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        theme: &ThemeTokens,
    ) {
        for (index, label, active, active_color) in [
            (0, "M", channel.muted, theme.accent_danger),
            (1, "S", channel.solo, theme.accent_warning),
            (2, "R", channel.armed, theme.highlight_cyan),
        ] {
            let rect = self.button_rect(strip, index);
            push_rect(
                primitives,
                self.common.id,
                rect,
                if active {
                    active_color
                } else {
                    theme.bg_tertiary
                },
            );
            push_stroke(primitives, self.common.id, rect, theme.border, 1.0);
            push_text(
                primitives,
                self.common.id,
                label,
                rect,
                theme.text_primary,
                PaintTextAlign::Center,
            );
        }
    }

    fn append_strip_footer(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        push_text(
            primitives,
            self.common.id,
            format!("{:+.1} dB", channel.gain_db),
            Rect::from_min_size(
                Point::new(strip.min.x + 6.0, strip.max.y - 48.0),
                Vector2::new(strip.width() - 12.0, 18.0),
            ),
            if solo_dimmed {
                translucent(theme.text_muted, 150)
            } else {
                theme.text_muted
            },
            PaintTextAlign::Center,
        );
        push_text(
            primitives,
            self.common.id,
            format!("{:+.0}", channel.pan * 100.0),
            Rect::from_min_size(
                Point::new(strip.min.x + 6.0, strip.max.y - 28.0),
                Vector2::new(strip.width() - 12.0, 18.0),
            ),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
}
