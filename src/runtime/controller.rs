//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod commands;
mod context;
mod effects;
mod events;
mod focus;
mod hit_order;
mod hit_test;
mod host;
mod input;
mod interaction_state;
mod layout;
mod owner;
mod platform;
mod pointer;
mod refresh;
mod scratch;
mod scroll;
mod state;
mod timers;
mod traversal_state;
mod work;

pub use commands::CommandOutcome;
pub use context::{RuntimeContext, RuntimeSurfaceFrame, RuntimeSurfaceFrameRef};
pub use events::{Event, PointerClickOutcome, PointerMoveOutcome};
pub(crate) use refresh::BasePaintPlanContext;
pub use refresh::{
    IdentityAudit, SurfaceIdentityDiagnostics, SurfaceIdentityOwnership, SurfaceIdentityPath,
    SurfaceIdentityReplacement, SurfaceRefreshCounters, SurfaceRefreshDiagnostics,
    SurfaceRefreshTimings,
};
pub(crate) use scroll::WheelOrScrollRoute;
pub use scroll::{ScrollUpdate, ScrollUpdateMetadata};

use super::{
    ClipAncestors, Command, DevtoolsOverlayOptions, DragSession, ExternalDragCompletion,
    ExternalDragIdentity, ExternalDragSession, PendingExternalDragCompletion, RuntimeBridge,
    RuntimeDiagnosticsRecorder, SurfaceTraversalIndex, UiSurface, UiUpdateHandlerDiagnosticsPolicy,
    WidgetDispatchResult, WidgetPath, WindowEnvironment,
};
use crate::{
    gui::types::Rect,
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    widgets::{WidgetId, WidgetInput},
};
use effects::WorkerEffects;
use interaction_state::{RuntimeInteractionState, ScrollDragCapture};
use owner::RuntimeOwner;
use platform::{PlatformCompletionRegistry, PlatformResultIngress};
use scratch::RuntimeScratch;
use timers::TimerEffects;
use traversal_state::RuntimeTraversalState;
use work::RuntimeWorkQueues;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimePhase {
    Starting,
    Running,
    Closing,
    Recovering,
    Stopped,
}

impl RuntimePhase {
    fn accepts_work(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Recovering)
    }

    fn begin_closing(&mut self) -> bool {
        if !self.accepts_work() {
            return false;
        }
        *self = Self::Closing;
        true
    }
}

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
    host_capabilities: super::RuntimeHostCapabilities<Bridge, Message>,
    viewport: Rect,
    window_environment: WindowEnvironment,
    surface: UiSurface<Message>,
    layout_root: crate::layout::LayoutNode,
    layout_engine: LayoutEngine,
    layout: LayoutOutput,
    layout_state: LayoutState,
    layout_state_generation: u64,
    layout_debug_options: LayoutDebugOptions,
    completed_layout: Option<CompletedLayoutContext>,
    external_layout_dirty: bool,
    traversal: RuntimeTraversalState<Message>,
    scratch: RuntimeScratch,
    interaction: RuntimeInteractionState<Message>,
    phase: RuntimePhase,
    host_closing_hook_called: bool,
    host_exit_hook_called: bool,
    pub(in crate::runtime) repaint_requested: bool,
    exit_requested: bool,
    pending_input_command_outcome: CommandOutcome,
    effect_owner: RuntimeOwner,
    runtime_work: RuntimeWorkQueues<Message>,
    platform_registry: PlatformCompletionRegistry<Message>,
    platform_results: std::sync::Arc<std::sync::Mutex<PlatformResultIngress>>,
    worker_effects: WorkerEffects<Message>,
    timer_effects: TimerEffects<Message>,
    diagnostics: RuntimeDiagnosticsRecorder,
    last_refresh_diagnostics: SurfaceRefreshDiagnostics,
    last_view_delta_diagnostics: crate::runtime::surface::ViewDeltaDiagnostics,
    latest_paint_segment_observation: crate::runtime::PaintSegmentObservation,
    pending_frame_refresh: refresh::SurfaceRefreshFrameDiagnostics,
    refresh_counters: SurfaceRefreshCounters,
    pub(in crate::runtime) base_paint_plan_reuse_eligible: bool,
    identity_audit: IdentityAudit,
    update_handler_diagnostics_policy: UiUpdateHandlerDiagnosticsPolicy,
    pub(in crate::runtime) devtools_overlay: DevtoolsOverlayOptions,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CompletedLayoutContext {
    viewport: Rect,
    window_environment: WindowEnvironment,
    layout_state_generation: u64,
    layout_debug_options: LayoutDebugOptions,
}

/// Runtime controller for shared-surface declarative hosts.
///
/// This alias keeps app tests and lightweight hosts from spelling the full
/// `SurfaceRuntime<DeclarativeRuntimeBridge<...>, Message>` stack when they do
/// not need to name the bridge type separately.
pub type DeclarativeSurfaceRuntime<State, Message, Project, Reduce> =
    SurfaceRuntime<super::DeclarativeRuntimeBridge<State, Message, Project, Reduce>, Message>;

/// Runtime controller for owned-surface declarative hosts.
///
/// Use this when a host projects a fresh [`UiSurface`] on each refresh and only
/// needs to name the runtime controller type, not the intermediate owned bridge.
pub type DeclarativeOwnedSurfaceRuntime<State, Message, Project, Reduce> =
    SurfaceRuntime<super::DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>, Message>;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(crate) fn timed_repaint_deadline(&self) -> Option<std::time::Instant> {
        if !self.phase.accepts_work() {
            return None;
        }
        earlier_deadline(
            self.surface.timed_repaint_deadline(),
            self.interaction.tooltip.deadline,
        )
    }

    /// Return the private observational delta used at the paint materialization
    /// boundary. This evidence is not a renderer or public refresh policy API.
    pub(crate) const fn view_delta_diagnostics(
        &self,
    ) -> crate::runtime::surface::ViewDeltaDiagnostics {
        self.last_view_delta_diagnostics
    }

    pub(crate) fn record_paint_segment_observation(
        &mut self,
        observation: crate::runtime::PaintSegmentObservation,
    ) {
        self.latest_paint_segment_observation = observation;
    }

    pub(crate) const fn latest_paint_segment_observation(
        &self,
    ) -> crate::runtime::PaintSegmentObservation {
        self.latest_paint_segment_observation
    }

    pub(crate) fn advance_timed_repaints(&mut self, now: std::time::Instant) -> bool {
        if !self.phase.accepts_work() {
            return false;
        }
        let mut changed = self.surface.advance_timed_repaints(now);
        let Some(deadline) = self.interaction.tooltip.deadline else {
            return changed;
        };
        if now < deadline {
            return changed;
        }
        let target = self.interaction.tooltip.target;
        if target.is_some_and(|target| {
            self.interaction.hover.widget == Some(target)
                && self.interaction.pointer.capture.is_none()
                && self
                    .surface_widget(target)
                    .and_then(|widget| widget.tooltip())
                    .is_some_and(|tooltip| !tooltip.is_empty())
        }) {
            self.interaction.tooltip.deadline = None;
            self.interaction.tooltip.revealed = true;
            changed = true;
        } else {
            self.reset_tooltip_hover_intent();
        }
        changed
    }

    /// Route one normalized widget interaction by widget id.
    ///
    /// Returns `true` when the interaction targeted a projected widget, even if
    /// that interaction did not emit a host-defined message.
    pub fn dispatch_input(&mut self, widget_id: WidgetId, input: WidgetInput) -> bool {
        self.dispatch_input_output(widget_id, input).is_some()
    }

    /// Configure whether incompatible same-ID replacements are observational
    /// or fail after safe cleanup and diagnostics recording.
    pub fn set_identity_audit(&mut self, audit: IdentityAudit) {
        self.identity_audit = audit;
    }

    /// Return the active incompatible-replacement audit policy.
    pub const fn identity_audit(&self) -> IdentityAudit {
        self.identity_audit
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
        if !self.phase.accepts_work() {
            return None;
        }
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
                    self.dispatch_message_inner_deferred_refresh(message, &mut outcome);
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

fn earlier_deadline(
    current: Option<std::time::Instant>,
    candidate: Option<std::time::Instant>,
) -> Option<std::time::Instant> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (current, candidate) => current.or(candidate),
    }
}
