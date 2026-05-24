use super::super::paint::{push_stroke, translucent};
use super::super::{MixerPanelMessage, is_reorder_noop};
use super::interaction::{button_or_select_message, list_selection_modifiers};
use super::{MixerDragTarget, MixerPanelWidget};
use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

impl Widget for MixerPanelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => self.handle_pointer_move(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if bounds.contains(position) => {
                self.handle_primary_press(bounds, position, modifiers)
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => self.handle_primary_release(bounds, position),
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.interaction = previous.interaction;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.append_console_paint(primitives, bounds, theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        if let Some(channel) = self.interaction.hover_channel {
            let strip = self.strip_rect(bounds, channel);
            push_stroke(
                primitives,
                self.common.id,
                strip,
                translucent(theme.highlight_cyan, 170),
                2.0,
            );
        }
        match self.interaction.drag_target {
            Some(MixerDragTarget::Fader(channel)) => {
                self.append_fader_drag_overlay(primitives, bounds, channel, theme);
            }
            Some(MixerDragTarget::Send { channel, send }) => {
                self.append_send_drag_overlay(primitives, bounds, channel, send, theme);
            }
            Some(MixerDragTarget::Strip(channel)) => {
                self.append_reorder_drag_overlay(primitives, bounds, channel, theme);
            }
            None => {}
        }
    }
}

impl MixerPanelWidget {
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.interaction.hover_position = bounds.contains(position).then_some(position);
        self.interaction.hover_channel = self.channel_at(bounds, position);
        if let Some(target) = self.interaction.drag_target {
            match target {
                MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => {
                    self.interaction.drag_preview_ratio =
                        Some(self.drag_ratio(bounds, target, position));
                }
                MixerDragTarget::Strip(_) => {
                    self.interaction.reorder_insert =
                        Some(self.insertion_index_at(bounds, position));
                }
            }
        }
        None
    }

    fn handle_primary_press(
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

    fn handle_fader_press(
        &mut self,
        strip: Rect,
        channel: usize,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let selection_update = (!self.selection.is_selected(channel)
            || modifiers.shift
            || modifiers.command
            || self.selection.selected_indices().len() <= 1)
            .then_some(list_selection_modifiers(modifiers));
        if selection_update.is_some() {
            self.apply_selection(channel, modifiers);
        }
        self.interaction.drag_target = Some(MixerDragTarget::Fader(channel));
        self.interaction.drag_preview_ratio = Some(self.fader_ratio_at(strip, position));
        self.interaction.drag_start_gains =
            Some(self.channels.map(|channel| channel.controls.gain_db));
        Some(WidgetOutput::custom(MixerPanelMessage::SetGain {
            channel,
            ratio: self.fader_ratio_at(strip, position),
            selection: selection_update,
        }))
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

    fn handle_primary_release(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
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
