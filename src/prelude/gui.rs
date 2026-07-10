//! Backend-neutral GUI model and helper prelude exports.
//!
//! Keep this module as a subsystem facade. Add new exports to the smallest
//! owning sibling module instead of growing a broad `crate::gui` list.

mod chrome;
mod feedback;
mod interaction;
mod list;
mod paint;
mod range;
mod text;
mod visualization;

pub use chrome::*;
pub use feedback::*;
pub use interaction::*;
pub use list::*;
pub use paint::*;
pub use range::*;
pub use text::*;
pub use visualization::*;
