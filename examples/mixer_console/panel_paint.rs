use super::model::{CHANNEL_COUNT, MixerChannel, SEND_COUNT, ratio_for_gain, ratio_for_meter_db};
use super::paint::{
    blend_color, group_color, meter_color, push_rect, push_stroke, push_text, rgba, send_color,
    translucent,
};
use super::panel::{MeterReadout, MixerPanelWidget};
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
        let selected = self.selection.is_selected(channel_index);
        let solo_active = self.channels.iter().any(|channel| channel.solo);
        let solo_dimmed = channel.is_visually_dimmed_by_solo(solo_active);
        let group_tint = group_color(channel.group(), theme);
        let fill = if selected {
            blend_color(theme.surface_raised, theme.highlight_blue, 0.20)
        } else if solo_dimmed {
            blend_color(theme.surface_base, theme.bg_primary, 0.42)
        } else {
            blend_color(theme.surface_base, group_tint, 0.10)
        };
        push_rect(primitives, self.common.id, strip, fill);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(strip.min.x, strip.min.y),
                Point::new(strip.max.x, strip.min.y + 4.0),
            ),
            if solo_dimmed {
                rgba(78, 82, 88, 180)
            } else {
                group_tint
            },
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
        self.append_fader(primitives, channel_index, strip, solo_dimmed, theme);
        self.append_sends(primitives, channel_index, strip, solo_dimmed, theme);
        self.append_channel_buttons(primitives, channel, strip, theme);
        self.append_strip_footer(
            primitives,
            channel_index,
            channel,
            strip,
            solo_dimmed,
            theme,
        );
    }

    fn should_paint_fader_overlay_for(&self, source_channel: usize, channel: usize) -> bool {
        if self.selection.is_selected(source_channel) && self.selection.selected_indices().len() > 1
        {
            self.selection.is_selected(channel)
        } else {
            channel == source_channel
        }
    }

    pub(super) fn append_fader_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        for channel_index in 0..CHANNEL_COUNT {
            if !self.should_paint_fader_overlay_for(source_channel, channel_index) {
                continue;
            }
            let strip = self.strip_rect(bounds, channel_index);
            let fader = self.fader_rect(strip);
            let center_x = fader.center().x;
            self.append_meter_drag_overlay(primitives, channel_index, strip, theme);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(fader.min.x - 2.0, fader.min.y - 12.0),
                    Point::new(fader.max.x + 2.0, fader.max.y + 12.0),
                ),
                translucent(theme.surface_base, 245),
            );
            self.append_fader_track(primitives, fader, center_x, false, theme);
            let knob_y = fader.max.y - fader.height() * self.fader_display_ratio(channel_index);
            let knob = Rect::from_min_size(
                Point::new(fader.min.x, knob_y - 8.0),
                Vector2::new(fader.width(), 16.0),
            );
            push_rect(primitives, self.common.id, knob, theme.highlight_blue);
            push_stroke(primitives, self.common.id, knob, theme.border_emphasis, 1.0);
        }
    }

    fn append_meter_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel_index: usize,
        strip: Rect,
        theme: &ThemeTokens,
    ) {
        let channel = self.channels[channel_index];
        let readout = MeterReadout {
            meter_db: self
                .meter_display_db_for_drag(channel_index)
                .unwrap_or(channel.meter_db),
            peak_db: self
                .peak_display_db_for_drag(channel_index)
                .unwrap_or(channel.peak_db),
        };
        self.append_meter_values(primitives, channel, strip, false, readout, theme);
    }

    pub(super) fn append_send_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        send: usize,
        theme: &ThemeTokens,
    ) {
        let strip = self.strip_rect(bounds, channel_index);
        let rect = self.send_rect(strip, send);
        push_rect(primitives, self.common.id, rect, theme.bg_tertiary);
        let fill = Rect::from_min_max(
            rect.min,
            Point::new(
                rect.min.x + rect.width() * self.send_display_ratio(channel_index, send),
                rect.max.y,
            ),
        );
        push_rect(primitives, self.common.id, fill, send_color(send, theme));
        push_stroke(primitives, self.common.id, rect, theme.border_emphasis, 1.0);
    }

    pub(super) fn append_reorder_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        let source = self.strip_rect(bounds, source_channel);
        push_stroke(
            primitives,
            self.common.id,
            source,
            translucent(theme.text_primary, 135),
            2.0,
        );
        if let Some(insert) = self.reorder_insert {
            let line = self.insertion_line_rect(bounds, insert);
            push_rect(
                primitives,
                self.common.id,
                line,
                translucent(theme.highlight_cyan, 235),
            );
            push_stroke(primitives, self.common.id, line, theme.border_emphasis, 1.0);
        }
    }

    fn append_meter(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        self.append_meter_values(
            primitives,
            channel,
            strip,
            solo_dimmed,
            MeterReadout {
                meter_db: channel.meter_db,
                peak_db: channel.peak_db,
            },
            theme,
        );
    }

    fn append_meter_values(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        readout: MeterReadout,
        theme: &ThemeTokens,
    ) {
        let meter = self.meter_rect(strip);
        push_rect(
            primitives,
            self.common.id,
            meter,
            if solo_dimmed {
                rgba(14, 15, 17, 255)
            } else {
                rgba(8, 13, 18, 255)
            },
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
        let meter_ratio = ratio_for_meter_db(readout.meter_db);
        let left_ratio = meter_ratio
            * if channel.pan > 0.0 {
                1.0 - channel.pan * 0.55
            } else {
                1.0
            };
        let right_ratio = meter_ratio
            * if channel.pan < 0.0 {
                1.0 + channel.pan * 0.55
            } else {
                1.0
            };
        for (index, ratio) in [left_ratio, right_ratio].into_iter().enumerate() {
            let lane_gap = 2.0;
            let lane_width = ((meter.width() - 6.0 - lane_gap) / 2.0).max(1.0);
            let x = meter.min.x + 3.0 + index as f32 * (lane_width + lane_gap);
            let meter_fill = Rect::from_min_max(
                Point::new(x, meter.max.y - (meter.height() - 6.0) * ratio),
                Point::new(x + lane_width, meter.max.y - 3.0),
            );
            push_rect(
                primitives,
                self.common.id,
                meter_fill,
                if solo_dimmed {
                    rgba(75, 80, 86, 180)
                } else {
                    meter_color(readout.meter_db)
                },
            );
        }
        let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(readout.peak_db);
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
            format!("{:+.0}", readout.meter_db),
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
        channel_index: usize,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let fader = self.fader_rect(strip);
        self.append_fader_track(primitives, fader, fader.center().x, solo_dimmed, theme);
        let knob_y = fader.max.y - fader.height() * self.fader_display_ratio(channel_index);
        let knob = Rect::from_min_size(
            Point::new(fader.min.x, knob_y - 8.0),
            Vector2::new(fader.width(), 16.0),
        );
        push_rect(
            primitives,
            self.common.id,
            knob,
            if solo_dimmed {
                rgba(86, 92, 100, 220)
            } else {
                theme.highlight_blue
            },
        );
        push_stroke(primitives, self.common.id, knob, theme.border_emphasis, 1.0);
    }

    fn append_fader_track(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        fader: Rect,
        center_x: f32,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
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

    fn append_sends(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel_index: usize,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        for send in 0..SEND_COUNT {
            let rect = self.send_rect(strip, send);
            push_rect(
                primitives,
                self.common.id,
                rect,
                if solo_dimmed {
                    rgba(24, 26, 29, 255)
                } else {
                    theme.bg_tertiary
                },
            );
            let fill = Rect::from_min_max(
                rect.min,
                Point::new(
                    rect.min.x + rect.width() * self.send_display_ratio(channel_index, send),
                    rect.max.y,
                ),
            );
            push_rect(
                primitives,
                self.common.id,
                fill,
                if solo_dimmed {
                    rgba(86, 92, 100, 170)
                } else {
                    send_color(send, theme)
                },
            );
            push_stroke(primitives, self.common.id, rect, theme.border, 1.0);
        }
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
        channel_index: usize,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        push_text(
            primitives,
            self.common.id,
            format!("{:+.1} dB", self.fader_display_db(channel_index)),
            Rect::from_min_size(
                Point::new(strip.min.x + 4.0, strip.max.y - 44.0),
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
                Point::new(strip.min.x + 4.0, strip.max.y - 24.0),
                Vector2::new(strip.width() - 12.0, 18.0),
            ),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
}
