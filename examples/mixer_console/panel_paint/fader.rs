use super::super::model::ratio_for_gain;
use super::super::paint::{push_rect, push_stroke, rgba, translucent};
use super::super::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_fader(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    let fader = widget.fader_rect(strip);
    append_fader_track(
        widget,
        primitives,
        fader,
        fader.center().x,
        solo_dimmed,
        theme,
    );
    append_fader_knob(widget, primitives, channel_index, fader, solo_dimmed, theme);
}

pub(super) fn append_fader_track(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    fader: Rect,
    center_x: f32,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    if let Some(track) = vertical_center_track_rect(fader, 4.0) {
        push_rect(
            primitives,
            widget.common.id,
            track,
            fader_track_color(solo_dimmed, theme),
        );
    }
    append_fader_marks(widget, primitives, fader, center_x, theme);
}

fn append_fader_marks(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    fader: Rect,
    center_x: f32,
    theme: &ThemeTokens,
) {
    for db in [-48.0, -24.0, -12.0, 0.0, 6.0] {
        if let Some(mark) = vertical_value_line_rect(
            Rect::from_min_max(
                Point::new(center_x - 10.0, fader.min.y),
                Point::new(center_x + 10.0, fader.max.y),
            ),
            ratio_for_gain(db),
            0.0,
            1.0,
        ) {
            push_rect(primitives, widget.common.id, mark, theme.grid_soft);
        }
    }
}

fn append_fader_knob(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    fader: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    if let Some(knob) =
        vertical_value_knob_rect(fader, widget.fader_display_ratio(channel_index), 16.0)
    {
        push_rect(
            primitives,
            widget.common.id,
            knob,
            fader_knob_color(solo_dimmed, theme),
        );
        push_stroke(
            primitives,
            widget.common.id,
            knob,
            theme.border_emphasis,
            1.0,
        );
    }
}

fn fader_track_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        translucent(theme.grid_soft, 130)
    } else {
        theme.grid_strong
    }
}

fn fader_knob_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(86, 92, 100, 220)
    } else {
        theme.highlight_blue
    }
}
