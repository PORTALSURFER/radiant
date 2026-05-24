use super::super::{CHANNEL_COUNT, MixerPanelMessage};
use super::{MixerDragTarget, MixerPanelWidget};
use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

impl MixerPanelWidget {
    pub(crate) fn apply_selection(&mut self, channel: usize, modifiers: PointerModifiers) {
        self.selected_channel = channel;
        self.selection
            .select(channel, CHANNEL_COUNT, list_selection_modifiers(modifiers));
    }

    pub(crate) fn drag_message(
        &self,
        bounds: Rect,
        target: MixerDragTarget,
        position: Point,
    ) -> MixerPanelMessage {
        match target {
            MixerDragTarget::Fader(channel) => {
                let strip = self.strip_rect(bounds, channel);
                MixerPanelMessage::SetGain {
                    channel,
                    ratio: self.fader_ratio_at(strip, position),
                    selection: None,
                }
            }
            MixerDragTarget::Send { channel, send } => {
                let strip = self.strip_rect(bounds, channel);
                MixerPanelMessage::SetSend {
                    channel,
                    send,
                    ratio: self.send_ratio_at(strip, send, position),
                }
            }
            MixerDragTarget::Strip(channel) => MixerPanelMessage::Reorder {
                from: channel,
                insert: self.insertion_index_at(bounds, position),
            },
        }
    }

    pub(crate) fn drag_ratio(&self, bounds: Rect, target: MixerDragTarget, position: Point) -> f32 {
        match target {
            MixerDragTarget::Fader(channel) => {
                let strip = self.strip_rect(bounds, channel);
                self.fader_ratio_at(strip, position)
            }
            MixerDragTarget::Send { channel, send } => {
                let strip = self.strip_rect(bounds, channel);
                self.send_ratio_at(strip, send, position)
            }
            MixerDragTarget::Strip(_) => 0.0,
        }
    }
}

pub(super) fn button_or_select_message(
    widget: &MixerPanelWidget,
    strip: Rect,
    channel: usize,
    position: Point,
    modifiers: PointerModifiers,
) -> MixerPanelMessage {
    if widget.button_rect(strip, 0).contains(position) {
        MixerPanelMessage::ToggleMute(channel)
    } else if widget.button_rect(strip, 1).contains(position) {
        MixerPanelMessage::ToggleSolo(channel)
    } else if widget.button_rect(strip, 2).contains(position) {
        MixerPanelMessage::ToggleArm(channel)
    } else {
        MixerPanelMessage::Select {
            channel,
            modifiers: list_selection_modifiers(modifiers),
        }
    }
}

pub(super) fn list_selection_modifiers(modifiers: PointerModifiers) -> ListSelectionModifiers {
    if modifiers.shift {
        ListSelectionModifiers::extend()
    } else if modifiers.command {
        ListSelectionModifiers::toggle()
    } else {
        ListSelectionModifiers::new()
    }
}
