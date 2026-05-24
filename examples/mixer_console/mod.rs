mod model;
mod paint;
mod panel;
mod panel_paint;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use model::{CHANNEL_COUNT, MixerChannel, MixerState};
pub(crate) use panel::MixerPanelWidget;
use radiant::prelude::{ListSelectionController, ListSelectionModifiers};
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
        MixerPanelMessage::Select { channel, modifiers } => {
            let channel = channel.min(CHANNEL_COUNT - 1);
            state.selected_channel = channel;
            state.selection.select(channel, CHANNEL_COUNT, modifiers);
        }
        MixerPanelMessage::SetGain {
            channel,
            ratio,
            selection,
        } => {
            let channel = channel.min(CHANNEL_COUNT - 1);
            if let Some(modifiers) = selection {
                state.selection.select(channel, CHANNEL_COUNT, modifiers);
            }
            if state.selection.is_selected(channel) && state.selection.selected_indices().len() > 1
            {
                let target_gain = model::gain_for_ratio(ratio);
                let source_gain = state.channels[channel].gain_db;
                let delta = target_gain - source_gain;
                let selected = state.selection.selected_indices().to_vec();
                for selected_channel in selected {
                    if let Some(channel_state) = state.channels.get_mut(selected_channel) {
                        channel_state.set_gain_from_db(channel_state.gain_db + delta);
                    }
                }
            } else if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.set_gain_from_ratio(ratio);
                state.selected_channel = channel;
            }
            state.selected_channel = channel;
        }
        MixerPanelMessage::SetSend {
            channel,
            send,
            ratio,
        } => {
            if let Some(channel_state) = state.channels.get_mut(channel)
                && let Some(send_state) = channel_state.sends.get_mut(send)
            {
                *send_state = ratio.clamp(0.0, 1.0);
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::Reorder { from, insert } => {
            reorder_channels(state, from, insert);
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

fn reorder_channels(state: &mut MixerState, from: usize, insert: usize) {
    let from = from.min(CHANNEL_COUNT - 1);
    let insert = insert.min(CHANNEL_COUNT);
    if is_reorder_noop(from, insert) {
        return;
    }

    let selected_ids: Vec<_> = state
        .selection
        .selected_indices()
        .iter()
        .filter_map(|channel| state.channels.get(*channel))
        .map(|channel| channel.id)
        .collect();
    let focused_id = state.channels[state.selected_channel.min(CHANNEL_COUNT - 1)].id;
    let mut channels = state.channels.to_vec();
    let moved = channels.remove(from);
    let adjusted_insert = if insert > from { insert - 1 } else { insert };
    channels.insert(adjusted_insert, moved);
    state.channels = channels
        .try_into()
        .expect("mixer reorder should preserve channel count");

    restore_selection_by_ids(&mut state.selection, &state.channels, &selected_ids);
    state.selected_channel = state
        .channels
        .iter()
        .position(|channel| channel.id == focused_id)
        .unwrap_or(adjusted_insert);
    if state.selection.selected_indices().is_empty() {
        state.selection.select(
            state.selected_channel,
            CHANNEL_COUNT,
            ListSelectionModifiers::new(),
        );
    }
}

fn restore_selection_by_ids(
    selection: &mut ListSelectionController,
    channels: &[MixerChannel; CHANNEL_COUNT],
    selected_ids: &[usize],
) {
    selection.clear();
    let mut restored_any = false;
    for (index, channel) in channels.iter().enumerate() {
        if selected_ids.contains(&channel.id) {
            let modifiers = if restored_any {
                ListSelectionModifiers::toggle()
            } else {
                ListSelectionModifiers::new()
            };
            selection.select(index, CHANNEL_COUNT, modifiers);
            restored_any = true;
        }
    }
}

pub(crate) fn is_reorder_noop(from: usize, insert: usize) -> bool {
    insert == from || insert == from + 1
}
