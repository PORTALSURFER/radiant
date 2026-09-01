//! Runtime-owned admission and capture for version-3/version-4 layout
//! interactions.

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
        LayoutTargetIdentity, supports_layout_input_contract, supports_layout_state_input_contract,
    },
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers},
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LayoutInputDispatch {
    pub(super) handled: bool,
    pub(super) emitted_output: bool,
}

struct LayoutTargetBinding<Message> {
    identity: LayoutTargetIdentity,
    contract_version: u16,
    state_id: Option<crate::layout::ContainerStateId>,
    interaction: Rc<dyn LayoutInteraction<Message>>,
    revision: LayoutInteractionRevision,
    container_bounds: Option<crate::gui::types::Rect>,
    target_bounds: Option<crate::gui::types::Rect>,
    divider_bounds: Option<crate::gui::types::Rect>,
    split_capture_witness: Option<crate::gui::layout_core::SplitPaneCaptureWitness>,
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

fn layout_input_button(input: LayoutInput) -> Option<PointerButton> {
    match input {
        LayoutInput::PointerPress { button, .. }
        | LayoutInput::PointerDoubleClick { button, .. } => Some(button),
        LayoutInput::PointerMove { .. }
        | LayoutInput::PointerModifiersChanged { .. }
        | LayoutInput::PointerRelease { .. }
        | LayoutInput::PointerCaptureCancelled { .. } => None,
    }
}

impl<Message> Clone for LayoutTargetBinding<Message> {
    fn clone(&self) -> Self {
        Self {
            identity: self.identity,
            contract_version: self.contract_version,
            state_id: self.state_id,
            interaction: Rc::clone(&self.interaction),
            revision: self.revision.clone(),
            container_bounds: self.container_bounds,
            target_bounds: self.target_bounds,
            divider_bounds: self.divider_bounds,
            split_capture_witness: self.split_capture_witness,
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

    pub(super) fn layout_input_target_identity_and_bounds_at(
        &self,
        point: Point,
    ) -> Option<(LayoutTargetIdentity, crate::gui::types::Rect)> {
        let binding = self.layout_target_binding_at(point)?;
        Some((binding.identity, binding.target_bounds?))
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
        let terminal_input = matches!(
            input,
            LayoutInput::PointerRelease { .. } | LayoutInput::PointerCaptureCancelled { .. }
        );
        if let LayoutInput::PointerRelease { button, .. } = input
            && self
                .interaction
                .layout_capture
                .as_ref()
                .and_then(|capture| capture.button)
                .is_some_and(|expected| expected != button)
        {
            return LayoutInputDispatch::default();
        }
        let capture = if terminal_input {
            self.interaction.layout_capture.take()
        } else {
            self.interaction.layout_capture.clone()
        };
        let Some(capture) = capture else {
            return LayoutInputDispatch::default();
        };
        let binding = LayoutTargetBinding {
            identity: capture.identity,
            contract_version: capture.contract_version,
            state_id: capture.state_id,
            interaction: Rc::clone(&capture.interaction),
            revision: capture.revision.clone(),
            container_bounds: capture.container_bounds,
            target_bounds: capture.target_bounds,
            divider_bounds: capture.divider_bounds,
            split_capture_witness: capture.split_capture_witness,
        };
        self.dispatch_layout_binding(binding, input, refresh_after_message, false)
    }

    pub(super) fn cancel_layout_pointer_capture(&mut self) -> bool {
        let Some(capture) = self.interaction.layout_capture.take() else {
            return false;
        };
        if let Some(button) = capture.button {
            self.interaction.pointer.set_release_tombstone(button, true);
        }
        let binding = LayoutTargetBinding {
            identity: capture.identity,
            contract_version: capture.contract_version,
            state_id: capture.state_id,
            interaction: capture.interaction,
            revision: capture.revision,
            container_bounds: capture.container_bounds,
            target_bounds: capture.target_bounds,
            divider_bounds: capture.divider_bounds,
            split_capture_witness: capture.split_capture_witness,
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

    /// Retain an active layout capture only across exact, compatible evidence.
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
            if let Some(button) = capture.button {
                self.interaction.pointer.set_release_tombstone(button, true);
            }
            let _ = self.dispatch_layout_binding(
                LayoutTargetBinding {
                    identity: capture.identity,
                    contract_version: capture.contract_version,
                    state_id: capture.state_id,
                    interaction: capture.interaction,
                    revision: capture.revision,
                    container_bounds: capture.container_bounds,
                    target_bounds: capture.target_bounds,
                    divider_bounds: capture.divider_bounds,
                    split_capture_witness: capture.split_capture_witness,
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
            && capture.contract_version == current.contract_version
            && capture.state_id == current.state_id
            && capture.revision == current.revision
            && capture.split_capture_witness == current.split_capture_witness
        {
            self.interaction.layout_capture = Some(RuntimeLayoutPointerCapture {
                identity: capture.identity,
                contract_version: capture.contract_version,
                state_id: capture.state_id,
                revision: capture.revision,
                interaction: capture.interaction,
                button: capture.button,
                container_bounds: capture.container_bounds,
                target_bounds: capture.target_bounds,
                divider_bounds: capture.divider_bounds,
                split_capture_witness: capture.split_capture_witness,
                last_position: capture.last_position,
                last_modifiers: capture.last_modifiers,
                last_timestamp: capture.last_timestamp,
                last_sequence_range: capture.last_sequence_range,
            });
            return;
        }

        self.interaction.layout_capture = None;
        if let Some(button) = capture.button {
            self.interaction.pointer.set_release_tombstone(button, true);
        }
        let _ = self.dispatch_layout_binding(
            LayoutTargetBinding {
                identity: capture.identity,
                contract_version: capture.contract_version,
                state_id: capture.state_id,
                interaction: capture.interaction,
                revision: capture.revision,
                container_bounds: capture.container_bounds,
                target_bounds: capture.target_bounds,
                divider_bounds: capture.divider_bounds,
                split_capture_witness: capture.split_capture_witness,
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
        let mut context = LayoutEventContext::with_geometry(
            binding.identity,
            binding.container_bounds,
            binding.target_bounds,
            binding.divider_bounds,
        );
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
        let has_layout_state_input = supports_layout_state_input_contract(binding.contract_version)
            && binding.state_id.is_some();
        if supports_layout_state_input_contract(binding.contract_version) {
            let mut state = self
                .layout_container_state_context(binding.identity.container_id, binding.state_id);
            binding
                .interaction
                .handle_layout_input_with_state(input, &mut context, &mut state);
        } else {
            binding.interaction.handle_layout_input(input, &mut context);
        }
        if has_layout_state_input {
            self.note_mounted_layout_source_mutation(false);
        }

        // A release tombstone represents an already-retired capture. It is
        // cleared only after a fresh press/double-click was admitted by the
        // layout target, rather than merely because a target was hit.
        if context.handled()
            && let Some(button) = layout_input_button(input)
        {
            self.clear_pointer_release_tombstone_for_new_press(button);
        }

        if context.repaint_requested() || context.work_requested() {
            self.repaint_requested = true;
        }
        if context.work_requested() {
            self.queue_current_surface_relayout();
        }

        let terminal_input = matches!(
            input,
            LayoutInput::PointerRelease { .. } | LayoutInput::PointerCaptureCancelled { .. }
        );
        if allow_capture
            && !terminal_input
            && context.handled()
            && context.capture_requested()
            && let Some(button) = layout_input_button(input)
        {
            let fallback_position = self
                .interaction
                .pointer
                .current_position
                .unwrap_or_else(|| Point::new(0.0, 0.0));
            let (position, modifiers, timestamp, sequence_range) =
                layout_input_metadata(input, fallback_position);
            self.interaction.layout_capture = Some(RuntimeLayoutPointerCapture {
                identity: binding.identity,
                contract_version: binding.contract_version,
                state_id: binding.state_id,
                revision: binding.revision.clone(),
                interaction: Rc::clone(&binding.interaction),
                button: Some(button),
                container_bounds: binding.container_bounds,
                target_bounds: binding.target_bounds,
                divider_bounds: binding.divider_bounds,
                split_capture_witness: binding.split_capture_witness,
                last_position: position,
                last_modifiers: modifiers,
                last_timestamp: timestamp,
                last_sequence_range: sequence_range,
            });
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
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
            contract_version: self.contract_version,
            state_id: self.state_id,
            interaction: Rc::clone(&self.interaction),
            revision: self.revision.clone(),
            container_bounds: self.container_bounds,
            target_bounds: self.target_bounds,
            divider_bounds: self.divider_bounds,
            split_capture_witness: self.split_capture_witness,
        }
    }
}
