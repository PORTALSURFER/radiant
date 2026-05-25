//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod commands;
mod context;
mod events;
mod focus;
mod hit_order;
mod hit_test;
mod input;
mod interaction_state;
mod pointer;
mod scratch;
mod scroll;
mod state;
mod traversal_state;
mod work;

pub use commands::CommandOutcome;
pub use context::{RuntimeContext, RuntimeSurfaceFrame, RuntimeSurfaceFrameRef};
pub use events::{Event, PointerMoveOutcome};
pub use scroll::ScrollUpdate;

use super::{
    ClipAncestors, Command, DragSession, ExternalDragSession, RuntimeBridge, SurfaceTraversalIndex,
    UiSurface, WidgetDispatchResult, WidgetPath,
};
use crate::{
    gui::types::Rect,
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    widgets::{WidgetId, WidgetInput},
};
use interaction_state::{RuntimeInteractionState, ScrollDragCapture};
use scratch::RuntimeScratch;
use traversal_state::RuntimeTraversalState;
use work::RuntimeWorkQueues;

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
}

/// Stateful generic runtime controller for message-driven Radiant hosts.
///
/// The controller preserves one-way data flow:
/// 1. project an immutable [`UiSurface`] from host state
/// 2. run public layout on that surface
/// 3. route backend-neutral [`WidgetInput`] into a widget
/// 4. map widget output into a host-defined message
/// 5. reduce that message into host state
/// 6. project the next immutable surface snapshot
pub struct SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    bridge: Bridge,
    viewport: Rect,
    surface: UiSurface<Message>,
    layout_root: crate::layout::LayoutNode,
    layout_engine: LayoutEngine,
    layout: LayoutOutput,
    layout_state: LayoutState,
    layout_debug_options: LayoutDebugOptions,
    traversal: RuntimeTraversalState,
    scratch: RuntimeScratch,
    interaction: RuntimeInteractionState<Message>,
    repaint_requested: bool,
    exit_requested: bool,
    pending_input_command_outcome: CommandOutcome,
    runtime_work: RuntimeWorkQueues<Message>,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route one normalized widget interaction by widget id.
    ///
    /// Returns `true` when the interaction targeted a projected widget, even if
    /// that interaction did not emit a host-defined message.
    pub fn dispatch_input(&mut self, widget_id: WidgetId, input: WidgetInput) -> bool {
        self.dispatch_input_output(widget_id, input).is_some()
    }

    pub(super) fn dispatch_input_output(
        &mut self,
        widget_id: WidgetId,
        input: WidgetInput,
    ) -> Option<bool> {
        self.dispatch_input_output_with_refresh(widget_id, input, true)
    }

    pub(super) fn dispatch_input_output_with_refresh(
        &mut self,
        widget_id: WidgetId,
        input: WidgetInput,
        refresh_after_message: bool,
    ) -> Option<bool> {
        let bounds = self.layout.rects.get(&widget_id).copied()?;
        let result = self.dispatch_surface_input(widget_id, bounds, input)?;
        self.capture_pointer_capture_state(widget_id);
        let emitted_output = !matches!(result, WidgetDispatchResult::NoOutput);
        match result {
            WidgetDispatchResult::Message(message) => {
                let outcome = if refresh_after_message {
                    self.dispatch_message(message)
                } else {
                    let mut outcome = CommandOutcome::default();
                    self.dispatch_message_inner(message, &mut outcome);
                    outcome
                };
                self.pending_input_command_outcome.merge(outcome);
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => {}
        }
        Some(emitted_output)
    }
}
