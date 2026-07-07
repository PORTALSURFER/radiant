use super::parts::{AnchoredPopoverParts, AnchoredPopoverPlacement};
use crate::application::{
    ViewNode, dismiss_layer, floating_layer_with_input_and_vertical_overflow, stack,
};
use crate::gui::types::Point;
use crate::layout::{FloatingLayerHorizontalOverflow, FloatingLayerVerticalOverflow, Vector2};

/// Build a generic anchored popover from named parts.
pub fn anchored_popover_from_parts<Message>(
    parts: AnchoredPopoverParts<Message>,
) -> ViewNode<Message> {
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
    let offset = anchored_popover_offset(
        parts.anchor.origin,
        parts.anchor.height(),
        parts.gap,
        size,
        parts.placement,
    );
    let horizontal_overflow = if parts.clamp_to_viewport {
        FloatingLayerHorizontalOverflow::ClampToViewport
    } else {
        FloatingLayerHorizontalOverflow::Fixed
    };
    let vertical_overflow = if parts.flip_when_clipped {
        FloatingLayerVerticalOverflow::FlipUpWhenClipped
    } else {
        FloatingLayerVerticalOverflow::Fixed
    };

    floating_layer_with_input_and_vertical_overflow(
        offset,
        size,
        parts.child.width(size.x).height(size.y),
        parts.interactive,
        horizontal_overflow,
        vertical_overflow,
    )
    .fill()
}

/// Build a dismissible anchored popover with an input-only outside-click backing.
pub fn dismissible_anchored_popover_from_parts<Message>(
    parts: AnchoredPopoverParts<Message>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    stack([
        dismiss_layer(dismiss_message).key("anchored-popover-dismiss"),
        anchored_popover_from_parts(parts).key("anchored-popover-content"),
    ])
    .fill()
}

fn anchored_popover_offset(
    origin: Point,
    anchor_height: f32,
    gap: f32,
    size: Vector2,
    placement: AnchoredPopoverPlacement,
) -> Point {
    let x = origin.x.max(0.0);
    let y = match placement {
        AnchoredPopoverPlacement::Below => origin.y.max(0.0) + anchor_height.max(0.0) + gap,
        AnchoredPopoverPlacement::Above => (origin.y.max(0.0) - gap - size.y.max(0.0)).max(0.0),
    };
    Point::new(x, y)
}
