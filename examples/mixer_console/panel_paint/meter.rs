use super::super::model::{MixerChannel, ratio_for_meter_db};
use super::super::paint::{meter_color, push_rect, push_text, rgba, translucent};
use super::super::panel::{MeterReadout, MixerPanelWidget};
use radiant::prelude::*;

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
            meter_db: channel.meter_db,
            peak_db: channel.peak_db,
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
        let y = meter.max.y - meter.height() * fraction;
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(meter.min.x, y), Point::new(meter.max.x, y + 1.0)),
            translucent(theme.grid_soft, 120),
        );
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
    let left_ratio = if channel.pan > 0.0 {
        meter_ratio * (1.0 - channel.pan * 0.55)
    } else {
        meter_ratio
    };
    let right_ratio = if channel.pan < 0.0 {
        meter_ratio * (1.0 + channel.pan * 0.55)
    } else {
        meter_ratio
    };
    for (index, ratio) in [left_ratio, right_ratio].into_iter().enumerate() {
        push_rect(
            primitives,
            widget.common.id,
            meter_lane_rect(meter, index, ratio),
            meter_fill_color(solo_dimmed, readout.meter_db),
        );
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
    let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(readout.peak_db);
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(meter.min.x + 2.0, peak_y),
            Point::new(meter.max.x - 2.0, peak_y + 2.0),
        ),
        peak_color(solo_dimmed, theme),
    );
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
        rgba(14, 15, 17, 255)
    } else {
        rgba(8, 13, 18, 255)
    }
}

fn meter_lane_rect(meter: Rect, index: usize, ratio: f32) -> Rect {
    let lane_gap = 2.0;
    let lane_width = ((meter.width() - 6.0 - lane_gap) / 2.0).max(1.0);
    let x = meter.min.x + 3.0 + index as f32 * (lane_width + lane_gap);
    Rect::from_min_max(
        Point::new(x, meter.max.y - (meter.height() - 6.0) * ratio),
        Point::new(x + lane_width, meter.max.y - 3.0),
    )
}

fn meter_fill_color(solo_dimmed: bool, meter_db: f32) -> Rgba8 {
    if solo_dimmed {
        rgba(75, 80, 86, 180)
    } else {
        meter_color(meter_db)
    }
}

fn peak_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(90, 94, 98, 160)
    } else {
        theme.highlight_orange
    }
}

fn readout_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(118, 123, 128, 180)
    } else {
        theme.text_muted
    }
}
