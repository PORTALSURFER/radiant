use super::super::{
    SurfaceChild, SurfaceContainer, SurfaceFloatingLayer, SurfaceNode, SurfaceOverlay,
};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{
        ContainerKind, ContainerPolicy, FloatingLayerHorizontalOverflow, FloatingLayerPolicy,
        FloatingLayerVerticalOverflow, NodeId, SlotParams,
    },
    runtime::PaintText,
    widgets::WidgetStyle,
};

impl<Message> SurfaceNode<Message> {
    /// Build a non-interactive floating overlay panel in surface coordinates.
    pub fn overlay_panel(
        id: NodeId,
        rect: Rect,
        label: impl Into<PaintText>,
        style: WidgetStyle,
    ) -> Self {
        Self::Overlay(SurfaceOverlay {
            id,
            rect,
            label: Some(label.into()),
            style,
        })
    }

    /// Build a non-interactive floating overlay marker in surface coordinates.
    pub fn overlay_marker(id: NodeId, rect: Rect, style: WidgetStyle) -> Self {
        Self::Overlay(SurfaceOverlay {
            id,
            rect,
            label: None,
            style,
        })
    }

    /// Build a floating child tree that paints above normal content.
    pub fn floating_layer(
        id: NodeId,
        offset: Point,
        size: Vector2,
        child: SurfaceNode<Message>,
        interactive: bool,
    ) -> Self {
        Self::floating_layer_with_vertical_overflow(
            id,
            offset,
            size,
            child,
            interactive,
            FloatingLayerHorizontalOverflow::Fixed,
            FloatingLayerVerticalOverflow::Fixed,
        )
    }

    /// Build a floating child tree with vertical overflow placement policy.
    pub fn floating_layer_with_vertical_overflow(
        id: NodeId,
        offset: Point,
        size: Vector2,
        child: SurfaceNode<Message>,
        interactive: bool,
        horizontal_overflow: FloatingLayerHorizontalOverflow,
        vertical_overflow: FloatingLayerVerticalOverflow,
    ) -> Self {
        let policy = ContainerPolicy {
            kind: ContainerKind::FloatingLayer,
            floating: FloatingLayerPolicy {
                offset,
                size,
                horizontal_overflow,
                vertical_overflow,
            },
            ..ContainerPolicy::default()
        };
        Self::FloatingLayer(SurfaceFloatingLayer {
            container: SurfaceContainer::new(
                id,
                policy,
                vec![SurfaceChild::new(SlotParams::fill(), child)],
            ),
            interactive,
        })
    }
}
