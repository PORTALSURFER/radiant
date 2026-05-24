use super::super::model::{MixerChannel, ratio_for_gain};
use super::super::paint::{fader_knob_color, push_rect, push_stroke, translucent};
use super::super::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_fader(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    let fader = widget.fader_rect(strip);
    let center_x = fader.center().x;
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(center_x - 2.0, fader.min.y),
            Point::new(center_x + 2.0, fader.max.y),
        ),
        fader_track_color(solo_dimmed, theme),
    );
    append_fader_marks(widget, primitives, fader, center_x, theme);
    append_fader_knob(widget, primitives, channel, fader, solo_dimmed, theme);
}

fn append_fader_marks(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    fader: Rect,
    center_x: f32,
    theme: &ThemeTokens,
) {
    for db in [-48.0, -24.0, -12.0, 0.0, 6.0] {
        let y = fader.max.y - fader.height() * ratio_for_gain(db);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(center_x - 10.0, y),
                Point::new(center_x + 10.0, y + 1.0),
            ),
            theme.grid_soft,
        );
    }
}

fn append_fader_knob(
    widget: &MixerPanelWidget,
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

fn fader_track_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        translucent(theme.grid_soft, 130)
    } else {
        theme.grid_strong
    }
}
