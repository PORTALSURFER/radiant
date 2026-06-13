#[path = "timing/animation.rs"]
mod animation;
#[path = "timing/fixtures.rs"]
mod fixtures;
#[path = "timing/overlay_diagnostics.rs"]
mod overlay_diagnostics;
#[path = "timing/resize.rs"]
mod resize;
#[path = "timing/route_frames.rs"]
mod route_frames;

mod shared {
    pub(super) use super::super::*;
    pub(super) use crate::runtime::PaintPrimitive;
    pub(super) use winit::dpi::PhysicalSize;
}
