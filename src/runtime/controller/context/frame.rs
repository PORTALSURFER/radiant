use super::super::*;

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
    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = empty_paint_plan_for_layout(&self.layout, theme);
        self.paint_plan_into(theme, &mut plan);
        plan
    }

    /// Project the current runtime paint data into an existing plan buffer.
    ///
    /// This avoids reallocating primitive storage for renderers that rebuild a
    /// paint plan every frame.
    pub fn paint_plan_into(&self, theme: &ThemeTokens, plan: &mut SurfacePaintPlan) {
        self.base_paint_plan_into(theme, plan);
        self.runtime_overlay_paint_into(theme, &mut plan.primitives);
    }

    /// Project the current declarative surface into an existing plan buffer.
    ///
    /// Native retained renderers use this for the cached base scene, then paint
    /// runtime-owned overlays separately so pointer-local affordances can move
    /// without leaving stale copies in the base frame.
    pub fn base_paint_plan_into(&self, theme: &ThemeTokens, plan: &mut SurfacePaintPlan) {
        self.surface.paint_plan_with_hover_into(
            &self.layout,
            theme,
            self.interaction.hover.container,
            self.interaction.hover.scroll_affordance,
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
        self.append_widget_runtime_overlay(self.interaction.hover.widget, theme, primitives);
        if self.interaction.pointer.capture != self.interaction.hover.widget {
            self.append_widget_runtime_overlay(self.interaction.pointer.capture, theme, primitives);
        }
        self.append_drag_preview_overlay(theme, primitives);
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

    fn append_drag_preview_overlay(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(session) = self
            .interaction
            .drag
            .session
            .as_ref()
            .filter(|session| session.visible)
        else {
            return;
        };
        let rect = Rect::from_min_size(
            Point::new(
                session.pointer.x + crate::runtime::drag::DRAG_PREVIEW_OFFSET.x,
                session.pointer.y + crate::runtime::drag::DRAG_PREVIEW_OFFSET.y,
            ),
            session.preview.size,
        );
        crate::runtime::paint::push_overlay_panel(
            primitives,
            u64::MAX - 1024,
            rect,
            Some(session.preview.label.clone()),
            theme,
            crate::widgets::WidgetStyle {
                tone: crate::widgets::WidgetTone::Accent,
                prominence: crate::widgets::WidgetProminence::Strong,
            },
        );
    }
}
