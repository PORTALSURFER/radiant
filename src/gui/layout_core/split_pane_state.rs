//! Runtime-local state for the additive split-pane ratio modes.

use super::{ContainerStateDeclaration, ContainerStateId, Controlled, NodeId};
use crate::gui::panel::{
    PanelResizeState, SplitPaneAxis, SplitPaneCollapsePolicy, sanitized_split_pane_ratio,
};

pub(crate) const SPLIT_PANE_RUNTIME_STATE_SCHEMA_VERSION: u16 = 3;

/// Explicit source of a split-pane ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SplitPaneRuntimeMode {
    /// The mounted slot owns the live ratio, seeded from the declarative policy.
    RuntimeOwned {
        collapse_policy: Option<SplitPaneCollapsePolicy>,
    },
    /// The mounted slot accepts controlled values by strictly newer generation.
    Controlled(Controlled<f32>),
}

impl SplitPaneRuntimeMode {
    pub(crate) const fn ownership(self) -> SplitPaneRuntimeOwnership {
        match self {
            Self::RuntimeOwned { .. } => SplitPaneRuntimeOwnership::RuntimeOwned,
            Self::Controlled(_) => SplitPaneRuntimeOwnership::Controlled,
        }
    }

    pub(crate) const fn collapse_policy(self) -> Option<SplitPaneCollapsePolicy> {
        match self {
            Self::RuntimeOwned { collapse_policy } => collapse_policy,
            Self::Controlled(_) => None,
        }
    }
}

/// Stored ownership discriminator for a mounted split-pane slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SplitPaneRuntimeOwnership {
    RuntimeOwned,
    Controlled,
}

/// Declarative input used to reconcile one split-pane runtime slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneRuntimeStateInput {
    pub(crate) container_id: NodeId,
    pub(crate) initial_ratio: f32,
    pub(crate) mode: SplitPaneRuntimeMode,
    pub(crate) policy_revision: SplitPaneRuntimePolicyRevision,
}

impl SplitPaneRuntimeStateInput {
    pub(crate) fn state_id(self) -> ContainerStateId {
        ContainerStateId::new::<SplitPaneRuntimeState>(
            self.container_id,
            SPLIT_PANE_RUNTIME_STATE_SCHEMA_VERSION,
        )
    }

    pub(crate) fn declaration(self) -> ContainerStateDeclaration {
        ContainerStateDeclaration::new::<SplitPaneRuntimeState, _>(
            self.container_id,
            SPLIT_PANE_RUNTIME_STATE_SCHEMA_VERSION,
            move || SplitPaneRuntimeState::from_input(self),
        )
    }
}

/// Exact private revision of the split inputs that affect runtime geometry or
/// collapse/restore authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SplitPaneRuntimePolicyRevision {
    pub(crate) axis: SplitPaneAxis,
    pub(crate) initial_ratio: u32,
    pub(crate) divider_extent: u32,
    pub(crate) first_min_extent: u32,
    pub(crate) second_min_extent: u32,
    pub(crate) collapse_policy: Option<SplitPaneCollapsePolicy>,
}

impl SplitPaneRuntimePolicyRevision {
    pub(crate) fn new(
        policy: crate::layout::SplitPanePolicy,
        collapse_policy: Option<SplitPaneCollapsePolicy>,
    ) -> Self {
        Self {
            axis: policy.axis,
            initial_ratio: policy.initial_ratio.to_bits(),
            divider_extent: policy.divider_extent.to_bits(),
            first_min_extent: policy.first_min_extent.to_bits(),
            second_min_extent: policy.second_min_extent.to_bits(),
            collapse_policy,
        }
    }

    pub(crate) fn runtime_state_compatible(self, other: Self) -> bool {
        self.axis == other.axis
            && self.divider_extent == other.divider_extent
            && self.first_min_extent == other.first_min_extent
            && self.second_min_extent == other.second_min_extent
            && self.collapse_policy == other.collapse_policy
    }
}

impl Default for SplitPaneRuntimePolicyRevision {
    fn default() -> Self {
        Self::new(crate::layout::SplitPanePolicy::default(), None)
    }
}

/// Accepted state retained in the existing mounted container-state slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneRuntimeState {
    pub(crate) ownership: SplitPaneRuntimeOwnership,
    pub(crate) ratio: f32,
    pub(crate) accepted_controlled_generation: Option<u64>,
    pub(crate) policy_revision: SplitPaneRuntimePolicyRevision,
    pub(crate) last_expanded_ratio: Option<f32>,
    pub(crate) collapsed_policy: Option<SplitPaneCollapsePolicy>,
    pub(crate) resize: PanelResizeState,
}

impl SplitPaneRuntimeState {
    pub(crate) fn from_input(input: SplitPaneRuntimeStateInput) -> Self {
        let fallback = sanitized_split_pane_ratio(input.initial_ratio);
        match input.mode {
            SplitPaneRuntimeMode::RuntimeOwned { collapse_policy } => Self {
                ownership: SplitPaneRuntimeOwnership::RuntimeOwned,
                ratio: fallback,
                accepted_controlled_generation: None,
                policy_revision: input.policy_revision,
                last_expanded_ratio: collapse_policy.map(|_| fallback),
                collapsed_policy: None,
                resize: PanelResizeState::new(fallback),
            },
            SplitPaneRuntimeMode::Controlled(controlled) => Self {
                ownership: SplitPaneRuntimeOwnership::Controlled,
                ratio: sanitize_controlled_ratio(*controlled.value(), fallback),
                accepted_controlled_generation: Some(controlled.generation()),
                policy_revision: input.policy_revision,
                last_expanded_ratio: None,
                collapsed_policy: None,
                resize: PanelResizeState::new(sanitize_controlled_ratio(
                    *controlled.value(),
                    fallback,
                )),
            },
        }
    }

    pub(crate) fn reconcile(
        self,
        input: SplitPaneRuntimeStateInput,
    ) -> Option<SplitPaneRuntimeState> {
        match (self.ownership, input.mode) {
            (
                SplitPaneRuntimeOwnership::RuntimeOwned,
                SplitPaneRuntimeMode::RuntimeOwned { .. },
            ) if self
                .policy_revision
                .runtime_state_compatible(input.policy_revision) =>
            {
                None
            }
            (
                SplitPaneRuntimeOwnership::Controlled,
                SplitPaneRuntimeMode::Controlled(controlled),
            ) if self
                .policy_revision
                .runtime_state_compatible(input.policy_revision)
                && self
                    .accepted_controlled_generation
                    .is_some_and(|generation| controlled.generation() <= generation) =>
            {
                None
            }
            _ => Some(Self::from_input(input)),
        }
    }
}

pub(crate) fn sanitize_runtime_ratio(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn sanitize_controlled_ratio(value: f32, fallback: f32) -> f32 {
    sanitize_runtime_ratio(value, fallback)
}
