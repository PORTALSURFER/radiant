use crate::{
    gui::{
        paint::PaintFrame,
        types::{Rect, Vector2},
    },
    runtime::{PaintPrimitive, TransientOverlayContext},
    widgets::RetainedSurfaceDescriptor,
};

/// Optional host capability for retained custom-surface rendering.
pub trait RuntimeRetainedSurfaceHost {
    /// Render a host-retained custom surface into backend-neutral paint data.
    fn render_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: Rect,
        viewport: Vector2,
    ) -> Option<PaintFrame>;
}

/// Optional host capability for transient overlay painting.
pub trait RuntimeTransientOverlayHost {
    /// Paint lightweight transient primitives over the cached scene.
    fn paint_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    );
}

pub(crate) struct RuntimeRetainedSurfaceCapability<Bridge> {
    render_retained_surface:
        fn(&mut Bridge, RetainedSurfaceDescriptor, Rect, Vector2) -> Option<PaintFrame>,
}

impl<Bridge> RuntimeRetainedSurfaceCapability<Bridge>
where
    Bridge: RuntimeRetainedSurfaceHost,
{
    pub const fn new() -> Self {
        Self {
            render_retained_surface: Bridge::render_retained_surface,
        }
    }
}

impl<Bridge> RuntimeRetainedSurfaceCapability<Bridge> {
    pub fn render(
        self,
        bridge: &mut Bridge,
        descriptor: RetainedSurfaceDescriptor,
        rect: Rect,
        viewport: Vector2,
    ) -> Option<PaintFrame> {
        (self.render_retained_surface)(bridge, descriptor, rect, viewport)
    }
}

impl<Bridge> Clone for RuntimeRetainedSurfaceCapability<Bridge> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Bridge> Copy for RuntimeRetainedSurfaceCapability<Bridge> {}

pub(crate) struct RuntimeTransientOverlayCapability<Bridge> {
    pub paint_transient_overlay:
        for<'a> fn(&mut Bridge, TransientOverlayContext<'a>, &mut Vec<PaintPrimitive>),
}

impl<Bridge> RuntimeTransientOverlayCapability<Bridge>
where
    Bridge: RuntimeTransientOverlayHost,
{
    pub const fn new() -> Self {
        Self {
            paint_transient_overlay: Bridge::paint_transient_overlay,
        }
    }
}
