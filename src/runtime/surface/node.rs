use super::widget::SurfaceWidget;
use crate::{
    gui::types::Rect,
    layout::{ContainerPolicy, NodeId, SlotParams},
    runtime::PaintText,
    widgets::WidgetStyle,
};

/// One slot-owned child attachment inside a surface container.
pub struct SurfaceChild<Message> {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: SurfaceNode<Message>,
}

/// Named construction fields for a [`SurfaceChild`].
pub struct SurfaceChildParts<Message> {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: SurfaceNode<Message>,
}

impl<Message> Clone for SurfaceChildParts<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

impl<Message> Clone for SurfaceChild<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

impl<Message> SurfaceChild<Message> {
    /// Build a container-owned surface child from named parts.
    pub fn from_parts(parts: SurfaceChildParts<Message>) -> Self {
        Self {
            slot: parts.slot,
            child: parts.child,
        }
    }

    /// Build a container-owned surface child.
    pub fn new(slot: SlotParams, child: SurfaceNode<Message>) -> Self {
        Self::from_parts(SurfaceChildParts { slot, child })
    }

    /// Build a child that fills the parent slot on both axes.
    pub fn fill(child: SurfaceNode<Message>) -> Self {
        Self::from_parts(SurfaceChildParts {
            slot: SlotParams::fill(),
            child,
        })
    }
}

/// A generic declarative container node built on top of public layout policy.
pub struct SurfaceContainer<Message> {
    pub(super) id: NodeId,
    pub(super) policy: ContainerPolicy,
    pub(super) style: Option<WidgetStyle>,
    pub(super) hoverable: bool,
    pub(super) children: Vec<SurfaceChild<Message>>,
}

/// Named construction fields for a [`SurfaceContainer`].
pub struct SurfaceContainerParts<Message> {
    /// Stable layout node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SurfaceChild<Message>>,
}

impl<Message> Clone for SurfaceContainerParts<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
            children: self.children.clone(),
        }
    }
}

impl<Message> Clone for SurfaceContainer<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
            style: self.style,
            hoverable: self.hoverable,
            children: self.children.clone(),
        }
    }
}

impl<Message> SurfaceContainer<Message> {
    /// Build a generic container node from named parts.
    pub fn from_parts(parts: SurfaceContainerParts<Message>) -> Self {
        Self {
            id: parts.id,
            policy: parts.policy,
            style: None,
            hoverable: false,
            children: parts.children,
        }
    }

    /// Build a generic container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SurfaceChild<Message>>) -> Self {
        Self::from_parts(SurfaceContainerParts {
            id,
            policy,
            children,
        })
    }

    /// Return this container with explicit chrome styling.
    pub fn with_style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Return this container with hover chrome enabled.
    pub fn with_hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }
}

/// One node in a generic declarative [`crate::runtime::UiSurface`].
pub enum SurfaceNode<Message> {
    /// A root scene with base content plus typed transient layers.
    Scene(SurfaceScene<Message>),
    /// A layout container that owns slot children.
    Container(SurfaceContainer<Message>),
    /// A widget leaf plus host-defined message mapping.
    Widget(SurfaceWidget<Message>),
    /// A non-interactive floating overlay painted above normal layout content.
    Overlay(SurfaceOverlay),
    /// A floating child tree that can paint out of normal layout flow.
    FloatingLayer(SurfaceFloatingLayer<Message>),
}

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

impl<Message> Clone for SurfaceLayer<Message> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            input: self.input.clone(),
            node: self.node.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum SurfaceLayerChildKind {
    Input,
    Foreground,
}

/// A root scene with base content plus typed transient layers.
pub struct SurfaceScene<Message> {
    pub(super) id: NodeId,
    pub(super) base: Box<SurfaceNode<Message>>,
    pub(super) layers: Vec<SurfaceLayer<Message>>,
}

impl<Message> SurfaceScene<Message> {
    /// Build a surface scene.
    pub fn new(id: NodeId, base: SurfaceNode<Message>, layers: Vec<SurfaceLayer<Message>>) -> Self {
        Self {
            id,
            base: Box::new(base),
            layers,
        }
    }

    pub(in crate::runtime) fn ordered_layers(
        &self,
    ) -> impl Iterator<Item = &SurfaceLayer<Message>> {
        self.ordered_layer_indices()
            .map(|layer_index| &self.layers[layer_index])
    }

    pub(in crate::runtime) fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    pub(in crate::runtime) fn ordered_layer_indices(&self) -> impl Iterator<Item = usize> + '_ {
        LayerKind::ORDER.into_iter().flat_map(|kind| {
            self.layers
                .iter()
                .enumerate()
                .filter_map(move |(index, layer)| (layer.kind == kind).then_some(index))
        })
    }

    pub(in crate::runtime) fn ordered_layer_child_for_child(
        &self,
        child_index: usize,
    ) -> Option<(usize, SurfaceLayerChildKind)> {
        let mut remaining = child_index;
        for layer_index in self.ordered_layer_indices() {
            let layer = &self.layers[layer_index];
            if layer.input.is_some() {
                if remaining == 0 {
                    return Some((layer_index, SurfaceLayerChildKind::Input));
                }
                remaining -= 1;
            }
            if remaining == 0 {
                return Some((layer_index, SurfaceLayerChildKind::Foreground));
            }
            remaining -= 1;
        }
        None
    }

    pub(in crate::runtime) fn ordered_layer_child_count(&self) -> usize {
        self.layers.iter().map(SurfaceLayer::child_count).sum()
    }
}

impl<Message> Clone for SurfaceScene<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            base: self.base.clone(),
            layers: self.layers.clone(),
        }
    }
}

/// Non-interactive floating overlay descriptor.
#[derive(Clone)]
pub struct SurfaceOverlay {
    pub(super) id: NodeId,
    pub(super) rect: Rect,
    pub(super) label: Option<PaintText>,
    pub(super) style: WidgetStyle,
}

/// One floating child tree with explicit layout placement and input policy.
pub struct SurfaceFloatingLayer<Message> {
    pub(super) container: SurfaceContainer<Message>,
    pub(super) interactive: bool,
}

impl<Message> Clone for SurfaceFloatingLayer<Message> {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            interactive: self.interactive,
        }
    }
}

impl<Message> Clone for SurfaceNode<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Scene(scene) => Self::Scene(scene.clone()),
            Self::Container(container) => Self::Container(container.clone()),
            Self::Widget(widget) => Self::Widget(widget.clone()),
            Self::Overlay(overlay) => Self::Overlay(overlay.clone()),
            Self::FloatingLayer(layer) => Self::FloatingLayer(layer.clone()),
        }
    }
}

impl<Message> SurfaceNode<Message> {
    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Scene(scene) => scene.id,
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
            Self::Overlay(overlay) => overlay.id,
            Self::FloatingLayer(layer) => layer.container.id,
        }
    }
}
