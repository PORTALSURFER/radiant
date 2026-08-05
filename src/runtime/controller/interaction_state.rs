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
    widgets::{PointerModifiers, WidgetId, WidgetState},
};
use std::rc::Rc;
use std::time::Instant;

pub(super) struct RuntimeInteractionState<Message> {
    pub(super) focus: RuntimeFocusState,
    pub(super) hover: RuntimeHoverState,
    pub(super) tooltip: RuntimeTooltipState,
    pub(super) pointer: RuntimePointerState,
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
    pub(super) scroll_drag_capture: Option<ScrollDragCapture>,
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
