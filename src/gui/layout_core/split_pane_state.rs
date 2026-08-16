//! Runtime-local state for the additive split-pane ratio modes.

use super::{ContainerStateDeclaration, ContainerStateId, Controlled, NodeId};
use crate::gui::panel::{PanelResizeState, sanitized_split_pane_ratio};

pub(crate) const SPLIT_PANE_RUNTIME_STATE_SCHEMA_VERSION: u16 = 2;

/// Explicit source of a split-pane ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SplitPaneRuntimeMode {
    /// The mounted slot owns the live ratio, seeded from the declarative policy.
    RuntimeOwned,
    /// The mounted slot accepts controlled values by strictly newer generation.
    Controlled(Controlled<f32>),
}

impl SplitPaneRuntimeMode {
    pub(crate) const fn ownership(self) -> SplitPaneRuntimeOwnership {
        match self {
            Self::RuntimeOwned => SplitPaneRuntimeOwnership::RuntimeOwned,
            Self::Controlled(_) => SplitPaneRuntimeOwnership::Controlled,
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

/// Accepted state retained in the existing mounted container-state slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneRuntimeState {
    pub(crate) ownership: SplitPaneRuntimeOwnership,
    pub(crate) ratio: f32,
    pub(crate) accepted_controlled_generation: Option<u64>,
    pub(crate) resize: PanelResizeState,
}

impl SplitPaneRuntimeState {
    pub(crate) fn from_input(input: SplitPaneRuntimeStateInput) -> Self {
        let fallback = sanitized_split_pane_ratio(input.initial_ratio);
        match input.mode {
            SplitPaneRuntimeMode::RuntimeOwned => Self {
                ownership: SplitPaneRuntimeOwnership::RuntimeOwned,
                ratio: fallback,
                accepted_controlled_generation: None,
                resize: PanelResizeState::new(fallback),
            },
            SplitPaneRuntimeMode::Controlled(controlled) => Self {
                ownership: SplitPaneRuntimeOwnership::Controlled,
                ratio: sanitize_controlled_ratio(*controlled.value(), fallback),
                accepted_controlled_generation: Some(controlled.generation()),
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
            (SplitPaneRuntimeOwnership::RuntimeOwned, SplitPaneRuntimeMode::RuntimeOwned) => None,
            (
                SplitPaneRuntimeOwnership::Controlled,
                SplitPaneRuntimeMode::Controlled(controlled),
            ) if self
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
