use super::{StripPaintState, style};
use crate::mixer_console::model::SEND_COUNT;
use crate::mixer_console::paint::{push_rect, push_stroke};
use crate::mixer_console::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_sends(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    theme: &ThemeTokens,
) {
    for send in 0..SEND_COUNT {
        append_send(widget, primitives, state, send, theme);
    }
}

fn append_send(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    send: usize,
    theme: &ThemeTokens,
) {
    let rect = widget.send_rect(state.rect, send);
    push_rect(
        primitives,
        widget.common.id,
        rect,
        style::send_track_color(state, theme),
    );
    if let Some(fill) =
        horizontal_progress_fill_rect(rect, widget.send_display_ratio(state.channel_index, send))
    {
        push_rect(
            primitives,
            widget.common.id,
            fill,
            style::send_fill_color(state, send, theme),
        );
    }
    push_stroke(primitives, widget.common.id, rect, theme.border, 1.0);
}
