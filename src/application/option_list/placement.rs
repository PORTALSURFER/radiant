use crate::application::{
    FloatingLayerAnchorParts, FloatingLayerPlacement, LayerHorizontalAnchor, LayerVerticalAnchor,
    ViewNode, anchored_layer, empty, floating_layer_around_from_parts,
};
use crate::layout::Vector2;

/// Placement for a compact option list anchored to its parent layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompactOptionListAnchor {
    width: f32,
    horizontal: LayerHorizontalAnchor,
    vertical: LayerVerticalAnchor,
    inset_x: f32,
    inset_y: f32,
}

/// Placement for a compact option list floating above a local trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompactOptionListFloatingAbove {
    x: f32,
    trigger_y: f32,
    gap: f32,
    width: f32,
}

pub(super) enum CompactOptionListPlacement {
    Inline,
    Anchored(CompactOptionListAnchor),
    FloatingAbove(CompactOptionListFloatingAbove),
}

impl CompactOptionListAnchor {
    /// Describe a fixed-width option list anchored inside its parent layer.
    pub const fn new(
        width: f32,
        horizontal: LayerHorizontalAnchor,
        vertical: LayerVerticalAnchor,
    ) -> Self {
        Self {
            width,
            horizontal,
            vertical,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Offset the option list from the selected parent edges.
    pub const fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x;
        self.inset_y = y;
        self
    }
}

impl CompactOptionListFloatingAbove {
    /// Describe an option list floating above a local trigger rectangle.
    pub const fn new(x: f32, trigger_y: f32, gap: f32, width: f32) -> Self {
        Self {
            x,
            trigger_y,
            gap,
            width,
        }
    }
}

pub(super) fn place_compact_option_list<Message: 'static>(
    placement: CompactOptionListPlacement,
    child: ViewNode<Message>,
    height: f32,
    interactive: bool,
) -> ViewNode<Message> {
    match placement {
        CompactOptionListPlacement::Inline => child,
        CompactOptionListPlacement::Anchored(anchor) => {
            if height <= 0.0 {
                return empty().fill_width();
            }
            let width = anchor.width.max(1.0);
            anchored_layer(
                child.fill_width().height(height),
                Vector2::new(width, height),
                anchor.horizontal,
                anchor.vertical,
                anchor.inset_x,
                anchor.inset_y,
            )
        }
        CompactOptionListPlacement::FloatingAbove(floating) => {
            if height <= 0.0 {
                return empty().fill_width();
            }
            let width = floating.width.max(1.0);
            floating_layer_around_from_parts(
                FloatingLayerAnchorParts::new(
                    child.fill_width().height(height),
                    Vector2::new(width, height),
                    floating.x,
                    floating.trigger_y,
                    0.0,
                    floating.gap,
                    FloatingLayerPlacement::Above,
                )
                .interactive(interactive),
            )
        }
    }
}
