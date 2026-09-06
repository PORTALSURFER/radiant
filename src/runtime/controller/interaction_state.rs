//! Focus, hover, pointer-capture, and drag state for the surface controller.

use super::pointer_ingress::PointerIngressState;
use super::{
    DragSession, ExternalDragCompletion, ExternalDragIdentity, ExternalDragSession,
    PendingExternalDragCompletion, layout_state::RuntimeLayoutContainerStateStore,
};
#[cfg(test)]
use crate::widgets::CompositionPhase;
use crate::{
    gui::input::{InputSequenceRange, InputTimestamp, KeyPress},
    gui::layout_core::{MountedContainerStateId, SplitPaneRuntimePolicyRevision},
    gui::panel::SplitPaneAxis,
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
    pub(super) gesture: Option<super::gestures::GestureCapture>,
    pub(super) focus_restoration: super::focus_restoration::FocusRestorationState,
    pub(super) hover: RuntimeHoverState,
    pub(super) tooltip: RuntimeTooltipState,
    pub(super) pointer: RuntimePointerState,
    pub(super) wheel: RuntimeWheelState,
    pub(super) composition: RuntimeCompositionState,
    #[cfg(test)]
    pub(super) composition_dispatch_observations:
        Vec<(CompositionPhase, RuntimeManagedCompositionState)>,
    pub(super) layout_capture: Option<RuntimeLayoutPointerCapture<Message>>,
    pub(super) layout_state: RuntimeLayoutContainerStateStore,
    pub(super) drag: RuntimeDragState<Message>,
}

impl<Message> Default for RuntimeInteractionState<Message> {
    fn default() -> Self {
        Self {
            focus: RuntimeFocusState::default(),
            focus_restoration: Default::default(),
            gesture: None,
            hover: RuntimeHoverState::default(),
            tooltip: RuntimeTooltipState::default(),
            pointer: RuntimePointerState::default(),
            wheel: RuntimeWheelState::default(),
            composition: RuntimeCompositionState::default(),
            #[cfg(test)]
            composition_dispatch_observations: Vec::new(),
            layout_capture: None,
            layout_state: RuntimeLayoutContainerStateStore::default(),
            drag: RuntimeDragState::default(),
        }
    }
}

impl<Message> RuntimeInteractionState<Message> {
    pub(super) fn with_runtime_identity(runtime_identity: u64) -> Self {
        Self {
            pointer: RuntimePointerState::new(runtime_identity),
            ..Self::default()
        }
    }
}

pub(super) struct RuntimeLayoutPointerCapture<Message> {
    pub(super) identity: LayoutTargetIdentity,
    pub(super) contract_version: u16,
    pub(super) state_id: Option<ContainerStateId>,
    pub(super) revision: LayoutInteractionRevision,
    pub(super) interaction: Rc<dyn LayoutInteraction<Message>>,
    pub(super) button: Option<PointerButton>,
    pub(super) container_bounds: Option<crate::gui::types::Rect>,
    pub(super) target_bounds: Option<crate::gui::types::Rect>,
    pub(super) divider_bounds: Option<crate::gui::types::Rect>,
    pub(super) split_capture_witness: Option<crate::gui::layout_core::SplitPaneCaptureWitness>,
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
            button: self.button,
            container_bounds: self.container_bounds,
            target_bounds: self.target_bounds,
            divider_bounds: self.divider_bounds,
            split_capture_witness: self.split_capture_witness,
            last_position: self.last_position,
            last_modifiers: self.last_modifiers,
            last_timestamp: self.last_timestamp,
            last_sequence_range: self.last_sequence_range,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeFocusState {
    pub(super) command_context_widget: Option<WidgetId>,
    pub(super) owner: Option<RuntimeFocusOwner>,
    pub(super) pending_key_chord: Option<KeyPress>,
    pub(super) focused_key_capture: Option<RuntimeFocusedKeyCapture>,
    pub(super) focused_key_host_block: Option<(WidgetId, WidgetKey)>,
    pub(super) focused_semantic_key_block: Option<(WidgetId, WidgetKey)>,
}

/// Fixed-size behavior evidence required to retain a private separator owner.
///
/// The initial ratio is deliberately not included: it seeds a mounted runtime
/// slot, while the policy revision's compatibility relation covers the inputs
/// that can change the separator's behavior after mounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SplitPaneSeparatorBehaviorEvidence {
    pub(super) contract_version: u16,
    pub(super) state_schema_version: u16,
    pub(super) policy_revision: SplitPaneRuntimePolicyRevision,
}

impl SplitPaneSeparatorBehaviorEvidence {
    pub(super) const fn new(
        contract_version: u16,
        state_schema_version: u16,
        policy_revision: SplitPaneRuntimePolicyRevision,
    ) -> Self {
        Self {
            contract_version,
            state_schema_version,
            policy_revision,
        }
    }

    pub(super) fn compatible_with(self, current: Self) -> bool {
        self.contract_version == current.contract_version
            && self.state_schema_version == current.state_schema_version
            && self
                .policy_revision
                .runtime_state_compatible(current.policy_revision)
    }
}

/// Exact identity for one private runtime-owned split-pane separator owner.
///
/// Geometry and live ratio remain in the committed projection. Keeping only
/// identity and behavior evidence makes this owner fixed-size and lets
/// compatible geometry changes retain ownership without retaining stale rects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeSplitPaneSeparatorFocusOwner {
    pub(super) target: LayoutTargetIdentity,
    pub(super) mounted_state_id: MountedContainerStateId,
    pub(super) axis: SplitPaneAxis,
    pub(super) behavior: SplitPaneSeparatorBehaviorEvidence,
}

/// The one runtime focus owner. The separator variant is private ownership for
/// pointer acquisition and explicit sequential traversal; it is not a
/// widget-focus compatibility projection or public target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeFocusOwner {
    Widget(WidgetId),
    #[allow(dead_code)]
    SplitPaneSeparator(RuntimeSplitPaneSeparatorFocusOwner),
}

impl RuntimeFocusOwner {
    pub(super) const fn widget_id(self) -> Option<WidgetId> {
        match self {
            Self::Widget(widget_id) => Some(widget_id),
            Self::SplitPaneSeparator(_) => None,
        }
    }
}

impl RuntimeFocusState {
    pub(super) const fn focused_widget(self) -> Option<WidgetId> {
        match self.owner {
            Some(RuntimeFocusOwner::Widget(widget_id)) => Some(widget_id),
            Some(RuntimeFocusOwner::SplitPaneSeparator(_)) | None => None,
        }
    }
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
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RuntimeWheelState {
    pub(super) managed_sequence: RuntimeManagedWheelSequenceState,
    /// The latest accepted offset for each container in an explicit scroll
    /// sequence. Nested chaining may settle more than one owner per sample.
    pub(super) pending_scroll_settlement: Vec<(NodeId, crate::gui::types::Vector2)>,
    pub(super) scroll_settlement_deadline: Option<Instant>,
    /// Scroll owners whose Auto affordance is currently visible because of
    /// wheel activity. `None` means a phaseful sequence or drag is live;
    /// `Some` is the visual-idle expiry for phase-less/discrete input.
    pub(super) scroll_activity: std::collections::BTreeMap<NodeId, Option<Instant>>,
    pub(super) scroll_visibility_revision: u64,
    pub(super) scroll_visibility_revision_exhausted: bool,
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
    pub(super) scroll_viewport: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeTooltipState {
    pub(super) target: Option<WidgetId>,
    pub(super) deadline: Option<Instant>,
    pub(super) revealed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimePointerState {
    pub(super) current_position: Option<Point>,
    pub(super) capture: Option<WidgetId>,
    pub(super) capture_button: Option<PointerButton>,
    pub(super) capture_state: Option<(WidgetId, WidgetState)>,
    pub(super) managed_capture: Option<RuntimeManagedPointerCapture>,
    pub(super) release_tombstones: [bool; POINTER_BUTTON_COUNT],
    pub(super) scroll_drag_capture: Option<ScrollDragCapture>,
    pub(super) ingress: PointerIngressState,
}

impl Default for RuntimePointerState {
    fn default() -> Self {
        Self::new(1)
    }
}

impl RuntimePointerState {
    pub(super) fn new(runtime_identity: u64) -> Self {
        Self {
            current_position: None,
            capture: None,
            capture_button: None,
            capture_state: None,
            managed_capture: None,
            release_tombstones: [false; POINTER_BUTTON_COUNT],
            scroll_drag_capture: None,
            ingress: PointerIngressState::new(runtime_identity),
        }
    }
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
    pub(super) fn has_release_tombstone(&self, button: PointerButton) -> bool {
        self.release_tombstones[pointer_button_index(button)]
    }

    pub(super) fn set_release_tombstone(&mut self, button: PointerButton, value: bool) {
        self.release_tombstones[pointer_button_index(button)] = value;
    }

    pub(super) fn has_any_release_tombstone(&self) -> bool {
        self.release_tombstones.iter().any(|tombstone| *tombstone)
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
    pub(super) button: PointerButton,
    pub(super) axis: ScrollbarAxis,
    pub(super) start_offset: crate::gui::types::Vector2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScrollbarAxis {
    Horizontal,
    Vertical,
}

pub(super) struct RuntimeDragState<Message> {
    pub(super) typed: Option<super::gestures::drag_drop::TypedDragSession<Message>>,
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
            typed: None,
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
