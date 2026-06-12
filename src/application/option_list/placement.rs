use super::model::CompactOptionListParts;
use super::view::{
    compact_option_list_from_parts, compact_option_list_from_parts_with_interaction_impl,
};
use crate::application::{
    LayerHorizontalAnchor, LayerVerticalAnchor, ViewNode, anchored_layer, empty,
    floating_layer_above,
};
use crate::layout::Vector2;

/// Named construction fields for a compact option list floating above a trigger.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListFloatingAboveParts {
    /// Option-list content and row metrics.
    pub list: CompactOptionListParts,
    /// Layer x offset inside the parent stack.
    pub x: f32,
    /// Trigger top y offset inside the parent stack.
    pub trigger_y: f32,
    /// Gap between the trigger and floating option list.
    pub gap: f32,
    /// Floating option-list width.
    pub width: f32,
}

/// Named construction fields for a compact option list in an anchored layer.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListAnchoredParts {
    /// Option-list content and row metrics.
    pub list: CompactOptionListParts,
    /// Floating option-list width.
    pub width: f32,
    /// Horizontal anchor inside the parent layer.
    pub horizontal_anchor: LayerHorizontalAnchor,
    /// Vertical anchor inside the parent layer.
    pub vertical_anchor: LayerVerticalAnchor,
    /// Horizontal inset from the selected horizontal anchor.
    pub inset_x: f32,
    /// Vertical inset from the selected vertical anchor.
    pub inset_y: f32,
}

impl CompactOptionListFloatingAboveParts {
    /// Build named parts for a compact option list floating above a trigger.
    pub const fn new(
        list: CompactOptionListParts,
        x: f32,
        trigger_y: f32,
        gap: f32,
        width: f32,
    ) -> Self {
        Self {
            list,
            x,
            trigger_y,
            gap,
            width,
        }
    }
}

impl CompactOptionListAnchoredParts {
    /// Build named parts for a compact option list in an anchored layer.
    pub const fn new(
        list: CompactOptionListParts,
        width: f32,
        horizontal_anchor: LayerHorizontalAnchor,
        vertical_anchor: LayerVerticalAnchor,
        inset_x: f32,
        inset_y: f32,
    ) -> Self {
        Self {
            list,
            width,
            horizontal_anchor,
            vertical_anchor,
            inset_x,
            inset_y,
        }
    }
}

/// Build a compact option list in a floating layer above a trigger rectangle.
///
/// This is useful for autocomplete popups and compact editor pickers that should
/// stay in the same stack layer as their trigger while sharing Radiant's capped
/// option-list height and empty-list behavior.
pub fn compact_option_list_floating_above<Message: 'static>(
    parts: CompactOptionListFloatingAboveParts,
) -> ViewNode<Message> {
    let height = parts.list.height();
    if height <= 0.0 {
        return empty().fill_width();
    }
    let width = parts.width.max(1.0);
    let child = compact_option_list_from_parts(parts.list)
        .fill_width()
        .height(height);
    floating_layer_above(
        parts.x,
        parts.trigger_y,
        parts.gap,
        Vector2::new(width, height),
        child,
    )
}

/// Build a compact option list in a parent-anchored layer.
///
/// This is useful for autocomplete popups and compact editor pickers that are
/// projected in a full-surface overlay layer instead of beside their trigger in
/// the local stack.
pub fn compact_option_list_anchored<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_activation(parts, |_| None::<Message>)
}

/// Build a parent-anchored compact option list and map row activation to host messages.
pub fn compact_option_list_anchored_with_activation<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_interaction_impl(parts, activate, |_| None::<Message>, false)
}

/// Build a parent-anchored compact option list and map row hover/activation to host messages.
pub fn compact_option_list_anchored_with_interaction<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_interaction_impl(parts, activate, hover, true)
}

fn compact_option_list_anchored_with_interaction_impl<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    pointer_move: bool,
) -> ViewNode<Message> {
    let height = parts.list.height();
    if height <= 0.0 {
        return empty().fill_width();
    }
    let width = parts.width.max(1.0);
    let child = compact_option_list_from_parts_with_interaction_impl(
        parts.list,
        activate,
        hover,
        pointer_move,
    )
    .fill_width()
    .height(height);
    anchored_layer(
        child,
        Vector2::new(width, height),
        parts.horizontal_anchor,
        parts.vertical_anchor,
        parts.inset_x,
        parts.inset_y,
    )
}
