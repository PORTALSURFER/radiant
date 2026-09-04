use super::SurfaceRuntime;
use crate::gui::types::Point;
use crate::runtime::ResolvedEnvironment;
use crate::runtime::UiUpdateHandlerDiagnosticsPolicy;
use crate::{
    gui::types::{Rect, Vector2},
    layout::{LayoutDebugOptions, LayoutOutput, NodeId},
    runtime::{
        GpuShaderPresentationUniformUpdate, RuntimeBridge, RuntimeDiagnostics, UiSurface,
        WindowEnvironment,
    },
    widgets::WidgetId,
};

mod frame;

pub use frame::{RuntimeSurfaceFrame, RuntimeSurfaceFrameRef};

/// Borrowed runtime context for one projected Radiant surface.
///
/// This context exposes the current viewport, immutable view tree, and resolved
/// layout without giving renderers or host code ownership of the runtime
/// controller. Style remains an explicit argument to paint-plan generation so
/// hosts can swap themes without rebuilding runtime state.
pub struct RuntimeContext<'a, Message> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Current immutable declarative view snapshot.
    pub surface: &'a UiSurface<Message>,
    /// Current resolved layout output for the surface.
    pub layout: &'a LayoutOutput,
}

impl<'a, Message> RuntimeContext<'a, Message> {
    /// Return the current immutable native environment for this window.
    pub const fn window_environment(&self) -> WindowEnvironment {
        self.surface.window_environment()
    }

    /// Return the current widget-facing environment projection.
    pub const fn resolved_environment(&self) -> ResolvedEnvironment {
        self.surface.window_environment().resolved()
    }

    /// Return the immutable application presentation snapshot for this surface.
    pub fn application_environment(&self) -> &crate::application::ApplicationEnvironment {
        self.surface.application_environment()
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return the current projected surface snapshot.
    pub fn surface(&self) -> &UiSurface<Message> {
        &self.surface
    }

    /// Return the current layout output for the projected surface.
    pub fn layout(&self) -> &LayoutOutput {
        &self.layout
    }

    /// Set the layout debug primitive policy and recompute the current layout.
    pub fn set_layout_debug_options(&mut self, options: LayoutDebugOptions) {
        if self.layout_debug_options == options {
            return;
        }
        self.layout_debug_options = options;
        self.relayout_current_surface();
    }

    /// Return the active layout debug primitive policy.
    pub fn layout_debug_options(&self) -> LayoutDebugOptions {
        self.layout_debug_options
    }

    /// Return a borrowed context view of the current runtime state.
    pub fn context(&self) -> RuntimeContext<'_, Message> {
        RuntimeContext {
            viewport: self.viewport,
            surface: &self.surface,
            layout: &self.layout,
        }
    }

    /// Return a generic runtime diagnostics snapshot for tests and debug panels.
    pub fn runtime_diagnostics(&self) -> RuntimeDiagnostics {
        let mut snapshot = self.host_runtime_diagnostics();
        let controller = self.diagnostics.snapshot();
        let host_current_messages = snapshot.queue.current_pending_messages;
        let controller_current_completions =
            controller.queue.current_pending_controller_completions;
        snapshot.queue.current_pending_messages =
            host_current_messages.saturating_add(controller_current_completions);
        snapshot.queue.max_pending_messages = snapshot
            .queue
            .max_pending_messages
            .max(controller.queue.max_pending_controller_completions)
            .max(snapshot.queue.current_pending_messages);
        snapshot.queue.current_pending_controller_completions = controller_current_completions;
        snapshot.queue.max_pending_controller_completions = snapshot
            .queue
            .max_pending_controller_completions
            .max(controller.queue.max_pending_controller_completions);
        snapshot.queue.controller_completion_deferrals = snapshot
            .queue
            .controller_completion_deferrals
            .saturating_add(controller.queue.controller_completion_deferrals);
        snapshot.queue.stream_events_coalesced = snapshot
            .queue
            .stream_events_coalesced
            .saturating_add(controller.queue.stream_events_coalesced);
        snapshot.queue.stream_events_stale = snapshot
            .queue
            .stream_events_stale
            .saturating_add(controller.queue.stream_events_stale);
        snapshot.queue.stream_events_dropped = snapshot
            .queue
            .stream_events_dropped
            .saturating_add(controller.queue.stream_events_dropped);
        snapshot.queue.last_platform_owner_kind = controller.queue.last_platform_owner_kind;
        snapshot.ui = controller.ui;
        snapshot.lifecycle = self.lifecycle_diagnostics();
        snapshot
    }

    /// Configure update-handler responsiveness diagnostics for this runtime.
    ///
    /// Use [`UiUpdateHandlerDiagnosticsPolicy::panic_at`] in tests or
    /// development harnesses that should fail when UI handlers block. Use
    /// [`UiUpdateHandlerDiagnosticsPolicy::disabled`] only for hosts that need
    /// to remove even the timing read from an otherwise verified release path.
    pub fn set_update_handler_diagnostics_policy(
        &mut self,
        policy: UiUpdateHandlerDiagnosticsPolicy,
    ) {
        self.update_handler_diagnostics_policy = policy;
    }

    /// Return the active update-handler diagnostics policy.
    pub fn update_handler_diagnostics_policy(&self) -> UiUpdateHandlerDiagnosticsPolicy {
        self.update_handler_diagnostics_policy
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    ///
    /// Returns `None` while private runtime ownership is held by a split-pane
    /// separator; separators are not public widget focus targets.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.interaction.focus.focused_widget()
    }

    /// Return the widget that currently owns pointer capture.
    pub fn pointer_capture(&self) -> Option<WidgetId> {
        self.interaction.pointer.capture
    }

    /// Return the layout target that currently owns runtime pointer capture.
    pub fn layout_pointer_capture(&self) -> Option<crate::layout::LayoutTargetIdentity> {
        self.interaction
            .layout_capture
            .as_ref()
            .map(|capture| capture.identity)
    }

    /// Return the latest logical pointer position observed by this runtime.
    pub fn current_pointer_position(&self) -> Option<Point> {
        self.interaction.pointer.current_position
    }

    /// Replace the latest logical pointer position observed by this runtime.
    pub fn set_current_pointer_position(&mut self, position: Option<Point>) {
        self.interaction.pointer.current_position = position;
    }

    pub(crate) fn interactive_pointer_route_active(&self) -> bool {
        self.interaction.pointer.capture.is_some()
            || self.interaction.layout_capture.is_some()
            || self.interaction.drag.session.is_some()
    }

    /// Return the widget currently receiving hover state.
    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.interaction.hover.widget
    }

    /// Return the styled container currently receiving hover chrome.
    pub fn hovered_container(&self) -> Option<NodeId> {
        self.interaction.hover.container
    }

    /// Return the scroll affordance currently receiving hover or drag emphasis.
    pub fn hovered_scroll_affordance(&self) -> Option<NodeId> {
        self.interaction.hover.scroll_affordance
    }

    /// Return whether the host update flow requested another repaint.
    pub fn repaint_requested(&self) -> bool {
        self.repaint_requested
    }

    /// Return and clear the current repaint request flag.
    pub fn take_repaint_requested(&mut self) -> bool {
        let repaint_requested = self.repaint_requested;
        self.repaint_requested = false;
        repaint_requested
    }

    #[cfg(test)]
    pub(crate) fn take_gpu_shader_presentation_updates(
        &mut self,
    ) -> &[GpuShaderPresentationUniformUpdate] {
        self.gpu_shader_presentation_uniform_mailbox
            .drain_into(&mut self.gpu_shader_presentation_uniform_updates);
        &self.gpu_shader_presentation_uniform_updates
    }

    /// Stage the currently admitted volatile GPU-shader updates without
    /// removing them from the mailbox. The native frame must call
    /// `commit_gpu_shader_presentation_updates` only after successful present,
    /// or `abort_gpu_shader_presentation_updates` on every veto/failure path.
    pub(crate) fn snapshot_gpu_shader_presentation_updates(
        &mut self,
    ) -> &[GpuShaderPresentationUniformUpdate] {
        self.gpu_shader_presentation_uniform_mailbox
            .snapshot_into(&mut self.gpu_shader_presentation_uniform_updates);
        &self.gpu_shader_presentation_uniform_updates
    }

    /// Borrow the staged volatile updates without changing their ownership.
    /// The slice is valid until the current native frame stops using it and
    /// commits or aborts the snapshot.
    pub(crate) fn staged_gpu_shader_presentation_updates(
        &self,
    ) -> &[GpuShaderPresentationUniformUpdate] {
        &self.gpu_shader_presentation_uniform_updates
    }

    /// Commit the exact volatile update snapshot staged for the current frame.
    pub(crate) fn commit_gpu_shader_presentation_updates(&mut self) {
        self.gpu_shader_presentation_uniform_mailbox
            .commit_snapshot();
    }

    /// Retain every volatile update selected by the current frame snapshot
    /// after an acquisition, lifecycle, or renderer veto.
    pub(crate) fn abort_gpu_shader_presentation_updates(&mut self) {
        self.gpu_shader_presentation_uniform_mailbox
            .abort_snapshot();
    }

    /// Return and clear the current runtime-exit request flag.
    pub fn take_exit_requested(&mut self) -> bool {
        let exit_requested = self.exit_requested;
        self.exit_requested = false;
        exit_requested
    }

    /// Return an immutable reference to the owned bridge.
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Return a mutable reference to the owned bridge.
    pub fn bridge_mut(&mut self) -> &mut Bridge {
        &mut self.bridge
    }

    /// Consume the runtime controller and return the owned bridge.
    pub fn into_bridge(self) -> Bridge {
        self.bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::ContainerPolicy,
        runtime::{RuntimeBridge, SurfaceNode, WindowColorScheme},
        theme::DpiScale,
        widgets::{
            Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetPaintContext, WidgetSizing,
        },
    };
    use std::sync::{Arc, Mutex};

    struct EnvironmentBridge {
        changes: Arc<Mutex<Vec<WindowEnvironment>>>,
    }

    impl RuntimeBridge<()> for EnvironmentBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn set_window_environment(&mut self, environment: WindowEnvironment) {
            self.changes.lock().unwrap().push(environment);
        }
    }

    #[test]
    fn environment_snapshot_updates_context_and_notifies_only_on_delta() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            EnvironmentBridge {
                changes: Arc::clone(&changes),
            },
            Vector2::new(100.0, 60.0),
        );
        let environment = WindowEnvironment::new(
            DpiScale::new(2.0),
            Some(WindowColorScheme::Light),
            true,
            false,
        );

        assert!(runtime.set_window_environment(environment));
        assert!(!runtime.set_window_environment(environment));
        assert_eq!(runtime.window_environment(), environment);
        assert_eq!(runtime.context().window_environment(), environment);
        assert_eq!(
            runtime.context().resolved_environment(),
            environment.resolved()
        );
        assert_eq!(
            *changes.lock().unwrap(),
            vec![WindowEnvironment::default(), environment]
        );
    }

    #[test]
    fn environment_snapshot_survives_refresh_and_stays_independent_per_runtime() {
        let mut first = SurfaceRuntime::new(
            EnvironmentBridge {
                changes: Arc::new(Mutex::new(Vec::new())),
            },
            Vector2::new(100.0, 60.0),
        );
        let mut second = SurfaceRuntime::new(
            EnvironmentBridge {
                changes: Arc::new(Mutex::new(Vec::new())),
            },
            Vector2::new(100.0, 60.0),
        );
        let first_environment = WindowEnvironment::new(
            DpiScale::new(1.25),
            Some(WindowColorScheme::Dark),
            false,
            true,
        );
        let second_environment = WindowEnvironment::new(
            DpiScale::new(2.0),
            Some(WindowColorScheme::Light),
            true,
            false,
        );

        assert!(first.set_window_environment(first_environment));
        assert!(second.set_window_environment(second_environment));
        first.refresh();

        assert_eq!(
            first.context().resolved_environment(),
            first_environment.resolved()
        );
        assert_eq!(
            second.context().resolved_environment(),
            second_environment.resolved()
        );
        assert_ne!(
            first.context().resolved_environment(),
            second.context().resolved_environment()
        );
    }

    #[derive(Clone)]
    struct OverlayProbe {
        common: WidgetCommon,
        base: Arc<Mutex<Option<ResolvedEnvironment>>>,
        overlay: Arc<Mutex<Option<ResolvedEnvironment>>>,
    }

    impl OverlayProbe {
        fn new(
            base: Arc<Mutex<Option<ResolvedEnvironment>>>,
            overlay: Arc<Mutex<Option<ResolvedEnvironment>>>,
        ) -> Self {
            Self {
                common: WidgetCommon::new(9, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
                base,
                overlay,
            }
        }
    }

    impl Widget for OverlayProbe {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }

        fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
            *self.base.lock().unwrap() = Some(context.environment());
        }

        fn append_runtime_overlay_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
            *self.overlay.lock().unwrap() = Some(context.environment());
        }
    }

    struct OverlayBridge {
        probe: OverlayProbe,
    }

    impl RuntimeBridge<()> for OverlayBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::custom_widget(
                self.probe.clone(),
                crate::runtime::WidgetMessageMapper::none(),
            )))
        }
    }

    #[test]
    fn base_and_runtime_overlay_paint_receive_the_same_environment() {
        let base = Arc::new(Mutex::new(None));
        let overlay = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            OverlayBridge {
                probe: OverlayProbe::new(Arc::clone(&base), Arc::clone(&overlay)),
            },
            Vector2::new(100.0, 60.0),
        );
        let environment = WindowEnvironment::new(
            DpiScale::new(1.5),
            Some(WindowColorScheme::Dark),
            true,
            true,
        );
        assert!(runtime.set_window_environment(environment));
        let mut base_plan =
            crate::runtime::SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default());
        runtime.base_paint_plan_into(&crate::theme::ThemeTokens::default(), &mut base_plan);
        runtime.interaction.hover.widget = Some(9);
        let mut overlay_primitives = Vec::new();
        runtime.runtime_overlay_paint_into(
            &crate::theme::ThemeTokens::default(),
            &mut overlay_primitives,
        );

        assert_eq!(*base.lock().unwrap(), Some(environment.resolved()));
        assert_eq!(*overlay.lock().unwrap(), Some(environment.resolved()));
    }
}
