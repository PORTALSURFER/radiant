use super::super::super::{MixerPanelMessage, is_reorder_noop};
use super::super::interaction::button_or_select_message;
use super::super::{MixerDragTarget, MixerPanelWidget};
use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

impl MixerPanelWidget {
    pub(super) fn handle_primary_press(
        &mut self,
        bounds: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let channel = self.channel_at(bounds, position)?;
        let strip = self.strip_rect(bounds, channel);
        self.interaction.hover_channel = Some(channel);
        if self.fader_rect(strip).contains(position) {
            return self.handle_fader_press(strip, channel, position, modifiers);
        }
        if let Some(send) = self.send_at(strip, position) {
            return Some(self.handle_send_press(strip, channel, send, position, modifiers));
        }
        Some(self.handle_strip_press(strip, channel, position, modifiers))
    }

    fn handle_send_press(
        &mut self,
        strip: Rect,
        channel: usize,
        send: usize,
        position: Point,
        modifiers: PointerModifiers,
    ) -> WidgetOutput {
        self.apply_selection(channel, modifiers);
        self.interaction.drag_target = Some(MixerDragTarget::Send { channel, send });
        self.interaction.drag_preview_ratio = Some(self.send_ratio_at(strip, send, position));
        WidgetOutput::custom(MixerPanelMessage::SetSend {
            channel,
            send,
            ratio: self.send_ratio_at(strip, send, position),
        })
    }

    fn handle_strip_press(
        &mut self,
        strip: Rect,
        channel: usize,
        position: Point,
        modifiers: PointerModifiers,
    ) -> WidgetOutput {
        let message = button_or_select_message(self, strip, channel, position, modifiers);
        self.apply_selection(channel, modifiers);
        if matches!(message, MixerPanelMessage::Select { .. }) {
            self.interaction.drag_target = Some(MixerDragTarget::Strip(channel));
            self.interaction.reorder_insert = Some(channel);
        }
        WidgetOutput::custom(message)
    }

    pub(super) fn handle_primary_release(
        &mut self,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        let drag = self.interaction.drag_target.take();
        self.interaction.drag_preview_ratio = None;
        self.interaction.drag_start_gains = None;
        let reorder_insert = self.interaction.reorder_insert.take();
        self.interaction.hover_channel = self.channel_at(bounds, position);
        drag.and_then(|target| match target {
            MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => Some(WidgetOutput::custom(
                self.drag_message(bounds, target, position),
            )),
            MixerDragTarget::Strip(channel) => {
                let insert =
                    reorder_insert.unwrap_or_else(|| self.insertion_index_at(bounds, position));
                (!is_reorder_noop(channel, insert)).then(|| {
                    WidgetOutput::custom(MixerPanelMessage::Reorder {
                        from: channel,
                        insert,
                    })
                })
            }
        })
    }
}
