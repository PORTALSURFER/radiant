//! Crate-private focused ratio adjustment for runtime-owned split separators.

use super::{
    SurfaceRuntime,
    interaction_state::{
        RuntimeFocusOwner, RuntimeSplitPaneSeparatorFocusOwner, SplitPaneSeparatorBehaviorEvidence,
    },
};
use crate::{
    gui::layout_core::{
        SplitPaneDividerDescriptor, SplitPaneRatioAdjustment, SplitPaneRuntimeOwnership,
        SplitPaneRuntimePolicyRevision, SplitPaneRuntimeState, apply_split_pane_ratio_delta,
    },
    layout::{
        LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION, LayoutInteractionRevision, LayoutTargetIdentity,
    },
    runtime::{RuntimeBridge, RuntimeLifecyclePhase},
};
use std::rc::Rc;

/// Fixed upper bound for the runtime-owned focused ratio action authority.
pub(super) const MAX_SPLIT_PANE_RATIO_ACTION_AUTHORITIES: usize = 64;

/// Exact result of one focused split-separator ratio adjustment attempt.
///
/// Only `NoDestination` permits a caller-owned fallback. Veto and invalidation
/// are terminal for this attempt and never select another separator.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SplitPaneRatioAdjustmentDisposition {
    Applied { ratio: f32 },
    Unchanged,
    NoDestination,
    Vetoed,
    Invalidated,
}

/// Controller-owned action authority for one exact mounted runtime split.
///
/// This is intentionally distinct from `SplitPaneSeparatorProjection`, which
/// remains observational evidence for passive semantics and focus admission.
pub(super) struct SplitPaneRatioActionAuthority<Message> {
    pub(super) target: LayoutTargetIdentity,
    pub(super) state_id: crate::gui::layout_core::ContainerStateId,
    pub(super) mounted_state_id: crate::gui::layout_core::MountedContainerStateId,
    pub(super) descriptor: SplitPaneDividerDescriptor,
    pub(super) ownership: SplitPaneRuntimeOwnership,
    pub(super) axis: crate::gui::panel::SplitPaneAxis,
    pub(super) contract_version: u16,
    pub(super) state_schema_version: u16,
    pub(super) policy_revision: SplitPaneRuntimePolicyRevision,
    pub(super) container_bounds: crate::gui::types::Rect,
    pub(super) target_bounds: crate::gui::types::Rect,
    pub(super) divider_bounds: crate::gui::types::Rect,
    pub(super) on_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
}

impl<Message> Clone for SplitPaneRatioActionAuthority<Message> {
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            state_id: self.state_id,
            mounted_state_id: self.mounted_state_id,
            descriptor: self.descriptor,
            ownership: self.ownership,
            axis: self.axis,
            contract_version: self.contract_version,
            state_schema_version: self.state_schema_version,
            policy_revision: self.policy_revision,
            container_bounds: self.container_bounds,
            target_bounds: self.target_bounds,
            divider_bounds: self.divider_bounds,
            on_ratio_settled: self.on_ratio_settled.clone(),
        }
    }
}

#[allow(dead_code)]
impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Adjust the exact private split separator currently holding runtime
    /// focus by one finite logical-axis delta.
    pub(crate) fn adjust_focused_split_pane_ratio(
        &mut self,
        delta: f32,
    ) -> SplitPaneRatioAdjustmentDisposition {
        let Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) = self.interaction.focus.owner
        else {
            return SplitPaneRatioAdjustmentDisposition::NoDestination;
        };

        if self.interaction.layout_capture.is_some()
            || (self.gesture_owns_pointer_capture() || self.interaction.pointer.capture.is_some())
            || self.interaction.pointer.managed_capture.is_some()
            || self.interaction.pointer.scroll_drag_capture.is_some()
            || !delta.is_finite()
        {
            return SplitPaneRatioAdjustmentDisposition::Vetoed;
        }
        if self.lifecycle_phase() != RuntimeLifecyclePhase::Running
            || self.layout_authority_exhausted
        {
            self.invalidate_separator_focus(owner);
            return SplitPaneRatioAdjustmentDisposition::Invalidated;
        }
        if self
            .traversal
            .containers
            .split_pane_ratio_action_capacity_exhausted
        {
            return SplitPaneRatioAdjustmentDisposition::Vetoed;
        }

        let Some(authority) = self.action_authority_for_target(owner.target) else {
            self.invalidate_separator_focus(owner);
            return SplitPaneRatioAdjustmentDisposition::Invalidated;
        };
        if !separator_owner_matches_action_authority(owner, &authority)
            || !self.action_authority_is_current(&authority)
        {
            self.invalidate_separator_focus(owner);
            return SplitPaneRatioAdjustmentDisposition::Invalidated;
        }

        let Some(state) = self
            .interaction
            .layout_state
            .lookup_current_state_view(authority.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        else {
            self.invalidate_separator_focus(owner);
            return SplitPaneRatioAdjustmentDisposition::Invalidated;
        };
        if state.resize.is_resizing() {
            return SplitPaneRatioAdjustmentDisposition::Vetoed;
        }

        let adjustment = {
            let mut state_context = self.layout_container_state_context(
                authority.target.container_id,
                Some(authority.state_id),
            );
            state_context
                .state_mut::<SplitPaneRuntimeState>()
                .map(|state| {
                    apply_split_pane_ratio_delta(
                        state,
                        authority.descriptor,
                        authority.container_bounds,
                        authority.divider_bounds,
                        delta,
                    )
                })
        };

        let Some(adjustment) = adjustment else {
            self.invalidate_separator_focus(owner);
            return SplitPaneRatioAdjustmentDisposition::Invalidated;
        };
        let ratio = match adjustment {
            SplitPaneRatioAdjustment::Applied(ratio) => ratio,
            SplitPaneRatioAdjustment::Unchanged => {
                return SplitPaneRatioAdjustmentDisposition::Unchanged;
            }
            SplitPaneRatioAdjustment::Vetoed => {
                return SplitPaneRatioAdjustmentDisposition::Vetoed;
            }
        };

        // The state mutation is committed before either layout work or mapped
        // application output can observe it.
        self.note_mounted_layout_source_mutation(false);
        self.queue_current_surface_relayout();

        if let Some(map) = authority.on_ratio_settled {
            let outcome = self.dispatch_message(map(ratio));
            self.pending_input_command_outcome.merge(outcome);
        }
        self.revalidate_focus_owner();
        SplitPaneRatioAdjustmentDisposition::Applied { ratio }
    }

    fn action_authority_for_target(
        &self,
        target: LayoutTargetIdentity,
    ) -> Option<SplitPaneRatioActionAuthority<Message>> {
        let mut authorities = self
            .traversal
            .containers
            .split_pane_ratio_action_authorities
            .iter()
            .filter(|authority| authority.target == target);
        let authority = authorities.next()?.clone();
        authorities.next().is_none().then_some(authority)
    }

    fn action_authority_is_current(
        &self,
        authority: &SplitPaneRatioActionAuthority<Message>,
    ) -> bool {
        let candidates = self
            .traversal
            .containers
            .split_pane_ratio_action_candidates
            .iter()
            .filter(|candidate| candidate.target == authority.target);
        let Some(candidate) = candidates.clone().next() else {
            return false;
        };
        if candidates.count() != 1
            || candidate.state_id != authority.state_id
            || candidate.descriptor != authority.descriptor
            || candidate.ownership != authority.ownership
            || candidate.contract_version != authority.contract_version
            || candidate.state_schema_version != authority.state_schema_version
            || candidate.policy_revision != authority.policy_revision
        {
            return false;
        }

        let matching_targets = self
            .traversal
            .containers
            .layout_targets
            .iter()
            .filter(|target| target.target.identity() == authority.target);
        let Some(target) = matching_targets.clone().next() else {
            return false;
        };
        if matching_targets.count() != 1
            || target.target.identity() != authority.target
            || target.state_id != Some(authority.state_id)
            || target.contract_version != LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION
            || target.contract_version != authority.contract_version
            || target.revision != LayoutInteractionRevision::exact(candidate.policy_revision)
            || target.target_bounds != Some(target.target.bounds)
            || target.container_bounds != Some(authority.container_bounds)
            || target.target_bounds != Some(authority.target_bounds)
            || target.divider_bounds != Some(authority.divider_bounds)
            || target.mounted_state_id != Some(authority.mounted_state_id)
        {
            return false;
        }

        let mut descriptors = self
            .traversal
            .containers
            .split_pane_dividers
            .iter()
            .filter(|descriptor| descriptor.container_id == authority.target.container_id);
        if descriptors.clone().count() != 1 || descriptors.next() != Some(&authority.descriptor) {
            return false;
        }

        self.interaction
            .layout_state
            .current_mounted_state_id(authority.state_id)
            == Some(authority.mounted_state_id)
            && self
                .interaction
                .layout_state
                .lookup_current_state_view(authority.mounted_state_id)
                .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
                .is_some_and(|state| {
                    state.ownership == SplitPaneRuntimeOwnership::RuntimeOwned
                        && state.ratio.is_finite()
                        && (0.0..=1.0).contains(&state.ratio)
                        && state
                            .policy_revision
                            .runtime_state_compatible(authority.policy_revision)
                })
    }

    fn invalidate_separator_focus(&mut self, owner: RuntimeSplitPaneSeparatorFocusOwner) {
        if self.interaction.focus.owner == Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) {
            self.interaction.focus.owner = None;
            self.interaction.focus.command_context_widget = None;
        }
        self.revalidate_focus_owner();
    }
}

#[allow(dead_code)]
fn separator_owner_matches_action_authority<Message>(
    owner: RuntimeSplitPaneSeparatorFocusOwner,
    authority: &SplitPaneRatioActionAuthority<Message>,
) -> bool {
    owner.target == authority.target
        && owner.mounted_state_id == authority.mounted_state_id
        && owner.axis == authority.axis
        && owner.behavior
            == SplitPaneSeparatorBehaviorEvidence::new(
                authority.contract_version,
                authority.state_schema_version,
                authority.policy_revision,
            )
        && authority.ownership == SplitPaneRuntimeOwnership::RuntimeOwned
        && authority.state_schema_version == authority.state_id.schema_version()
        && authority.descriptor.axis == authority.policy_revision.axis
}
