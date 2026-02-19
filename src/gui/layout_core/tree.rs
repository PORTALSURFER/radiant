//! Layout tree node definitions for the slot-based engine.

use super::model::{ContainerPolicy, SlotParams};
use crate::gui::types::Vector2;

/// Stable node identifier for layout cache keys and output maps.
pub(crate) type NodeId = u64;

/// A child attachment entry owned by a parent container slot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SlotChild {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: LayoutNode,
}

/// A container node with deterministic layout policy and slot children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContainerNode {
    /// Stable node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SlotChild>,
}

/// A widget node with intrinsic size hints.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WidgetNode {
    /// Stable node id.
    pub id: NodeId,
    /// Intrinsic preferred size in logical pixels.
    pub intrinsic: Vector2,
}

/// A layout node in the strict slot-based tree.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LayoutNode {
    Container(ContainerNode),
    Widget(WidgetNode),
}

impl LayoutNode {
    /// Return this node's stable id.
    pub(crate) fn id(&self) -> NodeId {
        match self {
            Self::Container(node) => node.id,
            Self::Widget(node) => node.id,
        }
    }

    /// Convenience constructor for a leaf widget node.
    pub(crate) fn widget(id: NodeId, intrinsic: Vector2) -> Self {
        Self::Widget(WidgetNode { id, intrinsic })
    }

    /// Convenience constructor for a container node.
    pub(crate) fn container(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self::Container(ContainerNode {
            id,
            policy,
            children,
        })
    }
}
