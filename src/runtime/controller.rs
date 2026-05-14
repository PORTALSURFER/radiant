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
mod pointer;
mod scratch;
mod scroll;
mod state;

pub use commands::CommandOutcome;
pub use context::{RuntimeContext, RuntimeSurfaceFrame};
pub use events::{Event, PointerMoveOutcome};
pub use scroll::ScrollUpdate;

use super::{
    ClipAncestors, Command, PaintPrimitive, RuntimeBridge, SurfaceFrame, SurfacePaintPlan,
    SurfaceRuntimeProjection, SurfaceTraversalIndex, SurfaceWidget, UiSurface,
    WidgetDispatchResult, WidgetPath, estimated_paint_primitive_capacity,
};
use crate::{
    gui::{
        focus::FocusSurface,
        input::KeyPress,
        types::{Point, Rect, Vector2},
    },
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState, NodeId, OverflowPolicy},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput, WidgetKey, WidgetState},
};
use hit_order::HitOrderIndex;
use scratch::RuntimeScratch;
use std::collections::{HashMap, HashSet};

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScrollDragCapture {
    node_id: NodeId,
    grip_fraction: f32,
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
    widget_hit_order: Vec<WidgetId>,
    focusable_widgets: HitOrderIndex,
    pointer_widgets: HitOrderIndex,
    widget_paths: HashMap<WidgetId, WidgetPath>,
    previous_widget_paths: HashMap<WidgetId, WidgetPath>,
    container_hover_suppression: HashSet<WidgetId>,
    keyboard_focus_widgets: HitOrderIndex,
    wheel_widgets: HitOrderIndex,
    stateful_widget_order: Vec<WidgetId>,
    styled_containers: HitOrderIndex,
    scroll_containers: HitOrderIndex,
    widget_clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    container_clip_ancestors: HashMap<NodeId, ClipAncestors>,
    scroll_content_by_container: HashMap<NodeId, NodeId>,
    scratch: RuntimeScratch,
    focused_widget: Option<WidgetId>,
    pending_key_chord: Option<KeyPress>,
    hovered_container: Option<NodeId>,
    hovered_widget: Option<WidgetId>,
    pointer_capture: Option<WidgetId>,
    pointer_capture_state: Option<(WidgetId, WidgetState)>,
    hovered_scroll_affordance: Option<NodeId>,
    scroll_drag_capture: Option<ScrollDragCapture>,
    repaint_requested: bool,
    exit_requested: bool,
    runtime_commands: Vec<Command<Message>>,
    runtime_command_batch: Vec<Command<Message>>,
    runtime_messages: Vec<Message>,
    runtime_message_batch: Vec<Message>,
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
        let bounds = self.layout.rects.get(&widget_id).copied()?;
        let result = self.dispatch_surface_input(widget_id, bounds, input)?;
        self.capture_pointer_capture_state(widget_id);
        let emitted_output = !matches!(result, WidgetDispatchResult::NoOutput);
        match result {
            WidgetDispatchResult::Message(message) => {
                self.dispatch_message(message);
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => {}
        }
        Some(emitted_output)
    }
}
