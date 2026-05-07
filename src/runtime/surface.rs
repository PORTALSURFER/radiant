//! Generic declarative view-tree types for message-driven Radiant hosts.

use super::paint::{
    SurfacePaintPlan, push_clip_end, push_clip_start, push_container_chrome, push_overlay_panel,
    push_scroll_affordance, scroll_content_clip_rect,
};
use crate::{
    gui::types::{ImageRgba, Rect},
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, LayoutNode, LayoutOutput, NodeId,
        OverflowPolicy, SlotChild, SlotParams, Vector2, VirtualizationAxis, VirtualizationPolicy,
    },
    theme::ThemeTokens,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CanvasMessage, CanvasWidget,
        CardWidget, DragHandleMessage, DragHandleWidget, FocusBehavior, ImageWidget,
        ListItemMessage, ListItemWidget, RetainedSurfaceDescriptor, ScrollbarAxis,
        ScrollbarMessage, ScrollbarWidget, SelectableMessage, SelectableWidget, TextInputMessage,
        TextInputWidget, TextWidget, ToggleMessage, ToggleWidget, Widget, WidgetId, WidgetInput,
        WidgetOutput, WidgetSizing, WidgetState, WidgetStyle,
    },
};
use std::{collections::BTreeMap, sync::Arc};

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Message bindings that turn widget output payloads into host-defined messages.
#[derive(Default)]
pub struct WidgetMessageMapper<Message> {
    map: Option<Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.as_ref().map(Arc::clone),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper that does not emit host-defined messages.
    pub fn none() -> Self {
        Self { map: None }
    }

    /// Build a button-message mapper.
    pub fn button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a badge-message mapper.
    pub fn badge(map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a toggle-message mapper.
    pub fn toggle(map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a text-input-message mapper.
    pub fn text_input(map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a scrollbar-message mapper.
    pub fn scrollbar(map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a drag-handle-message mapper.
    pub fn drag_handle(map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a list-item-message mapper.
    pub fn list_item(map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a selectable-message mapper.
    pub fn selectable(map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a canvas-message mapper.
    pub fn canvas(map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build a mapper for any typed widget output payload.
    pub fn typed<Output>(map: impl Fn(Output) -> Message + Send + Sync + 'static) -> Self
    where
        Output: Clone + Send + Sync + 'static,
    {
        Self::dynamic(move |output| output.typed_ref::<Output>().cloned().map(&map))
    }

    /// Build a dynamic output mapper for custom widgets.
    pub fn dynamic(map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static) -> Self {
        Self {
            map: Some(Arc::new(map)),
        }
    }

    fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        self.map.as_ref().and_then(|map| map(output))
    }
}

/// One widget leaf inside a generic declarative [`UiSurface`].
pub struct SurfaceWidget<Message> {
    widget: Box<dyn Widget>,
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
    pub fn new(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self {
            widget: Box::new(widget),
            messages,
        }
    }

    /// Build a custom widget leaf plus host-defined message mapper.
    pub fn custom(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self {
            widget: Box::new(widget),
            messages,
        }
    }

    /// Build a custom boxed widget leaf plus host-defined message mapper.
    pub fn custom_box(widget: Box<dyn Widget>, messages: WidgetMessageMapper<Message>) -> Self {
        Self { widget, messages }
    }

    /// Return the stable widget identifier.
    pub fn id(&self) -> WidgetId {
        self.widget.common().id
    }

    /// Return the runtime widget object.
    pub fn widget(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
    }

    /// Return the runtime widget object.
    pub fn widget_object(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_object_mut(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
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
        self.widget.common().layout_node()
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
    style: Option<WidgetStyle>,
    hoverable: bool,
    children: Vec<SurfaceChild<Message>>,
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

/// One node in a generic declarative [`UiSurface`].
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
    id: NodeId,
    rect: Rect,
    label: Option<String>,
    style: WidgetStyle,
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
    /// Build a container node.
    pub fn container(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::Container(SurfaceContainer::new(id, policy, children))
    }

    /// Build a styled container node.
    pub fn styled_container(
        id: NodeId,
        policy: ContainerPolicy,
        style: WidgetStyle,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::Container(SurfaceContainer::new(id, policy, children).with_style(style))
    }

    /// Return this node with container hover chrome enabled when it is a container.
    pub fn with_container_hoverable(mut self, hoverable: bool) -> Self {
        if let Self::Container(container) = &mut self {
            container.hoverable = hoverable;
        }
        self
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
    pub fn widget(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::new(widget, messages))
    }

    /// Build a custom widget leaf node.
    pub fn custom_widget(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::custom(widget, messages))
    }

    /// Build a custom boxed widget leaf node.
    pub fn custom_widget_box(
        widget: Box<dyn Widget>,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::custom_box(widget, messages))
    }

    /// Build a widget leaf node that does not emit host-defined messages.
    pub fn static_widget(widget: impl Widget + Clone + 'static) -> Self {
        Self::widget(widget, WidgetMessageMapper::none())
    }

    /// Build a non-interactive floating overlay panel in surface coordinates.
    pub fn overlay_panel(
        id: NodeId,
        rect: Rect,
        label: impl Into<String>,
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

    /// Build a non-emitting text leaf node.
    pub fn text(id: WidgetId, text: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(TextWidget::new(id, text, sizing))
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
            ButtonWidget::new(id, label, sizing),
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
            BadgeWidget::new(id, label, sizing),
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
            TextInputWidget::new(id, value, sizing),
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
            ToggleWidget::new(id, label, sizing).with_checked(checked),
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
            ScrollbarWidget::new(id, axis, sizing),
            WidgetMessageMapper::scrollbar(map),
        )
    }

    /// Build a drag handle with a custom widget-to-host message mapper.
    pub fn drag_handle_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            DragHandleWidget::new(id, sizing),
            WidgetMessageMapper::drag_handle(map),
        )
    }

    /// Build a non-emitting list item leaf node.
    pub fn list_item(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ListItemWidget::new(id, label, sizing))
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
            ListItemWidget::new(id, label, sizing),
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
            SelectableWidget::new(id, label, selected, sizing),
            WidgetMessageMapper::selectable(map),
        )
    }

    /// Build a non-emitting card or panel leaf node.
    pub fn card(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CardWidget::new(id, sizing))
    }

    /// Build a non-emitting raster image leaf node.
    pub fn image(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ImageWidget::new(id, image, sizing))
    }

    /// Build a non-emitting canvas leaf node for custom paint or routed input surfaces.
    pub fn canvas(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CanvasWidget::new(id, sizing))
    }

    /// Build a canvas leaf node with a custom widget-to-host message mapper.
    pub fn canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            CanvasWidget::new(id, sizing),
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
            CanvasWidget::new(id, sizing).with_retained_surface(retained),
            WidgetMessageMapper::canvas(map),
        )
    }

    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
            Self::Overlay(overlay) => overlay.id,
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
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
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
                .find_map(|child| child.child.handle_input(widget_id, bounds, input.clone())),
            Self::Widget(widget) => widget.handle_input(widget_id, bounds, input),
            Self::Overlay(_) => None,
        }
    }

    fn dispatch_output(&self, widget_id: WidgetId, output: &WidgetOutput) -> Option<Message> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::Widget(widget) => widget.dispatch_output(widget_id, output.clone()),
            Self::Overlay(_) => None,
        }
    }

    fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
        }
    }

    fn find_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
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
            Self::Overlay(_) => {}
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
            Self::Overlay(_) => {}
        }
    }

    fn collect_widget_clip_ancestors(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        clips: &mut BTreeMap<WidgetId, Vec<NodeId>>,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    scroll_stack.push(container.id);
                }
                for child in &container.children {
                    child
                        .child
                        .collect_widget_clip_ancestors(scroll_stack, clips);
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(widget) => {
                if !scroll_stack.is_empty() {
                    clips.insert(widget.id(), scroll_stack.clone());
                }
            }
            Self::Overlay(_) => {}
        }
    }

    fn collect_scroll_container_order(&self, order: &mut Vec<NodeId>) {
        match self {
            Self::Container(container) => {
                if container.policy.kind == ContainerKind::ScrollView {
                    order.push(container.id);
                }
                for child in &container.children {
                    child.child.collect_scroll_container_order(order);
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }

    fn collect_styled_container_order(&self, order: &mut Vec<NodeId>) {
        match self {
            Self::Container(container) => {
                if container.style.is_some() && container.hoverable {
                    order.push(container.id);
                }
                for child in &container.children {
                    child.child.collect_styled_container_order(order);
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }

    fn collect_container_clip_ancestors(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        clips: &mut BTreeMap<NodeId, Vec<NodeId>>,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    scroll_stack.push(container.id);
                }
                if container.style.is_some() && container.hoverable && !scroll_stack.is_empty() {
                    clips.insert(container.id, scroll_stack.clone());
                }
                for child in &container.children {
                    child
                        .child
                        .collect_container_clip_ancestors(scroll_stack, clips);
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }

    fn scroll_content_id(&self, scroll_id: NodeId) -> Option<NodeId> {
        match self {
            Self::Container(container) => {
                if container.id == scroll_id && container.policy.kind == ContainerKind::ScrollView {
                    return container.children.first().map(|child| child.child.id());
                }
                container
                    .children
                    .iter()
                    .find_map(|child| child.child.scroll_content_id(scroll_id))
            }
            Self::Widget(_) => None,
            Self::Overlay(_) => None,
        }
    }

    fn append_paint(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
        hovered_container: Option<NodeId>,
    ) {
        match self {
            Self::Container(container) => {
                if let Some(style) = container.style {
                    push_container_chrome(
                        &mut plan.primitives,
                        container.id,
                        layout,
                        theme,
                        style,
                        WidgetState {
                            hovered: hovered_container == Some(container.id),
                            ..WidgetState::default()
                        },
                    );
                }
                if container.policy.kind == ContainerKind::ScrollView {
                    if let Some(bounds) = layout.rects.get(&container.id).copied() {
                        push_clip_start(
                            &mut plan.primitives,
                            container.id,
                            scroll_content_clip_rect(container.id, layout, bounds),
                        );
                        for child in &container.children {
                            child
                                .child
                                .append_paint(layout, theme, plan, hovered_container);
                        }
                        push_clip_end(&mut plan.primitives, container.id);
                        if let Some(content_id) =
                            container.children.first().map(|child| child.child.id())
                        {
                            push_scroll_affordance(
                                &mut plan.primitives,
                                container.id,
                                content_id,
                                layout,
                                theme,
                            );
                        }
                    }
                } else {
                    for child in &container.children {
                        child
                            .child
                            .append_paint(layout, theme, plan, hovered_container);
                    }
                }
            }
            Self::Widget(widget) => {
                let Some(bounds) = layout.rects.get(&widget.id()).copied() else {
                    return;
                };
                widget
                    .widget_object()
                    .append_paint(&mut plan.primitives, bounds, layout, theme);
            }
            Self::Overlay(overlay) => {
                push_overlay_panel(
                    &mut plan.primitives,
                    overlay.id,
                    overlay.rect,
                    overlay.label.clone(),
                    theme,
                    overlay.style,
                );
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
        self.paint_plan_with_hover(layout, theme, None)
    }

    pub(super) fn paint_plan_with_hover(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        hovered_container: Option<NodeId>,
    ) -> SurfacePaintPlan {
        let mut plan = SurfacePaintPlan::empty(theme);
        self.root
            .append_paint(layout, theme, &mut plan, hovered_container);
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

    pub(super) fn scroll_container_order(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        self.root.collect_scroll_container_order(&mut order);
        order
    }

    pub(super) fn styled_container_order(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        self.root.collect_styled_container_order(&mut order);
        order
    }

    pub(super) fn scroll_content_id(&self, scroll_id: NodeId) -> Option<NodeId> {
        self.root.scroll_content_id(scroll_id)
    }

    pub(super) fn widget_clip_ancestors(&self) -> BTreeMap<WidgetId, Vec<NodeId>> {
        let mut clips = BTreeMap::new();
        self.root
            .collect_widget_clip_ancestors(&mut Vec::new(), &mut clips);
        clips
    }

    pub(super) fn container_clip_ancestors(&self) -> BTreeMap<NodeId, Vec<NodeId>> {
        let mut clips = BTreeMap::new();
        self.root
            .collect_container_clip_ancestors(&mut Vec::new(), &mut clips);
        clips
    }
}
