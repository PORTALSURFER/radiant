use super::widget::{ScrollMessageMapper, SurfaceWidget};
use crate::{
    gui::types::Rect,
    layout::{ContainerPolicy, NodeId, SlotParams},
    runtime::{NativeFileDrop, NativeFileDropMessageMapper, PaintText, ScrollUpdate},
    widgets::WidgetStyle,
};
use std::sync::Arc;

/// One slot-owned child attachment inside a surface container.
pub struct SurfaceChild<Message> {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: SurfaceNode<Message>,
}

/// Runtime-internal named construction fields for a [`SurfaceChild`].
pub(in crate::runtime) struct SurfaceChildParts<Message> {
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

// These surface tree clones stay manual so recursive nodes and message mappers
// can be cloned without requiring host application message types to implement
// `Clone`.
impl<Message> Clone for SurfaceChild<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

impl<Message> SurfaceChild<Message> {
    /// Build a container-owned surface child from runtime-internal named parts.
    pub(in crate::runtime) fn from_parts(parts: SurfaceChildParts<Message>) -> Self {
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
    pub(super) scroll_message: Option<ScrollMessageMapper<Message>>,
    pub(super) children: Vec<SurfaceChild<Message>>,
}

/// Runtime-internal named construction fields for a [`SurfaceContainer`].
pub(in crate::runtime) struct SurfaceContainerParts<Message> {
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
            scroll_message: self.scroll_message.clone(),
            children: self.children.clone(),
        }
    }
}

impl<Message> SurfaceContainer<Message> {
    /// Build a generic container node from runtime-internal named parts.
    pub(in crate::runtime) fn from_parts(parts: SurfaceContainerParts<Message>) -> Self {
        Self {
            id: parts.id,
            policy: parts.policy,
            style: None,
            hoverable: false,
            scroll_message: None,
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

    /// Return this container with a scroll movement message mapper.
    pub fn with_scroll_message(mut self, message: ScrollMessageMapper<Message>) -> Self {
        self.scroll_message = Some(message);
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

// Keep this recursive clone implementation explicit: surface trees can be
// retained and replayed by the runtime even when the host message type is not
// itself cloneable.
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

    pub(in crate::runtime) fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        match self {
            Self::Scene(scene) => scene.scroll_message(update),
            Self::Container(container) => container.scroll_message(update),
            Self::FloatingLayer(layer) => layer.container.scroll_message(update),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }

    pub(crate) fn with_native_file_drop_mapper(
        self,
        mapper: NativeFileDropMessageMapper<Message>,
    ) -> Self
    where
        Message: 'static,
    {
        match self {
            Self::Scene(mut scene) => {
                scene.base =
                    Box::new((*scene.base).with_native_file_drop_mapper(Arc::clone(&mapper)));
                scene.layers = scene
                    .layers
                    .into_iter()
                    .map(|layer| layer.with_native_file_drop_mapper(Arc::clone(&mapper)))
                    .collect();
                Self::Scene(scene)
            }
            Self::Container(mut container) => {
                container.children = container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child
                            .child
                            .with_native_file_drop_mapper(Arc::clone(&mapper)),
                    })
                    .collect();
                Self::Container(container)
            }
            Self::Widget(widget) => {
                Self::Widget(widget.with_native_file_drop(move |drop: NativeFileDrop| mapper(drop)))
            }
            Self::FloatingLayer(mut layer) => {
                layer.container.children = layer
                    .container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child
                            .child
                            .with_native_file_drop_mapper(Arc::clone(&mapper)),
                    })
                    .collect();
                Self::FloatingLayer(layer)
            }
            Self::Overlay(overlay) => Self::Overlay(overlay),
        }
    }

    pub(crate) fn accepting_native_file_drop(self) -> Self {
        match self {
            Self::Scene(mut scene) => {
                scene.base = Box::new((*scene.base).accepting_native_file_drop());
                scene.layers = scene
                    .layers
                    .into_iter()
                    .map(SurfaceLayer::accepting_native_file_drop)
                    .collect();
                Self::Scene(scene)
            }
            Self::Container(mut container) => {
                container.children = container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child.child.accepting_native_file_drop(),
                    })
                    .collect();
                Self::Container(container)
            }
            Self::Widget(widget) => Self::Widget(widget.accepting_native_file_drop()),
            Self::FloatingLayer(mut layer) => {
                layer.container.children = layer
                    .container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child.child.accepting_native_file_drop(),
                    })
                    .collect();
                Self::FloatingLayer(layer)
            }
            Self::Overlay(overlay) => Self::Overlay(overlay),
        }
    }
}

impl<Message> SurfaceLayer<Message> {
    fn with_native_file_drop_mapper(self, mapper: NativeFileDropMessageMapper<Message>) -> Self
    where
        Message: 'static,
    {
        Self {
            kind: self.kind,
            input: self
                .input
                .map(|input| input.with_native_file_drop_mapper(Arc::clone(&mapper))),
            node: self.node.with_native_file_drop_mapper(mapper),
        }
    }

    fn accepting_native_file_drop(self) -> Self {
        Self {
            kind: self.kind,
            input: self.input.map(SurfaceNode::accepting_native_file_drop),
            node: self.node.accepting_native_file_drop(),
        }
    }
}

impl<Message> SurfaceContainer<Message> {
    fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        if self.id == update.node_id
            && let Some(message) = &self.scroll_message
        {
            return Some(message(update));
        }
        self.children
            .iter()
            .find_map(|child| child.child.scroll_message(update))
    }
}

impl<Message> SurfaceScene<Message> {
    fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        self.base.scroll_message(update).or_else(|| {
            self.ordered_layers().find_map(|layer| {
                layer
                    .input
                    .as_ref()
                    .and_then(|input| input.scroll_message(update))
                    .or_else(|| layer.node.scroll_message(update))
            })
        })
    }
}
