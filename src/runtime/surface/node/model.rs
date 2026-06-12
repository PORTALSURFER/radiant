use super::{SurfaceFloatingLayer, SurfaceOverlay, SurfaceScene};
use crate::{
    layout::{ContainerPolicy, NodeId, SlotParams},
    runtime::surface::widget::{ScrollMessageMapper, SurfaceWidget},
    widgets::WidgetStyle,
};

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
    pub(in crate::runtime::surface) id: NodeId,
    pub(in crate::runtime::surface) policy: ContainerPolicy,
    pub(in crate::runtime::surface) style: Option<WidgetStyle>,
    pub(in crate::runtime::surface) hoverable: bool,
    pub(in crate::runtime::surface) scroll_message: Option<ScrollMessageMapper<Message>>,
    pub(in crate::runtime::surface) children: Vec<SurfaceChild<Message>>,
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
