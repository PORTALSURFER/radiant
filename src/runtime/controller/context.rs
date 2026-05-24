use super::*;

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

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.interaction.focus.focused_widget
    }

    /// Return the widget that currently owns pointer capture.
    pub fn pointer_capture(&self) -> Option<WidgetId> {
        self.interaction.pointer.capture
    }

    pub(crate) fn interactive_pointer_route_active(&self) -> bool {
        self.interaction.pointer.capture.is_some() || self.interaction.drag.session.is_some()
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
