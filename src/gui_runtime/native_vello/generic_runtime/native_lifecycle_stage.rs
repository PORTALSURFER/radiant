//! Exact private admission for one native lifecycle transition.
//!
//! Device-loss recovery is a lifecycle operation before it is a resource
//! operation. This module binds the shared stage-owner ticket to native
//! evidence that must remain unchanged between staging and the synchronous
//! transition boundary. `BeginClosing` is shared by whole-run shutdown and
//! independent destructive auxiliary retirement. Finish admission
//! distinguishes a materialized primary/auxiliary window from an
//! unmaterialized auxiliary: absence is exact evidence only for the latter
//! shape.

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
    FinishDeviceRecovery,
    /// A terminal transition for whole-run shutdown or one child-local close.
    BeginClosing,
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
        match self.transition {
            NativeLifecycleTransitionKind::BeginDeviceRecovery => {
                self.adapter_generation
                    .is_some_and(|generation| generation.is_known())
                    && self.source_phase == NativeLifecycle::Running
            }
            NativeLifecycleTransitionKind::FinishDeviceRecovery => {
                let Some(adapter_generation) = self
                    .adapter_generation
                    .filter(|generation| generation.is_known())
                else {
                    return false;
                };
                if !self.source_phase.is_recovering() {
                    return false;
                }
                let materialized = self.window_id.is_some()
                    && self.active_resource_generation == Some(adapter_generation)
                    && self.target_generation.is_known()
                    && !self.target_fenced;
                let unmaterialized = matches!(self.key, FrameScheduleKey::Auxiliary(_))
                    && self.window_id.is_none()
                    && self.active_resource_generation.is_none()
                    && self.target_fenced;
                materialized || unmaterialized
            }
            NativeLifecycleTransitionKind::BeginClosing => {
                // Closing accepts a missing adapter as exact terminal
                // evidence.  An explicitly supplied unknown generation is
                // different evidence and remains invalid.
                self.adapter_generation
                    .is_none_or(|generation| generation.is_known())
                    && (self.source_phase.is_running() || self.source_phase.is_recovering())
            }
        }
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

    pub(super) const fn transition(&self) -> NativeLifecycleTransitionKind {
        self.evidence.transition
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
    let stage_ticket = match evidence.transition {
        NativeLifecycleTransitionKind::BeginClosing => owner
            .admit_terminal_lifecycle(evidence.adapter_generation, evidence.target_generation)?,
        NativeLifecycleTransitionKind::BeginDeviceRecovery
        | NativeLifecycleTransitionKind::FinishDeviceRecovery => {
            owner.admit_lifecycle(evidence.adapter_generation?, evidence.target_generation)?
        }
    };
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

    fn evidence(
        key: FrameScheduleKey,
        transition: NativeLifecycleTransitionKind,
    ) -> NativeLifecycleStageEvidence {
        NativeLifecycleStageEvidence {
            key,
            transition,
            source_phase: match transition {
                NativeLifecycleTransitionKind::BeginDeviceRecovery => NativeLifecycle::Running,
                NativeLifecycleTransitionKind::FinishDeviceRecovery => {
                    let mut phase = NativeLifecycle::default();
                    assert!(phase.admit_recovery());
                    phase
                }
                NativeLifecycleTransitionKind::BeginClosing => NativeLifecycle::Running,
            },
            window_id: Some(WindowId::dummy()),
            adapter_generation: Some(NativeAdapterGeneration::from_test_serial(3)),
            active_resource_generation: match transition {
                NativeLifecycleTransitionKind::BeginDeviceRecovery => None,
                NativeLifecycleTransitionKind::FinishDeviceRecovery => {
                    Some(NativeAdapterGeneration::from_test_serial(3))
                }
                NativeLifecycleTransitionKind::BeginClosing => None,
            },
            target_generation: match transition {
                NativeLifecycleTransitionKind::BeginDeviceRecovery => {
                    NativeTargetGeneration::unknown()
                }
                NativeLifecycleTransitionKind::FinishDeviceRecovery => {
                    NativeTargetGeneration::from_test_serial(4)
                }
                NativeLifecycleTransitionKind::BeginClosing => NativeTargetGeneration::unknown(),
            },
            target_fenced: matches!(
                transition,
                NativeLifecycleTransitionKind::BeginDeviceRecovery
                    | NativeLifecycleTransitionKind::BeginClosing
            ),
        }
    }

    #[test]
    fn lifecycle_ticket_accepts_exact_unknown_target_and_absent_resource_evidence() {
        let key = FrameScheduleKey::Primary;
        let mut owner = WindowStageOwner::new(key.clone());
        let captured = evidence(key, NativeLifecycleTransitionKind::BeginDeviceRecovery);
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
    fn closing_ticket_accepts_absent_adapter_and_unknown_target_exactly() {
        let key = FrameScheduleKey::Primary;
        let mut owner = WindowStageOwner::new(key.clone());
        let mut captured = evidence(key, NativeLifecycleTransitionKind::BeginClosing);
        captured.adapter_generation = None;
        let ticket = admit_native_lifecycle(&mut owner, captured.clone())
            .expect("closing ticket with absent adapter");

        assert!(ticket.is_current(&owner, &captured));
        assert_eq!(ticket.evidence().adapter_generation, None);
        assert_eq!(
            ticket.evidence().target_generation,
            NativeTargetGeneration::unknown()
        );
        assert!(ticket.evidence().target_fenced);
        assert!(complete_native_lifecycle(&mut owner, ticket));
    }

    #[test]
    fn closing_ticket_accepts_recovering_and_known_adapter_but_rejects_unknown_some() {
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let mut recovering = evidence(key.clone(), NativeLifecycleTransitionKind::BeginClosing);
        recovering.source_phase = {
            let mut phase = NativeLifecycle::default();
            assert!(phase.admit_recovery());
            phase
        };
        recovering.adapter_generation = Some(NativeAdapterGeneration::from_test_serial(3));
        let mut owner = WindowStageOwner::new(key.clone());
        let ticket = admit_native_lifecycle(&mut owner, recovering.clone())
            .expect("recovering closing ticket");
        assert!(ticket.is_current(&owner, &recovering));
        assert!(veto_native_lifecycle(&mut owner, ticket));

        let mut unknown_some = recovering;
        unknown_some.adapter_generation = Some(NativeAdapterGeneration::unknown());
        assert!(admit_native_lifecycle(&mut owner, unknown_some).is_none());
    }

    #[test]
    fn recovery_transitions_still_require_known_adapter() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let mut begin = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::BeginDeviceRecovery,
        );
        begin.adapter_generation = None;
        assert!(admit_native_lifecycle(&mut owner, begin).is_none());

        let mut finish = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::FinishDeviceRecovery,
        );
        finish.adapter_generation = Some(NativeAdapterGeneration::unknown());
        assert!(admit_native_lifecycle(&mut owner, finish).is_none());
    }

    #[test]
    fn every_native_evidence_mismatch_vetoes_without_clearing_the_owner() {
        let captured = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::BeginDeviceRecovery,
        );
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
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(4)),
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
    fn every_finish_evidence_mutation_vetoes_without_clearing_the_owner() {
        let captured = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::FinishDeviceRecovery,
        );
        let mutations = [
            NativeLifecycleStageEvidence {
                key: FrameScheduleKey::Auxiliary(String::from("other")),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                transition: NativeLifecycleTransitionKind::BeginDeviceRecovery,
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                source_phase: NativeLifecycle::Running,
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
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(4)),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                target_generation: NativeTargetGeneration::from_test_serial(7),
                ..captured.clone()
            },
            NativeLifecycleStageEvidence {
                target_fenced: true,
                ..captured.clone()
            },
        ];

        for current in mutations {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            let ticket =
                admit_native_lifecycle(&mut owner, captured.clone()).expect("finish ticket");
            assert!(!ticket.is_current(&owner, &current));
            assert!(owner.lifecycle_ticket_is_current(&ticket.stage_ticket));
            assert!(veto_native_lifecycle(&mut owner, ticket));
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn wrong_completion_or_veto_preserves_the_real_owner() {
        let captured = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::BeginDeviceRecovery,
        );
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
        let captured = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::BeginDeviceRecovery,
        );
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let ticket = admit_native_lifecycle(&mut owner, captured).expect("lifecycle ticket");
        let identity = ticket.stage_ticket.identity().clone();
        assert!(complete_native_lifecycle(&mut owner, ticket));
        assert!(!owner.has_in_flight());
        assert!(owner.stale(&identity));
    }

    #[test]
    fn wrong_finish_completion_preserves_the_real_owner() {
        let captured = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::FinishDeviceRecovery,
        );
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let real =
            admit_native_lifecycle(&mut owner, captured.clone()).expect("real finish ticket");
        let mut wrong_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let wrong =
            admit_native_lifecycle(&mut wrong_owner, captured).expect("wrong finish ticket");

        assert!(!complete_native_lifecycle(&mut owner, wrong));
        assert!(owner.lifecycle_ticket_is_current(&real.stage_ticket));
        assert!(veto_native_lifecycle(&mut owner, real));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn finish_ticket_accepts_recovering_with_unknown_target_and_absent_resource() {
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let mut owner = WindowStageOwner::new(key.clone());
        let mut captured = evidence(key, NativeLifecycleTransitionKind::FinishDeviceRecovery);
        captured.window_id = None;
        captured.active_resource_generation = None;
        captured.target_generation = NativeTargetGeneration::unknown();
        captured.target_fenced = true;
        let ticket =
            admit_native_lifecycle(&mut owner, captured.clone()).expect("finish lifecycle ticket");

        assert!(ticket.is_current(&owner, &captured));
        assert_eq!(
            ticket.transition(),
            NativeLifecycleTransitionKind::FinishDeviceRecovery
        );
        assert!(captured.source_phase.is_recovering());
        assert!(captured.active_resource_generation.is_none());
        assert_eq!(
            captured.target_generation,
            NativeTargetGeneration::unknown()
        );
        assert!(veto_native_lifecycle(&mut owner, ticket));
    }

    #[test]
    fn invalid_finish_shapes_are_rejected_without_clearing_real_owner() {
        let materialized = evidence(
            FrameScheduleKey::Primary,
            NativeLifecycleTransitionKind::FinishDeviceRecovery,
        );
        let materialized_invalid = vec![
            NativeLifecycleStageEvidence {
                window_id: None,
                ..materialized.clone()
            },
            NativeLifecycleStageEvidence {
                active_resource_generation: None,
                ..materialized.clone()
            },
            NativeLifecycleStageEvidence {
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(4)),
                ..materialized.clone()
            },
            NativeLifecycleStageEvidence {
                target_generation: NativeTargetGeneration::unknown(),
                ..materialized.clone()
            },
            NativeLifecycleStageEvidence {
                target_fenced: true,
                ..materialized.clone()
            },
        ];

        let unmaterialized = {
            let mut captured = evidence(
                FrameScheduleKey::Auxiliary(String::from("settings")),
                NativeLifecycleTransitionKind::FinishDeviceRecovery,
            );
            captured.window_id = None;
            captured.active_resource_generation = None;
            captured.target_generation = NativeTargetGeneration::unknown();
            captured.target_fenced = true;
            captured
        };
        let unmaterialized_invalid = vec![
            NativeLifecycleStageEvidence {
                key: FrameScheduleKey::Primary,
                ..unmaterialized.clone()
            },
            NativeLifecycleStageEvidence {
                window_id: Some(WindowId::dummy()),
                ..unmaterialized.clone()
            },
            NativeLifecycleStageEvidence {
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(3)),
                ..unmaterialized.clone()
            },
            NativeLifecycleStageEvidence {
                target_fenced: false,
                ..unmaterialized.clone()
            },
        ];

        for (valid, invalid) in [
            (materialized, materialized_invalid),
            (unmaterialized, unmaterialized_invalid),
        ] {
            let mut owner = WindowStageOwner::new(valid.key.clone());
            let real = admit_native_lifecycle(&mut owner, valid.clone())
                .expect("valid finish owner ticket");
            for invalid in invalid {
                assert!(admit_native_lifecycle(&mut owner, invalid.clone()).is_none());
                assert!(!real.is_current(&owner, &invalid));
                assert!(owner.lifecycle_ticket_is_current(real.stage_ticket()));
            }
            assert!(veto_native_lifecycle(&mut owner, real));
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn finish_ticket_rejects_running_source_but_allows_materialized_exact_evidence() {
        let key = FrameScheduleKey::Primary;
        let mut owner = WindowStageOwner::new(key.clone());
        let mut captured = evidence(key, NativeLifecycleTransitionKind::FinishDeviceRecovery);
        captured.active_resource_generation = Some(NativeAdapterGeneration::from_test_serial(3));
        captured.target_generation = NativeTargetGeneration::from_test_serial(4);
        captured.target_fenced = false;
        let ticket = admit_native_lifecycle(&mut owner, captured.clone())
            .expect("materialized finish ticket");
        assert!(ticket.is_current(&owner, &captured));

        let mut wrong_source = captured.clone();
        wrong_source.source_phase = NativeLifecycle::Running;
        assert!(!ticket.is_current(&owner, &wrong_source));
        assert!(!wrong_source.is_admissible());
        assert!(veto_native_lifecycle(&mut owner, ticket));
    }
}
