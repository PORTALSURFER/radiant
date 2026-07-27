//! Focus, hover, pointer-capture, and drag state for the surface controller.

use super::{
    DragSession, ExternalDragCompletion, ExternalDragIdentity, ExternalDragSession,
    PendingExternalDragCompletion,
};
use crate::{
    gui::input::KeyPress,
    gui::types::Point,
    layout::NodeId,
    widgets::{WidgetId, WidgetState},
};

pub(super) struct RuntimeInteractionState<Message> {
    pub(super) focus: RuntimeFocusState,
    pub(super) hover: RuntimeHoverState,
    pub(super) pointer: RuntimePointerState,
    pub(super) drag: RuntimeDragState<Message>,
}

impl<Message> Default for RuntimeInteractionState<Message> {
    fn default() -> Self {
        Self {
            focus: RuntimeFocusState::default(),
            hover: RuntimeHoverState::default(),
            pointer: RuntimePointerState::default(),
            drag: RuntimeDragState::default(),
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
