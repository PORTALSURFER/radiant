//! Focus, hover, pointer-capture, and drag state for the surface controller.

use super::{
    DragSession, ExternalDragCompletion, ExternalDragIdentity, ExternalDragSession,
    PendingExternalDragCompletion, layout_state::RuntimeLayoutContainerStateStore,
};
use crate::{
    gui::input::{InputSequenceRange, InputTimestamp, KeyPress},
    gui::types::Point,
    layout::{
        ContainerStateId, LayoutInteraction, LayoutInteractionRevision, LayoutTargetIdentity,
        NodeId,
    },
    widgets::{PointerButton, PointerModifiers, WidgetId, WidgetKey, WidgetState},
};
use std::rc::Rc;
use std::time::Instant;

pub(super) struct RuntimeInteractionState<Message> {
    pub(super) focus: RuntimeFocusState,
    pub(super) hover: RuntimeHoverState,
    pub(super) tooltip: RuntimeTooltipState,
    pub(super) pointer: RuntimePointerState,
    pub(super) wheel: RuntimeWheelState,
    pub(super) composition: RuntimeCompositionState,
    pub(super) layout_capture: Option<RuntimeLayoutPointerCapture<Message>>,
    pub(super) layout_state: RuntimeLayoutContainerStateStore,
    pub(super) drag: RuntimeDragState<Message>,
}

impl<Message> Default for RuntimeInteractionState<Message> {
    fn default() -> Self {
        Self {
            focus: RuntimeFocusState::default(),
            hover: RuntimeHoverState::default(),
            tooltip: RuntimeTooltipState::default(),
            pointer: RuntimePointerState::default(),
            wheel: RuntimeWheelState::default(),
            composition: RuntimeCompositionState::default(),
            layout_capture: None,
            layout_state: RuntimeLayoutContainerStateStore::default(),
            drag: RuntimeDragState::default(),
        }
    }
}

pub(super) struct RuntimeLayoutPointerCapture<Message> {
    pub(super) identity: LayoutTargetIdentity,
    pub(super) contract_version: u16,
    pub(super) state_id: Option<ContainerStateId>,
    pub(super) revision: LayoutInteractionRevision,
    pub(super) interaction: Rc<dyn LayoutInteraction<Message>>,
    pub(super) last_position: Point,
    pub(super) last_modifiers: PointerModifiers,
    pub(super) last_timestamp: Option<InputTimestamp>,
    pub(super) last_sequence_range: Option<InputSequenceRange>,
}

impl<Message> Clone for RuntimeLayoutPointerCapture<Message> {
    fn clone(&self) -> Self {
        Self {
            identity: self.identity,
            contract_version: self.contract_version,
            state_id: self.state_id,
            revision: self.revision.clone(),
            interaction: Rc::clone(&self.interaction),
            last_position: self.last_position,
            last_modifiers: self.last_modifiers,
            last_timestamp: self.last_timestamp,
            last_sequence_range: self.last_sequence_range,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeFocusState {
    pub(super) focused_widget: Option<WidgetId>,
    pub(super) pending_key_chord: Option<KeyPress>,
    pub(super) focused_key_capture: Option<RuntimeFocusedKeyCapture>,
}

/// Runtime-owned authority for one metadata-aware focused-key sequence.
///
/// The record is deliberately fixed-size and pins the stable focused widget
/// identity. `stale` is retained until the next routing boundary so a stale
/// sample can be ignored and cleaned up without being rebased to a successor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeFocusedKeyCapture {
    pub(super) widget_id: WidgetId,
    pub(super) key: WidgetKey,
    pub(super) stale: bool,
}

/// Runtime-owned lifecycle slot for one exact explicit wheel sequence.
///
/// The slot deliberately stores no history or orphan metadata. `Blocked` is a
/// current identityless state that prevents stale continuations from being
/// rebound to the widget currently under the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeWheelState {
    pub(super) managed_sequence: RuntimeManagedWheelSequenceState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RuntimeManagedWheelSequenceState {
    #[default]
    Idle,
    Active {
        widget_id: WidgetId,
    },
    Blocked,
}

/// Runtime-owned lifecycle slot for one exact managed composition.
///
/// Only the owner identity is retained.  `Blocked` is a fixed-size stale
/// boundary that prevents a later continuation from rebinding to another
/// focused widget; a fresh explicit `Start` is the only admission boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeCompositionState {
    pub(super) managed_composition: RuntimeManagedCompositionState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RuntimeManagedCompositionState {
    #[default]
    Idle,
    Active {
        widget_id: WidgetId,
    },
    Blocked,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeHoverState {
    pub(super) container: Option<NodeId>,
    pub(super) widget: Option<WidgetId>,
    pub(super) scroll_affordance: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeTooltipState {
    pub(super) target: Option<WidgetId>,
    pub(super) deadline: Option<Instant>,
    pub(super) revealed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct RuntimePointerState {
    pub(super) current_position: Option<Point>,
    pub(super) capture: Option<WidgetId>,
    pub(super) capture_state: Option<(WidgetId, WidgetState)>,
    pub(super) managed_capture: Option<RuntimeManagedPointerCapture>,
    pub(super) managed_release_tombstones: [bool; POINTER_BUTTON_COUNT],
    pub(super) scroll_drag_capture: Option<ScrollDragCapture>,
}

const POINTER_BUTTON_COUNT: usize = 3;

/// Fixed-size controller authority for one managed pointer press.
///
/// The record pins only the exact widget identity, initiating button, and the
/// short-lived lifecycle state needed while a press is dispatched or cancelled.
/// Orphaned releases live in the button-specific tombstone array on
/// [`RuntimePointerState`], without timestamps, sequences, generations, or
/// history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeManagedPointerCapture {
    pub(super) widget_id: WidgetId,
    pub(super) button: PointerButton,
    pub(super) state: RuntimeManagedPointerCaptureState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeManagedPointerCaptureState {
    Pending,
    Active,
    Cancelling,
}

impl RuntimePointerState {
    pub(super) fn has_managed_release_tombstone(&self, button: PointerButton) -> bool {
        self.managed_release_tombstones[pointer_button_index(button)]
    }

    pub(super) fn set_managed_release_tombstone(&mut self, button: PointerButton, value: bool) {
        self.managed_release_tombstones[pointer_button_index(button)] = value;
    }

    pub(super) fn has_any_managed_release_tombstone(&self) -> bool {
        self.managed_release_tombstones
            .iter()
            .any(|tombstone| *tombstone)
    }
}

fn pointer_button_index(button: PointerButton) -> usize {
    match button {
        PointerButton::Primary => 0,
        PointerButton::Secondary => 1,
        PointerButton::Auxiliary => 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ScrollDragCapture {
    pub(super) node_id: NodeId,
    pub(super) grip_fraction: f32,
}

pub(super) struct RuntimeDragState<Message> {
    pub(super) external_session: Option<ExternalDragSession<Message>>,
    pub(super) external_completion: Option<ExternalDragCompletion<Message>>,
    pub(super) external_identity: Option<ExternalDragIdentity>,
    pub(super) pending_external_completion: Option<PendingExternalDragCompletion<Message>>,
    pub(super) next_external_drag_id: u64,
    pub(super) external_drag_epoch: u64,
    pub(super) session: Option<DragSession>,
}

impl<Message> Default for RuntimeDragState<Message> {
    fn default() -> Self {
        Self {
            external_session: None,
            external_completion: None,
            external_identity: None,
            pending_external_completion: None,
            next_external_drag_id: 1,
            external_drag_epoch: 1,
            session: None,
        }
    }
}
