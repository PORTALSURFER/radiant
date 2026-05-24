use super::super::model::MixerChannel;
use super::super::panel::MixerPanelWidget;
use super::fader;
use super::meter;
use radiant::prelude::*;

#[path = "strip/controls.rs"]
mod controls;
#[path = "strip/footer.rs"]
mod footer;
#[path = "strip/sends.rs"]
mod sends;
#[path = "strip/shell.rs"]
mod shell;
#[path = "strip/style.rs"]
mod style;

#[derive(Clone, Copy, Debug)]
pub(super) struct StripPaintState {
    pub(super) channel_index: usize,
    pub(super) channel: MixerChannel,
    pub(super) rect: Rect,
    pub(super) solo_visual: SoloVisual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SoloVisual {
    Normal,
    Dimmed,
}

impl StripPaintState {
    fn new(widget: &MixerPanelWidget, bounds: Rect, channel_index: usize) -> Self {
        let channel = widget.channels[channel_index];
        let solo_active = widget.channels.iter().any(|channel| channel.flags.solo);
        let solo_visual = if channel.is_visually_dimmed_by_solo(solo_active) {
            SoloVisual::Dimmed
        } else {
            SoloVisual::Normal
        };
        Self {
            channel_index,
            channel,
            rect: widget.strip_rect(bounds, channel_index),
            solo_visual,
        }
    }

    pub(super) fn solo_dimmed(self) -> bool {
        self.solo_visual == SoloVisual::Dimmed
    }
}

pub(super) fn append_strip(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    channel_index: usize,
    theme: &ThemeTokens,
) {
    let state = StripPaintState::new(widget, bounds, channel_index);
    shell::append_shell(widget, primitives, state, theme);
    meter::append_meter(
        widget,
        primitives,
        state.channel,
        state.rect,
        state.solo_dimmed(),
        theme,
    );
    fader::append_fader(
        widget,
        primitives,
        state.channel_index,
        state.rect,
        state.solo_dimmed(),
        theme,
    );
    sends::append_sends(widget, primitives, state, theme);
    controls::append_controls(widget, primitives, state, theme);
    footer::append_footer(widget, primitives, state, theme);
}
