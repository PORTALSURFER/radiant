//! Generic declarative view-tree types for message-driven Radiant hosts.

use crate::{
    gui::types::Rect,
    layout::{ContainerPolicy, LayoutNode, NodeId, SlotChild, SlotParams},
    widgets::{
        ButtonMessage, ScrollbarMessage, TextInputMessage, ToggleMessage, WidgetId, WidgetInput,
        WidgetOutput, WidgetSpec,
    },
};
use std::sync::Arc;

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Message bindings for a concrete public widget primitive.
#[derive(Default)]
pub enum WidgetMessageMapper<Message> {
    /// The widget does not currently emit host-defined messages.
    #[default]
    None,
    /// Map a button activation payload into a host-defined message.
    Button(MessageMapper<ButtonMessage, Message>),
    /// Map a toggle value-change payload into a host-defined message.
    Toggle(MessageMapper<ToggleMessage, Message>),
    /// Map a text-input edit payload into a host-defined message.
    TextInput(MessageMapper<TextInputMessage, Message>),
    /// Map a scrollbar request payload into a host-defined message.
    Scrollbar(MessageMapper<ScrollbarMessage, Message>),
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Button(map) => Self::Button(Arc::clone(map)),
            Self::Toggle(map) => Self::Toggle(Arc::clone(map)),
            Self::TextInput(map) => Self::TextInput(Arc::clone(map)),
            Self::Scrollbar(map) => Self::Scrollbar(Arc::clone(map)),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a button-message mapper.
    pub fn button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Button(Arc::new(map))
    }

    /// Build a toggle-message mapper.
    pub fn toggle(map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Toggle(Arc::new(map))
    }

    /// Build a text-input-message mapper.
    pub fn text_input(map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::TextInput(Arc::new(map))
    }

    /// Build a scrollbar-message mapper.
    pub fn scrollbar(map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Scrollbar(Arc::new(map))
    }

    fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        match (self, output) {
            (Self::Button(map), WidgetOutput::Button(message)) => Some(map(message)),
            (Self::Toggle(map), WidgetOutput::Toggle(message)) => Some(map(message)),
            (Self::TextInput(map), WidgetOutput::TextInput(message)) => Some(map(message)),
            (Self::Scrollbar(map), WidgetOutput::Scrollbar(message)) => Some(map(message)),
            _ => None,
        }
    }
}

/// One widget leaf inside a generic declarative [`UiSurface`].
pub struct SurfaceWidget<Message> {
    widget: WidgetSpec,
    messages: WidgetMessageMapper<Message>,
}

impl<Message> Clone for SurfaceWidget<Message> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            messages: self.messages.clone(),
        }
    }
}

impl<Message> SurfaceWidget<Message> {
    /// Build a widget leaf plus host-defined message mapper.
    pub fn new(widget: WidgetSpec, messages: WidgetMessageMapper<Message>) -> Self {
        Self { widget, messages }
    }

    /// Return the stable widget identifier.
    pub fn id(&self) -> WidgetId {
        self.widget.id()
    }

    /// Return the projected widget descriptor.
    pub fn widget(&self) -> &WidgetSpec {
        &self.widget
    }

    fn layout_node(&self) -> LayoutNode {
        self.widget.layout_node()
    }

    fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        (self.id() == widget_id)
            .then(|| self.widget.handle_input(bounds, input))
            .flatten()
    }

    fn dispatch_output(&self, widget_id: WidgetId, output: WidgetOutput) -> Option<Message> {
        (self.id() == widget_id)
            .then(|| self.messages.map_output(output))
            .flatten()
    }
}

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
}

/// A generic declarative container node built on top of public layout policy.
pub struct SurfaceContainer<Message> {
    id: NodeId,
    policy: ContainerPolicy,
    children: Vec<SurfaceChild<Message>>,
}

impl<Message> Clone for SurfaceContainer<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
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
            children,
        }
    }
}

/// One node in a generic declarative [`UiSurface`].
pub enum SurfaceNode<Message> {
    /// A layout container that owns slot children.
    Container(SurfaceContainer<Message>),
    /// A widget leaf plus host-defined message mapping.
    Widget(SurfaceWidget<Message>),
}

impl<Message> Clone for SurfaceNode<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Container(container) => Self::Container(container.clone()),
            Self::Widget(widget) => Self::Widget(widget.clone()),
        }
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a container node.
    pub fn container(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::Container(SurfaceContainer::new(id, policy, children))
    }

    /// Build a widget leaf node.
    pub fn widget(widget: WidgetSpec, messages: WidgetMessageMapper<Message>) -> Self {
        Self::Widget(SurfaceWidget::new(widget, messages))
    }

    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
        }
    }

    fn layout_node(&self) -> LayoutNode {
        match self {
            Self::Container(container) => LayoutNode::container(
                container.id,
                container.policy.clone(),
                container
                    .children
                    .iter()
                    .map(|child| SlotChild::new(child.slot, child.child.layout_node()))
                    .collect(),
            ),
            Self::Widget(widget) => widget.layout_node(),
        }
    }

    fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        match self {
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.handle_input(widget_id, bounds, input)),
            Self::Widget(widget) => widget.handle_input(widget_id, bounds, input),
        }
    }

    fn dispatch_output(&self, widget_id: WidgetId, output: &WidgetOutput) -> Option<Message> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::Widget(widget) => widget.dispatch_output(widget_id, output.clone()),
        }
    }

    fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
        }
    }

    fn find_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
        }
    }
}

/// Top-level immutable UI surface projected by a generic Radiant host.
pub struct UiSurface<Message> {
    root: SurfaceNode<Message>,
}

impl<Message> Clone for UiSurface<Message> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<Message> UiSurface<Message> {
    /// Build a top-level UI surface from one declarative root node.
    pub fn new(root: SurfaceNode<Message>) -> Self {
        Self { root }
    }

    /// Return the root declarative node.
    pub fn root(&self) -> &SurfaceNode<Message> {
        &self.root
    }

    /// Project the surface into the public layout tree consumed by layout engines.
    pub fn layout_node(&self) -> LayoutNode {
        self.root.layout_node()
    }

    /// Map one widget output back into a host-defined message.
    pub fn dispatch_widget_output(
        &self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message> {
        self.root.dispatch_output(widget_id, &output)
    }

    /// Route one backend-neutral interaction into a projected widget.
    pub fn dispatch_widget_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.root.handle_input(widget_id, bounds, input)
    }

    /// Find one projected widget by stable id.
    pub fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.root.find_widget(widget_id)
    }

    /// Find one projected widget by stable id for in-place runtime interaction.
    pub fn find_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        self.root.find_widget_mut(widget_id)
    }
}
