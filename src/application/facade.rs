//! Focused public export facades for the application subsystem.
//!
//! These modules mirror the application prelude's API roles while keeping the
//! full `crate::application::*` surface available for compatibility.

mod controls;
mod details;
mod layout;
mod menus;
mod overlays;
mod panels;
mod runtime;
mod surfaces;
mod view;

pub use controls::*;
pub use details::*;
pub use layout::*;
pub use menus::*;
pub use overlays::*;
pub use panels::*;
pub use runtime::*;
pub use surfaces::*;
pub use view::*;
