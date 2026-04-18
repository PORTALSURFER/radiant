//! Generic declarative runtime surfaces for new Radiant host applications.
//!
//! This module exposes a message-driven top-level UI tree built from public
//! layout containers and widget primitives. Hosts project immutable
//! [`UiSurface`] snapshots and reduce host-defined messages while the current
//! Sempal-shaped [`crate::app`] surface remains available as compatibility.
//!
//! [`SurfaceRuntime`] closes the generic declarative loop by running public
//! layout, routing backend-neutral widget input, mapping widget outputs into
//! host-defined messages, reducing those messages, and reprojecting the next
//! immutable surface snapshot.
//!
//! The current native window runtime still consumes the compatibility bridge,
//! but new host applications can already compose against this generic runtime
//! controller without depending on Sempal-specific top-level contracts.

mod bridge;
mod controller;
mod surface;

pub use crate::gui_runtime::{
    NativeRunOptions, WindowIconRgba, capture_gui_automation_snapshot, run_native_vello_app,
    run_native_vello_app_declarative, run_native_vello_preview,
};
pub use bridge::{DeclarativeRuntimeBridge, RuntimeBridge, declarative_runtime_bridge};
pub use controller::SurfaceRuntime;
pub use surface::{
    MessageMapper, SurfaceChild, SurfaceContainer, SurfaceNode, SurfaceWidget, UiSurface,
    WidgetMessageMapper,
};
