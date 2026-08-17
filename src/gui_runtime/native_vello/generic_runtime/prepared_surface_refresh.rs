//! Synchronous native evidence for the private prepared surface refresh.
//!
//! This owner is deliberately separate from [`WindowStageOwner`]. It does not
//! admit scheduler work, queue a projection, or own a worker. It only fences
//! one synchronous refresh attempt against the native identities that can make
//! a prepared runtime publication stale.

use super::frame_scheduler::FrameScheduleKey;
use super::frame_scheduler_policy::SchedulerStage;
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

const PREPARED_SURFACE_REFRESH_STAGE: SchedulerStage = SchedulerStage::Projection;

/// A non-`Clone` witness for one synchronous native publication attempt.
pub(super) struct PreparedSurfaceRefreshTicket {
    frame_schedule_key: FrameScheduleKey,
    evidence: PreparedSurfaceRefreshNativeEvidence,
    stage: SchedulerStage,
    owner_generation: u64,
    attempt_revision: u64,
}

/// One-at-a-time owner for synchronous prepared refresh tickets.
pub(super) struct PreparedSurfaceRefreshOwner {
    frame_schedule_key: FrameScheduleKey,
    owner_generation: u64,
    next_attempt_revision: Option<u64>,
    in_flight_attempt_revision: Option<u64>,
}

impl PreparedSurfaceRefreshOwner {
    pub(super) fn new(frame_schedule_key: FrameScheduleKey) -> Self {
        Self {
            frame_schedule_key,
            owner_generation: 1,
            next_attempt_revision: Some(1),
            in_flight_attempt_revision: None,
        }
    }

    pub(super) fn begin(
        &mut self,
        evidence: PreparedSurfaceRefreshNativeEvidence,
    ) -> Option<PreparedSurfaceRefreshTicket> {
        if self.in_flight_attempt_revision.is_some() || !evidence.is_admissible() {
            return None;
        }
        let attempt_revision = self.next_attempt_revision?;
        if attempt_revision == 0 || attempt_revision == u64::MAX {
            self.next_attempt_revision = None;
            return None;
        }
        let next_revision = attempt_revision.checked_add(1);
        self.next_attempt_revision = next_revision.filter(|revision| *revision != u64::MAX);
        if evidence.window_id.is_none() || evidence.adapter_generation.is_none() {
            return None;
        }
        self.in_flight_attempt_revision = Some(attempt_revision);
        Some(PreparedSurfaceRefreshTicket {
            frame_schedule_key: self.frame_schedule_key.clone(),
            evidence,
            stage: PREPARED_SURFACE_REFRESH_STAGE,
            owner_generation: self.owner_generation,
            attempt_revision,
        })
    }

    pub(super) fn is_current(
        &self,
        ticket: &PreparedSurfaceRefreshTicket,
        evidence: PreparedSurfaceRefreshNativeEvidence,
    ) -> bool {
        self.in_flight_attempt_revision == Some(ticket.attempt_revision)
            && evidence.is_admissible()
            && ticket.evidence == evidence
            && ticket.stage == PREPARED_SURFACE_REFRESH_STAGE
            && ticket.owner_generation == self.owner_generation
            && ticket.frame_schedule_key == self.frame_schedule_key
    }

    pub(super) fn complete(&mut self, ticket: PreparedSurfaceRefreshTicket) -> bool {
        let exact = self.in_flight_attempt_revision == Some(ticket.attempt_revision)
            && ticket.owner_generation == self.owner_generation
            && ticket.frame_schedule_key == self.frame_schedule_key
            && ticket.stage == PREPARED_SURFACE_REFRESH_STAGE;
        if exact {
            self.in_flight_attempt_revision = None;
        }
        exact
    }
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
        let mut owner = PreparedSurfaceRefreshOwner::new(FrameScheduleKey::Primary);
        let evidence = evidence();
        let ticket = owner.begin(evidence).expect("ticket");
        assert_eq!(ticket.evidence, evidence);
        assert_eq!(ticket.stage, PREPARED_SURFACE_REFRESH_STAGE);
        assert_eq!(ticket.owner_generation, 1);
        assert_eq!(ticket.attempt_revision, 1);
        assert!(owner.begin(evidence).is_none());
        assert!(owner.is_current(&ticket, evidence));
        assert!(owner.complete(ticket));
        let next_ticket = owner.begin(evidence).expect("single-consumption release");
        assert_eq!(next_ticket.attempt_revision, 2);
        assert!(owner.complete(next_ticket));
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
        let mut owner = PreparedSurfaceRefreshOwner::new(FrameScheduleKey::Primary);
        let ticket = owner.begin(original).expect("ticket");
        assert!(!owner.is_current(&ticket, changed));
        assert!(owner.complete(ticket));
    }

    #[test]
    fn ticket_stage_owner_generation_and_attempt_revision_are_exact() {
        let mut owner = PreparedSurfaceRefreshOwner::new(FrameScheduleKey::Primary);
        let evidence = evidence();
        let mut ticket = owner.begin(evidence).expect("ticket");
        let owner_generation = ticket.owner_generation;
        let attempt_revision = ticket.attempt_revision;

        ticket.stage = SchedulerStage::Layout;
        assert!(!owner.is_current(&ticket, evidence));
        ticket.stage = PREPARED_SURFACE_REFRESH_STAGE;

        ticket.owner_generation = owner_generation.saturating_add(1);
        assert!(!owner.is_current(&ticket, evidence));
        ticket.owner_generation = owner_generation;

        ticket.attempt_revision = attempt_revision.saturating_add(1);
        assert!(!owner.is_current(&ticket, evidence));
        ticket.attempt_revision = attempt_revision;

        assert!(owner.is_current(&ticket, evidence));
        assert!(owner.complete(ticket));
    }

    #[test]
    fn completion_requires_the_exact_in_flight_attempt() {
        let mut owner = PreparedSurfaceRefreshOwner::new(FrameScheduleKey::Primary);
        let evidence = evidence();
        let mut ticket = owner.begin(evidence).expect("ticket");
        ticket.attempt_revision = ticket.attempt_revision.saturating_add(1);

        assert!(!owner.complete(ticket));
        assert!(owner.in_flight_attempt_revision.is_some());
        assert!(owner.begin(evidence).is_none());
    }
}
