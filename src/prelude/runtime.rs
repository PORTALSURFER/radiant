//! Runtime, command, paint, resource, and native host prelude exports.
//!
//! Keep this module as a subsystem facade. Add new exports to the smallest
//! owning sibling module instead of growing a broad `crate::runtime` list.

mod commands;
mod gpu;
mod native;
mod paint;
mod platform;
mod resources;
mod surface;
mod windowing;

pub use commands::*;
pub use gpu::*;
pub use native::*;
pub use paint::*;
pub use platform::*;
pub use resources::*;
pub use surface::*;
pub use windowing::*;
