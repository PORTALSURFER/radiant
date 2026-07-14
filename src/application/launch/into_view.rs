use crate::{
    application::{AppBridgeLifecycle, Presentation},
    gui::types::Rect,
    gui::{input::KeyPress, shortcuts::ShortcutResolution},
    layout::{LayoutOutput, Vector2},
    runtime::{SurfaceFrame, SurfaceNode, UiSurface},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput, WidgetOutput},
};
use std::any::Any;

/// One lowered application view plus its Scene lifecycle bindings.
///
/// `ViewProjection` is the lossless application projection artifact. Custom
/// [`IntoView`] wrappers should delegate [`IntoView::into_projection`] to the
/// wrapped value so Scene frame clocks, transient overlays, and shortcuts stay
/// attached. A projection built with [`Self::from_surface`] is explicitly
/// metadata-free.
pub struct ViewProjection<Message> {
    surface: UiSurface<Message>,
    scene: SceneProjection<Message>,
}

impl<Message> ViewProjection<Message> {
    /// Build an explicitly metadata-free projection from a lowered surface.
    pub fn from_surface(surface: UiSurface<Message>) -> Self {
        Self {
            surface,
            scene: SceneProjection::default(),
        }
    }

    /// Borrow the lowered runtime surface.
    pub fn surface(&self) -> &UiSurface<Message> {
        &self.surface
    }

    /// Consume the projection and return its lowered runtime surface.
    ///
    /// This intentionally discards application-only Scene lifecycle bindings.
    /// Return the `ViewProjection` itself from a stateful app projection when
    /// those bindings must remain active.
    pub fn into_surface(self) -> UiSurface<Message> {
        self.surface
    }

    pub(in crate::application) fn with_scene(
        surface: UiSurface<Message>,
        scene: SceneProjection<Message>,
    ) -> Self {
        Self { surface, scene }
    }

    pub(in crate::application) fn into_parts(
        self,
    ) -> (UiSurface<Message>, SceneProjection<Message>) {
        (self.surface, self.scene)
    }
}

pub(in crate::application) struct SceneProjection<Message> {
    presentations: Vec<Box<dyn Any>>,
    shortcuts: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
}

impl<Message> Default for SceneProjection<Message> {
    fn default() -> Self {
        Self {
            presentations: Vec::new(),
            shortcuts: None,
        }
    }
}

impl<Message: 'static> SceneProjection<Message> {
    pub(in crate::application) fn capture(
        &mut self,
        presentation: Option<Box<dyn Any>>,
        shortcuts: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
    ) {
        if self.shortcuts.is_none() {
            self.shortcuts = shortcuts;
        }
        if let Some(presentation) = presentation {
            self.presentations.push(presentation);
        }
    }

    pub(in crate::application) fn apply<State: 'static>(
        self,
        lifecycle: &mut AppBridgeLifecycle<State, Message>,
    ) {
        lifecycle.clear_scene_presentation();
        lifecycle.scene_shortcuts = self.shortcuts;
        for presentation in self.presentations {
            if let Ok(presentation) = presentation.downcast::<Presentation<State, Message>>() {
                presentation.apply_to_scene_lifecycle(lifecycle);
            }
        }
    }
}

/// Converts application view values into a lossless application projection.
pub trait IntoView<Message> {
    /// Lower this value into one surface plus its Scene lifecycle bindings.
    ///
    /// This method is required so custom wrappers must explicitly preserve a
    /// wrapped projection or construct a metadata-free one.
    fn into_projection(self) -> ViewProjection<Message>;

    /// Lower this value into a runtime surface node.
    ///
    /// This explicitly discards application-only Scene lifecycle bindings.
    fn into_node(self) -> SurfaceNode<Message>
    where
        Self: Sized,
    {
        self.into_projection().into_surface().into_root()
    }

    /// Lower this value into a top-level runtime surface.
    ///
    /// This explicitly discards application-only Scene lifecycle bindings.
    fn into_surface(self) -> UiSurface<Message>
    where
        Self: Sized,
    {
        self.into_projection().into_surface()
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

impl<Message> IntoView<Message> for ViewProjection<Message> {
    fn into_projection(self) -> ViewProjection<Message> {
        self
    }
}
