use crate::{gui::types::Rect, layout::LayoutOutput, runtime::SurfacePaintPlan};

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
