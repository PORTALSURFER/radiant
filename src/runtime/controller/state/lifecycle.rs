use super::super::super::gpu_surface::{
    GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY, GpuShaderPresentationUniformMailbox,
};
use super::super::{
    PlatformCompletionRegistry, RuntimeInteractionState, RuntimeLifecycleController,
    RuntimeLifecyclePhase, RuntimeScratch, RuntimeTraversalState, RuntimeWorkQueues,
    SurfaceRuntime,
};
use crate::{
    UiAffinity,
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    runtime::{
        CommandOutcome, DeclarativeOwnedRuntimeBridge, DeclarativeRuntimeBridge,
        DevtoolsOverlayOptions, RuntimeBridge, SurfaceRuntimeProjection, UiSurface,
        surface::WidgetStateSyncPolicy,
    },
    widgets::PointerCapturePolicy,
};
use std::sync::Arc;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(bridge: Bridge, viewport: Vector2) -> Self {
        Self::new_with_environment(
            bridge,
            viewport,
            crate::runtime::WindowEnvironment::default(),
        )
    }

    /// Build a generic runtime controller with an explicit initial environment.
    ///
    /// This crate-visible seam lets qualified deterministic hosts install the
    /// shipped environment snapshot before the first projection. Public hosts
    /// should continue to use [`Self::new`] unless they own an equivalent
    /// environment boundary.
    pub(crate) fn new_with_environment(
        mut bridge: Bridge,
        viewport: Vector2,
        initial_environment: crate::runtime::WindowEnvironment,
    ) -> Self {
        let viewport = normalized_viewport(viewport);
        // Give environment-aware bridges the runtime-owned value before their
        // first projection.
        bridge.set_window_environment(initial_environment);
        let application_environment = bridge.application_environment();
        let mut surface = bridge.pull_surface();
        if let Some(environment) = application_environment {
            surface = surface.with_application_environment(environment);
        }
        surface.set_window_environment(initial_environment);
        // The initial projection lets declarative hosts discover scene-provided
        // capabilities before this immutable dispatch table is cached.
        let host_capabilities = bridge.host_capabilities();
        let SurfaceRuntimeProjection {
            layout_root,
            mut traversal,
            source,
        } = surface.runtime_projection();
        let effect_owner = super::super::owner::RuntimeOwner::new();
        let scratch = RuntimeScratch {
            projection_source: source,
            ..RuntimeScratch::default()
        };
        let mut runtime = Self {
            _ui_affinity: UiAffinity::new(),
            bridge,
            host_capabilities,
            viewport,
            window_environment: initial_environment,
            surface,
            layout_root,
            layout_engine: LayoutEngine::default(),
            layout: LayoutOutput::default(),
            layout_state: LayoutState::default(),
            layout_state_generation: 0,
            layout_root_authority: crate::gui::layout_core::LayoutAuthorityEvidence::new(1, 1),
            layout_state_authority: crate::gui::layout_core::LayoutAuthorityEvidence::new(1, 1),
            mounted_layout_source_authority: crate::gui::layout_core::LayoutAuthorityEvidence::new(
                1, 1,
            ),
            mounted_layout_source_present: false,
            layout_authority_exhausted: false,
            last_layout_state_diagnostics: super::super::SurfaceLayoutStateDiagnostics::startup(),
            layout_debug_options: LayoutDebugOptions::default(),
            completed_layout: None,
            external_layout_dirty: false,
            traversal: RuntimeTraversalState::default(),
            scratch,
            interaction: RuntimeInteractionState::with_runtime_identity(effect_owner.id()),
            lifecycle: RuntimeLifecycleController::starting(),
            fresh_surface_active_generation: 1,
            fresh_surface_request_revision: 0,
            fresh_surface_request: None,
            fresh_surface_authority_exhausted: false,
            host_closing_hook_called: false,
            host_exit_hook_called: false,
            gpu_shader_presentation_uniform_mailbox: GpuShaderPresentationUniformMailbox::default(),
            gpu_shader_presentation_uniform_updates: Vec::with_capacity(
                GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY,
            ),
            repaint_requested: false,
            pending_current_surface_relayout: false,
            servicing_current_surface_relayout: false,
            exit_requested: false,
            pending_input_command_outcome: CommandOutcome::default(),
            pending_native_text_pointer_caret: None,
            effect_owner: effect_owner.clone(),
            auxiliary_effect_owners: std::collections::HashMap::new(),
            runtime_work: RuntimeWorkQueues::default(),
            platform_registry: PlatformCompletionRegistry::new(effect_owner.clone()),
            platform_results: std::sync::Arc::new(std::sync::Mutex::new(
                super::super::platform::PlatformResultIngress::default(),
            )),
            in_process_clipboard: super::super::clipboard::InProcessClipboard::default(),
            worker_effects: super::super::effects::WorkerEffects::new(effect_owner.clone()),
            timer_effects: super::super::timers::TimerEffects::new(effect_owner),
            diagnostics: Default::default(),
            last_refresh_diagnostics: super::super::SurfaceRefreshDiagnostics::startup(),
            last_view_delta_diagnostics: crate::runtime::surface::ViewDeltaDiagnostics::startup(),
            latest_paint_segment_observation: crate::runtime::PaintSegmentObservation::unavailable(
            ),
            pending_frame_refresh: super::super::refresh::SurfaceRefreshFrameDiagnostics::startup(),
            refresh_counters: super::super::SurfaceRefreshCounters::startup(),
            base_paint_plan_reuse_eligible: false,
            identity_audit: super::super::IdentityAudit::default(),
            update_handler_diagnostics_policy: Default::default(),
            timed_repaint_clock: None,
            devtools_overlay: DevtoolsOverlayOptions::default(),
            virtual_layout: Default::default(),
            pending_auxiliary_focus_requests: Vec::new(),
            declarative_owner: Default::default(),
            declarative_owner_ledger: Default::default(),
        };
        runtime.prepare_virtual_layout_surface(&traversal.virtual_layout_registrations);
        let traversal = if runtime.virtual_layout.is_empty() {
            traversal
        } else {
            let layout_root = runtime.surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut runtime.scratch.projection_scroll_stack,
                &mut runtime.scratch.projection_child_path,
                &mut runtime.scratch.projection_source,
            );
            runtime.replace_layout_root(layout_root);
            runtime.rebuild_virtual_layout_shell_layout();
            runtime.materialize_virtual_layout_surface();
            let layout_root = runtime.surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut runtime.scratch.projection_scroll_stack,
                &mut runtime.scratch.projection_child_path,
                &mut runtime.scratch.projection_source,
            );
            runtime.replace_layout_root(layout_root);
            traversal
        };
        runtime.relayout_with_traversal(traversal);
        runtime.install_declarative_owner_projection();
        let _ = runtime.transition_lifecycle(RuntimeLifecyclePhase::Running);
        runtime
    }

    /// Return the current immutable native environment snapshot for this window.
    pub fn window_environment(&self) -> crate::runtime::WindowEnvironment {
        self.window_environment
    }

    /// Replace the native environment snapshot and notify the bridge once.
    ///
    /// Native adapters should call this before queueing the corresponding
    /// [`WindowEnvironmentChange`](crate::runtime::WindowEnvironmentChange).
    /// Equal snapshots are ignored so duplicate platform notifications do not
    /// invoke application hooks or trigger redundant projections.
    pub fn set_window_environment(
        &mut self,
        environment: crate::runtime::WindowEnvironment,
    ) -> bool {
        if self.window_environment == environment {
            return false;
        }
        self.window_environment = environment;
        self.external_layout_dirty = true;
        self.surface.set_window_environment(environment);
        self.bridge.set_window_environment(environment);
        true
    }

    pub(in crate::runtime::controller) fn transition_lifecycle(
        &mut self,
        next: RuntimeLifecyclePhase,
    ) -> bool {
        let transitioned = self.lifecycle.transition(next);
        if transitioned {
            self.traversal
                .rebuild_mixed_focus_order(next, &self.interaction.layout_state);
            if matches!(
                next,
                RuntimeLifecyclePhase::Recovering
                    | RuntimeLifecyclePhase::Closing
                    | RuntimeLifecyclePhase::Stopped
            ) {
                self.clear_separator_focus_owner();
            }
        }
        transitioned
    }

    pub(crate) fn begin_native_recovery(&mut self) -> bool {
        self.transition_lifecycle(RuntimeLifecyclePhase::Recovering)
    }

    pub(crate) fn finish_native_recovery(&mut self) -> bool {
        self.transition_lifecycle(RuntimeLifecyclePhase::Running)
    }

    pub(crate) fn acquire_auxiliary_effect_owner(
        &mut self,
        key: &str,
    ) -> super::super::owner::AuxiliaryWindowOwner {
        self.auxiliary_effect_owners
            .entry(key.to_owned())
            .or_insert_with(|| super::super::owner::AuxiliaryWindowOwner::new(key))
            .clone()
    }

    pub(crate) fn auxiliary_effect_owner_is_active(
        &self,
        owner: &super::super::owner::AuxiliaryWindowOwner,
    ) -> bool {
        self.auxiliary_effect_owners
            .get(owner.key())
            .is_some_and(|current| current.is_same_generation(owner) && current.is_open())
    }

    pub(crate) fn retire_auxiliary_effect_owner(
        &mut self,
        owner: &super::super::owner::AuxiliaryWindowOwner,
    ) -> bool {
        self.discard_pending_auxiliary_focus_requests_for(owner);
        let matches_current = self
            .auxiliary_effect_owners
            .get(owner.key())
            .is_some_and(|current| current.is_same_generation(owner));
        if !matches_current {
            return false;
        }
        owner.retire();
        self.worker_effects.retire_auxiliary_owner(owner);
        self.timer_effects.retire_auxiliary_owner(owner);
        self.platform_registry.retire_auxiliary_owner(owner);
        self.auxiliary_effect_owners.remove(owner.key());
        true
    }

    pub(in crate::runtime::controller) fn lifecycle_phase(&self) -> RuntimeLifecyclePhase {
        self.lifecycle.phase()
    }

    pub(in crate::runtime::controller) fn lifecycle_accepts_work(&self) -> bool {
        self.lifecycle.accepts_work()
    }

    pub(in crate::runtime::controller) const fn lifecycle_transition_sequence(&self) -> u64 {
        self.lifecycle.transition_sequence()
    }

    pub(in crate::runtime::controller) fn lifecycle_diagnostics(
        &self,
    ) -> crate::runtime::RuntimeLifecycleDiagnostics {
        self.lifecycle.diagnostics()
    }

    pub(crate) fn begin_closing(&mut self) -> bool {
        self.cancel_gesture_capture(crate::widgets::GestureCancellation::CaptureLost);
        self.cancel_pointer_ingress_sequences();
        if !self.transition_lifecycle(RuntimeLifecyclePhase::Closing) {
            return false;
        }
        self.reset_tooltip_hover_intent();
        self.declarative_owner_ledger.retire_all();
        self.host_on_runtime_closing();
        self.invalidate_external_drag();
        self.retire_virtual_layout();
        self.effect_owner.cancel();
        self.worker_effects.shutdown();
        self.auxiliary_effect_owners.clear();
        self.clear_pending_auxiliary_focus_requests();
        self.timer_effects.shutdown();
        self.runtime_work.fence_all();
        self.in_process_clipboard.clear();
        self.shutdown_platform_services();
        true
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        let _ = self.set_viewport_and_report_relayout(viewport);
    }

    /// Replace the viewport and report whether the rounded layout root changed.
    pub(crate) fn set_viewport_and_report_relayout(&mut self, viewport: Vector2) -> bool {
        let viewport = normalized_viewport(viewport);
        if same_rect_bits(self.viewport, viewport) {
            return false;
        }
        let previous_layout_viewport = layout_effective_viewport(self.viewport);
        let next_layout_viewport = layout_effective_viewport(viewport);
        self.viewport = viewport;
        if previous_layout_viewport == next_layout_viewport {
            return false;
        }
        if !self.relayout_virtual_layout_for_geometry() {
            self.relayout_current_surface();
        }
        self.service_pending_current_surface_relayout();
        true
    }

    pub(in crate::runtime::controller) fn widget_state_sync_policy(&self) -> WidgetStateSyncPolicy {
        self.interaction
            .pointer
            .capture
            .filter(|widget_id| {
                self.widget_pointer_capture_policy(*widget_id) == PointerCapturePolicy::Exclusive
            })
            .map(WidgetStateSyncPolicy::exclusive_pointer_capture)
            .unwrap_or_else(|| {
                WidgetStateSyncPolicy::retained_hover_owner(self.interaction.hover.widget)
            })
    }

    pub(in crate::runtime::controller) fn clear_stale_interaction_state(&mut self) {
        self.revalidate_focus_owner();
        if let Some(widget_id) = self.interaction.pointer.capture
            && !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        {
            if let Some(button) = self.interaction.pointer.capture_button {
                self.interaction.pointer.set_release_tombstone(button, true);
            }
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
        }
        if let Some(capture) = self.interaction.pointer.scroll_drag_capture
            && !self.traversal.containers.scroll.contains(capture.node_id)
        {
            self.interaction
                .pointer
                .set_release_tombstone(capture.button, true);
            self.interaction.pointer.scroll_drag_capture = None;
        }
        if self
            .interaction
            .hover
            .scroll_affordance
            .is_some_and(|node_id| !self.traversal.containers.scroll.contains(node_id))
        {
            self.interaction.hover.scroll_affordance = None;
            self.note_scroll_visibility_mutation();
        }
        if self
            .interaction
            .hover
            .scroll_viewport
            .is_some_and(|node_id| !self.traversal.containers.scroll.contains(node_id))
        {
            self.interaction.hover.scroll_viewport = None;
            self.note_scroll_visibility_mutation();
        }
        let activity_count = self.interaction.wheel.scroll_activity.len();
        self.interaction
            .wheel
            .scroll_activity
            .retain(|node_id, _| self.traversal.containers.scroll.contains(*node_id));
        if self.interaction.wheel.scroll_activity.len() != activity_count {
            self.note_scroll_visibility_mutation();
        }
        if self.interaction.hover.widget.is_some_and(|widget_id| {
            !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        }) {
            self.interaction.hover.widget = None;
        }
        if self
            .interaction
            .hover
            .container
            .is_some_and(|node_id| !self.traversal.containers.styled.contains(node_id))
        {
            self.interaction.hover.container = None;
        }
        let tooltip_target_is_stale = self.interaction.tooltip.target.is_some_and(|widget_id| {
            self.interaction.hover.widget != Some(widget_id)
                || self.interaction.pointer.capture.is_some()
                || !self
                    .traversal
                    .widgets
                    .paths
                    .current
                    .contains_key(&widget_id)
        });
        if tooltip_target_is_stale {
            self.reset_tooltip_hover_intent();
        }
    }
}

impl<State, Message, Project, Reduce>
    SurfaceRuntime<DeclarativeRuntimeBridge<State, Message, Project, Reduce>, Message>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build a runtime controller from state, a shared-surface projector, and a reducer.
    ///
    /// This is the direct runtime counterpart to [`DeclarativeRuntimeBridge::new`]
    /// for hosts and tests that do not need to name the intermediate bridge.
    pub fn new_declarative(
        state: State,
        viewport: Vector2,
        project: Project,
        reduce: Reduce,
    ) -> Self {
        Self::new(
            DeclarativeRuntimeBridge::new(state, project, reduce),
            viewport,
        )
    }
}

impl<State, Message, Project, Reduce>
    SurfaceRuntime<DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>, Message>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build a runtime controller from state, an owned-surface projector, and a reducer.
    ///
    /// This is the allocation-lean counterpart to [`Self::new_declarative`] for
    /// hosts and tests whose projector naturally builds a fresh [`UiSurface`].
    pub fn new_declarative_owned(
        state: State,
        viewport: Vector2,
        project: Project,
        reduce: Reduce,
    ) -> Self {
        Self::new(
            DeclarativeOwnedRuntimeBridge::new(state, project, reduce),
            viewport,
        )
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(
            normalized_viewport_axis(viewport.x),
            normalized_viewport_axis(viewport.y),
        ),
    )
}

fn normalized_viewport_axis(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

fn layout_effective_viewport(viewport: Rect) -> Rect {
    Rect::from_min_size(
        Point::new(viewport.min.x.floor(), viewport.min.y.floor()),
        Vector2::new(
            viewport.width().round().max(0.0),
            viewport.height().round().max(0.0),
        ),
    )
}

fn same_rect_bits(left: Rect, right: Rect) -> bool {
    left.min.x.to_bits() == right.min.x.to_bits()
        && left.min.y.to_bits() == right.min.y.to_bits()
        && left.max.x.to_bits() == right.max.x.to_bits()
        && left.max.y.to_bits() == right.max.y.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Point;
    use crate::layout::ContainerPolicy;
    use crate::runtime::{
        Command, ExternalDragRequest, RuntimeAnimationActivity, RuntimeAnimationHost,
        RuntimeHostCapabilities, RuntimeLifecycleHost, SurfaceNode, WidgetMessageMapper,
    };
    use crate::widgets::{DragHandleWidget, WidgetInput, WidgetSizing};
    use std::{
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    #[derive(Default)]
    struct LifecycleBridge {
        closing_calls: usize,
        hook_calls: usize,
        event_log: Option<Arc<Mutex<Vec<&'static str>>>>,
    }

    impl RuntimeBridge<usize> for LifecycleBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_lifecycle()
        }
    }

    impl RuntimeLifecycleHost for LifecycleBridge {
        fn on_runtime_closing(&mut self) {
            self.closing_calls += 1;
            if let Some(event_log) = self.event_log.as_ref() {
                event_log
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push("closing");
            }
        }

        fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
            self.hook_calls += 1;
            Some(serde_json::json!({ "hook_calls": self.hook_calls }))
        }
    }

    #[derive(Default)]
    struct ClosingGateBridge {
        animation_polls: Arc<AtomicUsize>,
        frame_queues: Arc<AtomicUsize>,
    }

    impl RuntimeBridge<()> for ClosingGateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                DragHandleWidget::new(7, WidgetSizing::fixed(Vector2::new(24.0, 80.0)))
                    .with_hover_chrome_only(),
                WidgetMessageMapper::none(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new().with_animation()
        }
    }

    impl RuntimeAnimationHost for ClosingGateBridge {
        fn animation_activity(&mut self) -> RuntimeAnimationActivity {
            self.animation_polls.fetch_add(1, Ordering::AcqRel);
            RuntimeAnimationActivity::frame_messages()
        }

        fn queue_animation_frame(&mut self) -> bool {
            self.frame_queues.fetch_add(1, Ordering::AcqRel);
            true
        }
    }

    #[test]
    fn construction_enters_running_phase() {
        let runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.0, 40.0));
        assert_eq!(runtime.lifecycle_phase(), RuntimeLifecyclePhase::Running);
        let diagnostics = runtime.runtime_diagnostics();
        assert!(diagnostics.lifecycle.available);
        assert_eq!(diagnostics.lifecycle.phase, RuntimeLifecyclePhase::Running);
        assert_eq!(diagnostics.lifecycle.transition_count, 1);
        assert_eq!(
            diagnostics.lifecycle.history,
            vec![crate::runtime::RuntimeLifecycleTransition {
                sequence: 1,
                from: RuntimeLifecyclePhase::Starting,
                to: RuntimeLifecyclePhase::Running,
            }]
        );
    }

    #[test]
    fn viewport_normalization_replaces_only_non_finite_and_non_positive_axes() {
        let cases = [
            (f32::NAN, 4.0, 1.0, 4.0),
            (f32::INFINITY, 4.0, 1.0, 4.0),
            (f32::NEG_INFINITY, 4.0, 1.0, 4.0),
            (0.0, 4.0, 1.0, 4.0),
            (-2.0, 4.0, 1.0, 4.0),
            (3.5, f32::NAN, 3.5, 1.0),
            (3.5, f32::INFINITY, 3.5, 1.0),
            (3.5, f32::NEG_INFINITY, 3.5, 1.0),
            (3.5, 0.0, 3.5, 1.0),
            (3.5, -2.0, 3.5, 1.0),
            (3.5, 4.0, 3.5, 4.0),
        ];

        for (x, y, expected_x, expected_y) in cases {
            let rect = normalized_viewport(Vector2::new(x, y));
            assert_eq!(rect.width(), expected_x);
            assert_eq!(rect.height(), expected_y);
        }
    }

    #[test]
    fn viewport_normalization_controls_effective_viewport_and_relayout() {
        let mut runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.4, 40.4));
        assert_eq!(runtime.context().viewport.width(), 80.4);
        assert_eq!(runtime.context().viewport.height(), 40.4);
        assert!(!runtime.set_viewport_and_report_relayout(Vector2::new(80.49, 40.49)));
        assert!(runtime.set_viewport_and_report_relayout(Vector2::new(81.0, 40.4)));
        assert_eq!(runtime.context().viewport.width(), 81.0);

        runtime.set_viewport(Vector2::new(f32::NAN, -1.0));
        assert_eq!(runtime.context().viewport.width(), 1.0);
        assert_eq!(runtime.context().viewport.height(), 1.0);
    }

    #[test]
    fn command_exit_closes_once_and_fences_late_commands() {
        let mut runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.0, 40.0));
        assert!(runtime.execute_command(Command::Exit).exit_requested);
        assert_eq!(runtime.lifecycle_phase(), RuntimeLifecyclePhase::Closing);
        assert_eq!(runtime.bridge().closing_calls, 1);
        assert!(!runtime.execute_command(Command::Exit).exit_requested);
        assert_eq!(runtime.lifecycle_phase(), RuntimeLifecyclePhase::Closing);
    }

    #[test]
    fn host_exit_hook_runs_once_and_stops_runtime() {
        let mut runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.0, 40.0));
        assert_eq!(
            runtime.host_on_runtime_exit(),
            Some(serde_json::json!({ "hook_calls": 1 }))
        );
        assert_eq!(runtime.lifecycle_phase(), RuntimeLifecyclePhase::Stopped);
        let diagnostics = runtime.runtime_diagnostics();
        assert_eq!(diagnostics.lifecycle.phase, RuntimeLifecyclePhase::Stopped);
        assert_eq!(diagnostics.lifecycle.transition_count, 3);
        assert_eq!(
            diagnostics
                .lifecycle
                .history
                .last()
                .map(|transition| (transition.from, transition.to)),
            Some((
                RuntimeLifecyclePhase::Closing,
                RuntimeLifecyclePhase::Stopped
            ))
        );
        assert_eq!(runtime.bridge().hook_calls, 1);
        assert_eq!(runtime.bridge().closing_calls, 1);
        assert_eq!(runtime.host_on_runtime_exit(), None);
        assert_eq!(runtime.bridge().hook_calls, 1);
    }

    #[test]
    fn closing_callback_precedes_external_drag_teardown_and_runs_once() {
        struct TeardownProbe(Arc<Mutex<Vec<&'static str>>>);
        impl Drop for TeardownProbe {
            fn drop(&mut self) {
                self.0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push("teardown");
            }
        }

        let event_log = Arc::new(Mutex::new(Vec::new()));
        let bridge = LifecycleBridge {
            event_log: Some(Arc::clone(&event_log)),
            ..LifecycleBridge::default()
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let probe = TeardownProbe(Arc::clone(&event_log));
        runtime.execute_command(Command::begin_external_drag(
            ExternalDragRequest::files([PathBuf::from("kick.wav")], "kick.wav"),
            move |_| {
                drop(probe);
                0
            },
        ));
        let _ = runtime.take_external_drag_launch();

        assert!(runtime.execute_command(Command::Exit).exit_requested);
        assert!(!runtime.execute_command(Command::Exit).exit_requested);
        assert_eq!(runtime.bridge().closing_calls, 1);
        assert_eq!(
            *event_log
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec!["closing", "teardown"]
        );
    }

    #[test]
    fn closing_stops_animation_and_timed_repaint_admission() {
        let mut runtime =
            SurfaceRuntime::new(ClosingGateBridge::default(), Vector2::new(80.0, 80.0));
        assert_eq!(
            runtime.host_animation_activity(),
            RuntimeAnimationActivity::frame_messages()
        );
        assert!(runtime.host_queue_animation_frame());
        assert_eq!(runtime.bridge().animation_polls.load(Ordering::Acquire), 1);
        assert_eq!(runtime.bridge().frame_queues.load(Ordering::Acquire), 1);

        assert_eq!(
            runtime.dispatch_input_at(
                Point::new(4.0, 4.0),
                WidgetInput::pointer_move(Point::new(4.0, 4.0))
            ),
            Some(7)
        );
        let deadline = runtime
            .timed_repaint_deadline()
            .expect("hover should arm a finite timed repaint");
        assert!(
            !runtime
                .surface()
                .find_widget(7)
                .expect("drag handle")
                .widget()
                .as_any()
                .downcast_ref::<DragHandleWidget>()
                .expect("drag handle type")
                .hover_highlight_revealed
        );

        assert!(runtime.begin_closing());
        assert_eq!(
            runtime.host_animation_activity(),
            RuntimeAnimationActivity::idle()
        );
        assert!(!runtime.host_queue_animation_frame());
        assert_eq!(runtime.bridge().animation_polls.load(Ordering::Acquire), 1);
        assert_eq!(runtime.bridge().frame_queues.load(Ordering::Acquire), 1);
        assert_eq!(runtime.timed_repaint_deadline(), None);
        assert!(!runtime.advance_timed_repaints(deadline));
        assert!(
            !runtime
                .surface()
                .find_widget(7)
                .expect("drag handle")
                .widget()
                .as_any()
                .downcast_ref::<DragHandleWidget>()
                .expect("drag handle type")
                .hover_highlight_revealed
        );
    }
}
