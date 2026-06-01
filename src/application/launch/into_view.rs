use crate::{
    gui::types::Rect,
    layout::{LayoutOutput, Vector2},
    runtime::{SurfaceFrame, SurfaceNode, UiSurface},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput, WidgetOutput},
};

/// Converts application view values into the existing runtime surface.
pub trait IntoView<Message> {
    /// Lower this value into a runtime surface node.
    fn into_node(self) -> SurfaceNode<Message>;

    /// Lower this value into a top-level runtime surface.
    fn into_surface(self) -> UiSurface<Message>
    where
        Self: Sized,
    {
        UiSurface::new(self.into_node())
    }

    /// Resolve this view into layout rectangles for a host-controlled viewport.
    fn view_layout(self, viewport: Rect) -> LayoutOutput
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::layout(&surface, viewport)
    }

    /// Resolve this view into layout rectangles for an origin-based viewport.
    fn view_layout_at_size(self, size: Vector2) -> LayoutOutput
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::layout_at_size(&surface, size)
    }

    /// Prepare one layout plus paint-plan frame for a host-controlled viewport.
    fn view_frame(self, viewport: Rect, theme: &ThemeTokens) -> SurfaceFrame
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::frame(&surface, viewport, theme)
    }

    /// Prepare one frame for an origin-based viewport.
    fn view_frame_at_size(self, size: Vector2, theme: &ThemeTokens) -> SurfaceFrame
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::frame_at_size(&surface, size, theme)
    }

    /// Prepare one frame with Radiant's default theme.
    fn view_frame_with_default_theme(self, viewport: Rect) -> SurfaceFrame
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::frame_with_default_theme(&surface, viewport)
    }

    /// Prepare one origin-based frame with Radiant's default theme.
    fn view_frame_at_size_with_default_theme(self, size: Vector2) -> SurfaceFrame
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::frame_at_size_with_default_theme(&surface, size)
    }

    /// Map one synthetic widget output from this view back into a host message.
    fn view_dispatch_widget_output(
        self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message>
    where
        Self: Sized,
    {
        let surface = self.into_surface();
        UiSurface::dispatch_widget_output(&surface, widget_id, output)
    }

    /// Route one synthetic widget input into this view.
    fn view_dispatch_widget_input(
        self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput>
    where
        Self: Sized,
    {
        let mut surface = self.into_surface();
        UiSurface::dispatch_widget_input(&mut surface, widget_id, bounds, input)
    }
}

impl<Message> IntoView<Message> for SurfaceNode<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self
    }
}

impl<Message> IntoView<Message> for UiSurface<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self.into_root()
    }

    fn into_surface(self) -> UiSurface<Message> {
        self
    }
}
