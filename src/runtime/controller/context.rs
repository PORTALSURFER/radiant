use super::*;

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

/// Borrowed runtime frame for host renderers that do not need owned layout data.
///
/// Unlike [`SurfaceFrame`], this frame borrows the runtime's current layout
/// output while owning the freshly generated paint plan. It is useful for
/// embedded hosts and custom renderers that render immediately and want to
/// avoid cloning potentially large layout maps on every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceFrame<'a> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Borrowed resolved layout for the runtime's current surface.
    pub layout: &'a LayoutOutput,
    /// Backend-neutral paint plan for the borrowed layout.
    pub paint_plan: SurfacePaintPlan,
}

/// Borrowed runtime frame that reuses host-owned paint-plan storage.
///
/// This is the lowest-allocation runtime frame view for synchronous custom
/// hosts: both the resolved layout and backend-neutral paint plan are borrowed,
/// while the runtime fills the caller-provided paint plan before returning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeSurfaceFrameRef<'layout, 'paint> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Borrowed resolved layout for the runtime's current surface.
    pub layout: &'layout LayoutOutput,
    /// Borrowed backend-neutral paint plan filled for the current layout.
    pub paint_plan: &'paint SurfacePaintPlan,
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

    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = SurfacePaintPlan::empty_with_capacity(
            theme,
            estimated_paint_primitive_capacity(&self.layout),
        );
        self.paint_plan_into(theme, &mut plan);
        plan
    }

    /// Project the current runtime paint data into an existing plan buffer.
    ///
    /// This avoids reallocating primitive storage for renderers that rebuild a
    /// paint plan every frame.
    pub fn paint_plan_into(&self, theme: &ThemeTokens, plan: &mut SurfacePaintPlan) {
        self.surface.paint_plan_with_hover_into(
            &self.layout,
            theme,
            self.hovered_container,
            self.hovered_scroll_affordance,
            plan,
        );
    }

    /// Append runtime-local overlay primitives for active pointer widgets.
    ///
    /// These primitives are painted over the cached scene by native backends
    /// during paint-only pointer motion, so editor-style cursor and handle
    /// affordances can move without refreshing the declarative surface.
    pub fn runtime_overlay_paint_into(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.append_widget_runtime_overlay(self.hovered_widget, theme, primitives);
        if self.pointer_capture != self.hovered_widget {
            self.append_widget_runtime_overlay(self.pointer_capture, theme, primitives);
        }
    }

    /// Package the current runtime viewport, layout, and paint plan for a host renderer.
    ///
    /// Unlike [`UiSurface::frame`](crate::runtime::UiSurface::frame), this uses
    /// the runtime's current event-driven state, including hover-aware container
    /// paint and any layout refreshed by dispatched messages or resize events.
    pub fn frame(&self, theme: &ThemeTokens) -> SurfaceFrame {
        SurfaceFrame {
            viewport: self.viewport,
            layout: self.layout.clone(),
            paint_plan: self.paint_plan(theme),
        }
    }

    /// Package the current runtime viewport, borrowed layout, and paint plan.
    ///
    /// This is the lower-allocation counterpart to [`Self::frame`] for hosts
    /// that render synchronously and do not need to retain owned layout output
    /// after borrowing the runtime.
    pub fn borrowed_frame(&self, theme: &ThemeTokens) -> RuntimeSurfaceFrame<'_> {
        RuntimeSurfaceFrame {
            viewport: self.viewport,
            layout: &self.layout,
            paint_plan: self.paint_plan(theme),
        }
    }

    /// Fill a reusable paint plan and package borrowed frame references.
    ///
    /// This is the lower-allocation counterpart to [`Self::borrowed_frame`].
    /// Use it when a host render loop can keep a `SurfacePaintPlan` scratch
    /// buffer and render before mutating the runtime again.
    pub fn borrowed_frame_into<'layout, 'paint>(
        &'layout self,
        theme: &ThemeTokens,
        paint_plan: &'paint mut SurfacePaintPlan,
    ) -> RuntimeSurfaceFrameRef<'layout, 'paint> {
        self.paint_plan_into(theme, paint_plan);
        RuntimeSurfaceFrameRef {
            viewport: self.viewport,
            layout: &self.layout,
            paint_plan,
        }
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Return the widget that currently owns pointer capture.
    pub fn pointer_capture(&self) -> Option<WidgetId> {
        self.pointer_capture
    }

    /// Return the widget currently receiving hover state.
    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }

    /// Return the styled container currently receiving hover chrome.
    pub fn hovered_container(&self) -> Option<NodeId> {
        self.hovered_container
    }

    /// Return the scroll affordance currently receiving hover or drag emphasis.
    pub fn hovered_scroll_affordance(&self) -> Option<NodeId> {
        self.hovered_scroll_affordance
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

    fn append_widget_runtime_overlay(
        &self,
        widget_id: Option<WidgetId>,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(widget_id) = widget_id else {
            return;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let Some(widget) = self.surface_widget(widget_id) else {
            return;
        };
        widget.widget_object().append_runtime_overlay_paint(
            primitives,
            bounds,
            &self.layout,
            theme,
        );
    }
}
