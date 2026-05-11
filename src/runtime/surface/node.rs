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

impl<Message> Clone for SurfaceChild<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

impl<Message> SurfaceChild<Message> {
    /// Build a container-owned surface child.
    pub fn new(slot: SlotParams, child: SurfaceNode<Message>) -> Self {
        Self { slot, child }
    }

    /// Build a child that fills the parent slot on both axes.
    pub fn fill(child: SurfaceNode<Message>) -> Self {
        Self::new(SlotParams::fill(), child)
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
    /// Build a generic container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SurfaceChild<Message>>) -> Self {
        Self {
            id,
            policy,
            style: None,
            hoverable: false,
            children,
        }
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
    /// A layout container that owns slot children.
    Container(SurfaceContainer<Message>),
    /// A widget leaf plus host-defined message mapping.
    Widget(SurfaceWidget<Message>),
    /// A non-interactive floating overlay painted above normal layout content.
    Overlay(SurfaceOverlay),
}

/// Non-interactive floating overlay descriptor.
#[derive(Clone)]
pub struct SurfaceOverlay {
    pub(super) id: NodeId,
    pub(super) rect: Rect,
    pub(super) label: Option<PaintText>,
    pub(super) style: WidgetStyle,
}

impl<Message> Clone for SurfaceNode<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Container(container) => Self::Container(container.clone()),
            Self::Widget(widget) => Self::Widget(widget.clone()),
            Self::Overlay(overlay) => Self::Overlay(overlay.clone()),
        }
    }
}

impl<Message> SurfaceNode<Message> {
    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
            Self::Overlay(overlay) => overlay.id,
        }
    }
}
