//! Floating overlay layout builders.

mod drag_preview;
mod floating;
mod input;
mod layers;
mod markers;
mod panel;
mod parts;
mod popover;

#[cfg(test)]
mod tests;

pub use drag_preview::{drag_preview, drag_preview_sized};
pub use floating::{
    floating_layer, floating_layer_above, floating_layer_around_from_parts, floating_layer_below,
    floating_layer_with_input, floating_layer_with_input_and_vertical_overflow,
};
pub use input::{
    dismiss_layer, dismissible_overlay, dismissible_overlay_with_interactive_base, input_overlay,
    input_underlay,
};
pub use layers::{
    anchored_layer, anchored_layer_from_parts, centered_layer, centered_layer_from_parts,
};
pub use markers::{drop_marker, local_drop_marker};
pub use panel::overlay_panel;
pub use parts::{
    AnchoredLayerParts, AnchoredPopoverAnchor, AnchoredPopoverParts, AnchoredPopoverPlacement,
    CenteredLayerParts, FloatingLayerAnchorParts, FloatingLayerPlacement, LayerHorizontalAnchor,
    LayerVerticalAnchor,
};
pub use popover::{anchored_popover_from_parts, dismissible_anchored_popover_from_parts};
