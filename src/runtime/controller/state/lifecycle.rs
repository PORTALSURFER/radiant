use super::super::{
    PlatformCompletionRegistry, RuntimeInteractionState, RuntimePhase, RuntimeScratch,
    RuntimeTraversalState, RuntimeWorkQueues, SurfaceRuntime,
};
use crate::{
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
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = normalized_viewport(viewport);
        let surface = bridge.pull_surface();
        // The initial projection lets declarative hosts discover scene-provided
        // capabilities before this immutable dispatch table is cached.
        let host_capabilities = bridge.host_capabilities();
        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
        } = surface.runtime_projection();
        let mut runtime = Self {
            bridge,
            host_capabilities,
            viewport,
            surface,
            layout_root,
            layout_engine: LayoutEngine::default(),
            layout: LayoutOutput::default(),
            layout_state: LayoutState::default(),
            layout_debug_options: LayoutDebugOptions::default(),
            traversal: RuntimeTraversalState::default(),
            scratch: RuntimeScratch::default(),
            interaction: RuntimeInteractionState::default(),
            phase: RuntimePhase::Starting,
            host_closing_hook_called: false,
            host_exit_hook_called: false,
            repaint_requested: false,
            exit_requested: false,
            pending_input_command_outcome: CommandOutcome::default(),
            runtime_work: RuntimeWorkQueues::default(),
            platform_registry: PlatformCompletionRegistry::default(),
            platform_results: std::sync::Arc::new(std::sync::Mutex::new(
                super::super::platform::PlatformResultIngress::default(),
            )),
            worker_effects: super::super::effects::WorkerEffects::default(),
            timer_effects: super::super::timers::TimerEffects::default(),
            diagnostics: Default::default(),
            last_refresh_diagnostics: super::super::SurfaceRefreshDiagnostics::startup(),
            pending_frame_refresh_diagnostics: super::super::SurfaceRefreshDiagnostics::startup(),
            pending_frame_refresh_total: std::time::Duration::ZERO,
            refresh_counters: super::super::SurfaceRefreshCounters::startup(),
            update_handler_diagnostics_policy: Default::default(),
            devtools_overlay: DevtoolsOverlayOptions::default(),
        };
        runtime.relayout_with_traversal(traversal);
        runtime.phase = RuntimePhase::Running;
        runtime
    }

    pub(crate) fn begin_closing(&mut self) -> bool {
        if !self.phase.begin_closing() {
            return false;
        }
        self.host_on_runtime_closing();
        self.invalidate_external_drag();
        self.worker_effects.shutdown();
        self.timer_effects.shutdown();
        self.runtime_work.fence_all();
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
        if self.viewport == viewport {
            return false;
        }
        let previous_layout_viewport = layout_effective_viewport(self.viewport);
        let next_layout_viewport = layout_effective_viewport(viewport);
        self.viewport = viewport;
        if previous_layout_viewport == next_layout_viewport {
            return false;
        }
        self.relayout_current_surface();
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
        if self
            .interaction
            .focus
            .focused_widget
            .is_some_and(|widget_id| !self.traversal.widgets.focusable.contains(widget_id))
        {
            self.interaction.focus.focused_widget = None;
        }
        if self.interaction.pointer.capture.is_some_and(|widget_id| {
            !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        }) {
            self.interaction.pointer.capture = None;
        }
        if self
            .interaction
            .pointer
            .scroll_drag_capture
            .is_some_and(|capture| !self.traversal.containers.scroll.contains(capture.node_id))
        {
            self.interaction.pointer.scroll_drag_capture = None;
        }
        if self
            .interaction
            .hover
            .scroll_affordance
            .is_some_and(|node_id| !self.traversal.containers.scroll.contains(node_id))
        {
            self.interaction.hover.scroll_affordance = None;
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
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
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
        assert_eq!(runtime.phase, RuntimePhase::Running);
    }

    #[test]
    fn command_exit_closes_once_and_fences_late_commands() {
        let mut runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.0, 40.0));
        assert!(runtime.execute_command(Command::Exit).exit_requested);
        assert_eq!(runtime.phase, RuntimePhase::Closing);
        assert_eq!(runtime.bridge().closing_calls, 1);
        assert!(!runtime.execute_command(Command::Exit).exit_requested);
        assert_eq!(runtime.phase, RuntimePhase::Closing);
    }

    #[test]
    fn host_exit_hook_runs_once_and_stops_runtime() {
        let mut runtime = SurfaceRuntime::new(LifecycleBridge::default(), Vector2::new(80.0, 40.0));
        assert_eq!(
            runtime.host_on_runtime_exit(),
            Some(serde_json::json!({ "hook_calls": 1 }))
        );
        assert_eq!(runtime.phase, RuntimePhase::Stopped);
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
