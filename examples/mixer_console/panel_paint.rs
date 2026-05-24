use super::model::{CHANNEL_COUNT, MixerChannel, ratio_for_gain, ratio_for_meter_db};
use super::paint::{
    fader_knob_color, meter_color, meter_track_color, push_rect, push_stroke, push_text, rgba,
    strip_fill_color, translucent,
};
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
        self.append_meter(primitives, channel, strip, solo_dimmed, theme);
        self.append_fader(primitives, channel, strip, solo_dimmed, theme);
        self.append_channel_buttons(primitives, channel, strip, theme);
        self.append_strip_footer(primitives, channel, strip, solo_dimmed, theme);
    }

    fn append_meter(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let meter = self.meter_rect(strip);
        push_rect(
            primitives,
            self.common.id,
            meter,
            meter_track_color(solo_dimmed),
        );
        for fraction in [0.25, 0.5, 0.75] {
            let y = meter.max.y - meter.height() * fraction;
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(meter.min.x, y), Point::new(meter.max.x, y + 1.0)),
                translucent(theme.grid_soft, 120),
            );
        }
        self.append_meter_level(primitives, channel, meter, solo_dimmed, theme);
    }

    fn append_meter_level(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        meter: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let meter_ratio = ratio_for_meter_db(channel.meter_db);
        let meter_fill = Rect::from_min_max(
            Point::new(
                meter.min.x + 3.0,
                meter.max.y - (meter.height() - 6.0) * meter_ratio,
            ),
            Point::new(meter.max.x - 3.0, meter.max.y - 3.0),
        );
        push_rect(
            primitives,
            self.common.id,
            meter_fill,
            if solo_dimmed {
                rgba(75, 80, 86, 180)
            } else {
                meter_color(channel.meter_db)
            },
        );
        self.append_peak_and_readout(primitives, channel, meter, solo_dimmed, theme);
    }

    fn append_peak_and_readout(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        meter: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(channel.peak_db);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(meter.min.x + 2.0, peak_y),
                Point::new(meter.max.x - 2.0, peak_y + 2.0),
            ),
            if solo_dimmed {
                rgba(90, 94, 98, 160)
            } else {
                theme.highlight_orange
            },
        );
        push_text(
            primitives,
            self.common.id,
            format!("{:+.0}", channel.meter_db),
            Rect::from_min_size(
                Point::new(meter.min.x - 16.0, meter.max.y + 8.0),
                Vector2::new(meter.width() + 32.0, 18.0),
            ),
            if solo_dimmed {
                rgba(118, 123, 128, 180)
            } else {
                theme.text_muted
            },
            PaintTextAlign::Center,
        );
    }

    fn append_fader(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let fader = self.fader_rect(strip);
        let center_x = fader.center().x;
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(center_x - 2.0, fader.min.y),
                Point::new(center_x + 2.0, fader.max.y),
            ),
            if solo_dimmed {
                translucent(theme.grid_soft, 130)
            } else {
                theme.grid_strong
            },
        );
        self.append_fader_marks(primitives, fader, center_x, theme);
        self.append_fader_knob(primitives, channel, fader, solo_dimmed, theme);
    }

    fn append_fader_marks(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        fader: Rect,
        center_x: f32,
        theme: &ThemeTokens,
    ) {
        for db in [-48.0, -24.0, -12.0, 0.0, 6.0] {
            let y = fader.max.y - fader.height() * ratio_for_gain(db);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(center_x - 10.0, y),
                    Point::new(center_x + 10.0, y + 1.0),
                ),
                theme.grid_soft,
            );
        }
    }

    fn append_fader_knob(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        fader: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let knob_y = fader.max.y - fader.height() * channel.gain_ratio();
        let knob = Rect::from_min_size(
            Point::new(fader.min.x, knob_y - 8.0),
            Vector2::new(fader.width(), 16.0),
        );
        push_rect(
            primitives,
            self.common.id,
            knob,
            fader_knob_color(solo_dimmed, theme),
        );
        push_stroke(primitives, self.common.id, knob, theme.border_emphasis, 1.0);
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
