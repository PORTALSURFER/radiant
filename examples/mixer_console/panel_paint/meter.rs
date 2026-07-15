use super::super::model::{MixerChannel, ratio_for_meter_db};
use super::super::paint::{meter_color, push_rect, push_text};
use super::super::panel::{MeterReadout, MixerPanelWidget};
use radiant::gui::feedback::{vertical_meter_lane_fill_rect, vertical_value_line_rect};
use radiant::prelude::*;
use radiant::runtime::PaintTextAlign;

pub(super) fn append_meter(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    append_meter_values(
        widget,
        primitives,
        channel,
        strip,
        solo_dimmed,
        MeterReadout {
            meter_db: channel.meter.meter_db,
            peak_db: channel.meter.peak_db,
        },
        theme,
    );
}

pub(super) fn append_meter_values(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
    readout: MeterReadout,
    theme: &ThemeTokens,
) {
    let meter = widget.meter_rect(strip);
    push_rect(
        primitives,
        widget.common.id,
        meter,
        meter_track_color(solo_dimmed),
    );
    append_meter_grid(widget, primitives, meter, theme);
    append_meter_lanes(widget, primitives, channel, meter, solo_dimmed, readout);
    append_peak_and_readout(widget, primitives, meter, solo_dimmed, readout, theme);
}

fn append_meter_grid(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    meter: Rect,
    theme: &ThemeTokens,
) {
    for fraction in [0.25, 0.5, 0.75] {
        if let Some(line) = vertical_value_line_rect(meter, fraction, 0.0, 1.0) {
            push_rect(
                primitives,
                widget.common.id,
                line,
                theme.grid_soft.with_alpha(120),
            );
        }
    }
}

fn append_meter_lanes(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    meter: Rect,
    solo_dimmed: bool,
    readout: MeterReadout,
) {
    let meter_ratio = ratio_for_meter_db(readout.meter_db);
    let left_ratio = if channel.controls.pan > 0.0 {
        meter_ratio * (1.0 - channel.controls.pan * 0.55)
    } else {
        meter_ratio
    };
    let right_ratio = if channel.controls.pan < 0.0 {
        meter_ratio * (1.0 + channel.controls.pan * 0.55)
    } else {
        meter_ratio
    };
    for (index, ratio) in [left_ratio, right_ratio].into_iter().enumerate() {
        if let Some(lane) = vertical_meter_lane_fill_rect(meter, index, 2, ratio, 2.0, 3.0) {
            push_rect(
                primitives,
                widget.common.id,
                lane,
                meter_fill_color(solo_dimmed, readout.meter_db),
            );
        }
    }
}

fn append_peak_and_readout(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    meter: Rect,
    solo_dimmed: bool,
    readout: MeterReadout,
    theme: &ThemeTokens,
) {
    if let Some(peak) =
        vertical_value_line_rect(meter, ratio_for_meter_db(readout.peak_db), 2.0, 2.0)
    {
        push_rect(
            primitives,
            widget.common.id,
            peak,
            peak_color(solo_dimmed, theme),
        );
    }
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.0}", readout.meter_db),
        Rect::from_min_size(
            Point::new(meter.min.x - 16.0, meter.max.y + 8.0),
            Vector2::new(meter.width() + 32.0, 18.0),
        ),
        readout_color(solo_dimmed, theme),
        PaintTextAlign::Center,
    );
}

fn meter_track_color(solo_dimmed: bool) -> Rgba8 {
    if solo_dimmed {
        Rgba8::new(14, 15, 17, 255)
    } else {
        Rgba8::new(8, 13, 18, 255)
    }
}

fn meter_fill_color(solo_dimmed: bool, meter_db: f32) -> Rgba8 {
    if solo_dimmed {
        Rgba8::new(75, 80, 86, 180)
    } else {
        meter_color(meter_db)
    }
}

fn peak_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        Rgba8::new(90, 94, 98, 160)
    } else {
        theme.highlight_orange
    }
}

fn readout_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        Rgba8::new(118, 123, 128, 180)
    } else {
        theme.text_muted
    }
}
