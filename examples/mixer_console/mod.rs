mod model;
mod paint;
mod panel;
mod panel_paint;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use model::{CHANNEL_COUNT, MixerChannel, MixerState};
pub(crate) use panel::MixerPanelWidget;
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
    Select(usize),
    SetGain { channel: usize, ratio: f32 },
    ToggleMute(usize),
    ToggleSolo(usize),
    ToggleArm(usize),
}

pub(crate) fn update(state: &mut MixerState, message: MixerMessage) {
    match message {
        MixerMessage::Frame => state.tick(),
        MixerMessage::ToggleRun => {
            state.running = !state.running;
        }
        MixerMessage::Reset => state.reset(),
        MixerMessage::Panel(message) => update_panel(state, message),
    }
}

pub(crate) fn update_panel(state: &mut MixerState, message: MixerPanelMessage) {
    match message {
        MixerPanelMessage::Select(channel) => {
            state.selected_channel = channel.min(CHANNEL_COUNT - 1);
        }
        MixerPanelMessage::SetGain { channel, ratio } => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.set_gain_from_ratio(ratio);
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleMute(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.muted = !channel_state.muted;
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleSolo(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.solo = !channel_state.solo;
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleArm(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.armed = !channel_state.armed;
                state.selected_channel = channel;
            }
        }
    }
}
