//! Runtime-owned admission and capture for version-3 layout interactions.

use super::{
    SurfaceRuntime, interaction_state::RuntimeLayoutPointerCapture,
    traversal_state::RuntimeLayoutHitTarget,
};
use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::Point,
    },
    layout::{
        LayoutEventContext, LayoutInput, LayoutInteraction, LayoutInteractionRevision,
        LayoutTargetIdentity, supports_layout_input_contract,
    },
    runtime::RuntimeBridge,
    widgets::PointerModifiers,
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LayoutInputDispatch {
    pub(super) handled: bool,
    pub(super) emitted_output: bool,
}

struct LayoutTargetBinding<Message> {
    identity: LayoutTargetIdentity,
    interaction: Rc<dyn LayoutInteraction<Message>>,
    revision: LayoutInteractionRevision,
}

fn layout_input_metadata(
    input: LayoutInput,
    fallback_position: Point,
) -> (
    Point,
    PointerModifiers,
    Option<InputTimestamp>,
    Option<InputSequenceRange>,
) {
    match input {
        LayoutInput::PointerMove {
            position,
            modifiers,
            timestamp,
            sequence_range,
        }
        | LayoutInput::PointerCaptureCancelled {
            position,
            modifiers,
            timestamp,
            sequence_range,
        } => (position, modifiers, timestamp, sequence_range),
        LayoutInput::PointerModifiersChanged {
            modifiers,
            timestamp,
        } => (fallback_position, modifiers, timestamp, None),
        LayoutInput::PointerPress {
            position,
            modifiers,
            timestamp,
            ..
        }
        | LayoutInput::PointerDoubleClick {
            position,
            modifiers,
            timestamp,
            ..
        }
        | LayoutInput::PointerRelease {
            position,
            modifiers,
            timestamp,
            ..
        } => (position, modifiers, timestamp, None),
    }
}

impl<Message> Clone for LayoutTargetBinding<Message> {
    fn clone(&self) -> Self {
        Self {
            identity: self.identity,
            interaction: Rc::clone(&self.interaction),
            revision: self.revision.clone(),
        }
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn layout_pointer_capture_active(&self) -> bool {
        self.interaction.layout_capture.is_some()
    }

    pub(super) fn layout_input_target_at(&self, point: Point) -> bool {
        self.layout_target_binding_at(point).is_some()
    }

    pub(super) fn dispatch_layout_input_at(
        &mut self,
        point: Point,
        input: LayoutInput,
        refresh_after_message: bool,
    ) -> LayoutInputDispatch {
        let Some(binding) = self.layout_target_binding_at(point) else {
            return LayoutInputDispatch::default();
        };
        self.dispatch_layout_binding(binding, input, refresh_after_message, true)
    }

    pub(super) fn dispatch_captured_layout_input(
        &mut self,
        input: LayoutInput,
        refresh_after_message: bool,
    ) -> LayoutInputDispatch {
        let Some(capture) = self.interaction.layout_capture.clone() else {
            return LayoutInputDispatch::default();
        };
        let binding = LayoutTargetBinding {
            identity: capture.identity,
            interaction: capture.interaction,
            revision: capture.revision,
        };
        let dispatch = self.dispatch_layout_binding(binding, input, refresh_after_message, false);
        if matches!(
            input,
            LayoutInput::PointerRelease { .. } | LayoutInput::PointerCaptureCancelled { .. }
        ) {
            self.interaction.layout_capture = None;
        }
        dispatch
    }

    pub(super) fn cancel_layout_pointer_capture(&mut self) -> bool {
        let Some(capture) = self.interaction.layout_capture.take() else {
            return false;
        };
        let binding = LayoutTargetBinding {
            identity: capture.identity,
            interaction: capture.interaction,
            revision: capture.revision,
        };
        let _ = self.dispatch_layout_binding(
            binding,
            LayoutInput::PointerCaptureCancelled {
                position: capture.last_position,
                modifiers: capture.last_modifiers,
                timestamp: capture.last_timestamp,
                sequence_range: capture.last_sequence_range,
            },
            false,
            false,
        );
        true
    }

    /// Rebind an active layout capture only across exact, compatible evidence.
    ///
    /// This is called after the new traversal and layout target projection are
    /// installed. A stale capture is taken before its cancellation callback is
    /// invoked, so the callback cannot recursively observe or mutate the same
    /// capture. Any message emitted by cancellation is reduced with deferred
    /// refresh semantics and will be applied by the next normal refresh turn.
    pub(super) fn reconcile_layout_pointer_capture(&mut self) {
        let Some(capture) = self.interaction.layout_capture.clone() else {
            return;
        };
        let Some(current) = self.layout_target_binding_for_identity(capture.identity) else {
            self.interaction.layout_capture = None;
            let _ = self.dispatch_layout_binding(
                LayoutTargetBinding {
                    identity: capture.identity,
                    interaction: capture.interaction,
                    revision: capture.revision,
                },
                LayoutInput::PointerCaptureCancelled {
                    position: capture.last_position,
                    modifiers: capture.last_modifiers,
                    timestamp: capture.last_timestamp,
                    sequence_range: capture.last_sequence_range,
                },
                false,
                false,
            );
            return;
        };
        if capture.revision.is_exact()
            && current.revision.is_exact()
            && capture.revision == current.revision
        {
            self.interaction.layout_capture = Some(RuntimeLayoutPointerCapture {
                identity: current.identity,
                revision: current.revision,
                interaction: current.interaction,
                last_position: capture.last_position,
                last_modifiers: capture.last_modifiers,
                last_timestamp: capture.last_timestamp,
                last_sequence_range: capture.last_sequence_range,
            });
            return;
        }

        self.interaction.layout_capture = None;
        let _ = self.dispatch_layout_binding(
            LayoutTargetBinding {
                identity: capture.identity,
                interaction: capture.interaction,
                revision: capture.revision,
            },
            LayoutInput::PointerCaptureCancelled {
                position: capture.last_position,
                modifiers: capture.last_modifiers,
                timestamp: capture.last_timestamp,
                sequence_range: capture.last_sequence_range,
            },
            false,
            false,
        );
    }

    fn layout_target_binding_at(&self, point: Point) -> Option<LayoutTargetBinding<Message>> {
        self.traversal
            .containers
            .layout_targets
            .iter()
            .rev()
            .find(|target| {
                supports_layout_input_contract(target.contract_version)
                    && target.target.bounds.contains(point)
            })
            .map(RuntimeLayoutHitTarget::binding)
    }

    fn layout_target_binding_for_identity(
        &self,
        identity: LayoutTargetIdentity,
    ) -> Option<LayoutTargetBinding<Message>> {
        self.traversal
            .containers
            .layout_targets
            .iter()
            .rev()
            .find(|target| {
                supports_layout_input_contract(target.contract_version)
                    && target.target.identity() == identity
            })
            .map(RuntimeLayoutHitTarget::binding)
    }

    fn dispatch_layout_binding(
        &mut self,
        binding: LayoutTargetBinding<Message>,
        input: LayoutInput,
        refresh_after_message: bool,
        allow_capture: bool,
    ) -> LayoutInputDispatch {
        let mut context = LayoutEventContext::new(binding.identity);
        if !matches!(input, LayoutInput::PointerCaptureCancelled { .. }) {
            let fallback_position = self
                .interaction
                .pointer
                .current_position
                .unwrap_or_else(|| Point::new(0.0, 0.0));
            let (position, modifiers, timestamp, sequence_range) =
                layout_input_metadata(input, fallback_position);
            if let Some(capture) = self
                .interaction
                .layout_capture
                .as_mut()
                .filter(|capture| capture.identity == binding.identity)
            {
                capture.last_position = position;
                capture.last_modifiers = modifiers;
                capture.last_timestamp = timestamp;
                capture.last_sequence_range = sequence_range;
            }
        }
        binding.interaction.handle_layout_input(input, &mut context);

        if context.repaint_requested() || context.work_requested() {
            self.repaint_requested = true;
        }

        let terminal_input = matches!(
            input,
            LayoutInput::PointerRelease { .. } | LayoutInput::PointerCaptureCancelled { .. }
        );
        if allow_capture && !terminal_input && context.handled() && context.capture_requested() {
            let fallback_position = self
                .interaction
                .pointer
                .current_position
                .unwrap_or_else(|| Point::new(0.0, 0.0));
            let (position, modifiers, timestamp, sequence_range) =
                layout_input_metadata(input, fallback_position);
            self.interaction.layout_capture = Some(RuntimeLayoutPointerCapture {
                identity: binding.identity,
                revision: binding.revision.clone(),
                interaction: Rc::clone(&binding.interaction),
                last_position: position,
                last_modifiers: modifiers,
                last_timestamp: timestamp,
                last_sequence_range: sequence_range,
            });
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
        }
        if context.release_requested()
            && self
                .interaction
                .layout_capture
                .as_ref()
                .is_some_and(|capture| capture.identity == binding.identity)
        {
            self.interaction.layout_capture = None;
        }

        let emitted_output = if let Some(message) = context.take_message() {
            let mut outcome = super::CommandOutcome::default();
            if refresh_after_message {
                outcome = self.dispatch_message(message);
            } else {
                self.dispatch_message_inner_deferred_refresh(message, &mut outcome);
            }
            self.pending_input_command_outcome.merge(outcome);
            true
        } else {
            false
        };

        LayoutInputDispatch {
            handled: context.handled(),
            emitted_output,
        }
    }
}

impl<Message> RuntimeLayoutHitTarget<Message> {
    fn binding(&self) -> LayoutTargetBinding<Message> {
        LayoutTargetBinding {
            identity: self.target.identity(),
            interaction: Rc::clone(&self.interaction),
            revision: self.revision.clone(),
        }
    }
}
