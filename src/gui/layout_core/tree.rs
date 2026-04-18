//! Layout tree node definitions for the slot-based engine.

use super::model::{ContainerPolicy, SlotParams};
use crate::gui::types::Vector2;

/// Stable node identifier for layout cache keys and output maps.
pub type NodeId = u64;

/// A child attachment entry owned by a parent container slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotChild {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: LayoutNode,
}

impl SlotChild {
    /// Build a parent-owned slot attachment.
    pub fn new(slot: SlotParams, child: LayoutNode) -> Self {
        Self { slot, child }
    }
}

/// A container node with deterministic layout policy and slot children.
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerNode {
    /// Stable node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SlotChild>,
}

impl ContainerNode {
    /// Construct a container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self {
            id,
            policy,
            children,
        }
    }
}

/// A widget node with intrinsic size hints.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetNode {
    /// Stable node id.
    pub id: NodeId,
    /// Intrinsic preferred size in logical pixels.
    pub intrinsic: Vector2,
    /// Version used by persistent layout caches.
    pub state_version: u64,
}

impl WidgetNode {
    /// Construct a widget node with an intrinsic size hint.
    pub fn new(id: NodeId, intrinsic: Vector2) -> Self {
        Self {
            id,
            intrinsic,
            state_version: 0,
        }
    }
}

/// A layout node in the strict slot-based tree.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutNode {
    /// A container that owns slots and lays out child nodes.
    Container(ContainerNode),
    /// A widget leaf that contributes intrinsic sizing information.
    Widget(WidgetNode),
}

impl LayoutNode {
    /// Return this node's stable id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(node) => node.id,
            Self::Widget(node) => node.id,
        }
    }

    /// Return this node's cache state version.
    pub fn state_version(&self) -> u64 {
        match self {
            Self::Container(_) => 0,
            Self::Widget(node) => node.state_version,
        }
    }

    /// Convenience constructor for a leaf widget node.
    pub fn widget(id: NodeId, intrinsic: Vector2) -> Self {
        Self::Widget(WidgetNode::new(id, intrinsic))
    }

    /// Convenience constructor for a container node.
    pub fn container(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self::Container(ContainerNode::new(id, policy, children))
    }
}
