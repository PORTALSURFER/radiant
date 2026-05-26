use super::{SoloVisual, StripPaintState};
use crate::mixer_console::paint::{group_color, send_color};
use crate::mixer_console::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn strip_fill(
    widget: &MixerPanelWidget,
    state: StripPaintState,
    theme: &ThemeTokens,
) -> Rgba8 {
    if widget.selection.is_selected(state.channel_index) {
        theme
            .surface_raised
            .blend_opaque_toward(theme.highlight_blue, 0.20)
    } else if state.solo_visual == SoloVisual::Dimmed {
        theme
            .surface_base
            .blend_opaque_toward(theme.bg_primary, 0.42)
    } else {
        theme
            .surface_base
            .blend_opaque_toward(group_color(state.channel.group(), theme), 0.10)
    }
}

pub(super) fn strip_group_color(state: StripPaintState, theme: &ThemeTokens) -> Rgba8 {
    if state.solo_visual == SoloVisual::Dimmed {
        Rgba8::new(78, 82, 88, 180)
    } else {
        group_color(state.channel.group(), theme)
    }
}

pub(super) fn strip_label_color(state: StripPaintState, theme: &ThemeTokens) -> Rgba8 {
    if state.solo_visual == SoloVisual::Dimmed {
        theme.text_muted
    } else {
        theme.text_primary
    }
}

pub(super) fn send_track_color(state: StripPaintState, theme: &ThemeTokens) -> Rgba8 {
    if state.solo_visual == SoloVisual::Dimmed {
        Rgba8::new(24, 26, 29, 255)
    } else {
        theme.bg_tertiary
    }
}

pub(super) fn send_fill_color(state: StripPaintState, send: usize, theme: &ThemeTokens) -> Rgba8 {
    if state.solo_visual == SoloVisual::Dimmed {
        Rgba8::new(86, 92, 100, 170)
    } else {
        send_color(send, theme)
    }
}

pub(super) fn footer_gain_color(state: StripPaintState, theme: &ThemeTokens) -> Rgba8 {
    if state.solo_visual == SoloVisual::Dimmed {
        theme.text_muted.with_alpha(150)
    } else {
        theme.text_muted
    }
}
