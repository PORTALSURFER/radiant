//! Generic message-menu composition and context-menu compatibility surface.

mod actions;
mod model;
mod overlays;
mod projection;

pub use actions::{menu_height, message_menu, message_menu_height};
pub use model::{MenuCommand, MenuCommandParts, MessageMenuWidthPolicy};
pub use overlays::{AnchoredContextMenuBuilder, ContextMenuBuilder, context_menu};

#[cfg(test)]
mod tests;
