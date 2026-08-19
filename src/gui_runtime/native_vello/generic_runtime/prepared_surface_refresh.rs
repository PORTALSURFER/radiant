//! Synchronous native evidence for the private prepared surface refresh.
//!
//! The safe-boundary owner lives in [`WindowStageOwner`]. This module keeps the
//! prepared-refresh-specific native evidence and binds it to the one exact
//! Projection ticket returned by that owner.

use super::frame_stage_admission::{ProjectionStageTicket, WindowStageOwner};
use super::runner_state::NativeTargetGeneration;
use super::{NativeAdapterGeneration, NativeLifecycle};
use crate::runtime::WindowEnvironment;
use winit::window::WindowId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PreparedSurfaceRefreshNativeEvidence {
    pub(super) window_id: Option<WindowId>,
    pub(super) adapter_generation: Option<NativeAdapterGeneration>,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) environment: WindowEnvironment,
    pub(super) native_resources_present: bool,
    pub(super) target_fenced: bool,
    pub(super) pending_viewport_resize: bool,
    pub(super) pending_surface_resize: bool,
    pub(super) lifecycle: NativeLifecycle,
    pub(super) newer_visual_request: bool,
}

impl PreparedSurfaceRefreshNativeEvidence {
    fn is_admissible(self) -> bool {
        self.window_id.is_some()
            && self
                .adapter_generation
                .is_some_and(|generation| generation.is_known())
            && self.target_generation.is_known()
            && self.native_resources_present
            && !self.target_fenced
            && !self.pending_viewport_resize
            && !self.pending_surface_resize
            && self.lifecycle.is_running()
            && !self.newer_visual_request
    }
}

/// A non-`Clone` witness for one synchronous native publication attempt.
pub(super) struct PreparedSurfaceRefreshTicket {
    stage_ticket: ProjectionStageTicket,
    evidence: PreparedSurfaceRefreshNativeEvidence,
}

impl PreparedSurfaceRefreshTicket {
    fn new(
        stage_ticket: ProjectionStageTicket,
        evidence: PreparedSurfaceRefreshNativeEvidence,
    ) -> Self {
        Self {
            stage_ticket,
            evidence,
        }
    }

    pub(super) fn is_current(
        &self,
        owner: &WindowStageOwner,
        evidence: PreparedSurfaceRefreshNativeEvidence,
    ) -> bool {
        owner.projection_ticket_is_current(&self.stage_ticket)
            && evidence.is_admissible()
            && self.evidence == evidence
    }

    pub(super) fn into_stage_ticket(self) -> ProjectionStageTicket {
        self.stage_ticket
    }
}

/// Admit one prepared refresh only when all native evidence is currently
/// usable and the shared window owner can issue an exact Projection ticket.
pub(super) fn admit_prepared_surface_refresh(
    owner: &mut WindowStageOwner,
    evidence: PreparedSurfaceRefreshNativeEvidence,
) -> Option<PreparedSurfaceRefreshTicket> {
    if !evidence.is_admissible() {
        return None;
    }
    let adapter_generation = evidence.adapter_generation?;
    let stage_ticket = owner.admit_projection(adapter_generation, evidence.target_generation)?;
    Some(PreparedSurfaceRefreshTicket::new(stage_ticket, evidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::WindowEnvironment;

    fn evidence() -> PreparedSurfaceRefreshNativeEvidence {
        PreparedSurfaceRefreshNativeEvidence {
            window_id: Some(WindowId::dummy()),
            adapter_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            environment: WindowEnvironment::default(),
            native_resources_present: true,
            target_fenced: false,
            pending_viewport_resize: false,
            pending_surface_resize: false,
            lifecycle: NativeLifecycle::default(),
            newer_visual_request: false,
        }
    }

    #[test]
    fn owner_allows_one_exact_projection_ticket_and_rejects_overlap() {
        let mut owner =
            WindowStageOwner::new(super::super::frame_scheduler::FrameScheduleKey::Primary);
        let evidence = evidence();
        let ticket = admit_prepared_surface_refresh(&mut owner, evidence).expect("ticket");
        assert!(ticket.is_current(&owner, evidence));
        let identity = ticket.stage_ticket.identity();
        assert_eq!(
            identity.key(),
            &super::super::frame_scheduler::FrameScheduleKey::Primary
        );
        assert_eq!(
            identity.stage(),
            super::super::frame_scheduler_policy::SchedulerStage::Projection
        );
        assert_eq!(identity.owner_generation(), 1);
        assert_eq!(identity.revision(), 1);
        assert!(admit_prepared_surface_refresh(&mut owner, evidence).is_none());
        assert!(owner.complete_projection(ticket.into_stage_ticket()));
        assert!(!owner.has_in_flight());

        let next_ticket =
            admit_prepared_surface_refresh(&mut owner, evidence).expect("next ticket");
        assert_eq!(next_ticket.stage_ticket.identity().revision(), 2);
        assert!(owner.complete_projection(next_ticket.into_stage_ticket()));
    }

    #[test]
    fn unknown_generation_or_lifecycle_evidence_vetoes_admission() {
        let base = evidence();

        let mut unknown_adapter = base;
        unknown_adapter.adapter_generation = Some(NativeAdapterGeneration::unknown());
        assert!(
            admit_prepared_surface_refresh(
                &mut WindowStageOwner::new(
                    super::super::frame_scheduler::FrameScheduleKey::Primary
                ),
                unknown_adapter,
            )
            .is_none()
        );

        let mut unknown_target = base;
        unknown_target.target_generation = NativeTargetGeneration::default();
        assert!(
            admit_prepared_surface_refresh(
                &mut WindowStageOwner::new(
                    super::super::frame_scheduler::FrameScheduleKey::Primary
                ),
                unknown_target,
            )
            .is_none()
        );

        let mut recovering = NativeLifecycle::default();
        assert!(recovering.admit_recovery());
        let mut changed = base;
        changed.lifecycle = recovering;
        assert!(
            admit_prepared_surface_refresh(
                &mut WindowStageOwner::new(
                    super::super::frame_scheduler::FrameScheduleKey::Primary
                ),
                changed,
            )
            .is_none()
        );

        let mut closing = NativeLifecycle::default();
        assert!(closing.admit_closing(std::time::Instant::now()));
        let mut changed = base;
        changed.lifecycle = closing;
        assert!(
            admit_prepared_surface_refresh(
                &mut WindowStageOwner::new(
                    super::super::frame_scheduler::FrameScheduleKey::Primary
                ),
                changed,
            )
            .is_none()
        );

        let mut stopped = NativeLifecycle::default();
        assert!(stopped.admit_closing(std::time::Instant::now()));
        assert!(stopped.finish_closing());
        let mut changed = base;
        changed.lifecycle = stopped;
        assert!(
            admit_prepared_surface_refresh(
                &mut WindowStageOwner::new(
                    super::super::frame_scheduler::FrameScheduleKey::Primary
                ),
                changed,
            )
            .is_none()
        );
    }

    #[test]
    fn every_native_evidence_fence_stales_the_exact_ticket() {
        let base = evidence();

        let mut changed = base;
        changed.window_id = Some(WindowId::from(2));
        assert_stale(base, changed);

        let mut changed = base;
        changed.adapter_generation = Some(NativeAdapterGeneration::from_test_serial(2));
        assert_stale(base, changed);

        let mut changed = base;
        changed.target_generation = NativeTargetGeneration::from_test_serial(2);
        assert_stale(base, changed);

        let mut changed = base;
        changed.environment =
            WindowEnvironment::new(crate::theme::DpiScale::new(2.0), None, false, false);
        assert_stale(base, changed);

        let mut changed = base;
        changed.native_resources_present = false;
        assert_stale(base, changed);

        let mut changed = base;
        changed.target_fenced = true;
        assert_stale(base, changed);

        let mut changed = base;
        changed.pending_viewport_resize = true;
        assert_stale(base, changed);

        let mut changed = base;
        changed.pending_surface_resize = true;
        assert_stale(base, changed);

        let mut recovering = NativeLifecycle::default();
        assert!(recovering.admit_recovery());
        let mut changed = base;
        changed.lifecycle = recovering;
        assert_stale(base, changed);

        let mut closing = NativeLifecycle::default();
        assert!(closing.admit_closing(std::time::Instant::now()));
        let mut changed = base;
        changed.lifecycle = closing;
        assert_stale(base, changed);

        let mut stopped = NativeLifecycle::default();
        assert!(stopped.admit_closing(std::time::Instant::now()));
        assert!(stopped.finish_closing());
        let mut changed = base;
        changed.lifecycle = stopped;
        assert_stale(base, changed);

        let mut changed = base;
        changed.newer_visual_request = true;
        assert_stale(base, changed);
    }

    fn assert_stale(
        original: PreparedSurfaceRefreshNativeEvidence,
        changed: PreparedSurfaceRefreshNativeEvidence,
    ) {
        let mut owner =
            WindowStageOwner::new(super::super::frame_scheduler::FrameScheduleKey::Primary);
        let ticket = admit_prepared_surface_refresh(&mut owner, original).expect("ticket");
        assert!(!ticket.is_current(&owner, changed));
        assert!(owner.complete_projection(ticket.into_stage_ticket()));
    }

    #[test]
    fn newer_request_and_resize_evidence_veto_publication() {
        let base = evidence();
        let mut owner =
            WindowStageOwner::new(super::super::frame_scheduler::FrameScheduleKey::Primary);
        let ticket = admit_prepared_surface_refresh(&mut owner, base).expect("ticket");

        let mut newer_request = base;
        newer_request.newer_visual_request = true;
        assert!(!ticket.is_current(&owner, newer_request));

        let mut viewport_resize = base;
        viewport_resize.pending_viewport_resize = true;
        assert!(!ticket.is_current(&owner, viewport_resize));

        let mut surface_resize = base;
        surface_resize.pending_surface_resize = true;
        assert!(!ticket.is_current(&owner, surface_resize));

        assert!(owner.complete_projection(ticket.into_stage_ticket()));
    }

    #[test]
    fn ticket_stage_owner_generation_and_attempt_revision_are_exact() {
        let mut owner =
            WindowStageOwner::new(super::super::frame_scheduler::FrameScheduleKey::Primary);
        let evidence = evidence();
        let ticket = admit_prepared_surface_refresh(&mut owner, evidence).expect("ticket");
        let identity = ticket.stage_ticket.identity();
        assert_eq!(
            identity.stage(),
            super::super::frame_scheduler_policy::SchedulerStage::Projection
        );
        assert_eq!(identity.owner_generation(), owner.owner_generation());
        assert_eq!(identity.revision(), 1);
        assert!(ticket.is_current(&owner, evidence));
        assert!(owner.complete_projection(ticket.into_stage_ticket()));
    }

    #[test]
    fn completion_requires_the_exact_in_flight_attempt() {
        let mut owner =
            WindowStageOwner::new(super::super::frame_scheduler::FrameScheduleKey::Primary);
        let evidence = evidence();
        let ticket = admit_prepared_surface_refresh(&mut owner, evidence).expect("ticket");
        let wrong = owner
            .admit_projection(
                NativeAdapterGeneration::from_test_serial(2),
                NativeTargetGeneration::from_test_serial(1),
            )
            .is_none();
        assert!(wrong);
        assert!(owner.projection_ticket_is_current(&ticket.stage_ticket));
        assert!(owner.complete_projection(ticket.into_stage_ticket()));
    }
}
