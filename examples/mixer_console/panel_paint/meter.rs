use super::super::model::{MixerChannel, ratio_for_meter_db};
use super::super::paint::{
    meter_color, meter_track_color, push_rect, push_text, rgba, translucent,
};
use super::super::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_meter(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
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
    append_meter_level(widget, primitives, channel, meter, solo_dimmed, theme);
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

fn append_meter_level(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    meter: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget.common.id,
        meter_fill_rect(meter, ratio_for_meter_db(channel.meter_db)),
        if solo_dimmed {
            rgba(75, 80, 86, 180)
        } else {
            meter_color(channel.meter_db)
        },
    );
    append_peak_and_readout(widget, primitives, channel, meter, solo_dimmed, theme);
}

fn append_peak_and_readout(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    meter: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(channel.peak_db);
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
        format!("{:+.0}", channel.meter_db),
        Rect::from_min_size(
            Point::new(meter.min.x - 16.0, meter.max.y + 8.0),
            Vector2::new(meter.width() + 32.0, 18.0),
        ),
        readout_color(solo_dimmed, theme),
        PaintTextAlign::Center,
    );
}

fn meter_fill_rect(meter: Rect, ratio: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            meter.min.x + 3.0,
            meter.max.y - (meter.height() - 6.0) * ratio,
        ),
        Point::new(meter.max.x - 3.0, meter.max.y - 3.0),
    )
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
