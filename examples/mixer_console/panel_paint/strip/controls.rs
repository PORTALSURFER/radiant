use super::StripPaintState;
use crate::mixer_console::paint::{push_rect, push_stroke, push_text};
use crate::mixer_console::panel::MixerPanelWidget;
use radiant::prelude::*;

#[derive(Clone, Copy)]
struct StripButton {
    index: usize,
    label: &'static str,
    active: bool,
    active_color: Rgba8,
}

pub(super) fn append_controls(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    theme: &ThemeTokens,
) {
    for button in buttons_for(state, theme) {
        append_button(widget, primitives, state, button, theme);
    }
}

fn buttons_for(state: StripPaintState, theme: &ThemeTokens) -> [StripButton; 3] {
    [
        StripButton {
            index: 0,
            label: "M",
            active: state.channel.muted,
            active_color: theme.accent_danger,
        },
        StripButton {
            index: 1,
            label: "S",
            active: state.channel.solo,
            active_color: theme.accent_warning,
        },
        StripButton {
            index: 2,
            label: "R",
            active: state.channel.armed,
            active_color: theme.highlight_cyan,
        },
    ]
}

fn append_button(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    button: StripButton,
    theme: &ThemeTokens,
) {
    let rect = widget.button_rect(state.rect, button.index);
    push_rect(
        primitives,
        widget.common.id,
        rect,
        button_fill(button, theme),
    );
    push_stroke(primitives, widget.common.id, rect, theme.border, 1.0);
    push_text(
        primitives,
        widget.common.id,
        button.label,
        rect,
        theme.text_primary,
        PaintTextAlign::Center,
    );
}

fn button_fill(button: StripButton, theme: &ThemeTokens) -> Rgba8 {
    if button.active {
        button.active_color
    } else {
        theme.bg_tertiary
    }
}
