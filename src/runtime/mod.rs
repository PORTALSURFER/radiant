//! Generic declarative runtime surfaces for new Radiant host applications.
//!
//! This module exposes a message-driven top-level UI tree built from public
//! layout containers and widget primitives. Hosts project immutable
//! [`UiSurface`] snapshots and reduce host-defined messages while the current
//! Sempal-shaped [`crate::compat::sempal_shell`] surface remains available as
//! compatibility.
//!
//! [`SurfaceRuntime`] closes the generic declarative loop by running public
//! layout, routing backend-neutral widget input, mapping widget outputs into
//! host-defined messages, reducing those messages, and reprojecting the next
//! immutable surface snapshot.
//!
//! The current native window runtime still consumes the compatibility bridge.
//! Those shell-specific entry points live under
//! [`crate::compat::sempal_shell`], while new host applications can already
//! compose against this generic runtime controller without depending on
//! Sempal-specific top-level contracts.

mod bridge;
mod controller;
mod surface;

pub use crate::gui_runtime::{NativeRunOptions, WindowIconRgba};
pub use bridge::{DeclarativeRuntimeBridge, RuntimeBridge, declarative_runtime_bridge};
pub use controller::SurfaceRuntime;
pub use surface::{
    MessageMapper, SurfaceChild, SurfaceContainer, SurfaceNode, SurfaceWidget, UiSurface,
    WidgetMessageMapper,
};
