//! Generic shortcut resolution primitives for host-owned command catalogs.

mod catalog;
mod gesture;
mod layer;
mod resolution;
mod stack;

#[cfg(test)]
#[path = "shortcuts/tests.rs"]
mod tests;

pub use catalog::ShortcutCatalog;
pub use gesture::{ShortcutGesture, ShortcutModifier};
pub use layer::{ShortcutBinding, ShortcutLayer};
pub use resolution::ShortcutResolution;
pub use stack::ShortcutStack;
