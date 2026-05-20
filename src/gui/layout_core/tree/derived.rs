//! Derived layout-tree cache state and precomputed linear metrics.

mod hash;
mod metrics;

#[cfg(test)]
#[path = "derived/tests.rs"]
mod tests;

use hash::{policy_hash, slot_hash};
use metrics::{KnownMainMetrics, known_main_metrics};

use super::{NodeId, SlotChild};
use crate::gui::layout_core::model::{ContainerKind, ContainerPolicy};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContainerDerivedState {
    pub(super) state_version: u64,
    pub(super) horizontal_metrics: KnownMainMetrics,
    pub(super) vertical_metrics: KnownMainMetrics,
}

pub(super) fn container_derived_state(
    id: NodeId,
    policy: &ContainerPolicy,
    children: &[SlotChild],
) -> ContainerDerivedState {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    policy_hash(policy, &mut hasher);
    children.len().hash(&mut hasher);
    for child in children {
        child.child.id().hash(&mut hasher);
        child.child.state_version().hash(&mut hasher);
        slot_hash(&child.slot, &mut hasher);
    }
    let horizontal_metrics = if policy.kind == ContainerKind::Row {
        known_main_metrics(true, policy.spacing, children)
    } else {
        KnownMainMetrics::default()
    };
    let vertical_metrics = if policy.kind == ContainerKind::Column {
        known_main_metrics(false, policy.spacing, children)
    } else {
        KnownMainMetrics::default()
    };
    ContainerDerivedState {
        state_version: hasher.finish(),
        horizontal_metrics,
        vertical_metrics,
    }
}
