use super::widget::{SurfaceWidget, WidgetMessageMapper};
use crate::{
    gui::types::Rect,
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, LayoutNode, LayoutOutput, NodeId,
        OverflowPolicy, SlotChild, SlotParams, Vector2, VirtualizationAxis, VirtualizationPolicy,
    },
    runtime::paint::{
        SurfacePaintPlan, push_clip_end, push_clip_start, push_container_chrome,
        push_overlay_panel, push_scroll_affordance, scroll_content_clip_rect,
    },
    theme::ThemeTokens,
    widgets::{Widget, WidgetId, WidgetInput, WidgetOutput, WidgetState, WidgetStyle},
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

    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
            Self::Overlay(overlay) => overlay.id,
        }
    }

    pub(super) fn layout_node(&self) -> LayoutNode {
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

    pub(super) fn handle_input(
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

    pub(super) fn dispatch_output(
        &self,
        widget_id: WidgetId,
        output: &WidgetOutput,
    ) -> Option<Message> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::Widget(widget) => widget.dispatch_output(widget_id, output.clone()),
            Self::Overlay(_) => None,
        }
    }

    pub(super) fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
        }
    }

    pub(super) fn find_widget_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> Option<&mut SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
        }
    }

    pub(super) fn append_paint(
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
