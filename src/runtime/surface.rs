//! Generic declarative view-tree types for message-driven Radiant hosts.

use super::paint::{SurfacePaintPlan, push_widget_paint};
use crate::{
    gui::types::{ImageRgba, Rect},
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, LayoutNode, LayoutOutput, NodeId,
        OverflowPolicy, SlotChild, SlotParams, VirtualizationAxis, VirtualizationPolicy,
    },
    theme::ThemeTokens,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CanvasMessage, CanvasWidget,
        CardWidget, FocusBehavior, ImageWidget, ListItemMessage, ListItemWidget,
        RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget,
        SelectableMessage, SelectableWidget, TextInputMessage, TextInputWidget, TextWidget,
        ToggleMessage, ToggleWidget, WidgetId, WidgetInput, WidgetOutput, WidgetSizing, WidgetSpec,
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
    /// Map a badge activation payload into a host-defined message.
    Badge(MessageMapper<BadgeMessage, Message>),
    /// Map a toggle value-change payload into a host-defined message.
    Toggle(MessageMapper<ToggleMessage, Message>),
    /// Map a text-input edit payload into a host-defined message.
    TextInput(MessageMapper<TextInputMessage, Message>),
    /// Map a scrollbar request payload into a host-defined message.
    Scrollbar(MessageMapper<ScrollbarMessage, Message>),
    /// Map a list-item invocation payload into a host-defined message.
    ListItem(MessageMapper<ListItemMessage, Message>),
    /// Map a selectable state-change payload into a host-defined message.
    Selectable(MessageMapper<SelectableMessage, Message>),
    /// Map a canvas/custom-surface input payload into a host-defined message.
    Canvas(MessageMapper<CanvasMessage, Message>),
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Button(map) => Self::Button(Arc::clone(map)),
            Self::Badge(map) => Self::Badge(Arc::clone(map)),
            Self::Toggle(map) => Self::Toggle(Arc::clone(map)),
            Self::TextInput(map) => Self::TextInput(Arc::clone(map)),
            Self::Scrollbar(map) => Self::Scrollbar(Arc::clone(map)),
            Self::ListItem(map) => Self::ListItem(Arc::clone(map)),
            Self::Selectable(map) => Self::Selectable(Arc::clone(map)),
            Self::Canvas(map) => Self::Canvas(Arc::clone(map)),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a button-message mapper.
    pub fn button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Button(Arc::new(map))
    }

    /// Build a badge-message mapper.
    pub fn badge(map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Badge(Arc::new(map))
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

    /// Build a list-item-message mapper.
    pub fn list_item(map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::ListItem(Arc::new(map))
    }

    /// Build a selectable-message mapper.
    pub fn selectable(map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Selectable(Arc::new(map))
    }

    /// Build a canvas-message mapper.
    pub fn canvas(map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::Canvas(Arc::new(map))
    }

    fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        match (self, output) {
            (Self::Button(map), WidgetOutput::Button(message)) => Some(map(message)),
            (Self::Badge(map), WidgetOutput::Badge(message)) => Some(map(message)),
            (Self::Toggle(map), WidgetOutput::Toggle(message)) => Some(map(message)),
            (Self::TextInput(map), WidgetOutput::TextInput(message)) => Some(map(message)),
            (Self::Scrollbar(map), WidgetOutput::Scrollbar(message)) => Some(map(message)),
            (Self::ListItem(map), WidgetOutput::ListItem(message)) => Some(map(message)),
            (Self::Selectable(map), WidgetOutput::Selectable(message)) => Some(map(message)),
            (Self::Canvas(map), WidgetOutput::Canvas(message)) => Some(map(message)),
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

    /// Return whether this widget participates in runtime focus management.
    pub fn is_focusable(&self) -> bool {
        self.widget.common().focus != FocusBehavior::None
    }

    /// Return whether this widget participates in keyboard focus traversal.
    pub fn is_keyboard_focusable(&self) -> bool {
        self.widget.common().focus == FocusBehavior::Keyboard
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

    /// Build a child that fills the parent slot on both axes.
    pub fn fill(child: SurfaceNode<Message>) -> Self {
        Self::new(SlotParams::fill(), child)
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

    /// Build a row container with default policy and the requested spacing.
    pub fn row(id: NodeId, spacing: f32, children: Vec<SurfaceChild<Message>>) -> Self {
        Self::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing,
                ..ContainerPolicy::default()
            },
            children,
        )
    }

    /// Build a column container with default policy and the requested spacing.
    pub fn column(id: NodeId, spacing: f32, children: Vec<SurfaceChild<Message>>) -> Self {
        Self::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing,
                ..ContainerPolicy::default()
            },
            children,
        )
    }

    /// Build a stack container that overlays children in slot order.
    pub fn stack(id: NodeId, children: Vec<SurfaceChild<Message>>) -> Self {
        Self::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::Stack,
                ..ContainerPolicy::default()
            },
            children,
        )
    }

    /// Build a grid container with a fixed column count and explicit gaps.
    pub fn grid(
        id: NodeId,
        columns: usize,
        column_gap: f32,
        row_gap: f32,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::Grid,
                grid: GridPolicy {
                    columns,
                    column_gap,
                    row_gap,
                },
                ..ContainerPolicy::default()
            },
            children,
        )
    }

    /// Build a scroll-area container around one content child.
    pub fn scroll_area(id: NodeId, child: SurfaceNode<Message>) -> Self {
        Self::scroll_area_with_virtualization(id, child, None)
    }

    /// Build a scroll-area container with a linear virtualization policy.
    pub fn virtual_scroll_area(
        id: NodeId,
        child: SurfaceNode<Message>,
        axis: VirtualizationAxis,
        overscan_px: f32,
    ) -> Self {
        Self::scroll_area_with_virtualization(
            id,
            child,
            Some(VirtualizationPolicy {
                enabled: true,
                axis,
                overscan_px,
            }),
        )
    }

    fn scroll_area_with_virtualization(
        id: NodeId,
        child: SurfaceNode<Message>,
        virtualization: Option<VirtualizationPolicy>,
    ) -> Self {
        Self::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::ScrollView,
                overflow: OverflowPolicy::Scroll,
                virtualization,
                ..ContainerPolicy::default()
            },
            vec![SurfaceChild::fill(child)],
        )
    }

    /// Build a widget leaf node.
    pub fn widget(widget: WidgetSpec, messages: WidgetMessageMapper<Message>) -> Self {
        Self::Widget(SurfaceWidget::new(widget, messages))
    }

    /// Build a widget leaf node that does not emit host-defined messages.
    pub fn static_widget(widget: WidgetSpec) -> Self {
        Self::widget(widget, WidgetMessageMapper::None)
    }

    /// Build a non-emitting text leaf node.
    pub fn text(id: WidgetId, text: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(WidgetSpec::Text(TextWidget::new(id, text, sizing)))
    }

    /// Build a button leaf node that emits one cloned host message when activated.
    pub fn button(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::button_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build a button leaf node with a custom widget-to-host message mapper.
    pub fn button_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Button(ButtonWidget::new(id, label, sizing)),
            WidgetMessageMapper::button(map),
        )
    }

    /// Build a badge or pill leaf node that emits one cloned host message when activated.
    pub fn badge(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::badge_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build a badge or pill leaf node with a custom widget-to-host message mapper.
    pub fn badge_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Badge(BadgeWidget::new(id, label, sizing)),
            WidgetMessageMapper::badge(map),
        )
    }

    /// Build a single-line text input that maps edits and submissions by value.
    pub fn text_input(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::text_input_mapped(id, value, sizing, move |message| match message {
            TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                map(value)
            }
        })
    }

    /// Build a single-line text input with a custom widget-to-host message mapper.
    pub fn text_input_mapped(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::TextInput(TextInputWidget::new(id, value, sizing)),
            WidgetMessageMapper::text_input(map),
        )
    }

    /// Build a toggle leaf that maps value changes by checked state.
    pub fn toggle(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with an explicit checked state.
    pub fn toggle_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, checked, sizing, move |message| match message {
            ToggleMessage::ValueChanged { checked } => map(checked),
        })
    }

    /// Build a toggle leaf with a custom widget-to-host message mapper.
    pub fn toggle_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with explicit checked state and a custom mapper.
    pub fn toggle_mapped_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Toggle(ToggleWidget::new(id, label, sizing).with_checked(checked)),
            WidgetMessageMapper::toggle(map),
        )
    }

    /// Build a scrollbar leaf that maps offset changes by normalized offset.
    pub fn scrollbar(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::scrollbar_mapped(id, axis, sizing, move |message| match message {
            ScrollbarMessage::OffsetChanged { offset_fraction } => map(offset_fraction),
        })
    }

    /// Build a scrollbar leaf with a custom widget-to-host message mapper.
    pub fn scrollbar_mapped(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Scrollbar(ScrollbarWidget::new(id, axis, sizing)),
            WidgetMessageMapper::scrollbar(map),
        )
    }

    /// Build a non-emitting list item leaf node.
    pub fn list_item(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(WidgetSpec::ListItem(ListItemWidget::new(id, label, sizing)))
    }

    /// Build an invoking list item leaf node that emits one cloned host message.
    pub fn list_item_action(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::list_item_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build an invoking list item leaf node with a custom widget-to-host message mapper.
    pub fn list_item_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::ListItem(ListItemWidget::new(id, label, sizing)),
            WidgetMessageMapper::list_item(map),
        )
    }

    /// Build a selectable leaf that maps selection changes by selected state.
    pub fn selectable(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::selectable_mapped(id, label, selected, sizing, move |message| match message {
            SelectableMessage::SelectionChanged { selected } => map(selected),
        })
    }

    /// Build a selectable leaf with a custom widget-to-host message mapper.
    pub fn selectable_mapped(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Selectable(SelectableWidget::new(id, label, selected, sizing)),
            WidgetMessageMapper::selectable(map),
        )
    }

    /// Build a non-emitting card or panel leaf node.
    pub fn card(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(WidgetSpec::Card(CardWidget::new(id, sizing)))
    }

    /// Build a non-emitting raster image leaf node.
    pub fn image(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        Self::static_widget(WidgetSpec::Image(ImageWidget::new(id, image, sizing)))
    }

    /// Build a non-emitting canvas leaf node for custom paint or routed input surfaces.
    pub fn canvas(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(WidgetSpec::Canvas(CanvasWidget::new(id, sizing)))
    }

    /// Build a canvas leaf node with a custom widget-to-host message mapper.
    pub fn canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Canvas(CanvasWidget::new(id, sizing)),
            WidgetMessageMapper::canvas(map),
        )
    }

    /// Build a custom canvas with retained-surface metadata and a host-message mapper.
    pub fn retained_canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        retained: RetainedSurfaceDescriptor,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            WidgetSpec::Canvas(CanvasWidget::new(id, sizing).with_retained_surface(retained)),
            WidgetMessageMapper::canvas(map),
        )
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

    fn collect_keyboard_focus_order(&self, order: &mut Vec<WidgetId>) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_keyboard_focus_order(order);
                }
            }
            Self::Widget(widget) => {
                if widget.is_keyboard_focusable() {
                    order.push(widget.id());
                }
            }
        }
    }

    fn collect_widget_paint_order(&self, order: &mut Vec<WidgetId>) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_widget_paint_order(order);
                }
            }
            Self::Widget(widget) => order.push(widget.id()),
        }
    }

    fn append_paint(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
    ) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.append_paint(layout, theme, plan);
                }
            }
            Self::Widget(widget) => {
                push_widget_paint(&mut plan.primitives, widget.widget(), layout, theme);
            }
        }
    }
}

/// Top-level immutable UI surface projected by a generic Radiant host.
pub struct UiSurface<Message> {
    root: SurfaceNode<Message>,
}

/// Public declarative view snapshot alias for host applications.
///
/// `View<Message>` is the framework vocabulary for the top-level immutable UI
/// projection. It is an alias for [`UiSurface`] so existing code keeps the same
/// storage, cloning, layout, input, and paint behavior.
pub type View<Message> = UiSurface<Message>;

/// Public declarative element tree alias for host applications.
///
/// `Element<Message>` is the framework vocabulary for one node in a projected
/// view tree. It is an alias for [`SurfaceNode`] to keep identity and layout
/// behavior exactly shared with the existing runtime surface.
pub type Element<Message> = SurfaceNode<Message>;

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

    /// Consume the surface and return its root declarative node.
    pub fn into_root(self) -> SurfaceNode<Message> {
        self.root
    }

    /// Project the surface into the public layout tree consumed by layout engines.
    pub fn layout_node(&self) -> LayoutNode {
        self.root.layout_node()
    }

    /// Project the surface and its layout output into backend-neutral paint data.
    ///
    /// Primitives are emitted in declarative tree order so backends and tests can
    /// compare output deterministically without depending on the native shell.
    pub fn paint_plan(&self, layout: &LayoutOutput, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = SurfacePaintPlan::empty(theme);
        self.root.append_paint(layout, theme, &mut plan);
        plan
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

    /// Return whether a projected widget can own runtime focus.
    pub fn is_focusable_widget(&self, widget_id: WidgetId) -> bool {
        self.find_widget(widget_id)
            .is_some_and(SurfaceWidget::is_focusable)
    }

    /// Return keyboard-focusable widgets in deterministic declarative tree order.
    pub fn keyboard_focus_order(&self) -> Vec<WidgetId> {
        let mut order = Vec::new();
        self.root.collect_keyboard_focus_order(&mut order);
        order
    }

    pub(super) fn widget_paint_order(&self) -> Vec<WidgetId> {
        let mut order = Vec::new();
        self.root.collect_widget_paint_order(&mut order);
        order
    }
}
