//! Derived layout-tree cache state and precomputed linear metrics.

mod hash;
mod metrics;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::layout_core::{
            constraints::Constraints,
            model::{Insets, SizeModeCross, SizeModeMain, SlotParams},
        },
        gui::types::Vector2,
    };

    #[test]
    fn container_precomputes_uniform_main_size_with_extent() {
        let children = (0..4_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(24.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    crate::gui::layout_core::tree::LayoutNode::widget(
                        index + 10,
                        Vector2::new(8.0, 8.0),
                    ),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_vertical, Some(24.0));
        assert_eq!(
            container.known_main_extent_vertical,
            Some(24.0 * 4.0 + 2.0 * 3.0)
        );
        assert_eq!(container.known_main_extent_horizontal, None);
    }

    #[test]
    fn container_precomputes_only_matching_linear_axis() {
        let children = (0..3_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(18.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    crate::gui::layout_core::tree::LayoutNode::widget(
                        index + 10,
                        Vector2::new(8.0, 8.0),
                    ),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 3.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_horizontal, Some(18.0));
        assert_eq!(
            container.known_main_extent_horizontal,
            Some(18.0 * 3.0 + 3.0 * 2.0)
        );
        assert_eq!(container.known_main_extent_vertical, None);
    }

    #[test]
    fn container_does_not_mark_margin_rows_as_uniform() {
        let children = (0..4_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(24.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Insets {
                            top: -2.0,
                            bottom: 2.0,
                            ..Default::default()
                        },
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    crate::gui::layout_core::tree::LayoutNode::widget(
                        index + 10,
                        Vector2::new(8.0, 8.0),
                    ),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_vertical, None);
        assert_eq!(
            container.known_main_extent_vertical,
            Some(24.0 * 4.0 + 2.0 * 3.0)
        );
    }
}
