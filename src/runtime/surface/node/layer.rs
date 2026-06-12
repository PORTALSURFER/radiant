use super::{SurfaceContainer, SurfaceNode};
use crate::{gui::types::Rect, layout::NodeId, runtime::PaintText, widgets::WidgetStyle};

/// Stable category for one scene layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    /// Generic floating content above base layout.
    Floating,
    /// Popover content above generic floating layers.
    Popover,
    /// Modal content above popovers.
    Modal,
    /// Context-menu content above modals.
    ContextMenu,
    /// Tooltip content above context menus.
    Tooltip,
    /// Drag-preview content above every other transient category.
    DragPreview,
}

impl LayerKind {
    /// Stable scene-layer z-order from back to front.
    pub const ORDER: [Self; 6] = [
        Self::Floating,
        Self::Popover,
        Self::Modal,
        Self::ContextMenu,
        Self::Tooltip,
        Self::DragPreview,
    ];

    /// Return this layer kind's stable z-order bucket.
    pub const fn z_order(self) -> usize {
        match self {
            Self::Floating => 0,
            Self::Popover => 1,
            Self::Modal => 2,
            Self::ContextMenu => 3,
            Self::Tooltip => 4,
            Self::DragPreview => 5,
        }
    }
}

/// One typed transient layer inside a scene.
pub struct SurfaceLayer<Message> {
    /// Layer category used for scene z-ordering.
    pub kind: LayerKind,
    /// Optional synthesized input surface for the layer.
    pub input: Option<SurfaceNode<Message>>,
    /// Layer content node.
    pub node: SurfaceNode<Message>,
}

impl<Message> SurfaceLayer<Message> {
    /// Build a typed scene layer.
    pub fn new(kind: LayerKind, node: SurfaceNode<Message>) -> Self {
        Self::with_input(kind, None, node)
    }

    /// Build a typed scene layer with an optional input surface below content.
    pub fn with_input(
        kind: LayerKind,
        input: Option<SurfaceNode<Message>>,
        node: SurfaceNode<Message>,
    ) -> Self {
        Self { kind, input, node }
    }

    pub(in crate::runtime) fn child_count(&self) -> usize {
        usize::from(self.input.is_some()) + 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum SurfaceLayerChildKind {
    Input,
    Foreground,
}

/// Non-interactive floating overlay descriptor.
#[derive(Clone)]
pub struct SurfaceOverlay {
    pub(in crate::runtime::surface) id: NodeId,
    pub(in crate::runtime::surface) rect: Rect,
    pub(in crate::runtime::surface) label: Option<PaintText>,
    pub(in crate::runtime::surface) style: WidgetStyle,
}

/// One floating child tree with explicit layout placement and input policy.
pub struct SurfaceFloatingLayer<Message> {
    pub(in crate::runtime::surface) container: SurfaceContainer<Message>,
    pub(in crate::runtime::surface) interactive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{ContainerPolicy, SlotParams},
        runtime::SurfaceChild,
    };

    #[test]
    fn layer_child_count_includes_optional_input_surface() {
        let input = SurfaceNode::<()>::container(1, ContainerPolicy::default(), Vec::new());
        let node = SurfaceNode::<()>::container(2, ContainerPolicy::default(), Vec::new());

        assert_eq!(
            SurfaceLayer::new(LayerKind::Popover, node.clone()).child_count(),
            1
        );
        assert_eq!(
            SurfaceLayer::with_input(LayerKind::Popover, Some(input), node).child_count(),
            2
        );
    }

    #[test]
    fn floating_layer_owns_one_fill_child() {
        let child = SurfaceNode::<()>::container(2, ContainerPolicy::default(), Vec::new());
        let layer = SurfaceFloatingLayer {
            container: SurfaceContainer::new(
                1,
                ContainerPolicy::default(),
                vec![SurfaceChild::new(SlotParams::fill(), child)],
            ),
            interactive: true,
        };

        assert_eq!(layer.container.children.len(), 1);
        assert!(layer.interactive);
    }
}
