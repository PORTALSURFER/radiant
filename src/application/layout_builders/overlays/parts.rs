use crate::application::ViewNode;
use crate::gui::types::Point;
use crate::layout::Vector2;

/// Named construction fields for a centered fixed-size child layer.
pub struct CenteredLayerParts<Message> {
    /// Child view to center inside the layer.
    pub child: ViewNode<Message>,
    /// Fixed child size.
    pub size: Vector2,
}

impl<Message> CenteredLayerParts<Message> {
    /// Build centered-layer parts.
    pub fn new(child: ViewNode<Message>, size: Vector2) -> Self {
        Self { child, size }
    }
}

/// Horizontal child placement inside a full-size anchored layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerHorizontalAnchor {
    /// Place the child at the left edge after the configured inset.
    Start,
    /// Place the child centered horizontally.
    Center,
    /// Place the child at the right edge before the configured inset.
    End,
}

/// Vertical child placement inside a full-size anchored layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerVerticalAnchor {
    /// Place the child at the top edge after the configured inset.
    Start,
    /// Place the child centered vertically.
    Center,
    /// Place the child at the bottom edge before the configured inset.
    End,
}

/// Placement policy for a floating layer anchored to a trigger rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingLayerPlacement {
    /// Place the floating layer above the trigger.
    Above,
    /// Place the floating layer below the trigger.
    Below,
}

/// Anchor geometry for a generic anchored popover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredPopoverAnchor {
    /// Anchor origin in the owning overlay layer.
    pub origin: Point,
    /// Optional anchor size, present for trigger-relative popovers.
    pub size: Option<Vector2>,
}

impl AnchoredPopoverAnchor {
    /// Build a pointer-relative anchor.
    pub fn pointer(point: Point) -> Self {
        Self {
            origin: Point::new(point.x.max(0.0), point.y.max(0.0)),
            size: None,
        }
    }

    /// Build a trigger-relative anchor.
    pub fn trigger(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x.max(0.0), y.max(0.0)),
            size: Some(Vector2::new(width.max(0.0), height.max(0.0))),
        }
    }

    pub(in crate::application) fn height(self) -> f32 {
        self.size.map_or(0.0, |size| size.y)
    }
}

/// Vertical placement policy for an anchored popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchoredPopoverPlacement {
    /// Prefer placing the popover below the anchor.
    Below,
    /// Prefer placing the popover above the anchor.
    Above,
}

/// Named construction fields for a generic anchored popover.
pub struct AnchoredPopoverParts<Message> {
    /// Content rendered inside the popover.
    pub child: ViewNode<Message>,
    /// Anchor geometry in the owning overlay layer.
    pub anchor: AnchoredPopoverAnchor,
    /// Fixed popover size.
    pub size: Vector2,
    /// Preferred vertical placement.
    pub placement: AnchoredPopoverPlacement,
    /// Gap between the anchor and popover.
    pub gap: f32,
    /// Whether to flip upward when below placement would clip.
    pub flip_when_clipped: bool,
    /// Whether to clamp the horizontal origin inside the viewport.
    pub clamp_to_viewport: bool,
    /// Whether child widgets receive input traversal.
    pub interactive: bool,
}

impl<Message> AnchoredPopoverParts<Message> {
    /// Build a below-anchor popover from named content, anchor, and size.
    pub fn below(child: ViewNode<Message>, anchor: AnchoredPopoverAnchor, size: Vector2) -> Self {
        Self {
            child,
            anchor,
            size,
            placement: AnchoredPopoverPlacement::Below,
            gap: 0.0,
            flip_when_clipped: true,
            clamp_to_viewport: true,
            interactive: true,
        }
    }

    /// Build an above-anchor popover from named content, anchor, and size.
    pub fn above(child: ViewNode<Message>, anchor: AnchoredPopoverAnchor, size: Vector2) -> Self {
        Self {
            placement: AnchoredPopoverPlacement::Above,
            flip_when_clipped: false,
            ..Self::below(child, anchor, size)
        }
    }

    /// Set the anchor gap.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Enable or disable flip-on-clipped behavior.
    pub fn flip_when_clipped(mut self, flip_when_clipped: bool) -> Self {
        self.flip_when_clipped = flip_when_clipped;
        self
    }

    /// Enable or disable horizontal viewport clamping.
    pub fn clamp_to_viewport(mut self, clamp_to_viewport: bool) -> Self {
        self.clamp_to_viewport = clamp_to_viewport;
        self
    }

    /// Enable or disable input traversal through the popover content.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

/// Named construction fields for an anchored fixed-size child layer.
pub struct AnchoredLayerParts<Message> {
    /// Child view to place inside the layer.
    pub child: ViewNode<Message>,
    /// Fixed child size.
    pub size: Vector2,
    /// Horizontal placement policy.
    pub horizontal: LayerHorizontalAnchor,
    /// Vertical placement policy.
    pub vertical: LayerVerticalAnchor,
    /// Horizontal inset from the chosen edge.
    pub inset_x: f32,
    /// Vertical inset from the chosen edge.
    pub inset_y: f32,
}

impl<Message> AnchoredLayerParts<Message> {
    /// Build anchored-layer parts.
    pub fn new(child: ViewNode<Message>, size: Vector2) -> Self {
        Self {
            child,
            size,
            horizontal: LayerHorizontalAnchor::Center,
            vertical: LayerVerticalAnchor::Center,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Set the horizontal anchor.
    pub fn horizontal(mut self, anchor: LayerHorizontalAnchor) -> Self {
        self.horizontal = anchor;
        self
    }

    /// Set the vertical anchor.
    pub fn vertical(mut self, anchor: LayerVerticalAnchor) -> Self {
        self.vertical = anchor;
        self
    }

    /// Set both edge insets.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x.max(0.0);
        self.inset_y = y.max(0.0);
        self
    }
}

/// Named construction fields for a floating layer anchored to a trigger.
pub struct FloatingLayerAnchorParts<Message> {
    /// Child view to place in the floating layer.
    pub child: ViewNode<Message>,
    /// Fixed floating-layer size.
    pub size: Vector2,
    /// Trigger left edge in the owning stack layer.
    pub x: f32,
    /// Trigger top edge in the owning stack layer.
    pub trigger_y: f32,
    /// Trigger height.
    pub trigger_height: f32,
    /// Gap between the trigger and floating layer.
    pub gap: f32,
    /// Whether to place the layer above or below the trigger.
    pub placement: FloatingLayerPlacement,
    /// Whether child widgets receive input traversal.
    pub interactive: bool,
}

impl<Message> FloatingLayerAnchorParts<Message> {
    /// Build floating-layer anchor parts.
    pub fn new(
        child: ViewNode<Message>,
        size: Vector2,
        x: f32,
        trigger_y: f32,
        trigger_height: f32,
        gap: f32,
        placement: FloatingLayerPlacement,
    ) -> Self {
        Self {
            child,
            size,
            x,
            trigger_y,
            trigger_height,
            gap,
            placement,
            interactive: false,
        }
    }

    /// Enable or disable input traversal through the floating content.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}
