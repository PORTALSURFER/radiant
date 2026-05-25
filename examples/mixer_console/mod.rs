mod model;
mod paint;
mod panel;
mod panel_paint;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use model::{CHANNEL_COUNT, MixerChannel, MixerState};
pub(crate) use panel::MixerPanelWidget;
use radiant::prelude::ListSelectionModifiers;
#[cfg(test)]
pub(crate) use update::update_panel;
pub(crate) use update::{is_reorder_noop, update};
pub(crate) use view::project_surface;

pub(crate) const MIXER_WIDGET_ID: u64 = 90;
pub(crate) const STATUS_WIDGET_ID: u64 = 91;
pub(crate) const DATA_SOURCE_NOTE: &str = "without_dsp";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MixerMessage {
    Frame,
    ToggleRun,
    Reset,
    Panel(MixerPanelMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MixerPanelMessage {
    Select {
        channel: usize,
        modifiers: ListSelectionModifiers,
    },
    SetGain {
        channel: usize,
        ratio: f32,
        selection: Option<ListSelectionModifiers>,
    },
    SetSend {
        channel: usize,
        send: usize,
        ratio: f32,
    },
    Reorder {
        from: usize,
        insert: usize,
    },
    ToggleMute(usize),
    ToggleSolo(usize),
    ToggleArm(usize),
}
