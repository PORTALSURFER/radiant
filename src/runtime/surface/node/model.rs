use super::{SurfaceFloatingLayer, SurfaceOverlay, SurfaceScene};
use crate::{
    layout::{ContainerPolicy, NodeId, SlotParams},
    runtime::{
        DevtoolsLayoutDiagnostic, DevtoolsNodeKind, DevtoolsNodeSnapshot, DevtoolsWidgetSnapshot,
        surface::widget::{ScrollMessageMapper, SurfaceWidget},
    },
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

    pub(in crate::runtime) fn devtools_snapshot_node(
        &self,
        pointer_capture: Option<NodeId>,
        layout: &crate::layout::LayoutOutput,
    ) -> DevtoolsNodeSnapshot {
        let node_id = self.id();
        DevtoolsNodeSnapshot {
            node_id,
            kind: self.devtools_node_kind(),
            bounds: layout.rects.get(&node_id).copied(),
            widget: self.devtools_widget_snapshot(pointer_capture),
            layout_diagnostics: layout
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.node_id == node_id)
                .map(|diagnostic| DevtoolsLayoutDiagnostic {
                    code: diagnostic.code,
                    message: diagnostic.message.to_string(),
                })
                .collect(),
            children: self.devtools_children(pointer_capture, layout),
        }
    }

    fn devtools_node_kind(&self) -> DevtoolsNodeKind {
        match self {
            Self::Scene(_) => DevtoolsNodeKind::Scene,
            Self::Container(_) => DevtoolsNodeKind::Container,
            Self::Widget(_) => DevtoolsNodeKind::Widget,
            Self::Overlay(_) => DevtoolsNodeKind::Overlay,
            Self::FloatingLayer(_) => DevtoolsNodeKind::FloatingLayer,
        }
    }

    fn devtools_widget_snapshot(
        &self,
        pointer_capture: Option<NodeId>,
    ) -> Option<DevtoolsWidgetSnapshot> {
        let Self::Widget(widget) = self else {
            return None;
        };
        let common = widget.widget().common();
        Some(DevtoolsWidgetSnapshot {
            focus: common.focus,
            focusable: widget.is_focusable(),
            keyboard_focusable: widget.is_keyboard_focusable(),
            receives_pointer_hit_testing: widget.receives_pointer_hit_testing(),
            accepts_wheel_input: widget.receives_wheel_input(),
            accepts_pointer_move: widget.accepts_pointer_move(),
            captured: pointer_capture == Some(widget.id()),
            state: common.state,
        })
    }

    fn devtools_children(
        &self,
        pointer_capture: Option<NodeId>,
        layout: &crate::layout::LayoutOutput,
    ) -> Vec<DevtoolsNodeSnapshot> {
        match self {
            Self::Scene(scene) => std::iter::once(scene.base.as_ref())
                .chain(scene.ordered_layers().flat_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .into_iter()
                        .chain(std::iter::once(&layer.node))
                }))
                .map(|child| child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::Container(container) => container
                .children
                .iter()
                .map(|child| child.child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .map(|child| child.child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::Widget(_) | Self::Overlay(_) => Vec::new(),
        }
    }
}
