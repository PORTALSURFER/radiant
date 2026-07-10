//! Generic message-menu composition and context-menu compatibility surface.

mod actions;
mod model;
mod overlays;
mod projection;

pub use actions::{menu_height, message_menu, message_menu_from_parts, message_menu_height};
pub use model::{
    DismissibleContextMenuParts, MenuCommand, MenuCommandParts, MessageContextMenuOverlayParts,
    MessageMenuParts, MessageMenuWidthPolicy,
};
pub use overlays::{
    anchored_message_menu_overlay, anchored_message_menu_overlay_auto_width,
    anchored_message_menu_overlay_from_parts, anchored_message_menu_overlay_with_width_policy,
    dismissible_context_menu, dismissible_context_menu_auto_width,
    dismissible_context_menu_from_parts, dismissible_context_menu_with_width,
    dismissible_context_menu_with_width_policy, message_context_menu_overlay,
    message_context_menu_overlay_auto_width, message_context_menu_overlay_from_parts,
    message_context_menu_overlay_with_width, message_context_menu_overlay_with_width_policy,
};

#[cfg(test)]
mod tests;
