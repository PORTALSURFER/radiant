use super::super::{
    RuntimeInteractionState, RuntimeScratch, RuntimeTraversalState, RuntimeWorkQueues,
    SurfaceRuntime,
};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    runtime::{
        CommandOutcome, DeclarativeOwnedRuntimeBridge, DeclarativeRuntimeBridge, RuntimeBridge,
        SurfaceRuntimeProjection, UiSurface, surface::WidgetStateSyncPolicy,
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
        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
        } = surface.runtime_projection();
        let mut runtime = Self {
            bridge,
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
            repaint_requested: false,
            exit_requested: false,
            pending_input_command_outcome: CommandOutcome::default(),
            runtime_work: RuntimeWorkQueues::default(),
        };
        runtime.relayout_with_traversal(traversal);
        runtime
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

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        let mut next_surface = self.bridge.pull_surface();
        std::mem::swap(
            &mut self.traversal.widgets.paths.previous,
            &mut self.traversal.widgets.paths.current,
        );
        let mut traversal = self.take_reusable_traversal_index(true);
        let layout_root = next_surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        let sync_policy = self.widget_state_sync_policy();
        next_surface.synchronize_widget_state_from_paths(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            &self.traversal.widgets.paths.previous,
            sync_policy,
        );
        self.surface = next_surface;
        self.layout_root = layout_root;
        self.restore_pointer_capture_state();
        self.relayout_with_traversal(traversal);
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.interaction.focus.focused_widget {
            self.restore_focused_widget_state(widget_id);
        }
    }

    fn widget_state_sync_policy(&self) -> WidgetStateSyncPolicy {
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

    fn clear_stale_interaction_state(&mut self) {
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
