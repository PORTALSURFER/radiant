//! Bounded, per-window timed-frame admission for the native event loop.
//!
//! This module deliberately observes cadence demand without touching a window
//! runner, then returns one stable-key admission for the event-loop owner to
//! apply.  The native redraw and route paths remain the only owners of redraw
//! admission and presentation.

use super::{
    CpuFramePendingRedrawAge, FrameWork, FrameWorkReason, GenericNativeVelloRunner,
    GenericRouteOutcome, SceneRebuildMode, TimedFrameCadence, animation_frame_interval,
    timed_frame_cadence, timed_frame_target_fps,
};
use crate::runtime::{RuntimeAnimationActivity, RuntimeBridge};
use std::time::{Duration, Instant};

/// Stable identity used by the application-level scheduler cursor.
///
/// The scheduler never stores a `Vec` position.  A position is resolved only
/// against the current observation snapshot and the selected key is looked up
/// again immediately before auxiliary mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FrameScheduleKey {
    Primary,
    Auxiliary(String),
}

/// Independent eligibility evidence for one admitted auxiliary window.
///
/// These fields are intentionally explicit so every lifecycle, visibility,
/// native-window, and generation fence remains a visible veto in tests and in
/// the caller that collects live native evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AuxiliaryScheduleEligibility {
    pub(super) active: bool,
    pub(super) admitted: bool,
    pub(super) visible: bool,
    pub(super) local_running: bool,
    pub(super) live_window: bool,
    pub(super) recovering: bool,
    pub(super) closing: bool,
    pub(super) stopped: bool,
    pub(super) native_resources_present: bool,
    pub(super) resource_generation_current: bool,
    pub(super) target_generation_known: bool,
    pub(super) native_surface_target_unfenced: bool,
}

impl AuxiliaryScheduleEligibility {
    pub(super) const fn is_eligible(self) -> bool {
        self.active
            && self.admitted
            && self.visible
            && self.local_running
            && self.live_window
            && !self.recovering
            && !self.closing
            && !self.stopped
            && self.native_resources_present
            && self.resource_generation_current
            && self.target_generation_known
            && self.native_surface_target_unfenced
    }
}

/// The one newly due operation admitted for a selected window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameScheduleWork {
    pub(super) advance_timed_repaint: bool,
    pub(super) drain_timed_frame: bool,
    pub(super) reissue_pending_redraw: bool,
}

impl FrameScheduleWork {
    pub(super) const fn is_empty(self) -> bool {
        !self.advance_timed_repaint && !self.drain_timed_frame && !self.reissue_pending_redraw
    }
}

/// A pure observation of one window's existing cadence and redraw policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameScheduleRedrawEvidence {
    pub(super) timed_repaint_deadline: Option<Instant>,
    pub(super) pending_redraw_requested: bool,
    pub(super) pending_redraw_age: CpuFramePendingRedrawAge,
    pub(super) pending_redraw_retry_deadline: Option<Instant>,
    pub(super) pending_redraw_fresh: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FrameScheduleDemand {
    key: FrameScheduleKey,
    cadence: TimedFrameCadence,
    requested_target_fps: u32,
    frame_target_fps: u32,
    animation_activity: RuntimeAnimationActivity,
    needs_text_caret_animation: bool,
    timed_repaint_deadline: Option<Instant>,
    pending_redraw_requested: bool,
    pending_redraw_age: CpuFramePendingRedrawAge,
    pending_redraw_retry_deadline: Option<Instant>,
    pending_redraw_fresh: bool,
    fallback_interval: Duration,
}

impl FrameScheduleDemand {
    pub(super) fn from_cadence(
        key: FrameScheduleKey,
        cadence: TimedFrameCadence,
        frame_target_fps: u32,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
        redraw: FrameScheduleRedrawEvidence,
    ) -> Self {
        Self::from_cadence_with_requested_target_fps(
            key,
            cadence,
            frame_target_fps,
            frame_target_fps,
            animation_activity,
            needs_text_caret_animation,
            redraw,
        )
    }

    fn from_cadence_with_requested_target_fps(
        key: FrameScheduleKey,
        cadence: TimedFrameCadence,
        requested_target_fps: u32,
        frame_target_fps: u32,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
        redraw: FrameScheduleRedrawEvidence,
    ) -> Self {
        Self {
            key,
            cadence,
            requested_target_fps: crate::gui_runtime::options::normalize_native_target_fps(
                requested_target_fps,
            ),
            frame_target_fps,
            animation_activity,
            needs_text_caret_animation,
            timed_repaint_deadline: redraw.timed_repaint_deadline,
            pending_redraw_requested: redraw.pending_redraw_requested,
            pending_redraw_age: redraw.pending_redraw_age,
            pending_redraw_retry_deadline: redraw.pending_redraw_retry_deadline,
            pending_redraw_fresh: redraw.pending_redraw_fresh,
            fallback_interval: animation_frame_interval(frame_target_fps),
        }
    }

    /// Observe one runtime's complete timed-frame demand using the shared
    /// target-FPS and cadence policy.
    pub(super) fn observe_runtime(
        key: FrameScheduleKey,
        now: Instant,
        last_timed_frame_drain: Instant,
        native_target_fps: u32,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
        redraw: FrameScheduleRedrawEvidence,
    ) -> Self {
        let frame_target_fps = timed_frame_target_fps(
            native_target_fps,
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            last_timed_frame_drain,
            frame_target_fps,
            animation_activity.needs_animation() || needs_text_caret_animation,
        );
        Self::from_cadence_with_requested_target_fps(
            key,
            cadence,
            native_target_fps,
            frame_target_fps,
            animation_activity,
            needs_text_caret_animation,
            redraw,
        )
    }

    pub(super) const fn key(&self) -> &FrameScheduleKey {
        &self.key
    }

    pub(super) const fn cadence(&self) -> TimedFrameCadence {
        self.cadence
    }

    pub(super) const fn frame_target_fps(&self) -> u32 {
        self.frame_target_fps
    }

    pub(super) const fn requested_target_fps(&self) -> u32 {
        self.requested_target_fps
    }

    pub(super) const fn animation_activity(&self) -> RuntimeAnimationActivity {
        self.animation_activity
    }

    pub(super) const fn needs_text_caret_animation(&self) -> bool {
        self.needs_text_caret_animation
    }

    pub(super) const fn pending_redraw_age(&self) -> CpuFramePendingRedrawAge {
        self.pending_redraw_age
    }

    pub(super) const fn pending_redraw_requested(&self) -> bool {
        self.pending_redraw_requested
    }

    pub(super) fn work(&self, now: Instant) -> FrameScheduleWork {
        let advance_timed_repaint = self
            .timed_repaint_deadline
            .is_some_and(|deadline| deadline <= now);
        let drain_timed_frame = matches!(self.cadence, TimedFrameCadence::DrainNow { .. })
            && !self.pending_redraw_fresh;
        let reissue_pending_redraw = self.pending_redraw_requested
            && self
                .pending_redraw_retry_deadline
                .is_some_and(|deadline| deadline <= now)
            && !drain_timed_frame;
        FrameScheduleWork {
            advance_timed_repaint,
            drain_timed_frame,
            reissue_pending_redraw,
        }
    }

    pub(super) fn deadlines(&self, now: Instant) -> FrameScheduleDeadlines {
        let cadence = match self.cadence {
            TimedFrameCadence::Idle => None,
            TimedFrameCadence::WaitUntil(deadline)
            | TimedFrameCadence::DrainNow {
                next_wake: deadline,
                ..
            } => Some(deadline),
        };
        let repaint =
            future_or_opportunity(self.timed_repaint_deadline, now, self.fallback_interval);
        let reissue = self.pending_redraw_requested.then(|| {
            future_or_opportunity(
                self.pending_redraw_retry_deadline,
                now,
                self.fallback_interval,
            )
        });
        FrameScheduleDeadlines {
            cadence,
            repaint,
            reissue: reissue.flatten(),
            ..FrameScheduleDeadlines::default()
        }
    }

    pub(super) fn has_due_work(&self, now: Instant) -> bool {
        !self.work(now).is_empty()
    }
}

fn future_or_opportunity(
    deadline: Option<Instant>,
    now: Instant,
    fallback_interval: Duration,
) -> Option<Instant> {
    match deadline {
        Some(deadline) if deadline > now => Some(deadline),
        Some(_) => Some(now + fallback_interval),
        None => None,
    }
}

/// Deadline sources composed by the parent event-loop owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameScheduleDeadlines {
    pub(super) cadence: Option<Instant>,
    pub(super) repaint: Option<Instant>,
    pub(super) reissue: Option<Instant>,
    pub(super) activation: Option<Instant>,
    pub(super) maintenance: Option<Instant>,
    pub(super) recovery: Option<Instant>,
    pub(super) closing: Option<Instant>,
}

impl FrameScheduleDeadlines {
    pub(super) fn merge(self, other: Self) -> Self {
        Self {
            cadence: earlier(self.cadence, other.cadence),
            repaint: earlier(self.repaint, other.repaint),
            reissue: earlier(self.reissue, other.reissue),
            activation: earlier(self.activation, other.activation),
            maintenance: earlier(self.maintenance, other.maintenance),
            recovery: earlier(self.recovery, other.recovery),
            closing: earlier(self.closing, other.closing),
        }
    }

    pub(super) fn earliest(self) -> Option<Instant> {
        [
            self.cadence,
            self.repaint,
            self.reissue,
            self.activation,
            self.maintenance,
            self.recovery,
            self.closing,
        ]
        .into_iter()
        .flatten()
        .min()
    }
}

fn earlier(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(if left <= right { left } else { right }),
        (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
        (None, None) => None,
    }
}

/// Pure result of one scheduler observation turn.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameSchedulerPlan {
    pub(super) selected: Option<FrameScheduleKey>,
    pub(super) deadlines: FrameScheduleDeadlines,
}

/// Stable-key round-robin cursor for one parent event loop.
#[derive(Default)]
pub(super) struct NativeFrameScheduler {
    last_admitted: Option<FrameScheduleKey>,
}

impl NativeFrameScheduler {
    pub(super) fn observe(
        &self,
        now: Instant,
        demands: &[FrameScheduleDemand],
        external_deadlines: FrameScheduleDeadlines,
    ) -> FrameSchedulerPlan {
        let mut deadlines = external_deadlines;
        for demand in demands {
            deadlines = deadlines.merge(demand.deadlines(now));
        }
        FrameSchedulerPlan {
            selected: self.select_due(now, demands),
            deadlines,
        }
    }

    pub(super) fn record_admission(&mut self, key: FrameScheduleKey) {
        self.last_admitted = Some(key);
    }

    fn select_due(
        &self,
        now: Instant,
        demands: &[FrameScheduleDemand],
    ) -> Option<FrameScheduleKey> {
        if demands.is_empty() {
            return None;
        }
        let start = self
            .last_admitted
            .as_ref()
            .and_then(|last| demands.iter().position(|demand| demand.key() == last))
            .map_or(0, |index| (index + 1) % demands.len());
        (0..demands.len())
            .map(|offset| (start + offset) % demands.len())
            .map(|index| &demands[index])
            .find(|demand| demand.has_due_work(now))
            .map(|demand| demand.key.clone())
    }
}

/// Route/redraw work applied to one selected native runner.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct FrameScheduleAdmission {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) route_outcome: bool,
    pub(super) did_work: bool,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Apply one already-selected window's existing timed-frame policy.
    ///
    /// This function only advances runtime-local state and requests the normal
    /// redraw path. It does not call the renderer, touch native resources, or
    /// alter adapter/generation ownership.
    pub(super) fn admit_frame_schedule_work(
        &mut self,
        now: Instant,
        demand: &FrameScheduleDemand,
    ) -> FrameScheduleAdmission {
        let work = demand.work(now);
        if work.is_empty() || !self.is_running() {
            return FrameScheduleAdmission::default();
        }
        if work.advance_timed_repaint && self.core.advance_timed_repaints(now) {
            self.rebuild_scene();
            self.request_redraw_for_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            });
        }
        if work.reissue_pending_redraw {
            self.request_redraw_for_frame_work(FrameWork::None);
        }
        let should_route =
            work.drain_timed_frame && !self.should_defer_timed_frame_drain_for_pending_redraw(now);
        let outcome = if should_route {
            self.drain_timed_frame_now(
                now,
                demand.animation_activity(),
                demand.needs_text_caret_animation(),
            )
        } else {
            GenericRouteOutcome::default()
        };
        FrameScheduleAdmission {
            outcome,
            route_outcome: should_route,
            did_work: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeAnimationActivity;

    fn due_demand(key: FrameScheduleKey, now: Instant) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence::default(),
        )
    }

    fn eligible() -> AuxiliaryScheduleEligibility {
        AuxiliaryScheduleEligibility {
            active: true,
            admitted: true,
            visible: true,
            local_running: true,
            live_window: true,
            recovering: false,
            closing: false,
            stopped: false,
            native_resources_present: true,
            resource_generation_current: true,
            target_generation_known: true,
            native_surface_target_unfenced: true,
        }
    }

    #[test]
    fn every_auxiliary_eligibility_veto_is_dormant() {
        fn veto_active(state: &mut AuxiliaryScheduleEligibility) {
            state.active = false;
        }
        fn veto_admitted(state: &mut AuxiliaryScheduleEligibility) {
            state.admitted = false;
        }
        fn veto_visible(state: &mut AuxiliaryScheduleEligibility) {
            state.visible = false;
        }
        fn veto_local_running(state: &mut AuxiliaryScheduleEligibility) {
            state.local_running = false;
        }
        fn veto_live_window(state: &mut AuxiliaryScheduleEligibility) {
            state.live_window = false;
        }
        fn veto_recovering(state: &mut AuxiliaryScheduleEligibility) {
            state.recovering = true;
        }
        fn veto_closing(state: &mut AuxiliaryScheduleEligibility) {
            state.closing = true;
        }
        fn veto_stopped(state: &mut AuxiliaryScheduleEligibility) {
            state.stopped = true;
        }
        fn veto_resources(state: &mut AuxiliaryScheduleEligibility) {
            state.native_resources_present = false;
        }
        fn veto_generation(state: &mut AuxiliaryScheduleEligibility) {
            state.resource_generation_current = false;
        }
        fn veto_target_generation(state: &mut AuxiliaryScheduleEligibility) {
            state.target_generation_known = false;
        }
        fn veto_target_fence(state: &mut AuxiliaryScheduleEligibility) {
            state.native_surface_target_unfenced = false;
        }

        let vetoes: [fn(&mut AuxiliaryScheduleEligibility); 12] = [
            veto_active,
            veto_admitted,
            veto_visible,
            veto_local_running,
            veto_live_window,
            veto_recovering,
            veto_closing,
            veto_stopped,
            veto_resources,
            veto_generation,
            veto_target_generation,
            veto_target_fence,
        ];

        for veto in vetoes {
            let mut state = eligible();
            veto(&mut state);
            assert!(!state.is_eligible());
        }
        assert!(eligible().is_eligible());
    }

    #[test]
    fn due_auxiliary_is_selected_when_primary_is_idle() {
        let now = Instant::now();
        let demands = [due_demand(
            FrameScheduleKey::Auxiliary("settings".into()),
            now,
        )];
        let plan = NativeFrameScheduler::default().observe(
            now,
            &demands,
            FrameScheduleDeadlines::default(),
        );

        assert_eq!(
            plan.selected,
            Some(FrameScheduleKey::Auxiliary("settings".into()))
        );
    }

    #[test]
    fn focused_auxiliary_caret_uses_shared_cadence_policy() {
        let now = Instant::now();
        let demand = FrameScheduleDemand::observe_runtime(
            FrameScheduleKey::Auxiliary("editor".into()),
            now,
            now - animation_frame_interval(30),
            120,
            RuntimeAnimationActivity::idle(),
            true,
            FrameScheduleRedrawEvidence::default(),
        );

        assert_eq!(demand.frame_target_fps(), 30);
        assert!(matches!(
            demand.cadence(),
            TimedFrameCadence::DrainNow { .. }
        ));
        assert!(demand.work(now).drain_timed_frame);
    }

    #[test]
    fn one_observation_turn_selects_at_most_one_window() {
        let now = Instant::now();
        let demands = [
            due_demand(FrameScheduleKey::Primary, now),
            due_demand(FrameScheduleKey::Auxiliary("settings".into()), now),
            due_demand(FrameScheduleKey::Auxiliary("inspector".into()), now),
        ];
        let plan = NativeFrameScheduler::default().observe(
            now,
            &demands,
            FrameScheduleDeadlines::default(),
        );

        assert!(plan.selected.is_some());
        assert_eq!(
            demands
                .iter()
                .filter(|demand| demand.has_due_work(now))
                .count(),
            3
        );
    }

    #[test]
    fn stable_round_robin_prevents_continuously_due_primary_starvation() {
        let now = Instant::now();
        let primary = due_demand(FrameScheduleKey::Primary, now);
        let auxiliary = due_demand(FrameScheduleKey::Auxiliary("settings".into()), now);
        let demands = [primary.clone(), auxiliary.clone()];
        let mut scheduler = NativeFrameScheduler::default();

        let first = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        scheduler.record_admission(first.selected.clone().unwrap());
        let second = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        scheduler.record_admission(second.selected.clone().unwrap());
        let third = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());

        assert_eq!(first.selected, Some(FrameScheduleKey::Primary));
        assert_eq!(
            second.selected,
            Some(FrameScheduleKey::Auxiliary("settings".into()))
        );
        assert_eq!(third.selected, Some(FrameScheduleKey::Primary));
    }

    #[test]
    fn earliest_deadline_uses_minimum_composition() {
        let now = Instant::now();
        let earliest = now + Duration::from_millis(2);
        let deadlines = FrameScheduleDeadlines {
            cadence: Some(now + Duration::from_millis(8)),
            repaint: Some(now + Duration::from_millis(5)),
            reissue: Some(now + Duration::from_millis(4)),
            activation: Some(now + Duration::from_millis(7)),
            maintenance: Some(now + Duration::from_millis(3)),
            recovery: Some(now + Duration::from_millis(6)),
            closing: Some(earliest),
        };

        assert_eq!(deadlines.earliest(), Some(earliest));
    }

    #[test]
    fn due_deadlines_get_a_future_opportunity_without_busy_looping() {
        let now = Instant::now();
        let demand = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Auxiliary("settings".into()),
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::idle(),
            false,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(now - Duration::from_millis(1)),
                ..FrameScheduleRedrawEvidence::default()
            },
        );
        let plan = NativeFrameScheduler::default().observe(
            now,
            &[demand],
            FrameScheduleDeadlines::default(),
        );

        assert!(
            plan.deadlines
                .earliest()
                .is_some_and(|deadline| deadline > now)
        );
    }

    #[test]
    fn fresh_pending_redraw_defers_cadence_and_stale_pending_redraw_reissues() {
        let now = Instant::now();
        let fresh = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Auxiliary("settings".into()),
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence {
                pending_redraw_requested: true,
                pending_redraw_retry_deadline: Some(now + Duration::from_millis(16)),
                pending_redraw_fresh: true,
                ..FrameScheduleRedrawEvidence::default()
            },
        );
        assert!(!fresh.work(now).drain_timed_frame);

        let stale = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Auxiliary("settings".into()),
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::idle(),
            false,
            FrameScheduleRedrawEvidence {
                pending_redraw_requested: true,
                pending_redraw_retry_deadline: Some(now - Duration::from_millis(1)),
                ..FrameScheduleRedrawEvidence::default()
            },
        );
        assert!(stale.work(now).reissue_pending_redraw);
    }

    #[test]
    fn key_cursor_survives_insertion_removal_hide_and_compaction() {
        let now = Instant::now();
        let primary = due_demand(FrameScheduleKey::Primary, now);
        let first = due_demand(FrameScheduleKey::Auxiliary("first".into()), now);
        let second = due_demand(FrameScheduleKey::Auxiliary("second".into()), now);
        let inserted = due_demand(FrameScheduleKey::Auxiliary("inserted".into()), now);
        let mut scheduler = NativeFrameScheduler::default();

        let initial = scheduler.observe(
            now,
            &[primary.clone(), first.clone(), second.clone()],
            FrameScheduleDeadlines::default(),
        );
        assert_eq!(
            initial.selected,
            Some(FrameScheduleKey::Primary),
            "the initial primary choice preserves existing behavior"
        );
        scheduler.record_admission(initial.selected.unwrap());

        let after_insertion = scheduler.observe(
            now,
            &[inserted, primary.clone(), first.clone(), second.clone()],
            FrameScheduleDeadlines::default(),
        );
        assert_eq!(
            after_insertion.selected,
            Some(FrameScheduleKey::Auxiliary("first".into()))
        );
        scheduler.record_admission(after_insertion.selected.unwrap());

        let after_removal = scheduler.observe(
            now,
            &[primary.clone(), second.clone()],
            FrameScheduleDeadlines::default(),
        );
        assert_eq!(
            after_removal.selected,
            Some(FrameScheduleKey::Primary),
            "removing the remembered key must not retarget its old vector slot"
        );
        scheduler.record_admission(after_removal.selected.unwrap());

        let after_hide_and_compaction =
            scheduler.observe(now, &[second], FrameScheduleDeadlines::default());
        assert_eq!(
            after_hide_and_compaction.selected,
            Some(FrameScheduleKey::Auxiliary("second".into()))
        );
    }
}
