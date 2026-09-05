use super::frame_scheduler_policy::discrete_input_completion_disposition;
use super::native_discrete_input_stage::{NativeDiscreteInputKind, NativeDiscreteInputStageTicket};
use super::{
    CpuFrameObservationOwner, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    GenericRouteOutcome, NativeAdapterGeneration, key_code_from_winit,
    keyboard_modifiers_from_winit, keypress_from_input,
};
use crate::gui::input::{InputTimestamp, KeyCode, KeyPress};
use crate::runtime::FocusTraversal;
use crate::runtime::SequentialFocusTraversalDisposition;
use crate::{runtime::RuntimeBridge, widgets::WidgetKey};
use std::time::Instant;
use winit::{
    event::{ElementState, KeyEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey, PhysicalKey},
};

mod commands;
mod repeat;
mod text_edit;

use repeat::should_route_keypress;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_keyboard_event(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {
        let Some(adapter_generation) = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
        else {
            return;
        };
        let Some((ticket, outcome)) =
            self.route_keyboard_event_inner(event_loop, event, adapter_generation, true)
        else {
            return;
        };
        let Some(disposition) =
            discrete_input_completion_disposition(self.complete_native_discrete_input(ticket))
        else {
            // The route already ran. A completion mismatch must not replay it
            // or apply a lower-stage fallback.
            return;
        };
        if let Some(outcome) = outcome {
            self.handle_route_outcome(
                event_loop,
                outcome.with_native_input_stage_disposition(disposition),
            );
        }
    }

    pub(super) fn route_keyboard_event_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyEvent,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
        wrapper_eligible: bool,
    ) -> Option<(NativeDiscreteInputStageTicket, Option<GenericRouteOutcome>)> {
        let _ = observation;
        let adapter_generation = adapter.capture_generation()?;
        self.route_keyboard_event_inner(event_loop, event, adapter_generation, wrapper_eligible)
    }

    fn route_keyboard_event_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyEvent,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> Option<(NativeDiscreteInputStageTicket, Option<GenericRouteOutcome>)> {
        let timestamp = InputTimestamp::capture();
        let ticket = self.begin_native_discrete_input_event(
            event_loop,
            NativeDiscreteInputKind::KeyboardInput,
            timestamp,
            adapter_generation,
            wrapper_eligible,
        )?;
        self.frame.text_renderer.reset_native_caret_affinities();
        let outcome = self.route_native_keyboard_event_inner(event, timestamp);
        Some((ticket, outcome))
    }

    fn route_native_keyboard_event_inner(
        &mut self,
        event: KeyEvent,
        timestamp: InputTimestamp,
    ) -> Option<GenericRouteOutcome> {
        if event.state == ElementState::Released {
            return self
                .route_native_key_release_with_timestamp(event.physical_key, Some(timestamp));
        }
        if event.state != ElementState::Pressed {
            return None;
        }
        self.sync_runtime_pointer_from_native_cursor();
        let command = commands::command_input(
            &event.logical_key,
            event.physical_key,
            self.input.modifiers,
            event.repeat,
            self.core.managed_composition_is_active(),
        );
        let logical_text = keyboard_event_text(&event);
        let physical_key = match event.physical_key {
            PhysicalKey::Code(code) => key_code_from_winit(code),
            PhysicalKey::Unidentified(_) => None,
        };
        self.route_native_key_press_inner(
            physical_key,
            &event.logical_key,
            logical_text,
            Some(timestamp),
            command,
        )
    }

    fn route_native_key_press_inner(
        &mut self,
        physical_key: Option<KeyCode>,
        logical_key: &Key,
        logical_text: Option<&str>,
        timestamp: Option<InputTimestamp>,
        mut command: crate::application::CommandInput,
    ) -> Option<GenericRouteOutcome> {
        let logical_text = logical_text.or(match logical_key {
            Key::Character(text) => Some(text.as_str()),
            _ => None,
        });
        let editing_key = if self.core.has_focused_text_input() {
            commands::logical_editing_key(logical_key).or(physical_key)
        } else {
            physical_key
        };
        command.text_consumed |= self.core.has_focused_text_input()
            && commands::required_text_key(editing_key, logical_text, self.input.modifiers);
        let command = &command;
        let repeat = command.repeat;
        let mut repeat_accepted = !repeat;
        let mut route_outcome = GenericRouteOutcome::default();
        let widget_modifiers = keyboard_modifiers_from_winit(self.input.modifiers);
        if repeat && physical_key == Some(KeyCode::Tab) && self.input.tab_sequence_latch.is_some() {
            return Some(self.core.route_consumed_input());
        }
        if let Some(outcome) = self.core.route_metadata_command_key_press(
            physical_key.map(|key| keypress_from_input(key, self.input.modifiers)),
            physical_key.and_then(WidgetKey::from_key_code),
            widget_modifiers,
            timestamp,
            command,
        ) {
            return Some(outcome);
        }
        if let Some(key) = physical_key {
            let allow_text_deletion_repeat = repeat
                && self.core.has_focused_text_input()
                && !self.input.modifiers.alt_key()
                && matches!(key, KeyCode::Backspace | KeyCode::Delete);
            if !should_route_keypress(
                key,
                repeat,
                allow_text_deletion_repeat,
                &mut self.input.last_navigation_key_repeat,
                Instant::now(),
            ) {
                // Preserve legacy text-repeat behavior. Other repeated keys still
                // reach the explicit semantic repeat policy before legacy fallback.
                return self.core.route_semantic_key_input(command);
            }
            repeat_accepted = true;
            Some(key)
        } else {
            None
        };
        if !repeat_accepted {
            return self.core.route_semantic_key_input(command);
        }
        if let Some(key) = physical_key {
            if self.route_required_text_key(
                editing_key.unwrap_or(key),
                logical_text,
                timestamp,
                repeat,
                &mut route_outcome,
            ) {
                return Some(route_outcome);
            }
            if let Some(outcome) = self.core.route_semantic_key_input(command) {
                return Some(outcome);
            }
            if !repeat
                && key == KeyCode::Tab
                && let Some(direction) = tab_traversal_direction(self.input.modifiers)
            {
                let (disposition, traversal_outcome) =
                    self.core.route_sequential_focus_with_disposition(direction);
                if let Some(latch) = tab_sequence_latch_for_disposition(direction, disposition) {
                    self.input.tab_sequence_latch = Some(latch);
                    return Some(
                        traversal_outcome.unwrap_or_else(|| self.core.route_consumed_input()),
                    );
                }
            }
            let outcome = self.core.route_key_press_with_timestamp(
                keypress_from_input(key, self.input.modifiers),
                WidgetKey::from_key_code(key),
                widget_modifiers,
                timestamp,
                repeat,
            );
            route_outcome.merge(outcome);
        }
        if physical_key.is_none()
            && self.core.has_focused_text_input()
            && let Some(key) = editing_key
            && self.route_required_text_key(
                key,
                logical_text,
                timestamp,
                repeat,
                &mut route_outcome,
            )
        {
            return Some(route_outcome);
        }
        if physical_key.is_none()
            && !self.input.modifiers.control_key()
            && !self.input.modifiers.super_key()
            && !self.input.modifiers.alt_key()
            && self.core.has_focused_text_input()
            && let Some(text) = logical_text
        {
            self.route_text_input(text, timestamp, &mut route_outcome);
            return Some(route_outcome);
        }
        if physical_key.is_none()
            && let Some(outcome) = self.core.route_semantic_key_input(command)
        {
            return Some(outcome);
        }
        if !route_outcome.routed
            && !self.core.has_focused_text_input()
            && let Some(press) = logical_shortcut_keypress_from_text(logical_text)
        {
            let outcome = self.core.route_key_press_with_timestamp(
                press,
                None,
                widget_modifiers,
                timestamp,
                false,
            );
            route_outcome.merge(outcome);
            if route_outcome.routed {
                return Some(route_outcome);
            }
        }
        if let Some(text) = logical_text {
            self.route_text_input_after_unhandled_keypress(text, timestamp, &mut route_outcome);
        } else if matches!(logical_key, Key::Named(NamedKey::Space)) {
            self.route_text_input_after_unhandled_keypress(" ", timestamp, &mut route_outcome);
        } else if let Key::Character(text) = logical_key {
            self.route_text_input_after_unhandled_keypress(
                text.as_str(),
                timestamp,
                &mut route_outcome,
            );
        }
        if !route_outcome.routed && matches!(logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_widget_key_with_metadata(
                WidgetKey::Backspace,
                widget_modifiers,
                repeat,
                timestamp,
            );
            route_outcome.merge(outcome);
        }
        if !route_outcome.routed && matches!(logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_widget_key_with_metadata(
                WidgetKey::Delete,
                widget_modifiers,
                repeat,
                timestamp,
            );
            route_outcome.merge(outcome);
        }
        Some(route_outcome)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_native_key_release(
        &mut self,
        physical_key: PhysicalKey,
    ) -> Option<GenericRouteOutcome> {
        self.route_native_key_release_with_timestamp(physical_key, Some(InputTimestamp::capture()))
    }

    fn route_native_key_release_with_timestamp(
        &mut self,
        physical_key: PhysicalKey,
        timestamp: Option<InputTimestamp>,
    ) -> Option<GenericRouteOutcome> {
        let PhysicalKey::Code(code) = physical_key else {
            return None;
        };
        let modifiers = keyboard_modifiers_from_winit(self.input.modifiers);
        let key = key_code_from_winit(code);
        if key == Some(KeyCode::Tab) && self.input.tab_sequence_latch.take().is_some() {
            return Some(self.core.route_consumed_input());
        }
        let widget_key = key.and_then(WidgetKey::from_key_code);
        match widget_key {
            Some(widget_key) => Some(
                self.core
                    .route_key_release_with_metadata(widget_key, modifiers, timestamp),
            ),
            None => self
                .core
                .route_metadata_key_release_with_metadata(None, modifiers, timestamp),
        }
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_native_tab_for_test(
        &mut self,
        repeat: bool,
    ) -> Option<GenericRouteOutcome> {
        self.sync_runtime_pointer_from_native_cursor();
        let logical_key = Key::Named(NamedKey::Tab);
        let command = commands::command_input(
            &logical_key,
            PhysicalKey::Code(winit::keyboard::KeyCode::Tab),
            self.input.modifiers,
            repeat,
            self.core.managed_composition_is_active(),
        );
        self.route_native_key_press_inner(
            Some(KeyCode::Tab),
            &logical_key,
            None,
            Some(InputTimestamp::capture()),
            command,
        )
    }

    pub(super) fn sync_runtime_pointer_from_native_cursor(&mut self) {
        self.core
            .set_current_pointer_position(self.input.last_cursor);
    }
}

fn keyboard_event_text(event: &KeyEvent) -> Option<&str> {
    event.text.as_ref().map(|text| text.as_str()).or_else(|| {
        if let Key::Character(text) = &event.logical_key {
            Some(text.as_str())
        } else {
            None
        }
    })
}

fn logical_shortcut_keypress_from_text(text: Option<&str>) -> Option<KeyPress> {
    Some(KeyPress::new(match text? {
        "[" => KeyCode::OpenBracket,
        "]" => KeyCode::CloseBracket,
        _ => return None,
    }))
}

fn tab_traversal_direction(modifiers: winit::keyboard::ModifiersState) -> Option<FocusTraversal> {
    if modifiers.control_key() || modifiers.super_key() || modifiers.alt_key() {
        return None;
    }
    Some(if modifiers.shift_key() {
        FocusTraversal::Backward
    } else {
        FocusTraversal::Forward
    })
}

fn tab_sequence_latch_for_disposition(
    direction: FocusTraversal,
    disposition: SequentialFocusTraversalDisposition,
) -> Option<super::runner_state::NativeTabSequenceLatch> {
    (!matches!(
        disposition,
        SequentialFocusTraversalDisposition::NoDestination
    ))
    .then_some(super::runner_state::NativeTabSequenceLatch { direction })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_shortcut_keypress_from_text_maps_bracket_characters() {
        assert_eq!(
            logical_shortcut_keypress_from_text(Some("[")),
            Some(KeyPress::new(KeyCode::OpenBracket))
        );
        assert_eq!(
            logical_shortcut_keypress_from_text(Some("]")),
            Some(KeyPress::new(KeyCode::CloseBracket))
        );
    }

    #[test]
    fn logical_shortcut_keypress_from_text_ignores_non_exact_bracket_text() {
        assert_eq!(logical_shortcut_keypress_from_text(Some("{")), None);
        assert_eq!(logical_shortcut_keypress_from_text(Some("[]")), None);
        assert_eq!(logical_shortcut_keypress_from_text(None), None);
    }

    #[test]
    fn tab_sequence_latch_covers_every_consumed_disposition() {
        let direction = FocusTraversal::Backward;
        let dispositions = [
            SequentialFocusTraversalDisposition::NoDestination,
            SequentialFocusTraversalDisposition::AdmittedWidget(1),
            SequentialFocusTraversalDisposition::AdmittedPrivateSplitPaneSeparator,
            SequentialFocusTraversalDisposition::Vetoed,
            SequentialFocusTraversalDisposition::Invalidated,
        ];

        for disposition in dispositions {
            let latch = tab_sequence_latch_for_disposition(direction, disposition);
            let expected = if matches!(
                disposition,
                SequentialFocusTraversalDisposition::NoDestination
            ) {
                None
            } else {
                Some(direction)
            };
            assert_eq!(
                latch.is_some(),
                expected.is_some(),
                "unexpected latch state for {disposition:?}"
            );
            assert_eq!(latch.map(|latch| latch.direction), expected);
        }
    }
}
