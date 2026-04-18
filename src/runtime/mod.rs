//! Generic declarative runtime surfaces for new Radiant host applications.
//!
//! This module exposes a message-driven top-level UI tree built from public
//! layout containers and widget primitives. Hosts project immutable
//! [`UiSurface`] snapshots and reduce host-defined messages while the current
//! Sempal-shaped [`crate::app`] surface remains available as compatibility.
//!
//! The current native window runtime still consumes the compatibility bridge.
//! This generic surface provides the reusable host contract and deterministic
//! message routing that new applications can compose against today.

mod bridge;
mod surface;

pub use crate::gui_runtime::{
    NativeRunOptions, WindowIconRgba, capture_gui_automation_snapshot, run_native_vello_app,
    run_native_vello_app_declarative, run_native_vello_preview,
};
pub use bridge::{DeclarativeRuntimeBridge, RuntimeBridge, declarative_runtime_bridge};
pub use surface::{
    MessageMapper, SurfaceChild, SurfaceContainer, SurfaceNode, SurfaceWidget, UiSurface,
    WidgetMessageMapper,
};
