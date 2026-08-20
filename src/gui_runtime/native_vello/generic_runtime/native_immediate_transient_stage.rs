//! Exact private admission for native ImmediateTransient feedback.
//!
//! This stage binds the native focus, cursor-boundary, cursor-motion, and
//! wheel events that must update runtime-local interaction state at the event
//! boundary.  It deliberately does not change the existing wheel or pointer
//! coalescing policy.

use super::frame_scheduler::FrameScheduleKey;
use super::frame_stage_admission::{ImmediateTransientStageTicket, WindowStageOwner};
use super::runner_state::NativeTargetGeneration;
use super::{NativeAdapterGeneration, NativeLifecycle};
use crate::gui::input::InputTimestamp;
use winit::event::TouchPhase;

/// The native event families admitted by ImmediateTransient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeImmediateTransientKind {
    Focused(bool),
    CursorEntered,
    CursorMoved,
    CursorLeft,
    MouseWheel(TouchPhase),
}

/// Complete native evidence captured before one transient event is routed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NativeImmediateTransientStageEvidence {
    pub(super) key: FrameScheduleKey,
    pub(super) kind: NativeImmediateTransientKind,
    pub(super) timestamp: InputTimestamp,
    pub(super) window_id: Option<winit::window::WindowId>,
    pub(super) adapter_generation: NativeAdapterGeneration,
    pub(super) active_resource_generation: Option<NativeAdapterGeneration>,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) native_surface_target_fenced: bool,
    pub(super) lifecycle: NativeLifecycle,
    pub(super) native_window_eligible: bool,
    pub(super) wrapper_eligible: bool,
}

impl NativeImmediateTransientStageEvidence {
    fn is_admissible(&self) -> bool {
        self.window_id.is_some()
            && self.adapter_generation.is_known()
            && self.active_resource_generation == Some(self.adapter_generation)
            && self.target_generation.is_known()
            && !self.native_surface_target_fenced
            && self.lifecycle.is_running()
            && self.native_window_eligible
            && self.wrapper_eligible
    }
}

/// A non-`Clone` witness for one exact native ImmediateTransient event.
#[derive(Debug)]
pub(super) struct NativeImmediateTransientStageTicket {
    stage_ticket: ImmediateTransientStageTicket,
    evidence: NativeImmediateTransientStageEvidence,
}

impl NativeImmediateTransientStageTicket {
    fn new(
        stage_ticket: ImmediateTransientStageTicket,
        evidence: NativeImmediateTransientStageEvidence,
    ) -> Self {
        Self {
            stage_ticket,
            evidence,
        }
    }

    pub(super) fn is_current(
        &self,
        owner: &WindowStageOwner,
        evidence: NativeImmediateTransientStageEvidence,
    ) -> bool {
        owner.immediate_transient_ticket_is_current(&self.stage_ticket)
            && evidence.is_admissible()
            && self.evidence == evidence
    }

    pub(super) fn into_stage_ticket(self) -> ImmediateTransientStageTicket {
        self.stage_ticket
    }

    #[cfg(test)]
    pub(super) fn stage_ticket(&self) -> &ImmediateTransientStageTicket {
        &self.stage_ticket
    }

    pub(super) fn evidence(&self) -> NativeImmediateTransientStageEvidence {
        self.evidence.clone()
    }
}

/// Admit one exact native transient event under the shared per-window owner.
pub(super) fn admit_native_immediate_transient(
    owner: &mut WindowStageOwner,
    evidence: NativeImmediateTransientStageEvidence,
) -> Option<NativeImmediateTransientStageTicket> {
    if !owner.owns_key(&evidence.key) || !evidence.is_admissible() {
        return None;
    }
    let stage_ticket =
        owner.admit_immediate_transient(evidence.adapter_generation, evidence.target_generation)?;
    Some(NativeImmediateTransientStageTicket::new(
        stage_ticket,
        evidence,
    ))
}

/// Complete the exact staged native transient event.  A mismatch leaves the
/// owner in flight and never permits a fallback route.
pub(super) fn complete_native_immediate_transient(
    owner: &mut WindowStageOwner,
    ticket: NativeImmediateTransientStageTicket,
) -> bool {
    owner.complete_immediate_transient(ticket.into_stage_ticket())
}

/// Veto the exact staged native transient event before routing.  A mismatched
/// ticket is inert.
pub(super) fn veto_native_immediate_transient(
    owner: &mut WindowStageOwner,
    ticket: NativeImmediateTransientStageTicket,
) -> bool {
    owner.veto_immediate_transient(ticket.into_stage_ticket())
}

#[cfg(test)]
mod tests {
    use super::super::frame_scheduler_policy::SchedulerStage;
    use super::super::native_lifecycle_stage::{
        NativeLifecycleStageEvidence, NativeLifecycleTransitionKind, admit_native_lifecycle,
        complete_native_lifecycle,
    };
    use super::*;
    use std::time::Instant;

    fn evidence(kind: NativeImmediateTransientKind) -> NativeImmediateTransientStageEvidence {
        NativeImmediateTransientStageEvidence {
            key: FrameScheduleKey::Primary,
            kind,
            timestamp: InputTimestamp::capture(),
            window_id: Some(winit::window::WindowId::dummy()),
            adapter_generation: NativeAdapterGeneration::from_test_serial(1),
            active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        }
    }

    #[test]
    fn all_transient_kinds_share_one_exact_owner_boundary() {
        for kind in [
            NativeImmediateTransientKind::Focused(false),
            NativeImmediateTransientKind::Focused(true),
            NativeImmediateTransientKind::CursorEntered,
            NativeImmediateTransientKind::CursorMoved,
            NativeImmediateTransientKind::CursorLeft,
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Started),
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved),
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Ended),
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Cancelled),
        ] {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            let captured = evidence(kind);
            let ticket = admit_native_immediate_transient(&mut owner, captured)
                .expect("transient event should admit");
            assert_eq!(
                ticket.stage_ticket().identity().stage(),
                SchedulerStage::ImmediateTransient
            );
            assert!(complete_native_immediate_transient(&mut owner, ticket));
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn discrete_input_owner_is_never_replaced_by_transient_admission() {
        use super::super::native_discrete_input_stage::{
            NativeDiscreteInputKind, NativeDiscreteInputStageEvidence, admit_native_discrete_input,
        };

        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let input = NativeDiscreteInputStageEvidence {
            key: FrameScheduleKey::Primary,
            kind: NativeDiscreteInputKind::MouseInput,
            timestamp: InputTimestamp::capture(),
            window_id: Some(winit::window::WindowId::dummy()),
            adapter_generation: NativeAdapterGeneration::from_test_serial(1),
            active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        };
        let input_ticket = admit_native_discrete_input(&mut owner, input).expect("input");
        let transient = evidence(NativeImmediateTransientKind::CursorMoved);
        assert!(admit_native_immediate_transient(&mut owner, transient).is_none());
        assert!(owner.discrete_input_ticket_is_current(input_ticket.stage_ticket()));
        assert!(owner.has_in_flight());
    }

    #[test]
    fn primary_and_auxiliary_keys_never_share_a_transient_ticket() {
        let primary_key = FrameScheduleKey::Primary;
        let auxiliary_key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let mut primary_owner = WindowStageOwner::new(primary_key.clone());
        let mut auxiliary_owner = WindowStageOwner::new(auxiliary_key.clone());

        let primary_evidence = evidence(NativeImmediateTransientKind::CursorMoved);
        let primary_ticket =
            admit_native_immediate_transient(&mut primary_owner, primary_evidence.clone())
                .expect("primary ticket");
        let mut auxiliary_evidence = primary_evidence.clone();
        auxiliary_evidence.key = auxiliary_key;

        assert!(!primary_ticket.is_current(&auxiliary_owner, auxiliary_evidence.clone()));
        assert!(admit_native_immediate_transient(&mut primary_owner, auxiliary_evidence).is_none());
        assert!(primary_ticket.is_current(&primary_owner, primary_evidence));
        assert!(complete_native_immediate_transient(
            &mut primary_owner,
            primary_ticket
        ));

        let auxiliary_evidence = NativeImmediateTransientStageEvidence {
            key: auxiliary_owner.schedule_key().clone(),
            ..evidence(NativeImmediateTransientKind::CursorMoved)
        };
        let auxiliary_ticket =
            admit_native_immediate_transient(&mut auxiliary_owner, auxiliary_evidence)
                .expect("auxiliary ticket");
        assert!(complete_native_immediate_transient(
            &mut auxiliary_owner,
            auxiliary_ticket
        ));
    }

    #[test]
    fn lifecycle_admission_stales_live_transient_without_replay_or_clear() {
        let key = FrameScheduleKey::Primary;
        let mut owner = WindowStageOwner::new(key.clone());
        let transient = admit_native_immediate_transient(
            &mut owner,
            evidence(NativeImmediateTransientKind::CursorMoved),
        )
        .expect("transient ticket");
        let lifecycle_evidence = NativeLifecycleStageEvidence {
            key,
            transition: NativeLifecycleTransitionKind::BeginDeviceRecovery,
            source_phase: NativeLifecycle::Running,
            window_id: Some(winit::window::WindowId::dummy()),
            adapter_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
            active_resource_generation: None,
            target_generation: NativeTargetGeneration::unknown(),
            target_fenced: true,
        };
        let lifecycle = admit_native_lifecycle(&mut owner, lifecycle_evidence.clone())
            .expect("lifecycle admission should retire the transient owner");

        assert!(!complete_native_immediate_transient(&mut owner, transient));
        assert!(lifecycle.is_current(&owner, &lifecycle_evidence));
        assert!(complete_native_lifecycle(&mut owner, lifecycle));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn all_missing_or_stale_evidence_is_inert_without_owner_mutation() {
        let base = evidence(NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved));
        let mut recovering = NativeLifecycle::default();
        assert!(recovering.admit_recovery());
        let mut closing = NativeLifecycle::default();
        assert!(closing.admit_closing(Instant::now()));
        let mut stopped = closing;
        assert!(stopped.finish_closing());
        let invalid = [
            NativeImmediateTransientStageEvidence {
                adapter_generation: NativeAdapterGeneration::unknown(),
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                active_resource_generation: None,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                active_resource_generation: Some(NativeAdapterGeneration::from_test_serial(2)),
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                target_generation: NativeTargetGeneration::unknown(),
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                native_surface_target_fenced: true,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                lifecycle: recovering,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                lifecycle: closing,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                lifecycle: stopped,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                window_id: None,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                native_window_eligible: false,
                ..base.clone()
            },
            NativeImmediateTransientStageEvidence {
                wrapper_eligible: false,
                ..base.clone()
            },
        ];
        for evidence in invalid {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            assert!(admit_native_immediate_transient(&mut owner, evidence).is_none());
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn wrong_currentness_and_veto_do_not_route_or_clear_other_owner() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let captured = evidence(NativeImmediateTransientKind::Focused(false));
        let ticket =
            admit_native_immediate_transient(&mut owner, captured.clone()).expect("ticket");
        let mut changed = captured;
        changed.target_generation = NativeTargetGeneration::from_test_serial(2);
        assert!(!ticket.is_current(&owner, changed));
        assert!(owner.immediate_transient_ticket_is_current(ticket.stage_ticket()));
        assert!(veto_native_immediate_transient(&mut owner, ticket));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn wrong_ticket_never_clears_a_new_transient_owner() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let stale = admit_native_immediate_transient(
            &mut owner,
            evidence(NativeImmediateTransientKind::CursorMoved),
        )
        .expect("initial ticket");
        owner.invalidate();
        let current = admit_native_immediate_transient(
            &mut owner,
            evidence(NativeImmediateTransientKind::CursorMoved),
        )
        .expect("replacement ticket");
        assert!(!veto_native_immediate_transient(&mut owner, stale));
        assert!(owner.immediate_transient_ticket_is_current(current.stage_ticket()));
        assert!(complete_native_immediate_transient(&mut owner, current));
    }

    #[test]
    fn post_route_completion_mismatch_does_not_replay_or_clear_current_owner() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let stale = admit_native_immediate_transient(
            &mut owner,
            evidence(NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved)),
        )
        .expect("initial ticket");
        owner.invalidate();
        let current = admit_native_immediate_transient(
            &mut owner,
            evidence(NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved)),
        )
        .expect("replacement ticket");
        assert!(!complete_native_immediate_transient(&mut owner, stale));
        assert!(owner.immediate_transient_ticket_is_current(current.stage_ticket()));
        assert!(complete_native_immediate_transient(&mut owner, current));
    }

    #[test]
    fn fixed_transient_burst_has_one_monotonic_ticket_per_event() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let kinds = [
            NativeImmediateTransientKind::Focused(false),
            NativeImmediateTransientKind::Focused(true),
            NativeImmediateTransientKind::CursorEntered,
            NativeImmediateTransientKind::CursorMoved,
            NativeImmediateTransientKind::CursorLeft,
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Started),
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Moved),
            NativeImmediateTransientKind::MouseWheel(TouchPhase::Ended),
        ];
        let mut previous_revision = 0;
        for _ in 0..8 {
            for kind in kinds {
                let ticket = admit_native_immediate_transient(&mut owner, evidence(kind))
                    .expect("fixed burst event should admit");
                let revision = ticket.stage_ticket().identity().revision();
                assert!(revision > previous_revision);
                previous_revision = revision;
                assert!(complete_native_immediate_transient(&mut owner, ticket));
                assert!(!owner.has_in_flight());
            }
        }
        assert_eq!(previous_revision, (kinds.len() * 8) as u64);
    }
}
