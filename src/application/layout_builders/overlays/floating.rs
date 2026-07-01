use super::parts::{FloatingLayerAnchorParts, FloatingLayerPlacement};
use crate::application::{ViewNode, ViewNodeKind};
use crate::gui::types::Point;
use crate::layout::{FloatingLayerVerticalOverflow, Vector2};

/// Build a non-interactive floating child tree positioned relative to its parent.
///
/// The layer paints regular view content without contributing intrinsic size and
/// does not register its child widgets for pointer or wheel input.
pub fn floating_layer<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_with_input(offset, size, child, false)
}

/// Build a floating child tree positioned relative to its parent.
///
/// Set `interactive` when the floating content should receive pointer, wheel,
/// focus, and state synchronization traversal like normal content.
pub fn floating_layer_with_input<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
    interactive: bool,
) -> ViewNode<Message> {
    floating_layer_with_input_and_vertical_overflow(
        offset,
        size,
        child,
        interactive,
        FloatingLayerVerticalOverflow::Fixed,
    )
}

/// Build a floating child tree with explicit vertical overflow behavior.
pub fn floating_layer_with_input_and_vertical_overflow<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
    interactive: bool,
    vertical_overflow: FloatingLayerVerticalOverflow,
) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::FloatingLayer {
        offset,
        size,
        child: Box::new(child),
        interactive,
        vertical_overflow,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a floating child tree above a trigger rectangle.
///
/// This is useful for autocompletion, tooltips, and compact editor popups that
/// should stay in the same stack layer as their trigger without app-local
/// offset arithmetic.
pub fn floating_layer_above<Message>(
    x: f32,
    trigger_y: f32,
    gap: f32,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_around_from_parts(FloatingLayerAnchorParts::new(
        child,
        size,
        x,
        trigger_y,
        0.0,
        gap,
        FloatingLayerPlacement::Above,
    ))
}

/// Build a floating child tree below a trigger rectangle.
pub fn floating_layer_below<Message>(
    x: f32,
    trigger_y: f32,
    trigger_height: f32,
    gap: f32,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_around_from_parts(FloatingLayerAnchorParts::new(
        child,
        size,
        x,
        trigger_y,
        trigger_height,
        gap,
        FloatingLayerPlacement::Below,
    ))
}

/// Build a floating child tree around a trigger from named parts.
pub fn floating_layer_around_from_parts<Message>(
    parts: FloatingLayerAnchorParts<Message>,
) -> ViewNode<Message> {
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
    let offset = floating_layer_anchor_offset(
        parts.x,
        parts.trigger_y,
        parts.trigger_height,
        parts.gap,
        size,
        parts.placement,
    );
    floating_layer_with_input(offset, size, parts.child, parts.interactive)
}

fn floating_layer_anchor_offset(
    x: f32,
    trigger_y: f32,
    trigger_height: f32,
    gap: f32,
    size: Vector2,
    placement: FloatingLayerPlacement,
) -> Point {
    let x = x.max(0.0);
    let trigger_y = trigger_y.max(0.0);
    let gap = gap.max(0.0);
    let y = match placement {
        FloatingLayerPlacement::Above => (trigger_y - gap - size.y).max(0.0),
        FloatingLayerPlacement::Below => trigger_y + trigger_height.max(0.0) + gap,
    };
    Point::new(x, y)
}
