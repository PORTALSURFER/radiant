//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod accessibility;
mod automation_compositor;
mod auxiliary_focus;
mod clipboard;
mod commands;
mod composition;
mod context;
mod declarative_owner;
mod effects;
mod events;
mod focus;
mod fresh_surface_preparation;
mod hit_order;
mod hit_test;
mod host;
mod input;
mod interaction_state;
mod layout;
mod layout_state;
mod owner;
mod platform;
mod pointer;
mod prepared_layout;
mod refresh;
mod scratch;
mod scroll;
mod semantic_coordinate;
mod semantic_demand;
mod split_pane_ratio_action;
mod split_pane_semantics;
mod split_pane_separator;
mod state;
mod timers;
mod traversal_state;
mod virtual_layout;
mod work;

#[cfg(test)]
pub(crate) use auxiliary_focus::AuxiliaryFocusCommand;
pub(crate) use auxiliary_focus::AuxiliaryFocusRequest;
pub(crate) use focus::SequentialFocusTraversalDisposition;
#[allow(unused_imports)]
pub(crate) use split_pane_ratio_action::SplitPaneRatioAdjustmentDisposition;

#[cfg(target_os = "macos")]
pub(crate) use automation_compositor::{
    NormalizedSemanticPublicationFenceSet, VirtualLayoutNormalizedSemanticSidecar,
    VirtualLayoutNormalizedSemanticSidecarEntry,
};
pub(crate) use automation_compositor::{
    VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError,
};
pub use commands::CommandOutcome;
pub use context::{RuntimeContext, RuntimeSurfaceFrame, RuntimeSurfaceFrameRef};
pub use events::{Event, PointerClickOutcome, PointerMoveOutcome};
pub(crate) use fresh_surface_preparation::PreparedSurfaceRefresh;
pub use layout_state::{SurfaceLayoutStateDiagnostics, SurfaceLayoutStateReplacement};
pub(crate) use owner::AuxiliaryWindowOwner;
pub(crate) use refresh::BasePaintPlanContext;
pub use refresh::{
    IdentityAudit, SurfaceIdentityDiagnostics, SurfaceIdentityOwnership, SurfaceIdentityPath,
    SurfaceIdentityReplacement, SurfaceRefreshCounters, SurfaceRefreshDiagnostics,
    SurfaceRefreshTimings,
};

enum NativeTextPointerCaret {
    Pending(WidgetId, String, usize, crate::widgets::NativeCaretAffinity),
    Accepted(WidgetId, crate::widgets::NativeCaretAffinity),
}
pub(crate) use scroll::WheelOrScrollRoute;
pub use scroll::{ScrollUpdate, ScrollUpdateMetadata};
pub(crate) use virtual_layout::VirtualLayoutSemanticClassificationBatch;

use super::gpu_surface::GpuShaderPresentationUniformMailbox;
use super::{
    ClipAncestors, Command, DevtoolsOverlayOptions, DragSession, ExternalDragCompletion,
    ExternalDragIdentity, ExternalDragSession, PendingExternalDragCompletion, RuntimeBridge,
    RuntimeDiagnosticsRecorder, RuntimeLifecycleController, SurfaceTraversalIndex, UiSurface,
    UiUpdateHandlerDiagnosticsPolicy, WidgetDispatchResult, WidgetPath, WindowEnvironment,
};
use crate::{
    UiAffinity,
    gui::layout_core::{
        LayoutAuthorityEvidence, LayoutStateAuthorityOwner, MountedLayoutSourceAuthorityOwner,
        RootLayoutAuthorityOwner,
    },
    gui::types::Rect,
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    runtime::RuntimeLifecyclePhase,
    widgets::{WidgetId, WidgetInput},
};
use clipboard::InProcessClipboard;
use declarative_owner::{DeclarativeOwnerLedger, DeclarativeOwnerProjection};
use effects::WorkerEffects;
use interaction_state::{RuntimeInteractionState, ScrollDragCapture};
use owner::RuntimeOwner;
use platform::{PlatformCompletionRegistry, PlatformResultIngress};
use scratch::RuntimeScratch;
use std::collections::HashMap;
use std::time::Instant;
use timers::TimerEffects;
use traversal_state::RuntimeTraversalState;
use work::RuntimeWorkQueues;

/// Direction for deterministic backend-neutral sequential focus traversal.
///
/// [`SurfaceRuntime::traverse_focus`] may visit a private runtime-owned split
/// separator stop, while its public return value remains widget-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next focus stop in committed declarative order.
    Forward,
    /// Move to the previous focus stop in committed declarative order.
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
///
/// The runtime controller and its lifecycle state are owned by the UI runtime
/// and cannot be moved into a worker thread.
///
/// ```compile_fail
/// use radiant::{
///     layout::{ContainerPolicy, Vector2},
///     runtime::{SurfaceNode, SurfaceRuntime, UiSurface},
/// };
///
/// let runtime = SurfaceRuntime::new_declarative_owned(
///     (),
///     Vector2::new(1.0, 1.0),
///     |_| UiSurface::new(SurfaceNode::container(1, ContainerPolicy::default(), Vec::new())),
///     |_, _| {},
/// );
/// std::thread::spawn(move || drop(runtime));
/// ```
pub struct SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    _ui_affinity: UiAffinity,
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
    layout_root_authority: LayoutAuthorityEvidence<RootLayoutAuthorityOwner>,
    layout_state_authority: LayoutAuthorityEvidence<LayoutStateAuthorityOwner>,
    mounted_layout_source_authority: LayoutAuthorityEvidence<MountedLayoutSourceAuthorityOwner>,
    mounted_layout_source_present: bool,
    layout_authority_exhausted: bool,
    last_layout_state_diagnostics: SurfaceLayoutStateDiagnostics,
    layout_debug_options: LayoutDebugOptions,
    completed_layout: Option<CompletedLayoutContext>,
    external_layout_dirty: bool,
    traversal: RuntimeTraversalState<Message>,
    scratch: RuntimeScratch,
    interaction: RuntimeInteractionState<Message>,
    lifecycle: RuntimeLifecycleController,
    fresh_surface_active_generation: u64,
    #[allow(dead_code)]
    fresh_surface_request_revision: u64,
    #[allow(dead_code)]
    fresh_surface_request: Option<fresh_surface_preparation::FreshSurfaceRefreshRequest>,
    #[allow(dead_code)]
    fresh_surface_authority_exhausted: bool,
    host_closing_hook_called: bool,
    host_exit_hook_called: bool,
    gpu_shader_presentation_uniform_mailbox: GpuShaderPresentationUniformMailbox,
    gpu_shader_presentation_uniform_updates: Vec<super::GpuShaderPresentationUniformUpdate>,
    pub(in crate::runtime) repaint_requested: bool,
    pub(in crate::runtime) pending_current_surface_relayout: bool,
    pub(in crate::runtime) servicing_current_surface_relayout: bool,
    exit_requested: bool,
    pending_input_command_outcome: CommandOutcome,
    pending_native_text_pointer_caret: Option<NativeTextPointerCaret>,
    effect_owner: RuntimeOwner,
    auxiliary_effect_owners: HashMap<String, AuxiliaryWindowOwner>,
    runtime_work: RuntimeWorkQueues<Message>,
    platform_registry: PlatformCompletionRegistry<Message>,
    platform_results: std::sync::Arc<std::sync::Mutex<PlatformResultIngress>>,
    in_process_clipboard: InProcessClipboard,
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
    timed_repaint_clock: Option<Instant>,
    pub(in crate::runtime) devtools_overlay: DevtoolsOverlayOptions,
    pub(in crate::runtime) virtual_layout: virtual_layout::RuntimeVirtualLayoutState<Message>,
    pending_auxiliary_focus_requests: Vec<auxiliary_focus::AuxiliaryFocusRequest>,
    declarative_owner: DeclarativeOwnerProjection,
    declarative_owner_ledger: DeclarativeOwnerLedger,
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
    pub(in crate::runtime) const fn runtime_identity(&self) -> u64 {
        self.effect_owner.id()
    }

    pub(in crate::runtime) fn compose_split_pane_automation_snapshot(
        &self,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
    ) -> crate::gui::automation::GuiAutomationSnapshot {
        split_pane_semantics::compose_split_pane_automation_snapshot(
            ordinary,
            &self.traversal.containers.split_pane_separator_projections,
        )
    }

    pub(crate) fn timed_repaint_deadline(&self) -> Option<std::time::Instant> {
        if !self.lifecycle.accepts_work() {
            return None;
        }
        earlier_deadline(
            self.surface.timed_repaint_deadline(),
            self.interaction.tooltip.deadline,
        )
    }

    /// Install the clock used by runtime-owned timed interaction seams.
    ///
    /// Native callers leave this unset and retain the wall-clock fallback.
    /// Qualified deterministic hosts set it for the duration of each host
    /// operation so tooltip and widget hover deadlines share one virtual clock.
    pub(crate) fn set_timed_repaint_clock(&mut self, now: Option<Instant>) {
        self.timed_repaint_clock = now;
    }

    pub(in crate::runtime::controller) fn timed_repaint_now(&self) -> Instant {
        self.timed_repaint_clock.unwrap_or_else(Instant::now)
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
        if !self.lifecycle.accepts_work() {
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
        self.dispatch_direct_input_output(widget_id, input)
            .is_some()
    }

    /// Stage native retained-text hit-test geometry for the next pointer
    /// dispatch. The source string is carried with the caret so a stale paint
    /// plan cannot mutate a newly projected input.
    pub(crate) fn set_native_text_pointer_caret(
        &mut self,
        widget_id: WidgetId,
        source: &str,
        caret: usize,
        affinity: crate::widgets::NativeCaretAffinity,
    ) {
        self.pending_native_text_pointer_caret = Some(NativeTextPointerCaret::Pending(
            widget_id,
            source.to_owned(),
            caret,
            affinity,
        ));
    }

    pub(crate) fn clear_native_text_pointer_caret(&mut self) {
        self.pending_native_text_pointer_caret = None;
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
        if !self.lifecycle.accepts_work() {
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
