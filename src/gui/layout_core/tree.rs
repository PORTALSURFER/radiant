//! Layout tree node definitions for the slot-based engine.

mod derived;

use super::{
    SplitPaneRuntimeMode,
    model::{ContainerPolicy, SlotParams},
    policy::LayoutPolicy,
};
use crate::gui::types::Vector2;
use derived::container_derived_state;
use std::{
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

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

/// Named construction fields for a [`SlotChild`].
#[derive(Clone, Debug, PartialEq)]
pub struct SlotChildParts {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: LayoutNode,
}

impl SlotChild {
    /// Build a parent-owned slot attachment from named parts.
    pub fn from_parts(parts: SlotChildParts) -> Self {
        Self {
            slot: parts.slot,
            child: parts.child,
        }
    }

    /// Build a parent-owned slot attachment.
    pub fn new(slot: SlotParams, child: LayoutNode) -> Self {
        Self::from_parts(SlotChildParts { slot, child })
    }
}

/// A container node with deterministic layout policy and slot children.
pub struct ContainerNode {
    /// Stable node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SlotChild>,
    pub(crate) layout_policy: Option<Rc<dyn LayoutPolicy>>,
    pub(crate) contains_layout_policy: bool,
    pub(crate) split_pane_runtime: Option<SplitPaneRuntimeMode>,
    /// Version used by persistent layout caches.
    pub(crate) state_version: u64,
    /// Precomputed horizontal row/column extent when every child has a direct known main size.
    pub(crate) known_main_extent_horizontal: Option<f32>,
    /// Precomputed vertical row/column extent when every child has a direct known main size.
    pub(crate) known_main_extent_vertical: Option<f32>,
    /// Precomputed horizontal row/column item size when all direct main sizes are uniform.
    pub(crate) known_uniform_main_horizontal: Option<f32>,
    /// Precomputed vertical row/column item size when all direct main sizes are uniform.
    pub(crate) known_uniform_main_vertical: Option<f32>,
}

/// Named construction fields for a [`ContainerNode`].
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerNodeParts {
    /// Stable node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SlotChild>,
}

impl ContainerNode {
    /// Construct a container node from named parts.
    pub fn from_parts(parts: ContainerNodeParts) -> Self {
        let derived = container_derived_state(parts.id, &parts.policy, &parts.children);
        let contains_layout_policy = parts
            .children
            .iter()
            .any(|child| child.child.contains_layout_policy());
        Self {
            id: parts.id,
            policy: parts.policy,
            children: parts.children,
            layout_policy: None,
            contains_layout_policy,
            split_pane_runtime: None,
            state_version: derived.state_version,
            known_main_extent_horizontal: derived.horizontal_metrics.extent,
            known_main_extent_vertical: derived.vertical_metrics.extent,
            known_uniform_main_horizontal: derived.horizontal_metrics.uniform_main,
            known_uniform_main_vertical: derived.vertical_metrics.uniform_main,
        }
    }

    /// Construct a container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self::from_parts(ContainerNodeParts {
            id,
            policy,
            children,
        })
    }

    pub(crate) fn with_layout_policy(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SlotChild>,
        layout_policy: Rc<dyn LayoutPolicy>,
    ) -> Self {
        let mut container = Self::from_parts(ContainerNodeParts {
            id,
            policy,
            children,
        });
        container.layout_policy = Some(layout_policy);
        container.contains_layout_policy = true;
        container
    }

    pub(crate) fn layout_policy(&self) -> Option<&dyn LayoutPolicy> {
        self.layout_policy.as_deref()
    }
}

impl Clone for ContainerNode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
            children: self.children.clone(),
            layout_policy: self.layout_policy.clone(),
            contains_layout_policy: self.contains_layout_policy,
            split_pane_runtime: self.split_pane_runtime,
            state_version: self.state_version,
            known_main_extent_horizontal: self.known_main_extent_horizontal,
            known_main_extent_vertical: self.known_main_extent_vertical,
            known_uniform_main_horizontal: self.known_uniform_main_horizontal,
            known_uniform_main_vertical: self.known_uniform_main_vertical,
        }
    }
}

impl fmt::Debug for ContainerNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerNode")
            .field("id", &self.id)
            .field("policy", &self.policy)
            .field("children", &self.children)
            .field(
                "layout_policy",
                &self.layout_policy.as_ref().map(|_| "custom"),
            )
            .field("contains_layout_policy", &self.contains_layout_policy)
            .field("split_pane_runtime", &self.split_pane_runtime)
            .field("state_version", &self.state_version)
            .field(
                "known_main_extent_horizontal",
                &self.known_main_extent_horizontal,
            )
            .field(
                "known_main_extent_vertical",
                &self.known_main_extent_vertical,
            )
            .field(
                "known_uniform_main_horizontal",
                &self.known_uniform_main_horizontal,
            )
            .field(
                "known_uniform_main_vertical",
                &self.known_uniform_main_vertical,
            )
            .finish()
    }
}

impl PartialEq for ContainerNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.policy == other.policy
            && self.children == other.children
            && match (&self.layout_policy, &other.layout_policy) {
                (None, None) => true,
                (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                _ => false,
            }
            && self.contains_layout_policy == other.contains_layout_policy
            && self.split_pane_runtime == other.split_pane_runtime
            && self.state_version == other.state_version
            && self.known_main_extent_horizontal == other.known_main_extent_horizontal
            && self.known_main_extent_vertical == other.known_main_extent_vertical
            && self.known_uniform_main_horizontal == other.known_uniform_main_horizontal
            && self.known_uniform_main_vertical == other.known_uniform_main_vertical
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

/// Named construction fields for a [`WidgetNode`].
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetNodeParts {
    /// Stable node id.
    pub id: NodeId,
    /// Intrinsic preferred size in logical pixels.
    pub intrinsic: Vector2,
}

impl WidgetNode {
    /// Construct a widget node from named parts.
    pub fn from_parts(parts: WidgetNodeParts) -> Self {
        let state_version = widget_state_version(parts.intrinsic);
        Self {
            id: parts.id,
            intrinsic: parts.intrinsic,
            state_version,
        }
    }

    /// Construct a widget node with an intrinsic size hint.
    pub fn new(id: NodeId, intrinsic: Vector2) -> Self {
        Self::from_parts(WidgetNodeParts { id, intrinsic })
    }
}

fn widget_state_version(intrinsic: Vector2) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    intrinsic.x.to_bits().hash(&mut hasher);
    intrinsic.y.to_bits().hash(&mut hasher);
    hasher.finish()
}

/// A layout node in the strict slot-based tree.
#[allow(clippy::large_enum_variant)]
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
            Self::Container(node) => node.state_version,
            Self::Widget(node) => node.state_version,
        }
    }

    /// Convenience constructor for a leaf widget node.
    pub fn widget(id: NodeId, intrinsic: Vector2) -> Self {
        Self::widget_from_parts(WidgetNodeParts { id, intrinsic })
    }

    /// Convenience constructor for a leaf widget node from named parts.
    pub fn widget_from_parts(parts: WidgetNodeParts) -> Self {
        Self::Widget(WidgetNode::from_parts(parts))
    }

    /// Convenience constructor for a container node.
    pub fn container(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self::container_from_parts(ContainerNodeParts {
            id,
            policy,
            children,
        })
    }

    /// Convenience constructor for a container node from named parts.
    pub fn container_from_parts(parts: ContainerNodeParts) -> Self {
        Self::Container(ContainerNode::from_parts(parts))
    }

    /// Construct a container driven by an object-safe custom measure/place
    /// policy.
    pub fn custom_container<Policy: LayoutPolicy>(
        id: NodeId,
        policy: Policy,
        children: Vec<SlotChild>,
    ) -> Self {
        let layout_policy: Rc<dyn LayoutPolicy> = Rc::new(policy);
        Self::Container(ContainerNode::with_layout_policy(
            id,
            ContainerPolicy::default(),
            children,
            layout_policy,
        ))
    }

    /// Construct a container with an internal runtime-owned split-pane mode.
    #[cfg(test)]
    pub(crate) fn container_with_split_pane_runtime_mode(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SlotChild>,
        split_pane_runtime: Option<SplitPaneRuntimeMode>,
    ) -> Self {
        Self::container_with_layout_policy_mode(id, policy, children, None, split_pane_runtime)
    }

    pub(crate) fn container_with_layout_policy_mode(
        id: NodeId,
        policy: ContainerPolicy,
        children: Vec<SlotChild>,
        layout_policy: Option<Rc<dyn LayoutPolicy>>,
        split_pane_runtime: Option<SplitPaneRuntimeMode>,
    ) -> Self {
        let mut container = ContainerNode::from_parts(ContainerNodeParts {
            id,
            policy,
            children,
        });
        container.layout_policy = layout_policy;
        container.contains_layout_policy =
            container.contains_layout_policy || container.layout_policy.is_some();
        container.split_pane_runtime = split_pane_runtime;
        Self::Container(container)
    }

    pub(crate) fn contains_layout_policy(&self) -> bool {
        match self {
            Self::Container(container) => container.contains_layout_policy,
            Self::Widget(_) => false,
        }
    }
}
