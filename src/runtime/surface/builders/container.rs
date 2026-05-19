use super::super::{SurfaceChild, SurfaceContainer, SurfaceContainerParts, SurfaceNode};
use crate::{
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, NodeId, OverflowPolicy, VirtualizationAxis,
        VirtualizationPolicy,
    },
    widgets::WidgetStyle,
};

impl<Message> SurfaceNode<Message> {
    /// Build a container node.
    pub fn container(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::container_from_parts(SurfaceContainerParts {
            id,
            policy,
            children,
        })
    }

    /// Build a container node from named parts.
    pub fn container_from_parts(parts: SurfaceContainerParts<Message>) -> Self {
        Self::Container(SurfaceContainer::from_parts(parts))
    }

    /// Build a styled container node.
    pub fn styled_container(
        id: NodeId,
        policy: ContainerPolicy,
        style: WidgetStyle,
        children: Vec<SurfaceChild<Message>>,
    ) -> Self {
        Self::container_from_parts(SurfaceContainerParts {
            id,
            policy,
            children,
        })
        .with_container_style(style)
    }

    /// Return this node with explicit container chrome styling when it is a container.
    pub fn with_container_style(mut self, style: WidgetStyle) -> Self {
        if let Self::Container(container) = &mut self {
            container.style = Some(style);
        }
        self
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
}
