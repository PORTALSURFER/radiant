//! Generic declarative runtime surfaces for new Radiant host applications.
//!
//! This module exposes a message-driven top-level UI tree built from public
//! layout containers and widget primitives. Hosts project immutable
//! [`UiSurface`] snapshots and reduce host-defined messages while compatibility
//! adapters continue to live outside this generic surface.
//!
//! [`SurfaceRuntime`] closes the generic declarative flow by running public
//! layout, routing backend-neutral widget input, mapping widget outputs into
//! host-defined messages, reducing those messages, reprojecting the next
//! immutable surface snapshot, and exposing deterministic backend-neutral paint
//! plans for generic renderers.
//!
//! Native window adapters can compose against this controller without coupling
//! the public runtime API to any host application's top-level contracts.

mod bridge;
mod controller;
mod paint;
mod surface;

pub use crate::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, NativeGenericRunReport, NativeGenericRuntimeArtifacts,
    NativeRunOptions, NativeStartupTimingArtifact, WindowIconRgba, run_native_vello_runtime,
    run_native_vello_runtime_with_artifacts,
};
pub use bridge::{DeclarativeRuntimeBridge, RuntimeBridge, declarative_runtime_bridge};
pub use controller::SurfaceRuntime;
pub use paint::{
    PaintCustomSurface, PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign,
    PaintTextRun, SurfacePaintPlan,
};
pub use surface::{
    MessageMapper, SurfaceChild, SurfaceContainer, SurfaceNode, SurfaceWidget, UiSurface,
    WidgetMessageMapper,
};
