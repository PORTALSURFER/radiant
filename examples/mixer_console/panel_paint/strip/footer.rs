use super::{StripPaintState, style};
use crate::mixer_console::paint::push_text;
use crate::mixer_console::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_footer(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    theme: &ThemeTokens,
) {
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.1} dB", widget.fader_display_db(state.channel_index)),
        footer_rect(state.rect, 44.0),
        style::footer_gain_color(state, theme),
        PaintTextAlign::Center,
    );
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.0}", state.channel.controls.pan * 100.0),
        footer_rect(state.rect, 24.0),
        theme.text_muted,
        PaintTextAlign::Center,
    );
}

fn footer_rect(strip: Rect, bottom_offset: f32) -> Rect {
    Rect::from_min_size(
        Point::new(strip.min.x + 4.0, strip.max.y - bottom_offset),
        Vector2::new(strip.width() - 12.0, 18.0),
    )
}
