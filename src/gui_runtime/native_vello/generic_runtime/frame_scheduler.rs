//! Bounded, per-window timed-frame admission for the native event loop.
//!
//! This module deliberately observes cadence demand without touching a window
//! runner, then returns one stable-key admission for the event-loop owner to
//! apply.  The native redraw and route paths remain the only owners of redraw
//! admission and presentation.

use super::frame_scheduler_policy::{
    SchedulerDemand, SchedulerFairnessEligibility, SchedulerFairnessLedger, SchedulerStage,
    SchedulerWorkClass,
};
use super::frame_stage_admission::{FrameStageIdentity, NextReadyStage, TimedFrame};
use super::runner_state::NativeTargetGeneration;
use super::{
    CpuFramePendingRedrawAge, FrameWork, FrameWorkReason, GenericNativeVelloRunner,
    GenericRouteOutcome, NativeAdapterGeneration, SceneRebuildMode, TimedFrameCadence,
    animation_frame_interval, timed_frame_cadence, timed_frame_target_fps,
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
/// These fields are intentionally explicit so every lifecycle, native-window,
/// and generation fence remains a visible veto in tests and in the caller that
/// collects live native evidence. Host-reported window visibility is not part
/// of logical presentation eligibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AuxiliaryScheduleEligibility {
    pub(super) active: bool,
    pub(super) admitted: bool,
    pub(super) local_running: bool,
    pub(super) live_window: bool,
    pub(super) recovering: bool,
    pub(super) closing: bool,
    pub(super) stopped: bool,
    pub(super) native_resources_present: bool,
    pub(super) resource_generation_current: bool,
    pub(super) mailbox_suspended: bool,
    pub(super) target_generation_known: bool,
    pub(super) native_surface_target_unfenced: bool,
}

impl AuxiliaryScheduleEligibility {
    pub(super) const fn is_eligible(self) -> bool {
        self.active
            && self.admitted
            && self.local_running
            && self.live_window
            && !self.recovering
            && !self.closing
            && !self.stopped
            && self.native_resources_present
            && self.resource_generation_current
            && !self.mailbox_suspended
            && self.target_generation_known
            && self.native_surface_target_unfenced
    }

    pub(super) const fn is_recovery_eligible(self) -> bool {
        self.active
            && self.admitted
            && self.local_running
            && self.live_window
            && !self.recovering
            && !self.closing
            && !self.stopped
            && self.native_resources_present
            && self.resource_generation_current
            && !self.mailbox_suspended
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
    #[cfg(test)]
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

    pub(super) fn from_cadence_with_requested_target_fps(
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
    fairness: SchedulerFairnessLedger,
}

impl NativeFrameScheduler {
    pub(super) fn observe(
        &mut self,
        now: Instant,
        demands: &[FrameScheduleDemand],
        external_deadlines: FrameScheduleDeadlines,
    ) -> FrameSchedulerPlan {
        let mut deadlines = external_deadlines;
        for demand in demands {
            deadlines = deadlines.merge(demand.deadlines(now));
        }
        let policy_demands = demands
            .iter()
            .map(|demand| scheduler_policy_demand(demand, now))
            .collect::<Vec<_>>();
        self.fairness.remove_absent(&policy_demands);
        let has_eligible_demand = policy_demands
            .iter()
            .any(|demand| demand.eligibility().is_fairness_eligible());
        if has_eligible_demand
            && policy_demands
                .iter()
                .filter(|demand| demand.eligibility().is_fairness_eligible())
                .all(|demand| !self.fairness.can_admit(demand))
        {
            self.fairness.complete_epoch(&policy_demands);
        }
        FrameSchedulerPlan {
            selected: self
                .fairness
                .select_candidate(&policy_demands, self.last_admitted.as_ref()),
            deadlines,
        }
    }

    pub(super) fn record_admission(&mut self, key: FrameScheduleKey) {
        self.fairness.record_admission(&key);
        self.last_admitted = Some(key);
    }
}

fn scheduler_policy_demand(demand: &FrameScheduleDemand, now: Instant) -> SchedulerDemand {
    let work = demand.work(now);
    let class = if work.drain_timed_frame {
        SchedulerWorkClass::Deadline
    } else if work.advance_timed_repaint || work.reissue_pending_redraw {
        SchedulerWorkClass::Animation
    } else {
        SchedulerWorkClass::Transient
    };
    let deadline = if work.drain_timed_frame {
        match demand.cadence() {
            TimedFrameCadence::DrainNow { due_at, .. } => due_at,
            TimedFrameCadence::Idle | TimedFrameCadence::WaitUntil(_) => now,
        }
    } else if work.advance_timed_repaint {
        demand.timed_repaint_deadline.unwrap_or(now)
    } else if work.reissue_pending_redraw {
        demand.pending_redraw_retry_deadline.unwrap_or(now)
    } else {
        now
    };
    SchedulerDemand::new(
        demand.key.clone(),
        class,
        deadline,
        SchedulerFairnessEligibility {
            due_work: !work.is_empty(),
            passes_lifecycle_native_generation_fences: true,
            ..SchedulerFairnessEligibility::default()
        },
    )
}

/// Route/redraw work applied to one selected native runner.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct FrameScheduleAdmission {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) route_outcome: bool,
    pub(super) did_work: bool,
    pub(super) timed_frame_already_handled: bool,
}

struct DeadlineBundleExecution {
    admission: FrameScheduleAdmission,
    drain_deferred: bool,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn execute_timed_frame_drain(&mut self, bundle: TimedFrame) -> FrameScheduleAdmission {
        let actual_drain_at = Instant::now();
        let outcome = self.drain_timed_frame_now(
            actual_drain_at,
            bundle.animation_activity(),
            bundle.needs_text_caret_animation(),
        );
        FrameScheduleAdmission {
            outcome,
            route_outcome: true,
            did_work: true,
            timed_frame_already_handled: true,
        }
    }

    /// Execute the operations selected by one deadline demand in their
    /// existing order.  Both exact-owner execution and compatibility fallback
    /// use this helper so repaint advancement, redraw requests, deferral, and
    /// timed-frame drainage cannot diverge.
    fn execute_deadline_bundle(
        &mut self,
        now: Instant,
        bundle: TimedFrame,
    ) -> DeadlineBundleExecution {
        let repaint_advanced =
            bundle.advance_timed_repaint() && self.core.advance_timed_repaints(now);
        if repaint_advanced {
            self.rebuild_scene();
            self.request_redraw_for_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            });
        }

        if bundle.reissue_pending_redraw() {
            self.request_redraw_for_frame_work(FrameWork::None);
        }

        if !bundle.drain_timed_frame() {
            return DeadlineBundleExecution {
                admission: FrameScheduleAdmission {
                    outcome: GenericRouteOutcome::default(),
                    route_outcome: false,
                    did_work: true,
                    timed_frame_already_handled: false,
                },
                drain_deferred: false,
            };
        }

        let drain_deferred =
            repaint_advanced && self.should_defer_timed_frame_drain_for_pending_redraw(now);
        let admission = if drain_deferred {
            FrameScheduleAdmission {
                outcome: GenericRouteOutcome::default(),
                route_outcome: false,
                did_work: true,
                timed_frame_already_handled: true,
            }
        } else {
            self.execute_timed_frame_drain(bundle)
        };
        DeadlineBundleExecution {
            admission,
            drain_deferred,
        }
    }

    fn admit_deferred_timed_frame_deadline(
        &mut self,
        now: Instant,
        key: &FrameScheduleKey,
        adapter_generation: Option<NativeAdapterGeneration>,
        target_generation: NativeTargetGeneration,
    ) -> FrameScheduleAdmission {
        if !self.frame_stage_owner.has_deferred_timed_frame()
            || !self.frame_stage_owner.owns_key(key)
        {
            return FrameScheduleAdmission::default();
        }
        if self.should_defer_timed_frame_drain_for_pending_redraw(now) {
            return FrameScheduleAdmission::default();
        }
        let Some(adapter_generation) = adapter_generation else {
            return FrameScheduleAdmission {
                did_work: true,
                ..FrameScheduleAdmission::default()
            };
        };
        let Some(payload) = self.frame_stage_owner.resume_deferred_timed_frame(
            key,
            adapter_generation,
            target_generation,
        ) else {
            return FrameScheduleAdmission {
                did_work: true,
                ..FrameScheduleAdmission::default()
            };
        };
        let admission = self.execute_timed_frame_drain(payload);
        if !self
            .frame_stage_owner
            .complete_deferred_timed_frame(Instant::now())
        {
            // The retained drain already ran. A completion mismatch must not
            // replay it or invoke any fallback path.
            return admission;
        }
        admission
    }

    fn admit_timed_frame_deadline(
        &mut self,
        now: Instant,
        key: FrameScheduleKey,
        adapter_generation: Option<NativeAdapterGeneration>,
        target_generation: NativeTargetGeneration,
        selected_payload: TimedFrame,
    ) -> FrameScheduleAdmission {
        if !self.frame_stage_owner.owns_key(&key) {
            return FrameScheduleAdmission::default();
        }
        let pre_begin_fallback = |runner: &mut Self| {
            if runner.frame_stage_owner.has_in_flight() {
                return FrameScheduleAdmission {
                    did_work: true,
                    ..FrameScheduleAdmission::default()
                };
            }
            runner.frame_stage_owner.invalidate();
            if selected_payload.reissue_pending_redraw() || !selected_payload.drain_timed_frame() {
                return FrameScheduleAdmission::default();
            }
            runner
                .execute_deadline_bundle(now, selected_payload)
                .admission
        };
        let Some(adapter_generation) = adapter_generation else {
            return pre_begin_fallback(self);
        };
        if !target_generation.is_known()
            || !self.frame_stage_owner.prepare_fence(
                adapter_generation,
                target_generation,
                SchedulerStage::Deadline,
            )
        {
            return pre_begin_fallback(self);
        }
        let Some(revision) = self.frame_stage_owner.next_revision() else {
            return pre_begin_fallback(self);
        };
        let identity = FrameStageIdentity::new(
            key,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            self.frame_stage_owner.owner_generation(),
            revision,
        );
        if self.frame_stage_owner.stale(&identity) {
            return pre_begin_fallback(self);
        }
        if !self
            .frame_stage_owner
            .queue(identity.clone(), selected_payload)
        {
            return pre_begin_fallback(self);
        }
        if self.frame_stage_owner.next_ready_stage(now) != NextReadyStage::Deadline {
            return pre_begin_fallback(self);
        }
        let Some(payload) = self.frame_stage_owner.begin(&identity, now) else {
            return pre_begin_fallback(self);
        };
        let started_at = Instant::now();
        let execution = self.execute_deadline_bundle(now, payload);
        if execution.drain_deferred {
            let _ = self.frame_stage_owner.defer_timed_frame_drain(&identity);
            return execution.admission;
        }
        let completed_at = Instant::now();
        if !self
            .frame_stage_owner
            .complete(&identity, started_at, completed_at)
        {
            // The bundle already ran after begin. A failed completion must not
            // fall back to a second repaint advance, redraw request, or drain.
            return execution.admission;
        }
        execution.admission
    }

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
        let adapter_generation = self
            .adapter
            .as_ref()
            .and_then(super::GenericNativeAdapterOwner::capture_generation);
        self.admit_frame_schedule_work_with_generation(now, demand, adapter_generation)
    }

    pub(super) fn admit_auxiliary_frame_schedule_work(
        &mut self,
        now: Instant,
        demand: &FrameScheduleDemand,
        adapter_generation: NativeAdapterGeneration,
    ) -> FrameScheduleAdmission {
        self.admit_frame_schedule_work_with_generation(now, demand, Some(adapter_generation))
    }

    fn admit_frame_schedule_work_with_generation(
        &mut self,
        now: Instant,
        demand: &FrameScheduleDemand,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> FrameScheduleAdmission {
        if !self.is_running() {
            self.frame_stage_owner.invalidate();
            return FrameScheduleAdmission::default();
        }
        let work = demand.work(now);
        self.require_primary_frame_diagnostics_schedule_admission();
        if self.frame_stage_owner.has_deferred_timed_frame()
            && self.frame_stage_owner.owns_key(demand.key())
        {
            return self.admit_deferred_timed_frame_deadline(
                now,
                demand.key(),
                adapter_generation,
                self.window.target_generation,
            );
        }
        if work.is_empty() {
            return FrameScheduleAdmission::default();
        }
        let Some(selected_payload) = deadline_payload(demand, work) else {
            return FrameScheduleAdmission::default();
        };
        self.admit_timed_frame_deadline(
            now,
            demand.key().clone(),
            adapter_generation,
            self.window.target_generation,
            selected_payload,
        )
    }
}

fn deadline_payload(demand: &FrameScheduleDemand, work: FrameScheduleWork) -> Option<TimedFrame> {
    let due_at = if work.drain_timed_frame {
        match demand.cadence() {
            TimedFrameCadence::DrainNow { due_at, .. } => due_at,
            TimedFrameCadence::Idle | TimedFrameCadence::WaitUntil(_) => return None,
        }
    } else if work.advance_timed_repaint {
        demand.timed_repaint_deadline?
    } else if work.reissue_pending_redraw {
        demand.pending_redraw_retry_deadline?
    } else {
        return None;
    };
    if work.advance_timed_repaint && !work.drain_timed_frame && !work.reissue_pending_redraw {
        return Some(TimedFrame::timed_repaint(due_at));
    }
    if work.reissue_pending_redraw && !work.advance_timed_repaint && !work.drain_timed_frame {
        return Some(TimedFrame::pending_redraw_reissue(due_at));
    }
    let has_timed_frame_drain = work.drain_timed_frame;
    Some(TimedFrame::with_operations(
        due_at,
        if has_timed_frame_drain {
            demand.animation_activity()
        } else {
            RuntimeAnimationActivity::idle()
        },
        has_timed_frame_drain && demand.needs_text_caret_animation(),
        work.advance_timed_repaint,
        work.drain_timed_frame,
        work.reissue_pending_redraw,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{
        DeviceLossRegistration, GenericNativeAdapterOwner, NativeAdapterGeneration,
    };
    use super::*;
    use crate::{
        application::empty,
        gui::types::{Rect, Vector2},
        gui_runtime::NativeRunOptions,
        layout::LayoutOutput,
        prelude::IntoView,
        runtime::{
            PaintPrimitive, RuntimeAnimationHost, RuntimeHostCapabilities, SurfaceNode, UiSurface,
        },
        theme::ThemeTokens,
        widgets::{
            DragHandleWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
        },
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

    #[derive(Default)]
    struct CountingFrameBridge {
        queue_calls: Cell<usize>,
    }

    impl RuntimeBridge<()> for CountingFrameBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(empty::<()>().into_surface())
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new().with_animation()
        }
    }

    impl RuntimeAnimationHost for CountingFrameBridge {
        fn animation_activity(&mut self) -> RuntimeAnimationActivity {
            RuntimeAnimationActivity::frame_messages()
        }

        fn queue_animation_frame(&mut self) -> bool {
            self.queue_calls.set(self.queue_calls.get() + 1);
            true
        }
    }

    fn due_demand(key: FrameScheduleKey, now: Instant) -> FrameScheduleDemand {
        due_demand_with_cadence(
            key,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
        )
    }

    fn due_demand_with_cadence(
        key: FrameScheduleKey,
        cadence: TimedFrameCadence,
    ) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            cadence,
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence::default(),
        )
    }

    fn mixed_due_demand(key: FrameScheduleKey, now: Instant) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(1),
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            false,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(now - Duration::from_millis(1)),
                ..FrameScheduleRedrawEvidence::default()
            },
        )
    }

    fn mixed_live_demand(key: FrameScheduleKey, due_at: Instant) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::DrainNow {
                due_at,
                next_wake: due_at + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            false,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(due_at),
                ..FrameScheduleRedrawEvidence::default()
            },
        )
    }

    fn pure_timed_repaint_demand(
        key: FrameScheduleKey,
        timed_repaint_deadline: Instant,
    ) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::Idle,
            60,
            // These fields deliberately carry animation/caret evidence to
            // prove that pure repaint ownership does not deliver either one.
            RuntimeAnimationActivity::frame_messages(),
            true,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(timed_repaint_deadline),
                ..FrameScheduleRedrawEvidence::default()
            },
        )
    }

    fn pure_pending_redraw_reissue_demand(
        key: FrameScheduleKey,
        pending_redraw_retry_deadline: Instant,
    ) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::Idle,
            60,
            // These fields deliberately carry animation/caret evidence to
            // prove that pure reissue does not deliver either one.
            RuntimeAnimationActivity::frame_messages(),
            true,
            FrameScheduleRedrawEvidence {
                pending_redraw_requested: true,
                pending_redraw_retry_deadline: Some(pending_redraw_retry_deadline),
                pending_redraw_fresh: false,
                ..FrameScheduleRedrawEvidence::default()
            },
        )
    }

    fn mixed_repaint_and_reissue_demand(
        key: FrameScheduleKey,
        timed_repaint_deadline: Instant,
        pending_redraw_retry_deadline: Instant,
    ) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::frame_messages(),
            true,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(timed_repaint_deadline),
                pending_redraw_requested: true,
                pending_redraw_retry_deadline: Some(pending_redraw_retry_deadline),
                pending_redraw_fresh: false,
                ..FrameScheduleRedrawEvidence::default()
            },
        )
    }

    fn retained_drain_demand(
        key: FrameScheduleKey,
        due_at: Instant,
        now: Instant,
    ) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            TimedFrameCadence::DrainNow {
                due_at,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            false,
            FrameScheduleRedrawEvidence::default(),
        )
    }

    #[derive(Clone)]
    struct CountingTimedRepaintWidget {
        inner: DragHandleWidget,
        advance_calls: Rc<Cell<usize>>,
    }

    impl CountingTimedRepaintWidget {
        fn new(advance_calls: Rc<Cell<usize>>) -> Self {
            Self {
                inner: DragHandleWidget::new(70, WidgetSizing::fixed(Vector2::new(8.0, 40.0)))
                    .with_hover_chrome_only()
                    .with_trailing_rail(1.0),
                advance_calls,
            }
        }
    }

    impl Widget for CountingTimedRepaintWidget {
        fn common(&self) -> &WidgetCommon {
            Widget::common(&self.inner)
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            Widget::common_mut(&mut self.inner)
        }

        fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
            Widget::handle_input(&mut self.inner, bounds, input)
        }

        fn append_paint(
            &self,
            primitives: &mut Vec<PaintPrimitive>,
            bounds: Rect,
            layout: &LayoutOutput,
            theme: &ThemeTokens,
        ) {
            Widget::append_paint(&self.inner, primitives, bounds, layout, theme);
        }

        fn timed_repaint_deadline(&self) -> Option<Instant> {
            Widget::timed_repaint_deadline(&self.inner)
        }

        fn advance_timed_repaint(&mut self, now: Instant) -> bool {
            self.advance_calls.set(self.advance_calls.get() + 1);
            Widget::advance_timed_repaint(&mut self.inner, now)
        }
    }

    struct DelayedRepaintBridge {
        advance_calls: Rc<Cell<usize>>,
        queue_calls: Cell<usize>,
    }

    impl DelayedRepaintBridge {
        fn new(advance_calls: Rc<Cell<usize>>) -> Self {
            Self {
                advance_calls,
                queue_calls: Cell::new(0),
            }
        }
    }

    impl crate::runtime::RuntimeBridge<()> for DelayedRepaintBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                CountingTimedRepaintWidget::new(Rc::clone(&self.advance_calls)),
                crate::runtime::WidgetMessageMapper::none(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new()
                .with_animation()
                .with_frame_diagnostics()
        }
    }

    impl RuntimeAnimationHost for DelayedRepaintBridge {
        fn animation_activity(&mut self) -> RuntimeAnimationActivity {
            RuntimeAnimationActivity::frame_messages()
        }

        fn queue_animation_frame(&mut self) -> bool {
            self.queue_calls.set(self.queue_calls.get() + 1);
            true
        }
    }

    fn arm_delayed_repaint<Bridge>(runner: &mut GenericNativeVelloRunner<Bridge, ()>) -> Instant
    where
        Bridge: crate::runtime::RuntimeBridge<()>,
    {
        let bounds = runner
            .core
            .runtime
            .layout()
            .rects
            .get(&70)
            .copied()
            .expect("delayed repaint widget should be laid out");
        assert!(
            runner
                .core
                .route_pointer_move(bounds.center())
                .needs_scene_rebuild()
        );
        runner
            .core
            .timed_repaint_deadline()
            .expect("hover should schedule a finite repaint")
    }

    impl crate::runtime::RuntimeFrameDiagnosticsHost for DelayedRepaintBridge {
        fn observe_frame_diagnostics(
            &mut self,
            _diagnostics: crate::runtime::NativeFrameDiagnostics,
        ) {
        }
    }

    fn auxiliary_runner() -> GenericNativeVelloRunner<CountingFrameBridge, ()> {
        GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
            String::from("settings"),
        )
    }

    #[test]
    fn selected_primary_demand_uses_exact_owner_payload_and_does_not_merge_again() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
        );
        runner.window.target_generation.advance();
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::from_test_serial(1),
            Arc::new(DeviceLossRegistration::new()),
        ));

        let scheduler_now = Instant::now() - Duration::from_millis(50);
        let due_at = scheduler_now - Duration::from_millis(50);
        let demand = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Primary,
            TimedFrameCadence::DrainNow {
                due_at,
                next_wake: scheduler_now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            true,
            FrameScheduleRedrawEvidence::default(),
        );

        let admission = runner.admit_frame_schedule_work(scheduler_now, &demand);
        assert!(admission.did_work);
        assert!(admission.route_outcome);
        let drained_at = runner.timing.last_timed_frame_drain;
        assert!(
            drained_at > scheduler_now,
            "the drain must use the actual execution timestamp rather than scheduler observation"
        );
        assert!(matches!(
            admission.outcome.frame_work(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::TextCaretAnimation,
                ..
            }
        ));
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "the owner payload must cause one frame-message drain"
        );

        runner.apply_route_outcome_with_timed_frame(admission.outcome, false);
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "the selected scheduled outcome must not invoke route-time merge"
        );
        assert_eq!(runner.timing.last_timed_frame_drain, drained_at);
    }

    #[test]
    fn selected_primary_pure_timed_repaint_uses_exact_owner_without_frame_drain() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let admission = runner.admit_frame_schedule_work(
            now,
            &pure_timed_repaint_demand(FrameScheduleKey::Primary, due_at),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(!admission.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.core.timed_repaint_deadline(), None);
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("pure repaint completion should remain owner evidence");
        assert!(completed.advance_timed_repaint());
        assert!(!completed.drain_timed_frame());
        assert!(!completed.animation_activity().needs_frame_message());
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn selected_primary_pure_pending_redraw_reissue_uses_exact_owner_without_frame_work() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let now = Instant::now();
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now - Duration::from_millis(50));
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let admission = runner.admit_frame_schedule_work(
            now,
            &pure_pending_redraw_reissue_demand(FrameScheduleKey::Primary, now),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(!admission.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 0);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("pure reissue completion should remain owner evidence");
        assert!(!completed.advance_timed_repaint());
        assert!(!completed.drain_timed_frame());
        assert!(completed.reissue_pending_redraw());
        assert_eq!(
            completed.animation_activity(),
            RuntimeAnimationActivity::idle()
        );
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn selected_auxiliary_pure_timed_repaint_uses_exact_owner_without_frame_drain() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);

        let admission = runner.admit_auxiliary_frame_schedule_work(
            now,
            &pure_timed_repaint_demand(key, due_at),
            adapter_generation,
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(!admission.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("auxiliary pure repaint completion should remain owner evidence");
        assert!(completed.advance_timed_repaint());
        assert!(!completed.drain_timed_frame());
        assert!(!completed.animation_activity().needs_frame_message());
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn selected_auxiliary_pure_pending_redraw_reissue_uses_exact_owner_without_frame_work() {
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let now = Instant::now();
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now - Duration::from_millis(50));
        runner.window.target_generation.advance();
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));

        let admission = runner.admit_auxiliary_frame_schedule_work(
            now,
            &pure_pending_redraw_reissue_demand(key, now),
            NativeAdapterGeneration::from_test_serial(1),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(!admission.timed_frame_already_handled);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("auxiliary pure reissue completion should remain owner evidence");
        assert!(completed.reissue_pending_redraw());
        assert!(!completed.advance_timed_repaint());
        assert!(!completed.drain_timed_frame());
        assert_eq!(
            completed.animation_activity(),
            RuntimeAnimationActivity::idle()
        );
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn pure_timed_repaint_adapter_and_target_vetoes_do_not_release_payload() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let demand = pure_timed_repaint_demand(FrameScheduleKey::Primary, due_at);
        runner.window.target_generation.advance();

        let missing_adapter = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(missing_adapter, FrameScheduleAdmission::default());
        assert_eq!(repaint_advance_calls.get(), 0);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::unknown(),
            Arc::new(DeviceLossRegistration::new()),
        ));
        let unknown_adapter = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(unknown_adapter, FrameScheduleAdmission::default());
        assert_eq!(repaint_advance_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::from_test_serial(1),
            Arc::new(DeviceLossRegistration::new()),
        ));
        runner.window.target_generation.invalidate_unknown();
        let unknown_target = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(unknown_target, FrameScheduleAdmission::default());
        assert_eq!(repaint_advance_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
    }

    #[test]
    fn pure_pending_redraw_reissue_adapter_and_target_vetoes_preserve_evidence() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
        );
        let now = Instant::now();
        let demand = pure_pending_redraw_reissue_demand(FrameScheduleKey::Primary, now);
        runner.window.target_generation.advance();

        let missing_adapter = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(missing_adapter, FrameScheduleAdmission::default());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::unknown(),
            Arc::new(DeviceLossRegistration::new()),
        ));
        let unknown_adapter = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(unknown_adapter, FrameScheduleAdmission::default());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::from_test_serial(1),
            Arc::new(DeviceLossRegistration::new()),
        ));
        runner.window.target_generation.invalidate_unknown();
        let unknown_target = runner.admit_frame_schedule_work(now, &demand);
        assert_eq!(unknown_target, FrameScheduleAdmission::default());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
    }

    #[test]
    fn lifecycle_recovery_veto_cancels_retained_owner_without_release() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
        );
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let target_generation = runner.window.target_generation;
        assert!(runner.frame_stage_owner.prepare_fence(
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
        ));
        let revision = runner
            .frame_stage_owner
            .next_revision()
            .expect("retained revision");
        let identity = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            runner.frame_stage_owner.owner_generation(),
            revision,
        );
        assert!(
            runner
                .frame_stage_owner
                .queue(identity, TimedFrame::timed_repaint(Instant::now()),)
        );
        let owner_generation = runner.frame_stage_owner.owner_generation();
        assert!(runner.admit_device_recovery());

        let demand = pure_timed_repaint_demand(FrameScheduleKey::Primary, Instant::now());
        let admission = runner.admit_frame_schedule_work(Instant::now(), &demand);

        assert_eq!(admission, FrameScheduleAdmission::default());
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner.frame_stage_owner.next_ready_stage(Instant::now()),
            NextReadyStage::None
        );
        assert!(runner.frame_stage_owner.owner_generation() > owner_generation);
        assert!(runner.finish_device_recovery());
    }

    #[test]
    fn pure_timed_repaint_without_change_does_not_rebuild_or_drain() {
        let mut runner = auxiliary_runner();
        runner.window.target_generation.advance();
        let now = Instant::now();
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let admission = runner.admit_auxiliary_frame_schedule_work(
            now,
            &pure_timed_repaint_demand(key, now),
            NativeAdapterGeneration::from_test_serial(1),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert!(!runner.timing.redraw_requested);
        assert!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .is_some_and(|bundle| {
                    bundle.advance_timed_repaint() && !bundle.drain_timed_frame()
                })
        );
    }

    #[test]
    fn selected_auxiliary_demand_uses_borrowed_generation_and_does_not_merge_again() {
        let mut runner = auxiliary_runner();
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let scheduler_now = Instant::now() - Duration::from_millis(50);
        let demand = FrameScheduleDemand::from_cadence(
            key.clone(),
            TimedFrameCadence::DrainNow {
                due_at: scheduler_now - Duration::from_millis(50),
                next_wake: scheduler_now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            true,
            FrameScheduleRedrawEvidence::default(),
        );

        assert_eq!(runner.frame_stage_owner.key(), &key);
        let admission =
            runner.admit_auxiliary_frame_schedule_work(scheduler_now, &demand, adapter_generation);

        assert!(admission.did_work);
        assert!(admission.route_outcome);
        let drained_at = runner.timing.last_timed_frame_drain;
        assert!(drained_at > scheduler_now);
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "borrowed parent generation should admit exactly one auxiliary frame"
        );

        runner.apply_route_outcome_with_timed_frame(admission.outcome, false);
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "the selected auxiliary route must not merge a second timed frame"
        );
        assert_eq!(runner.timing.last_timed_frame_drain, drained_at);
    }

    #[test]
    fn selected_primary_mixed_deadline_uses_one_owner_bundle_when_repaint_is_due() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
        );
        runner.window.target_generation.advance();
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::from_test_serial(1),
            Arc::new(DeviceLossRegistration::new()),
        ));

        let now = Instant::now() - Duration::from_millis(50);
        let demand = mixed_due_demand(FrameScheduleKey::Primary, now);
        let due_at = now - Duration::from_millis(1);

        let admission = runner.admit_frame_schedule_work(now, &demand);

        assert!(admission.did_work);
        assert!(admission.route_outcome);
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "mixed deadline work should drain exactly once"
        );
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.due_at()),
            Some(due_at)
        );
        assert!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .is_some_and(|bundle| bundle.advance_timed_repaint())
        );
    }

    #[test]
    fn selected_primary_mixed_repaint_and_reissue_preserve_stronger_work_without_drain() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now - Duration::from_millis(50));
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let admission = runner.admit_frame_schedule_work(
            now,
            &mixed_repaint_and_reissue_demand(FrameScheduleKey::Primary, due_at, now),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(matches!(
            runner.timing.pending_frame_work,
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                ..
            }
        ));
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("mixed repaint and reissue completion should remain owner evidence");
        assert!(completed.advance_timed_repaint());
        assert!(completed.reissue_pending_redraw());
        assert!(!completed.drain_timed_frame());
        assert_eq!(
            completed.animation_activity(),
            RuntimeAnimationActivity::idle()
        );
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn selected_auxiliary_mixed_deadline_uses_exact_key_and_borrowed_generation() {
        let mut runner = auxiliary_runner();
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let now = Instant::now() - Duration::from_millis(50);
        let demand = mixed_due_demand(key.clone(), now);

        let admission =
            runner.admit_auxiliary_frame_schedule_work(now, &demand, adapter_generation);

        assert!(admission.did_work);
        assert!(admission.route_outcome);
        assert_eq!(runner.frame_stage_owner.key(), &key);
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "auxiliary mixed deadline work should use the borrowed parent generation once"
        );
        assert!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .is_some_and(|bundle| bundle.advance_timed_repaint())
        );
    }

    #[test]
    fn selected_auxiliary_mixed_repaint_and_reissue_use_one_borrowed_owner_bundle() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now - Duration::from_millis(50));
        runner.window.target_generation.advance();
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));

        let admission = runner.admit_auxiliary_frame_schedule_work(
            now,
            &mixed_repaint_and_reissue_demand(key.clone(), due_at, now),
            NativeAdapterGeneration::from_test_serial(1),
        );

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert_eq!(runner.frame_stage_owner.key(), &key);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        let completed = runner
            .frame_stage_owner
            .completion_bundle()
            .expect("auxiliary mixed completion should remain owner evidence");
        assert!(completed.advance_timed_repaint());
        assert!(completed.reissue_pending_redraw());
        assert!(!completed.drain_timed_frame());
        assert_eq!(
            completed.animation_activity(),
            RuntimeAnimationActivity::idle()
        );
        assert!(!completed.needs_text_caret_animation());
    }

    #[test]
    fn fresh_pending_redraw_after_repaint_advancement_defers_the_timed_frame() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let repaint_deadline = arm_delayed_repaint(&mut runner);
        let now = repaint_deadline;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now - Duration::from_millis(1));
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        let demand = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Primary,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence {
                timed_repaint_deadline: Some(repaint_deadline),
                pending_redraw_requested: true,
                pending_redraw_retry_deadline: Some(now + Duration::from_millis(16)),
                pending_redraw_fresh: false,
                ..FrameScheduleRedrawEvidence::default()
            },
        );

        let admission = runner.admit_frame_schedule_work(now, &demand);

        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.core.timed_repaint_deadline(), None);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert!(matches!(
            runner.timing.pending_frame_work,
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                ..
            }
        ));
    }

    #[test]
    fn primary_live_redraw_request_retains_and_resumes_original_deferred_drain() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        let first_demand = mixed_live_demand(FrameScheduleKey::Primary, due_at);

        let first = runner.admit_frame_schedule_work(now, &first_demand);

        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert!(first.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.timing.redraw_requested);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert_eq!(runner.core.timed_repaint_deadline(), None);

        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        let later = runner.admit_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(FrameScheduleKey::Primary, due_at, Instant::now()),
        );

        assert!(later.did_work);
        assert!(later.route_outcome);
        assert!(later.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.due_at()),
            Some(due_at)
        );
        assert!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .is_some_and(|bundle| bundle.advance_timed_repaint())
        );
        assert!(!runner.timing.redraw_requested);
        runner.apply_route_outcome_with_timed_frame(later.outcome, false);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
    }

    #[test]
    fn auxiliary_live_redraw_request_retains_and_resumes_original_deferred_drain() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let first_demand = mixed_live_demand(key.clone(), due_at);

        let first =
            runner.admit_auxiliary_frame_schedule_work(now, &first_demand, adapter_generation);

        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert!(first.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.timing.redraw_requested);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);

        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        let later = runner.admit_auxiliary_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(key, due_at, Instant::now()),
            adapter_generation,
        );

        assert!(later.did_work);
        assert!(later.route_outcome);
        assert!(later.timed_frame_already_handled);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.due_at()),
            Some(due_at)
        );
        assert!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .is_some_and(|bundle| bundle.advance_timed_repaint())
        );
        assert!(!runner.timing.redraw_requested);
        runner.apply_route_outcome_with_timed_frame(later.outcome, false);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
    }

    #[test]
    fn primary_deferred_deadline_target_generation_cancels_without_replay() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let first = runner
            .admit_frame_schedule_work(now, &mixed_live_demand(FrameScheduleKey::Primary, due_at));
        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        let first_pending_frame_work = runner.timing.pending_frame_work;

        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        assert!(runner.window.target_generation.advance());
        let stale = runner.admit_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(FrameScheduleKey::Primary, due_at, Instant::now()),
        );
        assert!(stale.did_work);
        assert!(!stale.route_outcome);
        assert!(!runner.frame_stage_owner.has_deferred_timed_frame());
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);

        let later_now = due_at + Duration::from_millis(5);
        let later = runner.admit_frame_schedule_work(
            later_now,
            &retained_drain_demand(FrameScheduleKey::Primary, due_at, later_now),
        );
        assert!(later.did_work);
        assert!(later.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.advance_timed_repaint()),
            Some(false)
        );
    }

    #[test]
    fn primary_deferred_deadline_adapter_generation_cancels_without_replay() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let first = runner
            .admit_frame_schedule_work(now, &mixed_live_demand(FrameScheduleKey::Primary, due_at));
        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        let first_pending_frame_work = runner.timing.pending_frame_work;

        let mut next_adapter_generation = adapter_generation;
        assert!(next_adapter_generation.advance());
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            next_adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        let stale = runner.admit_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(FrameScheduleKey::Primary, due_at, Instant::now()),
        );
        assert!(stale.did_work);
        assert!(!stale.route_outcome);
        assert!(!runner.frame_stage_owner.has_deferred_timed_frame());
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);

        let later_now = due_at + Duration::from_millis(5);
        let later = runner.admit_frame_schedule_work(
            later_now,
            &retained_drain_demand(FrameScheduleKey::Primary, due_at, later_now),
        );
        assert!(later.did_work);
        assert!(later.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.advance_timed_repaint()),
            Some(false)
        );
    }

    #[test]
    fn auxiliary_deferred_deadline_target_generation_cancels_without_replay() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));

        let first = runner.admit_auxiliary_frame_schedule_work(
            now,
            &mixed_live_demand(key.clone(), due_at),
            adapter_generation,
        );
        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        let first_pending_frame_work = runner.timing.pending_frame_work;

        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        assert!(runner.window.target_generation.advance());
        let stale = runner.admit_auxiliary_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(key.clone(), due_at, Instant::now()),
            adapter_generation,
        );
        assert!(stale.did_work);
        assert!(!stale.route_outcome);
        assert!(!runner.frame_stage_owner.has_deferred_timed_frame());
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);

        let later_now = due_at + Duration::from_millis(5);
        let later = runner.admit_auxiliary_frame_schedule_work(
            later_now,
            &retained_drain_demand(key, due_at, later_now),
            adapter_generation,
        );
        assert!(later.did_work);
        assert!(later.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.advance_timed_repaint()),
            Some(false)
        );
    }

    #[test]
    fn auxiliary_deferred_deadline_adapter_generation_cancels_without_replay() {
        let repaint_advance_calls = Rc::new(Cell::new(0));
        let mut runner = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            DelayedRepaintBridge::new(Rc::clone(&repaint_advance_calls)),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );
        let due_at = arm_delayed_repaint(&mut runner);
        let now = due_at;
        let last_drain = runner.timing.last_timed_frame_drain;
        runner.window.target_generation.advance();
        runner.timing.redraw_requested = true;
        runner.timing.redraw_requested_at = Some(now);
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let next_adapter_generation = {
            let mut generation = adapter_generation;
            assert!(generation.advance());
            generation
        };
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));

        let first = runner.admit_auxiliary_frame_schedule_work(
            now,
            &mixed_live_demand(key.clone(), due_at),
            adapter_generation,
        );
        assert!(first.did_work);
        assert!(!first.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert!(runner.frame_stage_owner.has_deferred_timed_frame());
        let first_pending_frame_work = runner.timing.pending_frame_work;

        runner.timing.redraw_requested = false;
        runner.timing.redraw_requested_at = None;
        let stale = runner.admit_auxiliary_frame_schedule_work(
            Instant::now(),
            &retained_drain_demand(key.clone(), due_at, Instant::now()),
            next_adapter_generation,
        );
        assert!(stale.did_work);
        assert!(!stale.route_outcome);
        assert!(!runner.frame_stage_owner.has_deferred_timed_frame());
        assert!(!runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.frame_stage_owner.completion_bundle(), None);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert_eq!(runner.timing.last_timed_frame_drain, last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);

        let later_now = due_at + Duration::from_millis(5);
        let later = runner.admit_auxiliary_frame_schedule_work(
            later_now,
            &retained_drain_demand(key, due_at, later_now),
            next_adapter_generation,
        );
        assert!(later.did_work);
        assert!(later.route_outcome);
        assert_eq!(repaint_advance_calls.get(), 1);
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
        assert!(runner.timing.last_timed_frame_drain > last_drain);
        assert_eq!(runner.timing.pending_frame_work, first_pending_frame_work);
        assert_eq!(
            runner
                .frame_stage_owner
                .completion_bundle()
                .map(|bundle| bundle.advance_timed_repaint()),
            Some(false)
        );
    }

    #[test]
    fn auxiliary_key_mismatch_vetoes_before_payload_release() {
        let mut runner = auxiliary_runner();
        runner.window.target_generation.advance();
        let demand = due_demand(
            FrameScheduleKey::Auxiliary(String::from("other")),
            Instant::now(),
        );

        let admission = runner.admit_auxiliary_frame_schedule_work(
            Instant::now(),
            &demand,
            NativeAdapterGeneration::from_test_serial(1),
        );

        assert_eq!(admission, FrameScheduleAdmission::default());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 0);
        assert!(!runner.frame_stage_owner.has_in_flight());
    }

    #[test]
    fn auxiliary_stale_generation_and_target_vetoes_never_redrain_in_flight_work() {
        let mut runner = auxiliary_runner();
        runner.window.target_generation.advance();
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        let target_generation = runner.window.target_generation;
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let now = Instant::now();
        assert!(runner.frame_stage_owner.prepare_fence(
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
        ));
        let revision = runner
            .frame_stage_owner
            .next_revision()
            .expect("first auxiliary revision");
        let identity = FrameStageIdentity::new(
            key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            runner.frame_stage_owner.owner_generation(),
            revision,
        );
        assert!(runner.frame_stage_owner.queue(
            identity.clone(),
            TimedFrame::new(now, RuntimeAnimationActivity::frame_messages(), false),
        ));
        assert!(runner.frame_stage_owner.begin(&identity, now).is_some());
        runner.drain_timed_frame_now(now, RuntimeAnimationActivity::frame_messages(), false);

        let wrong_completion = FrameStageIdentity::new(
            key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            identity.owner_generation(),
            identity.revision() + 1,
        );
        assert!(
            !runner
                .frame_stage_owner
                .complete(&wrong_completion, now, Instant::now())
        );

        let demand = due_demand(key, now);
        let stale_generation = NativeAdapterGeneration::from_test_serial(2);
        let stale_admission =
            runner.admit_auxiliary_frame_schedule_work(Instant::now(), &demand, stale_generation);
        assert!(stale_admission.did_work);
        assert!(!stale_admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);

        assert!(runner.window.target_generation.advance());
        let target_admission =
            runner.admit_auxiliary_frame_schedule_work(Instant::now(), &demand, adapter_generation);
        assert!(target_admission.did_work);
        assert!(!target_admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);
    }

    #[test]
    fn in_flight_completion_vetoes_never_fallback_or_clear_owner() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            CountingFrameBridge::default(),
            Vector2::new(320.0, 240.0),
        );
        runner.window.target_generation.advance();
        let target_generation = runner.window.target_generation;
        let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));

        let now = Instant::now();
        assert!(runner.frame_stage_owner.prepare_fence(
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
        ));
        let revision = runner
            .frame_stage_owner
            .next_revision()
            .expect("first revision");
        let identity = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            runner.frame_stage_owner.owner_generation(),
            revision,
        );
        assert!(runner.frame_stage_owner.queue(
            identity.clone(),
            TimedFrame::new(now, RuntimeAnimationActivity::frame_messages(), false),
        ));
        assert!(runner.frame_stage_owner.begin(&identity, now).is_some());
        runner.drain_timed_frame_now(now, RuntimeAnimationActivity::frame_messages(), false);

        let wrong_completion = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            identity.owner_generation(),
            identity.revision() + 1,
        );
        assert!(
            !runner
                .frame_stage_owner
                .complete(&wrong_completion, now, Instant::now())
        );
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(runner.core.runtime.bridge_mut().queue_calls.get(), 1);

        let demand = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Primary,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::frame_messages(),
            false,
            FrameScheduleRedrawEvidence::default(),
        );

        runner.adapter = None;
        let admission = runner.admit_frame_schedule_work(Instant::now(), &demand);
        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "adapter-missing veto must not drain a begun frame again"
        );

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        let mut next_adapter_generation = adapter_generation;
        assert!(next_adapter_generation.advance());
        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            next_adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        let admission = runner.admit_frame_schedule_work(Instant::now(), &demand);
        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "known-adapter transition must not drain a begun frame again"
        );

        runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
            adapter_generation,
            Arc::new(DeviceLossRegistration::new()),
        ));
        assert!(runner.window.target_generation.advance());
        let admission = runner.admit_frame_schedule_work(Instant::now(), &demand);
        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "known-target transition must not drain a begun frame again"
        );

        runner.window.target_generation.invalidate_unknown();
        let admission = runner.admit_frame_schedule_work(Instant::now(), &demand);
        assert!(admission.did_work);
        assert!(!admission.route_outcome);
        assert!(runner.frame_stage_owner.has_in_flight());
        assert_eq!(
            runner.core.runtime.bridge_mut().queue_calls.get(),
            1,
            "unknown-target veto must not drain a begun frame again"
        );
    }

    fn eligible() -> AuxiliaryScheduleEligibility {
        AuxiliaryScheduleEligibility {
            active: true,
            admitted: true,
            local_running: true,
            live_window: true,
            recovering: false,
            closing: false,
            stopped: false,
            native_resources_present: true,
            resource_generation_current: true,
            mailbox_suspended: false,
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
        fn veto_suspended(state: &mut AuxiliaryScheduleEligibility) {
            state.mailbox_suspended = true;
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
            veto_local_running,
            veto_live_window,
            veto_recovering,
            veto_closing,
            veto_stopped,
            veto_resources,
            veto_generation,
            veto_suspended,
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
    fn production_scheduler_admits_all_current_eligible_windows_beyond_sixteen() {
        let now = Instant::now();
        let demands: Vec<_> = (0..17)
            .map(|index| {
                due_demand(
                    FrameScheduleKey::Auxiliary(format!("auxiliary-{index:02}")),
                    now,
                )
            })
            .collect();
        let expected = demands
            .iter()
            .map(|demand| demand.key().clone())
            .collect::<Vec<_>>();
        let mut scheduler = NativeFrameScheduler::default();
        let mut selected = Vec::with_capacity(demands.len());

        for _ in &demands {
            let plan = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
            let key = plan
                .selected
                .expect("every current fairness-eligible window must be selectable");
            scheduler.record_admission(key.clone());
            selected.push(key);
        }

        assert_eq!(selected, expected);
    }

    #[test]
    fn empty_or_ineligible_demands_have_no_scheduler_fallback() {
        let now = Instant::now();
        let mut scheduler = NativeFrameScheduler::default();
        assert!(
            scheduler
                .observe(now, &[], FrameScheduleDeadlines::default())
                .selected
                .is_none()
        );

        let idle = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Auxiliary("idle".into()),
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::idle(),
            false,
            FrameScheduleRedrawEvidence::default(),
        );
        assert!(
            scheduler
                .observe(now, &[idle], FrameScheduleDeadlines::default())
                .selected
                .is_none()
        );
    }

    #[test]
    fn stable_round_robin_prevents_continuously_due_primary_starvation() {
        let now = Instant::now();
        let primary = due_demand(FrameScheduleKey::Primary, now);
        let auxiliary = due_demand(FrameScheduleKey::Auxiliary("settings".into()), now);
        let demands = [auxiliary.clone(), primary.clone()];
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
    fn due_selection_uses_original_due_boundary_not_next_wake_order() {
        let now = Instant::now();
        let earlier = due_demand_with_cadence(
            FrameScheduleKey::Auxiliary("earlier".into()),
            TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(8),
                next_wake: now + Duration::from_millis(16),
            },
        );
        let later = due_demand_with_cadence(
            FrameScheduleKey::Auxiliary("later".into()),
            TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(2),
                next_wake: now + Duration::from_millis(1),
            },
        );
        let mut scheduler = NativeFrameScheduler::default();

        let plan = scheduler.observe(
            now,
            &[later, earlier.clone()],
            FrameScheduleDeadlines::default(),
        );

        assert_eq!(
            plan.selected,
            Some(FrameScheduleKey::Auxiliary("earlier".into()))
        );
    }

    #[test]
    fn new_deadline_precedes_unserved_stale_redraw_after_admission() {
        let now = Instant::now();
        let primary = due_demand_with_cadence(
            FrameScheduleKey::Primary,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
        );
        let stale_redraw = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Auxiliary("stale".into()),
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
        let mut scheduler = NativeFrameScheduler::default();

        let first = scheduler.observe(
            now,
            &[primary.clone(), stale_redraw.clone()],
            FrameScheduleDeadlines::default(),
        );
        assert_eq!(first.selected, Some(FrameScheduleKey::Primary));
        scheduler.record_admission(FrameScheduleKey::Primary);

        let newly_due_primary = due_demand_with_cadence(
            FrameScheduleKey::Primary,
            TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(1),
                next_wake: now + Duration::from_millis(15),
            },
        );
        let second = scheduler.observe(
            now,
            &[stale_redraw, newly_due_primary],
            FrameScheduleDeadlines::default(),
        );

        assert_eq!(second.selected, Some(FrameScheduleKey::Primary));
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
            Some(FrameScheduleKey::Auxiliary("second".into())),
            "removing a remembered key must preserve the stable-key fairness cursor"
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
