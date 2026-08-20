//! Exact private admission for one native lifecycle transition.
//!
//! Device-loss recovery is a lifecycle operation before it is a resource
//! operation.  This module binds the shared stage-owner ticket to the native
//! evidence that must remain unchanged between staging and the synchronous
//! transition boundary.  It deliberately permits unknown target and absent
//! resource/window evidence: those values are exact evidence, not a reason to
//! fabricate a usable post-transition target.

use super::NativeAdapterGeneration;
use super::NativeLifecycle;
use super::frame_scheduler::FrameScheduleKey;
use super::frame_stage_admission::{LifecycleStageTicket, WindowStageOwner};
use super::runner_state::NativeTargetGeneration;
use winit::window::WindowId;

/// The private native lifecycle transitions currently admitted by the stage
/// kernel.  Recovery candidate construction remains a later operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeLifecycleTransitionKind {
    BeginDeviceRecovery,
}

/// Complete native evidence captured when one lifecycle transition is staged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NativeLifecycleStageEvidence {
    pub(super) key: FrameScheduleKey,
    pub(super) transition: NativeLifecycleTransitionKind,
    pub(super) source_phase: NativeLifecycle,
    pub(super) window_id: Option<WindowId>,
    pub(super) adapter_generation: Option<NativeAdapterGeneration>,
    pub(super) active_resource_generation: Option<NativeAdapterGeneration>,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) target_fenced: bool,
}

impl NativeLifecycleStageEvidence {
    fn is_admissible(&self) -> bool {
        self.transition == NativeLifecycleTransitionKind::BeginDeviceRecovery
            && self.source_phase == NativeLifecycle::Running
            && self
                .adapter_generation
                .is_some_and(|generation| generation.is_known())
    }
}

/// A non-`Clone` witness for one exact native lifecycle transition.
#[derive(Debug)]
pub(super) struct NativeLifecycleStageTicket {
    stage_ticket: LifecycleStageTicket,
    evidence: NativeLifecycleStageEvidence,
}

impl NativeLifecycleStageTicket {
    fn new(stage_ticket: LifecycleStageTicket, evidence: NativeLifecycleStageEvidence) -> Self {
        Self {
            stage_ticket,
            evidence,
        }
    }

    pub(super) fn is_current(
        &self,
        owner: &WindowStageOwner,
        evidence: &NativeLifecycleStageEvidence,
    ) -> bool {
        owner.lifecycle_ticket_is_current(&self.stage_ticket)
            && self.evidence == *evidence
            && evidence.is_admissible()
    }

    pub(super) fn into_stage_ticket(self) -> LifecycleStageTicket {
        self.stage_ticket
    }

    pub(super) fn stage_ticket(&self) -> &LifecycleStageTicket {
        &self.stage_ticket
    }

    #[cfg(test)]
    pub(super) fn evidence(&self) -> NativeLifecycleStageEvidence {
        self.evidence.clone()
    }
}

/// Stage one exact native lifecycle transition under the shared per-window
/// owner.  The owner itself performs checked revision/generation admission and
/// atomically retires lower-stage state.
pub(super) fn admit_native_lifecycle(
    owner: &mut WindowStageOwner,
    evidence: NativeLifecycleStageEvidence,
) -> Option<NativeLifecycleStageTicket> {
    if !owner.owns_key(&evidence.key) || !evidence.is_admissible() {
        return None;
    }
    let adapter_generation = evidence.adapter_generation?;
    let stage_ticket = owner.admit_lifecycle(adapter_generation, evidence.target_generation)?;
    Some(NativeLifecycleStageTicket::new(stage_ticket, evidence))
}

/// Complete the exact staged lifecycle transition.  A mismatch leaves the
/// actual owner in flight.
pub(super) fn complete_native_lifecycle(
    owner: &mut WindowStageOwner,
    ticket: NativeLifecycleStageTicket,
) -> bool {
    owner.complete_lifecycle(ticket.into_stage_ticket())
}

/// Veto the exact staged lifecycle transition while preserving the owner's
/// advanced lifecycle fence.  A mismatched ticket is inert.
pub(super) fn veto_native_lifecycle(
    owner: &mut WindowStageOwner,
    ticket: NativeLifecycleStageTicket,
) -> bool {
    owner.veto_lifecycle(ticket.into_stage_ticket())
}

#[cfg(test)]
mod tests {
    use super::super::frame_scheduler_policy::SchedulerStage;
    use super::*;

    fn evidence(key: FrameScheduleKey) -> NativeLifecycleStageEvidence {
        NativeLifecycleStageEvidence {
            key,
            transition: NativeLifecycleTransitionKind::BeginDeviceRecovery,
            source_phase: NativeLifecycle::Running,
            window_id: Some(WindowId::dummy()),
            adapter_generation: Some(NativeAdapterGeneration::from_test_serial(3)),
            active_resource_generation: None,
            target_generation: NativeTargetGeneration::unknown(),
            target_fenced: true,
        }
    }

    #[test]
    fn lifecycle_ticket_accepts_exact_unknown_target_and_absent_resource_evidence() {
        let key = FrameScheduleKey::Primary;
        let mut owner = WindowStageOwner::new(key.clone());
        let captured = evidence(key);
        let ticket =
            admit_native_lifecycle(&mut owner, captured.clone()).expect("lifecycle ticket");

        assert!(ticket.is_current(&owner, &captured));
        assert_eq!(
            ticket.evidence().target_generation,
            NativeTargetGeneration::unknown()
        );
        assert!(ticket.evidence().active_resource_generation.is_none());
        assert_eq!(ticket.evidence().key, FrameScheduleKey::Primary);
        assert_eq!(
            ticket.stage_ticket.identity().stage(),
            SchedulerStage::Lifecycle
        );
    }

    #[test]
    fn every_native_evidence_mismatch_vetoes_without_clearing_the_owner() {
        let captured = evidence(FrameScheduleKey::Primary);
        let mutations = [
            NativeLifecycleStageEvidence {
                source_phase: {
                    let mut phase = NativeLifecycle::default();
                    assert!(phase.admit_recovery());
                    phase
                },
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                window_id: None,
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                adapter_generation: Some(NativeAdapterGeneration::from_test_serial(4)),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(3)),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                target_generation: NativeTargetGeneration::from_test_serial(7),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                target_fenced: false,
                ..captured.clone()
            },
        ];

        for current in mutations {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            let ticket =
                admit_native_lifecycle(&mut owner, captured.clone()).expect("lifecycle ticket");
            assert!(!ticket.is_current(&owner, &current));
            assert!(owner.lifecycle_ticket_is_current(&ticket.stage_ticket));
            assert!(veto_native_lifecycle(&mut owner, ticket));
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn wrong_completion_or_veto_preserves_the_real_owner() {
        let captured = evidence(FrameScheduleKey::Primary);
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let real =
            admit_native_lifecycle(&mut owner, captured.clone()).expect("real lifecycle ticket");
        let mut wrong_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let wrong =
            admit_native_lifecycle(&mut wrong_owner, captured.clone()).expect("wrong ticket");

        assert!(!complete_native_lifecycle(&mut owner, wrong));
        assert!(owner.lifecycle_ticket_is_current(&real.stage_ticket));
        assert!(veto_native_lifecycle(&mut owner, real));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn exact_completion_is_one_shot_and_identity_is_stale_afterward() {
        let captured = evidence(FrameScheduleKey::Primary);
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let ticket = admit_native_lifecycle(&mut owner, captured).expect("lifecycle ticket");
        let identity = ticket.stage_ticket.identity().clone();
        assert!(complete_native_lifecycle(&mut owner, ticket));
        assert!(!owner.has_in_flight());
        assert!(owner.stale(&identity));
    }
}
