//! Exact private admission for the native DiscreteInput stage.
//!
//! The stage binds one native input event to the shared per-window owner and
//! the native evidence that must remain current until synchronous routing and
//! message reduction finish.  It intentionally covers only the four bounded
//! event kinds implemented by this slice.

use super::frame_scheduler::FrameScheduleKey;
use super::frame_stage_admission::{DiscreteInputStageTicket, WindowStageOwner};
use super::runner_state::NativeTargetGeneration;
use super::{NativeAdapterGeneration, NativeLifecycle};
use crate::gui::input::InputTimestamp;

/// The native event families admitted by the DiscreteInput stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeDiscreteInputKind {
    MouseInput,
    KeyboardInput,
    ModifiersChanged,
    Ime,
}

/// Complete native evidence captured before one input event is routed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NativeDiscreteInputStageEvidence {
    pub(super) key: FrameScheduleKey,
    pub(super) kind: NativeDiscreteInputKind,
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

impl NativeDiscreteInputStageEvidence {
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

/// A non-`Clone` witness for one exact native DiscreteInput event.
#[derive(Debug)]
pub(super) struct NativeDiscreteInputStageTicket {
    stage_ticket: DiscreteInputStageTicket,
    evidence: NativeDiscreteInputStageEvidence,
}

impl NativeDiscreteInputStageTicket {
    fn new(
        stage_ticket: DiscreteInputStageTicket,
        evidence: NativeDiscreteInputStageEvidence,
    ) -> Self {
        Self {
            stage_ticket,
            evidence,
        }
    }

    pub(super) fn is_current(
        &self,
        owner: &WindowStageOwner,
        evidence: NativeDiscreteInputStageEvidence,
    ) -> bool {
        owner.discrete_input_ticket_is_current(&self.stage_ticket)
            && evidence.is_admissible()
            && self.evidence == evidence
    }

    pub(super) fn into_stage_ticket(self) -> DiscreteInputStageTicket {
        self.stage_ticket
    }

    #[cfg(test)]
    pub(super) fn stage_ticket(&self) -> &DiscreteInputStageTicket {
        &self.stage_ticket
    }

    pub(super) fn evidence(&self) -> NativeDiscreteInputStageEvidence {
        self.evidence.clone()
    }
}

/// Admit one exact native input event under the shared per-window owner.
pub(super) fn admit_native_discrete_input(
    owner: &mut WindowStageOwner,
    evidence: NativeDiscreteInputStageEvidence,
) -> Option<NativeDiscreteInputStageTicket> {
    if !owner.owns_key(&evidence.key) || !evidence.is_admissible() {
        return None;
    }
    let stage_ticket =
        owner.admit_discrete_input(evidence.adapter_generation, evidence.target_generation)?;
    Some(NativeDiscreteInputStageTicket::new(stage_ticket, evidence))
}

/// Complete the exact staged native input event. A mismatch leaves the owner
/// in flight and must not trigger any fallback route.
pub(super) fn complete_native_discrete_input(
    owner: &mut WindowStageOwner,
    ticket: NativeDiscreteInputStageTicket,
) -> bool {
    owner.complete_discrete_input(ticket.into_stage_ticket())
}

/// Veto the exact staged native input event before routing. A mismatched ticket
/// is inert.
pub(super) fn veto_native_discrete_input(
    owner: &mut WindowStageOwner,
    ticket: NativeDiscreteInputStageTicket,
) -> bool {
    owner.veto_discrete_input(ticket.into_stage_ticket())
}

#[cfg(test)]
mod tests {
    use super::super::frame_scheduler_policy::SchedulerStage;
    use super::*;

    fn evidence(kind: NativeDiscreteInputKind) -> NativeDiscreteInputStageEvidence {
        NativeDiscreteInputStageEvidence {
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
    fn owner_allows_exact_input_ticket_and_requires_completion() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let captured = evidence(NativeDiscreteInputKind::MouseInput);
        let ticket =
            admit_native_discrete_input(&mut owner, captured.clone()).expect("input ticket");

        assert!(ticket.is_current(&owner, captured.clone()));
        assert_eq!(
            ticket.stage_ticket().identity().stage(),
            SchedulerStage::DiscreteInput
        );
        assert!(admit_native_discrete_input(&mut owner, captured).is_none());
        assert!(complete_native_discrete_input(&mut owner, ticket));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn all_four_covered_kinds_share_the_same_exact_owner_boundary() {
        for kind in [
            NativeDiscreteInputKind::MouseInput,
            NativeDiscreteInputKind::KeyboardInput,
            NativeDiscreteInputKind::ModifiersChanged,
            NativeDiscreteInputKind::Ime,
        ] {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            let captured = evidence(kind);
            let ticket = admit_native_discrete_input(&mut owner, captured.clone())
                .expect("covered native input kind should admit");
            assert_eq!(ticket.evidence().kind, kind);
            assert!(ticket.is_current(&owner, captured));
            assert!(complete_native_discrete_input(&mut owner, ticket));
        }
    }

    #[test]
    fn primary_and_auxiliary_keys_cannot_share_an_input_ticket() {
        let mut primary_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let primary = evidence(NativeDiscreteInputKind::MouseInput);
        let primary_ticket =
            admit_native_discrete_input(&mut primary_owner, primary.clone()).expect("primary");

        let auxiliary_key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let mut auxiliary_evidence = primary.clone();
        auxiliary_evidence.key = auxiliary_key.clone();
        let mut auxiliary_owner = WindowStageOwner::new(auxiliary_key);
        let auxiliary_ticket =
            admit_native_discrete_input(&mut auxiliary_owner, auxiliary_evidence.clone())
                .expect("auxiliary");

        assert!(!primary_ticket.is_current(&auxiliary_owner, auxiliary_evidence.clone()));
        assert!(primary_ticket.is_current(&primary_owner, primary));
        assert!(complete_native_discrete_input(
            &mut auxiliary_owner,
            auxiliary_ticket,
        ));
        assert!(complete_native_discrete_input(
            &mut primary_owner,
            primary_ticket
        ));
    }

    #[test]
    fn unknown_or_non_running_native_evidence_vetoes_admission() {
        let base = evidence(NativeDiscreteInputKind::Ime);
        let mut unknown_adapter = base.clone();
        unknown_adapter.adapter_generation = NativeAdapterGeneration::unknown();
        assert!(
            admit_native_discrete_input(
                &mut WindowStageOwner::new(FrameScheduleKey::Primary),
                unknown_adapter,
            )
            .is_none()
        );

        let mut unknown_target = base.clone();
        unknown_target.target_generation = NativeTargetGeneration::unknown();
        assert!(
            admit_native_discrete_input(
                &mut WindowStageOwner::new(FrameScheduleKey::Primary),
                unknown_target,
            )
            .is_none()
        );

        let mut recovering = base.lifecycle;
        assert!(recovering.admit_recovery());
        let mut not_running = base;
        not_running.lifecycle = recovering;
        assert!(
            admit_native_discrete_input(
                &mut WindowStageOwner::new(FrameScheduleKey::Primary),
                not_running,
            )
            .is_none()
        );
    }

    #[test]
    fn missing_window_resources_or_fenced_target_vetoes_without_owner_mutation() {
        let base = evidence(NativeDiscreteInputKind::MouseInput);

        let mut missing_window = base.clone();
        missing_window.window_id = None;
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(admit_native_discrete_input(&mut owner, missing_window).is_none());
        assert!(!owner.has_in_flight());

        let mut absent_resources = base.clone();
        absent_resources.active_resource_generation = None;
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(admit_native_discrete_input(&mut owner, absent_resources).is_none());
        assert!(!owner.has_in_flight());

        let mut mismatched_resources = base.clone();
        mismatched_resources.active_resource_generation =
            Some(NativeAdapterGeneration::from_test_serial(2));
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(admit_native_discrete_input(&mut owner, mismatched_resources).is_none());
        assert!(!owner.has_in_flight());

        let mut fenced_target = base;
        fenced_target.native_surface_target_fenced = true;
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(admit_native_discrete_input(&mut owner, fenced_target).is_none());
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn primary_native_and_auxiliary_wrapper_eligibility_vetoes_are_inert() {
        let base = evidence(NativeDiscreteInputKind::KeyboardInput);

        let mut primary_ineligible = base.clone();
        primary_ineligible.native_window_eligible = false;
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(admit_native_discrete_input(&mut owner, primary_ineligible).is_none());
        assert!(!owner.has_in_flight());

        for (key, wrapper_eligible) in [
            (FrameScheduleKey::Auxiliary(String::from("inactive")), false),
            (
                FrameScheduleKey::Auxiliary(String::from("not-admitted")),
                false,
            ),
            (
                FrameScheduleKey::Auxiliary(String::from("unmaterialized")),
                false,
            ),
        ] {
            let mut auxiliary = base.clone();
            auxiliary.key = key.clone();
            auxiliary.wrapper_eligible = wrapper_eligible;
            let mut owner = WindowStageOwner::new(key);
            assert!(admit_native_discrete_input(&mut owner, auxiliary).is_none());
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn pre_route_veto_is_inert_and_wrong_ticket_cannot_clear_owner() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let captured = evidence(NativeDiscreteInputKind::KeyboardInput);
        let ticket =
            admit_native_discrete_input(&mut owner, captured.clone()).expect("input ticket");

        let mut changed = captured.clone();
        changed.target_generation = NativeTargetGeneration::from_test_serial(2);
        assert!(!ticket.is_current(&owner, changed));
        assert!(owner.discrete_input_ticket_is_current(ticket.stage_ticket()));
        assert!(veto_native_discrete_input(&mut owner, ticket));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn lifecycle_stales_input_ticket_without_replay() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let captured = evidence(NativeDiscreteInputKind::Ime);
        let ticket =
            admit_native_discrete_input(&mut owner, captured.clone()).expect("input ticket");
        let lifecycle = owner
            .admit_lifecycle(
                captured.adapter_generation,
                NativeTargetGeneration::unknown(),
            )
            .expect("lifecycle ticket");
        assert!(!owner.discrete_input_ticket_is_current(ticket.stage_ticket()));
        assert!(!complete_native_discrete_input(&mut owner, ticket));
        assert!(owner.complete_lifecycle(lifecycle));
    }

    #[test]
    fn invalidate_retires_completed_input_evidence() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let captured = evidence(NativeDiscreteInputKind::MouseInput);
        let ticket = admit_native_discrete_input(&mut owner, captured).expect("input ticket");
        let identity = ticket.stage_ticket().identity().clone();
        assert!(complete_native_discrete_input(&mut owner, ticket));

        let previous_owner_generation = owner.owner_generation();
        owner.invalidate();

        assert_eq!(owner.owner_generation(), previous_owner_generation + 1);
        assert!(owner.stale(&identity));
    }
}
