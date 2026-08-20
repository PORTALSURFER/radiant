//! Exact, bounded admission for one window's safe-boundary frame work.
//!
//! The owner keeps the payload behind an exact identity fence.  Readiness is
//! deliberately payload-free: callers must present the same identity again to
//! begin the work, and `begin` rechecks the current time before releasing the
//! payload.

use super::{
    adapter::NativeAdapterGeneration, frame_scheduler::FrameScheduleKey,
    frame_scheduler_policy::SchedulerStage,
    native_resource_maintenance::NativeResourceMaintenanceBinding,
    runner_state::NativeTargetGeneration,
};
use crate::runtime::RuntimeAnimationActivity;
use std::time::{Duration, Instant};

/// Exact identity for one admitted scheduler stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FrameStageIdentity {
    key: FrameScheduleKey,
    adapter_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    stage: SchedulerStage,
    owner_generation: u64,
    revision: u64,
}

impl FrameStageIdentity {
    pub(super) fn new(
        key: FrameScheduleKey,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        stage: SchedulerStage,
        owner_generation: u64,
        revision: u64,
    ) -> Self {
        Self {
            key,
            adapter_generation,
            target_generation,
            stage,
            owner_generation,
            revision,
        }
    }

    #[cfg(test)]
    pub(super) fn key(&self) -> &FrameScheduleKey {
        &self.key
    }

    #[cfg(test)]
    pub(super) const fn adapter_generation(&self) -> NativeAdapterGeneration {
        self.adapter_generation
    }

    #[cfg(test)]
    pub(super) const fn target_generation(&self) -> NativeTargetGeneration {
        self.target_generation
    }

    #[cfg(test)]
    pub(super) const fn stage(&self) -> SchedulerStage {
        self.stage
    }

    #[cfg(test)]
    pub(super) const fn owner_generation(&self) -> u64 {
        self.owner_generation
    }

    #[cfg(test)]
    pub(super) const fn revision(&self) -> u64 {
        self.revision
    }

    fn same_fence(&self, other: &Self) -> bool {
        self.key == other.key
            && self.adapter_generation == other.adapter_generation
            && self.target_generation == other.target_generation
            && self.stage == other.stage
            && self.owner_generation == other.owner_generation
    }

    const fn fence(&self) -> FrameStageFence {
        FrameStageFence {
            adapter_generation: self.adapter_generation,
            target_generation: self.target_generation,
            stage: self.stage,
        }
    }
}

/// Payload for one selected deadline bundle.  It is never returned by
/// readiness queries.  The repaint, timed-frame, and stale-redraw-reissue bits
/// are independent so a pure repaint or reissue does not need fabricated
/// animation or caret evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TimedFrame {
    due_at: Instant,
    animation_activity: RuntimeAnimationActivity,
    needs_text_caret_animation: bool,
    advance_timed_repaint: bool,
    drain_timed_frame: bool,
    reissue_pending_redraw: bool,
}

impl TimedFrame {
    #[cfg(test)]
    pub(super) const fn new(
        due_at: Instant,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> Self {
        Self::with_operations(
            due_at,
            animation_activity,
            needs_text_caret_animation,
            false,
            true,
            false,
        )
    }

    pub(super) const fn with_operations(
        due_at: Instant,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
        advance_timed_repaint: bool,
        drain_timed_frame: bool,
        reissue_pending_redraw: bool,
    ) -> Self {
        Self {
            due_at,
            animation_activity,
            needs_text_caret_animation,
            advance_timed_repaint,
            drain_timed_frame,
            reissue_pending_redraw,
        }
    }

    pub(super) const fn timed_repaint(due_at: Instant) -> Self {
        Self::with_operations(
            due_at,
            RuntimeAnimationActivity::idle(),
            false,
            true,
            false,
            false,
        )
    }

    pub(super) const fn pending_redraw_reissue(due_at: Instant) -> Self {
        Self::with_operations(
            due_at,
            RuntimeAnimationActivity::idle(),
            false,
            false,
            false,
            true,
        )
    }

    #[cfg(test)]
    pub(super) const fn due_at(self) -> Instant {
        self.due_at
    }

    pub(super) const fn animation_activity(self) -> RuntimeAnimationActivity {
        self.animation_activity
    }

    pub(super) const fn needs_text_caret_animation(self) -> bool {
        self.needs_text_caret_animation
    }

    pub(super) const fn advance_timed_repaint(self) -> bool {
        self.advance_timed_repaint
    }

    pub(super) const fn drain_timed_frame(self) -> bool {
        self.drain_timed_frame
    }

    pub(super) const fn reissue_pending_redraw(self) -> bool {
        self.reissue_pending_redraw
    }

    fn is_due(self, now: Instant) -> bool {
        self.due_at <= now
    }

    fn coalesce(self, newer: Self) -> Self {
        let drain_timed_frame = self.drain_timed_frame || newer.drain_timed_frame;
        Self {
            due_at: if self.due_at <= newer.due_at {
                self.due_at
            } else {
                newer.due_at
            },
            animation_activity: if newer.drain_timed_frame || !self.drain_timed_frame {
                newer.animation_activity
            } else {
                self.animation_activity
            },
            needs_text_caret_animation: if newer.drain_timed_frame || !self.drain_timed_frame {
                newer.needs_text_caret_animation
            } else {
                self.needs_text_caret_animation
            },
            advance_timed_repaint: self.advance_timed_repaint || newer.advance_timed_repaint,
            drain_timed_frame,
            reissue_pending_redraw: self.reissue_pending_redraw || newer.reissue_pending_redraw,
        }
    }
}

/// Payload-free result of a readiness query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum NextReadyStage {
    #[default]
    None,
    Deadline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameStageFence {
    adapter_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    stage: SchedulerStage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingTimedFrame {
    identity: FrameStageIdentity,
    frame: TimedFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InFlightTimedFramePhase {
    ExecutingBundle,
    DrainDeferredAfterTimedRepaint,
    ExecutingDeferredDrain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InFlightTimedFrame {
    identity: FrameStageIdentity,
    frame: TimedFrame,
    started_at: Instant,
    phase: InFlightTimedFramePhase,
}

/// A non-`Clone` witness for one admitted Projection-stage attempt.
///
/// Projection work is synchronous and therefore has no pending queue. The
/// owner retains only this exact identity until the caller completes the
/// attempt.
pub(super) struct ProjectionStageTicket {
    identity: FrameStageIdentity,
}

impl ProjectionStageTicket {
    #[cfg(test)]
    pub(super) fn identity(&self) -> &FrameStageIdentity {
        &self.identity
    }
}

#[derive(Debug)]
struct InFlightProjection {
    identity: FrameStageIdentity,
}

/// A non-`Clone` witness for one admitted Layout-stage attempt.
///
/// Layout work is synchronous and therefore has no pending queue. The owner
/// retains only this exact identity until the caller completes the attempt.
pub(super) struct LayoutStageTicket {
    identity: FrameStageIdentity,
}

impl LayoutStageTicket {
    #[cfg(test)]
    pub(super) fn identity(&self) -> &FrameStageIdentity {
        &self.identity
    }
}

#[derive(Debug)]
struct InFlightLayout {
    identity: FrameStageIdentity,
}

/// A non-`Clone` witness for one admitted PaintPlan-stage attempt.
///
/// Paint-plan work is synchronous and therefore has no pending queue. The
/// owner retains only this exact identity until the caller completes the
/// attempt.
pub(super) struct PaintPlanStageTicket {
    identity: FrameStageIdentity,
}

impl PaintPlanStageTicket {
    #[cfg(test)]
    pub(super) fn identity(&self) -> &FrameStageIdentity {
        &self.identity
    }
}

#[derive(Debug)]
struct InFlightPaintPlan {
    identity: FrameStageIdentity,
}

/// A non-`Clone` witness for the native encode/submit/present boundary.
///
/// This ticket is owned by the shared per-window stage owner until the native
/// presentation kernel reaches its exact completion boundary.
#[derive(Debug)]
pub(super) struct EncodePresentStageTicket {
    identity: FrameStageIdentity,
    owner_token: usize,
}

impl EncodePresentStageTicket {
    #[cfg(test)]
    pub(super) fn identity(&self) -> &FrameStageIdentity {
        &self.identity
    }
}

#[derive(Debug)]
struct InFlightEncodePresent {
    identity: FrameStageIdentity,
}

/// A non-`Clone` witness for one exact normal-Running native-resource unit.
#[derive(Debug)]
pub(super) struct MaintenanceStageTicket {
    identity: FrameStageIdentity,
    binding: NativeResourceMaintenanceBinding,
    owner_token: usize,
}

impl MaintenanceStageTicket {
    #[cfg(test)]
    pub(super) fn identity(&self) -> &FrameStageIdentity {
        &self.identity
    }

    #[cfg(test)]
    pub(super) const fn binding(&self) -> NativeResourceMaintenanceBinding {
        self.binding
    }
}

#[derive(Debug)]
struct InFlightMaintenance {
    identity: FrameStageIdentity,
    binding: NativeResourceMaintenanceBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameStageBudgetStatus {
    NotBudgeted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameStageCompletionEvidence {
    identity: FrameStageIdentity,
    frame: TimedFrame,
    elapsed: Duration,
    budget_status: FrameStageBudgetStatus,
}

/// One window's bounded safe-boundary stage owner.
///
/// At most one frame can be in flight and at most one newer frame can remain
/// pending.  Every transition is fenced by the exact key, native generations,
/// stage, owner generation, and revision carried by `FrameStageIdentity`.
#[derive(Debug)]
pub(super) struct WindowStageOwner {
    key: FrameScheduleKey,
    owner_generation: u64,
    next_revision: u64,
    revision_exhausted: bool,
    generation_exhausted: bool,
    fence: Option<FrameStageFence>,
    pending: Option<PendingTimedFrame>,
    in_flight: Option<InFlightTimedFrame>,
    projection_in_flight: Option<InFlightProjection>,
    layout_in_flight: Option<InFlightLayout>,
    paint_plan_in_flight: Option<InFlightPaintPlan>,
    encode_present_in_flight: Option<InFlightEncodePresent>,
    maintenance_in_flight: Option<InFlightMaintenance>,
    last_completion: Option<FrameStageCompletionEvidence>,
    last_projection_completion: Option<FrameStageIdentity>,
    last_layout_completion: Option<FrameStageIdentity>,
    last_paint_plan_completion: Option<FrameStageIdentity>,
    last_encode_present_completion: Option<FrameStageIdentity>,
    last_maintenance_completion: Option<FrameStageIdentity>,
}

impl WindowStageOwner {
    pub(super) fn new(key: FrameScheduleKey) -> Self {
        Self {
            key,
            owner_generation: 1,
            next_revision: 0,
            revision_exhausted: false,
            generation_exhausted: false,
            fence: None,
            pending: None,
            in_flight: None,
            projection_in_flight: None,
            layout_in_flight: None,
            paint_plan_in_flight: None,
            encode_present_in_flight: None,
            maintenance_in_flight: None,
            last_completion: None,
            last_projection_completion: None,
            last_layout_completion: None,
            last_paint_plan_completion: None,
            last_encode_present_completion: None,
            last_maintenance_completion: None,
        }
    }

    #[cfg(test)]
    pub(super) fn key(&self) -> &FrameScheduleKey {
        &self.key
    }

    pub(super) fn schedule_key(&self) -> &FrameScheduleKey {
        &self.key
    }

    pub(super) fn owns_key(&self, key: &FrameScheduleKey) -> bool {
        &self.key == key
    }

    pub(super) const fn owner_generation(&self) -> u64 {
        self.owner_generation
    }

    pub(super) const fn has_in_flight(&self) -> bool {
        self.in_flight.is_some()
            || self.projection_in_flight.is_some()
            || self.layout_in_flight.is_some()
            || self.paint_plan_in_flight.is_some()
            || self.encode_present_in_flight.is_some()
            || self.maintenance_in_flight.is_some()
    }

    pub(super) fn next_revision(&mut self) -> Option<u64> {
        if self.revision_exhausted {
            return None;
        }
        let Some(revision) = self.next_revision.checked_add(1) else {
            self.revision_exhausted = true;
            return None;
        };
        self.next_revision = revision;
        Some(revision)
    }

    /// Prepare the exact native fence used by the next identity.
    ///
    /// A changed native generation retires pending work before the new fence is
    /// installed, but an in-flight stage keeps the current fence intact. Unknown
    /// generations and stages without a concrete owner consumer cannot become
    /// owner state.
    pub(super) fn prepare_fence(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        stage: SchedulerStage,
    ) -> bool {
        if self.generation_exhausted
            || !adapter_generation.is_known()
            || !target_generation.is_known()
            || !stage_can_be_admitted(stage)
        {
            return false;
        }
        let fence = FrameStageFence {
            adapter_generation,
            target_generation,
            stage,
        };
        if self.fence != Some(fence) {
            if self.has_deferred_timed_frame() {
                if stage == SchedulerStage::Deadline {
                    self.invalidate();
                } else {
                    return false;
                }
            } else if self.in_flight.is_some()
                || self.projection_in_flight.is_some()
                || self.layout_in_flight.is_some()
                || self.paint_plan_in_flight.is_some()
                || self.encode_present_in_flight.is_some()
                || self.maintenance_in_flight.is_some()
            {
                return false;
            }
            if matches!(
                stage,
                SchedulerStage::Projection | SchedulerStage::Maintenance
            ) && self.pending.is_some()
            {
                // Do not discard a selected Deadline payload merely because a
                // later redraw or maintenance opportunity reaches a later
                // synchronous boundary.
                return false;
            }
            if self.fence.is_some() {
                self.invalidate();
            }
            if self.generation_exhausted {
                return false;
            }
            self.fence = Some(fence);
        }
        true
    }

    /// Admit one synchronous Projection-stage attempt under this window's
    /// existing owner. There is no second event loop or pending per-owner
    /// queue: the returned ticket is the one exact in-flight operation.
    pub(super) fn admit_projection(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
    ) -> Option<ProjectionStageTicket> {
        if self.pending.is_some()
            || self.in_flight.is_some()
            || self.projection_in_flight.is_some()
            || !self.prepare_fence(
                adapter_generation,
                target_generation,
                SchedulerStage::Projection,
            )
        {
            return None;
        }
        let revision = self.next_revision()?;
        let identity = FrameStageIdentity::new(
            self.key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::Projection,
            self.owner_generation,
            revision,
        );
        if self.stale(&identity) {
            return None;
        }
        self.projection_in_flight = Some(InFlightProjection {
            identity: identity.clone(),
        });
        Some(ProjectionStageTicket { identity })
    }

    pub(super) fn projection_ticket_is_current(&self, ticket: &ProjectionStageTicket) -> bool {
        self.projection_in_flight
            .as_ref()
            .is_some_and(|in_flight| in_flight.identity == ticket.identity)
    }

    /// Complete only the exact Projection-stage attempt that was admitted.
    /// A mismatch leaves the begun attempt in flight so callers cannot replay
    /// it through the conservative fallback.
    pub(super) fn complete_projection(&mut self, ticket: ProjectionStageTicket) -> bool {
        let Some(in_flight) = self.projection_in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != ticket.identity {
            return false;
        }
        self.projection_in_flight = None;
        self.last_projection_completion = Some(ticket.identity);
        true
    }

    /// Admit one synchronous Layout-stage attempt under this window's
    /// existing owner. There is no second event loop or pending per-owner
    /// queue: the returned ticket is the one exact in-flight operation.
    pub(super) fn admit_layout(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
    ) -> Option<LayoutStageTicket> {
        if self.pending.is_some()
            || self.in_flight.is_some()
            || self.projection_in_flight.is_some()
            || self.layout_in_flight.is_some()
            || !self.prepare_fence(
                adapter_generation,
                target_generation,
                SchedulerStage::Layout,
            )
        {
            return None;
        }
        let revision = self.next_revision()?;
        let identity = FrameStageIdentity::new(
            self.key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::Layout,
            self.owner_generation,
            revision,
        );
        if self.stale(&identity) {
            return None;
        }
        self.layout_in_flight = Some(InFlightLayout {
            identity: identity.clone(),
        });
        Some(LayoutStageTicket { identity })
    }

    pub(super) fn layout_ticket_is_current(&self, ticket: &LayoutStageTicket) -> bool {
        self.layout_in_flight
            .as_ref()
            .is_some_and(|in_flight| in_flight.identity == ticket.identity)
    }

    /// Complete only the exact Layout-stage attempt that was admitted.
    /// A mismatch leaves the begun attempt in flight so callers cannot replay
    /// it through the conservative fallback.
    pub(super) fn complete_layout(&mut self, ticket: LayoutStageTicket) -> bool {
        let Some(in_flight) = self.layout_in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != ticket.identity {
            return false;
        }
        self.layout_in_flight = None;
        self.last_layout_completion = Some(ticket.identity);
        true
    }

    /// Admit one synchronous PaintPlan-stage attempt under this window's
    /// existing owner. There is no second event loop or pending per-owner
    /// queue: the returned ticket is the one exact in-flight operation.
    pub(super) fn admit_paint_plan(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
    ) -> Option<PaintPlanStageTicket> {
        if self.pending.is_some()
            || self.in_flight.is_some()
            || self.projection_in_flight.is_some()
            || self.layout_in_flight.is_some()
            || self.paint_plan_in_flight.is_some()
            || !self.prepare_fence(
                adapter_generation,
                target_generation,
                SchedulerStage::PaintPlan,
            )
        {
            return None;
        }
        let revision = self.next_revision()?;
        let identity = FrameStageIdentity::new(
            self.key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::PaintPlan,
            self.owner_generation,
            revision,
        );
        if self.stale(&identity) {
            return None;
        }
        self.paint_plan_in_flight = Some(InFlightPaintPlan {
            identity: identity.clone(),
        });
        Some(PaintPlanStageTicket { identity })
    }

    pub(super) fn paint_plan_ticket_is_current(&self, ticket: &PaintPlanStageTicket) -> bool {
        self.paint_plan_in_flight
            .as_ref()
            .is_some_and(|in_flight| in_flight.identity == ticket.identity)
    }

    /// Complete only the exact PaintPlan-stage attempt that was admitted.
    /// A mismatch leaves the begun attempt in flight so callers cannot replay
    /// it through the conservative fallback.
    pub(super) fn complete_paint_plan(&mut self, ticket: PaintPlanStageTicket) -> bool {
        let Some(in_flight) = self.paint_plan_in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != ticket.identity {
            return false;
        }
        self.paint_plan_in_flight = None;
        self.last_paint_plan_completion = Some(ticket.identity);
        true
    }

    /// Admit one exact native encode/submit/present attempt after all
    /// synchronous CPU-side work has reached a stable snapshot.
    pub(super) fn admit_encode_present(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
    ) -> Option<EncodePresentStageTicket> {
        if self.pending.is_some()
            || self.in_flight.is_some()
            || self.projection_in_flight.is_some()
            || self.layout_in_flight.is_some()
            || self.paint_plan_in_flight.is_some()
            || self.encode_present_in_flight.is_some()
            || !self.prepare_fence(
                adapter_generation,
                target_generation,
                SchedulerStage::EncodePresent,
            )
        {
            return None;
        }
        let revision = self.next_revision()?;
        let identity = FrameStageIdentity::new(
            self.key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::EncodePresent,
            self.owner_generation,
            revision,
        );
        if self.stale(&identity) {
            return None;
        }
        self.encode_present_in_flight = Some(InFlightEncodePresent {
            identity: identity.clone(),
        });
        Some(EncodePresentStageTicket {
            identity,
            owner_token: self as *const Self as usize,
        })
    }

    pub(super) fn encode_present_ticket_is_current(
        &self,
        ticket: &EncodePresentStageTicket,
    ) -> bool {
        self.encode_present_in_flight
            .as_ref()
            .is_some_and(|in_flight| {
                in_flight.identity == ticket.identity
                    && ticket.owner_token == self as *const Self as usize
            })
    }

    /// Complete only the exact native encode/submit/present operation that was
    /// admitted. A mismatch leaves the real owner in flight.
    pub(super) fn complete_encode_present(&mut self, ticket: EncodePresentStageTicket) -> bool {
        let Some(in_flight) = self.encode_present_in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != ticket.identity
            || ticket.owner_token != self as *const Self as usize
        {
            return false;
        }
        self.encode_present_in_flight = None;
        self.last_encode_present_completion = Some(ticket.identity);
        true
    }

    /// Veto the exact native encode/submit/present attempt without recording a
    /// successful completion. A mismatched ticket preserves the real owner.
    pub(super) fn veto_encode_present(&mut self, ticket: EncodePresentStageTicket) -> bool {
        let Some(in_flight) = self.encode_present_in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != ticket.identity
            || ticket.owner_token != self as *const Self as usize
        {
            return false;
        }
        self.encode_present_in_flight = None;
        true
    }

    /// Admit one exact bounded normal-Running maintenance unit.  The resource
    /// binding is retained beside the stage identity so execution cannot scan
    /// for a replacement slot after a currentness veto.
    pub(super) fn admit_maintenance(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        binding: NativeResourceMaintenanceBinding,
    ) -> Option<MaintenanceStageTicket> {
        if self.pending.is_some()
            || self.has_in_flight()
            || !binding.generation().is_known()
            || binding.completion().generation() != binding.generation()
        {
            return None;
        }
        if !self.prepare_fence(
            adapter_generation,
            target_generation,
            SchedulerStage::Maintenance,
        ) {
            return None;
        }
        let revision = self.next_revision()?;
        let identity = FrameStageIdentity::new(
            self.key.clone(),
            adapter_generation,
            target_generation,
            SchedulerStage::Maintenance,
            self.owner_generation,
            revision,
        );
        if self.stale(&identity) {
            return None;
        }
        self.maintenance_in_flight = Some(InFlightMaintenance {
            identity: identity.clone(),
            binding,
        });
        Some(MaintenanceStageTicket {
            identity,
            binding,
            owner_token: self as *const Self as usize,
        })
    }

    pub(super) fn maintenance_ticket_is_current(&self, ticket: &MaintenanceStageTicket) -> bool {
        self.maintenance_in_flight
            .as_ref()
            .is_some_and(|in_flight| {
                in_flight.identity == ticket.identity
                    && in_flight.binding == ticket.binding
                    && ticket.owner_token == self as *const Self as usize
            })
    }

    /// Complete only the exact maintenance unit that was admitted.
    pub(super) fn complete_maintenance(&mut self, ticket: MaintenanceStageTicket) -> bool {
        if !self.maintenance_ticket_is_current(&ticket) {
            return false;
        }
        self.maintenance_in_flight = None;
        self.last_maintenance_completion = Some(ticket.identity);
        true
    }

    /// Veto the exact ticket after a resource currentness check fails.  A
    /// wrong ticket cannot clear the real owner and therefore cannot authorize
    /// a fallback maintenance scan.
    pub(super) fn veto_maintenance(&mut self, ticket: MaintenanceStageTicket) -> bool {
        if !self.maintenance_ticket_is_current(&ticket) {
            return false;
        }
        self.maintenance_in_flight = None;
        true
    }

    /// Queue one exact frame, coalescing only compatible work.
    ///
    /// A duplicate revision updates its payload while retaining the earliest
    /// due time.  A newer revision replaces an older pending revision, also
    /// retaining the earliest due time.  In-flight work is never replaced.
    pub(super) fn queue(&mut self, identity: FrameStageIdentity, frame: TimedFrame) -> bool {
        if !self.accepts_deadline_identity(&identity) {
            return false;
        }
        if self.fence.is_none() {
            self.fence = Some(identity.fence());
        }
        if self.last_completion.as_ref().is_some_and(|completion| {
            completion.identity.same_fence(&identity)
                && identity.revision <= completion.identity.revision
        }) {
            return false;
        }
        if let Some(in_flight) = self.in_flight.as_ref() {
            if in_flight.identity == identity {
                return true;
            }
            if !in_flight.identity.same_fence(&identity)
                || identity.revision <= in_flight.identity.revision
            {
                return false;
            }
        }
        match self.pending.as_mut() {
            None => {
                self.pending = Some(PendingTimedFrame { identity, frame });
            }
            Some(pending) => {
                if !pending.identity.same_fence(&identity)
                    || identity.revision < pending.identity.revision
                {
                    return false;
                }
                if identity.revision == pending.identity.revision {
                    pending.frame = pending.frame.coalesce(frame);
                } else {
                    pending.frame = pending.frame.coalesce(frame);
                    pending.identity = identity;
                }
            }
        }
        true
    }

    /// Return only the stage kind; the payload remains owned by this owner.
    pub(super) fn next_ready_stage(&self, now: Instant) -> NextReadyStage {
        if self.in_flight.is_some() {
            return NextReadyStage::None;
        }
        self.pending
            .as_ref()
            .filter(|pending| pending.frame.is_due(now))
            .map_or(NextReadyStage::None, |_| NextReadyStage::Deadline)
    }

    /// Begin one exact, currently due frame and release its payload.
    pub(super) fn begin(
        &mut self,
        identity: &FrameStageIdentity,
        now: Instant,
    ) -> Option<TimedFrame> {
        if self.in_flight.is_some() || !self.accepts_deadline_identity(identity) {
            return None;
        }
        let pending = self.pending.as_ref()?;
        if pending.identity != *identity || !pending.frame.is_due(now) {
            return None;
        }
        let pending = self.pending.take()?;
        self.in_flight = Some(InFlightTimedFrame {
            identity: identity.clone(),
            frame: pending.frame,
            started_at: Instant::now(),
            phase: InFlightTimedFramePhase::ExecutingBundle,
        });
        Some(pending.frame)
    }

    /// Retain the exact begun frame when repaint advancement completed but the
    /// existing pending-redraw policy deferred its remaining timed drain.
    pub(super) fn defer_timed_frame_drain(&mut self, identity: &FrameStageIdentity) -> bool {
        let Some(in_flight) = self.in_flight.as_mut() else {
            return false;
        };
        if in_flight.identity != *identity
            || in_flight.phase != InFlightTimedFramePhase::ExecutingBundle
        {
            return false;
        }
        in_flight.phase = InFlightTimedFramePhase::DrainDeferredAfterTimedRepaint;
        true
    }

    pub(super) fn has_deferred_timed_frame(&self) -> bool {
        self.in_flight.as_ref().is_some_and(|in_flight| {
            matches!(
                in_flight.phase,
                InFlightTimedFramePhase::DrainDeferredAfterTimedRepaint
            )
        })
    }

    /// Resume only the retained drain after the pending-redraw policy clears.
    /// The original identity and payload remain the owner of this operation.
    pub(super) fn resume_deferred_timed_frame(
        &mut self,
        key: &FrameScheduleKey,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
    ) -> Option<TimedFrame> {
        let in_flight = self.in_flight.as_ref()?;
        if in_flight.phase != InFlightTimedFramePhase::DrainDeferredAfterTimedRepaint {
            return None;
        }
        let identity = &in_flight.identity;
        let matches_deferred_fence = identity.key == *key
            && identity.adapter_generation == adapter_generation
            && identity.target_generation == target_generation
            && identity.stage == SchedulerStage::Deadline
            && identity.owner_generation == self.owner_generation
            && self.fence == Some(identity.fence());
        if !matches_deferred_fence {
            self.invalidate();
            return None;
        }
        let in_flight = self.in_flight.as_mut()?;
        in_flight.phase = InFlightTimedFramePhase::ExecutingDeferredDrain;
        Some(in_flight.frame)
    }

    /// Complete only the exact frame that was begun.
    pub(super) fn complete(
        &mut self,
        identity: &FrameStageIdentity,
        started_at: Instant,
        completed_at: Instant,
    ) -> bool {
        let Some(in_flight) = self.in_flight.as_ref() else {
            return false;
        };
        if in_flight.identity != *identity
            || !matches!(
                in_flight.phase,
                InFlightTimedFramePhase::ExecutingBundle
                    | InFlightTimedFramePhase::ExecutingDeferredDrain
            )
        {
            return false;
        }
        let Some(in_flight) = self.in_flight.take() else {
            return false;
        };
        self.last_completion = Some(FrameStageCompletionEvidence {
            identity: identity.clone(),
            frame: in_flight.frame,
            elapsed: completed_at.saturating_duration_since(started_at),
            budget_status: FrameStageBudgetStatus::NotBudgeted,
        });
        true
    }

    /// Complete the retained frame after its deferred drain executes.
    pub(super) fn complete_deferred_timed_frame(&mut self, completed_at: Instant) -> bool {
        let Some(in_flight) = self.in_flight.as_ref() else {
            return false;
        };
        if in_flight.phase != InFlightTimedFramePhase::ExecutingDeferredDrain {
            return false;
        }
        let Some(in_flight) = self.in_flight.take() else {
            return false;
        };
        self.last_completion = Some(FrameStageCompletionEvidence {
            identity: in_flight.identity,
            frame: in_flight.frame,
            elapsed: completed_at.saturating_duration_since(in_flight.started_at),
            budget_status: FrameStageBudgetStatus::NotBudgeted,
        });
        true
    }

    /// Report whether an identity cannot still be admitted at this owner.
    pub(super) fn stale(&self, identity: &FrameStageIdentity) -> bool {
        if !self.accepts_identity(identity) {
            return true;
        }
        if self.last_completion.as_ref().is_some_and(|completion| {
            completion.identity.same_fence(identity)
                && identity.revision <= completion.identity.revision
        }) {
            return true;
        }
        if self.in_flight.as_ref().is_some_and(|in_flight| {
            in_flight.identity.same_fence(identity)
                && identity.revision < in_flight.identity.revision
        }) {
            return true;
        }
        if self.projection_in_flight.as_ref().is_some_and(|in_flight| {
            in_flight.identity.same_fence(identity)
                && identity.revision < in_flight.identity.revision
        }) {
            return true;
        }
        if self.layout_in_flight.as_ref().is_some_and(|in_flight| {
            in_flight.identity.same_fence(identity)
                && identity.revision < in_flight.identity.revision
        }) {
            return true;
        }
        if self.paint_plan_in_flight.as_ref().is_some_and(|in_flight| {
            in_flight.identity.same_fence(identity)
                && identity.revision < in_flight.identity.revision
        }) {
            return true;
        }
        if self
            .encode_present_in_flight
            .as_ref()
            .is_some_and(|in_flight| {
                in_flight.identity.same_fence(identity)
                    && identity.revision < in_flight.identity.revision
            })
        {
            return true;
        }
        if self
            .maintenance_in_flight
            .as_ref()
            .is_some_and(|in_flight| {
                in_flight.identity.same_fence(identity)
                    && identity.revision < in_flight.identity.revision
            })
        {
            return true;
        }
        if self
            .last_encode_present_completion
            .as_ref()
            .is_some_and(|completion| {
                completion.same_fence(identity) && identity.revision <= completion.revision
            })
        {
            return true;
        }
        if self
            .last_projection_completion
            .as_ref()
            .is_some_and(|completion| {
                completion.same_fence(identity) && identity.revision <= completion.revision
            })
        {
            return true;
        }
        if self
            .last_layout_completion
            .as_ref()
            .is_some_and(|completion| {
                completion.same_fence(identity) && identity.revision <= completion.revision
            })
        {
            return true;
        }
        if self
            .last_paint_plan_completion
            .as_ref()
            .is_some_and(|completion| {
                completion.same_fence(identity) && identity.revision <= completion.revision
            })
        {
            return true;
        }
        if self
            .last_maintenance_completion
            .as_ref()
            .is_some_and(|completion| {
                completion.same_fence(identity) && identity.revision <= completion.revision
            })
        {
            return true;
        }
        self.pending.as_ref().is_some_and(|pending| {
            pending.identity.same_fence(identity) && identity.revision < pending.identity.revision
        })
    }

    /// Explicitly cancel all state and advance the owner generation.
    pub(super) fn invalidate(&mut self) {
        if self.fence.is_none()
            && self.pending.is_none()
            && self.in_flight.is_none()
            && self.projection_in_flight.is_none()
            && self.layout_in_flight.is_none()
            && self.paint_plan_in_flight.is_none()
            && self.encode_present_in_flight.is_none()
            && self.maintenance_in_flight.is_none()
            && self.last_completion.is_none()
            && self.last_projection_completion.is_none()
            && self.last_layout_completion.is_none()
            && self.last_paint_plan_completion.is_none()
            && self.last_encode_present_completion.is_none()
            && self.last_maintenance_completion.is_none()
        {
            return;
        }
        self.pending = None;
        self.in_flight = None;
        self.projection_in_flight = None;
        self.layout_in_flight = None;
        self.paint_plan_in_flight = None;
        self.encode_present_in_flight = None;
        self.maintenance_in_flight = None;
        self.last_completion = None;
        self.last_projection_completion = None;
        self.last_layout_completion = None;
        self.last_paint_plan_completion = None;
        self.last_encode_present_completion = None;
        self.last_maintenance_completion = None;
        self.fence = None;
        let Some(next_generation) = self.owner_generation.checked_add(1) else {
            self.generation_exhausted = true;
            return;
        };
        self.owner_generation = next_generation;
    }

    #[cfg(test)]
    pub(super) fn completion_bundle(&self) -> Option<TimedFrame> {
        self.last_completion
            .as_ref()
            .map(|completion| completion.frame)
    }

    fn accepts_identity(&self, identity: &FrameStageIdentity) -> bool {
        identity.key == self.key
            && identity.adapter_generation.is_known()
            && identity.target_generation.is_known()
            && stage_can_be_admitted(identity.stage)
            && identity.owner_generation == self.owner_generation
            && !self.generation_exhausted
            && !self.revision_exhausted
            && self.fence.is_none_or(|fence| fence == identity.fence())
    }

    fn accepts_deadline_identity(&self, identity: &FrameStageIdentity) -> bool {
        self.accepts_identity(identity) && identity.stage == SchedulerStage::Deadline
    }
}

const fn stage_can_be_admitted(stage: SchedulerStage) -> bool {
    matches!(
        stage,
        SchedulerStage::Deadline
            | SchedulerStage::Projection
            | SchedulerStage::Layout
            | SchedulerStage::PaintPlan
            | SchedulerStage::EncodePresent
            | SchedulerStage::Maintenance
    )
}

#[cfg(test)]
mod tests {
    use super::super::native_resource_maintenance::NativeResourceMaintenanceSlot;
    use super::super::submission_completion::NativeSubmissionCompletionIdentity;
    use super::*;
    use std::time::Duration;

    fn adapter(serial: u64) -> NativeAdapterGeneration {
        NativeAdapterGeneration::from_test_serial(serial)
    }

    fn target(serial: u64) -> NativeTargetGeneration {
        NativeTargetGeneration::from_test_serial(serial)
    }

    fn identity(
        owner: &WindowStageOwner,
        key: FrameScheduleKey,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        stage: SchedulerStage,
        revision: u64,
    ) -> FrameStageIdentity {
        FrameStageIdentity::new(
            key,
            adapter_generation,
            target_generation,
            stage,
            owner.owner_generation(),
            revision,
        )
    }

    fn frame(due_at: Instant, caret: bool) -> TimedFrame {
        TimedFrame::new(
            due_at,
            if caret {
                RuntimeAnimationActivity::frame_messages()
            } else {
                RuntimeAnimationActivity::paint_only()
            },
            caret,
        )
    }

    fn frame_with_timed_repaint(due_at: Instant, caret: bool) -> TimedFrame {
        TimedFrame::with_operations(
            due_at,
            if caret {
                RuntimeAnimationActivity::frame_messages()
            } else {
                RuntimeAnimationActivity::paint_only()
            },
            caret,
            true,
            true,
            false,
        )
    }

    #[test]
    fn deadline_owner_coalesces_repaint_and_frame_operations_independently() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(
            exact.clone(),
            TimedFrame::timed_repaint(now + Duration::from_millis(10)),
        ));
        assert!(owner.queue(
            exact.clone(),
            TimedFrame::new(
                now + Duration::from_millis(20),
                RuntimeAnimationActivity::frame_messages(),
                true,
            ),
        ));

        let payload = owner
            .begin(&exact, now + Duration::from_millis(10))
            .expect("coalesced payload should use the earliest due time");
        assert_eq!(payload.due_at(), now + Duration::from_millis(10));
        assert!(payload.advance_timed_repaint());
        assert!(payload.drain_timed_frame());
        assert!(payload.needs_text_caret_animation());
        assert!(complete(&mut owner, &exact));
    }

    #[test]
    fn deadline_owner_coalesces_reissue_without_losing_timed_frame_evidence() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let reissue = TimedFrame::pending_redraw_reissue(now + Duration::from_millis(10));
        let timed_frame = TimedFrame::new(
            now + Duration::from_millis(20),
            RuntimeAnimationActivity::frame_messages(),
            true,
        );
        assert!(owner.queue(exact.clone(), reissue));
        assert!(owner.queue(exact.clone(), timed_frame));

        let payload = owner
            .begin(&exact, now + Duration::from_millis(10))
            .expect("coalesced reissue and drain should be due");
        assert!(payload.reissue_pending_redraw());
        assert!(payload.drain_timed_frame());
        assert!(!payload.advance_timed_repaint());
        assert_eq!(
            payload.animation_activity(),
            RuntimeAnimationActivity::frame_messages()
        );
        assert!(payload.needs_text_caret_animation());
        assert!(complete(&mut owner, &exact));
        assert!(!complete(&mut owner, &exact));
    }

    fn complete(owner: &mut WindowStageOwner, identity: &FrameStageIdentity) -> bool {
        let started_at = Instant::now();
        owner.complete(identity, started_at, Instant::now())
    }

    #[test]
    fn deadline_owner_coalesces_exact_pending_frames_and_retains_earliest_due_time() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert_eq!(owner.key(), &FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let first_revision = owner.next_revision().expect("first revision");
        let first = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            first_revision,
        );
        assert_eq!(first.key(), &FrameScheduleKey::Primary);
        assert_eq!(first.adapter_generation(), adapter(1));
        assert_eq!(first.target_generation(), target(1));
        assert_eq!(first.stage(), SchedulerStage::Deadline);
        assert!(owner.queue(first.clone(), frame(now + Duration::from_millis(10), false)));
        assert!(owner.queue(first.clone(), frame(now + Duration::from_millis(20), true)));
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::None);
        let payload = owner
            .begin(&first, now + Duration::from_millis(10))
            .expect("earliest coalesced due time should be retained");
        assert_eq!(payload.due_at(), now + Duration::from_millis(10));
        assert!(payload.needs_text_caret_animation());
        assert_eq!(
            payload.animation_activity(),
            RuntimeAnimationActivity::frame_messages()
        );
        assert!(complete(&mut owner, &first));
        assert!(owner.stale(&first));
        assert!(!complete(&mut owner, &first));
    }

    #[test]
    fn auxiliary_deadline_owner_is_bound_to_its_exact_schedule_key() {
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let owner = WindowStageOwner::new(key.clone());

        assert_eq!(owner.key(), &key);
        assert!(owner.owns_key(&key));
        assert!(!owner.owns_key(&FrameScheduleKey::Primary));
    }

    #[test]
    fn completion_retains_exact_identity_monotonic_elapsed_and_not_budgeted_status() {
        let now = Instant::now();
        let started_at = now + Duration::from_millis(2);
        let completed_at = started_at + Duration::from_millis(3);
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(exact.clone(), frame(now, false)));
        assert!(owner.begin(&exact, now).is_some());
        assert!(owner.complete(&exact, started_at, completed_at));

        let evidence = owner
            .last_completion
            .as_ref()
            .expect("completion evidence should be retained");
        assert_eq!(evidence.identity, exact);
        assert_eq!(evidence.frame.due_at(), now);
        assert!(!evidence.frame.advance_timed_repaint());
        assert_eq!(evidence.elapsed, Duration::from_millis(3));
        assert_eq!(evidence.budget_status, FrameStageBudgetStatus::NotBudgeted);
    }

    #[test]
    fn deadline_bundle_retains_selected_timed_repaint_evidence() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let bundle = frame_with_timed_repaint(now, true);
        assert!(owner.queue(exact.clone(), bundle));
        let begun = owner.begin(&exact, now).expect("due deadline bundle");
        assert_eq!(begun, bundle);
        assert!(begun.advance_timed_repaint());
        assert!(begun.needs_text_caret_animation());
        assert_eq!(
            begun.animation_activity(),
            RuntimeAnimationActivity::frame_messages()
        );
        assert!(owner.complete(&exact, now, now + Duration::from_millis(1)));
        assert_eq!(owner.completion_bundle(), Some(bundle));
    }

    #[test]
    fn deferred_drain_retains_owner_until_exact_resume() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let bundle = frame_with_timed_repaint(now, true);
        assert!(owner.queue(exact.clone(), bundle));
        assert_eq!(owner.begin(&exact, now), Some(bundle));
        assert!(owner.defer_timed_frame_drain(&exact));
        assert!(owner.has_deferred_timed_frame());
        assert!(!owner.complete(&exact, now, Instant::now()));
        assert_eq!(owner.completion_bundle(), None);
        assert_eq!(
            owner.resume_deferred_timed_frame(&exact.key, adapter(1), target(1)),
            Some(bundle)
        );
        assert!(!owner.has_deferred_timed_frame());
        assert!(owner.complete_deferred_timed_frame(Instant::now()));
        assert_eq!(owner.completion_bundle(), Some(bundle));

        owner.invalidate();
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn deferred_drain_resume_mismatch_cancels_without_completion() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let bundle = frame_with_timed_repaint(now, true);
        assert!(owner.queue(exact.clone(), bundle));
        assert_eq!(owner.begin(&exact, now), Some(bundle));
        assert!(owner.defer_timed_frame_drain(&exact));
        let owner_generation = owner.owner_generation();

        assert!(
            owner
                .resume_deferred_timed_frame(&exact.key, adapter(2), target(1))
                .is_none()
        );
        assert!(!owner.has_deferred_timed_frame());
        assert!(!owner.has_in_flight());
        assert_eq!(owner.completion_bundle(), None);
        assert!(owner.owner_generation() > owner_generation);
        assert!(owner.prepare_fence(adapter(2), target(1), SchedulerStage::Deadline));
    }

    #[test]
    fn deferred_drain_fence_transition_cancels_before_new_fence() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let bundle = frame_with_timed_repaint(now, true);
        assert!(owner.queue(exact.clone(), bundle));
        assert_eq!(owner.begin(&exact, now), Some(bundle));
        assert!(owner.defer_timed_frame_drain(&exact));
        let owner_generation = owner.owner_generation();

        assert!(owner.prepare_fence(adapter(1), target(2), SchedulerStage::Deadline));
        assert!(!owner.has_deferred_timed_frame());
        assert!(!owner.has_in_flight());
        assert_eq!(owner.completion_bundle(), None);
        assert!(owner.owner_generation() > owner_generation);
    }

    #[test]
    fn newer_pending_revision_replaces_payload_but_never_replaces_in_flight_work() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let first_revision = owner.next_revision().expect("first revision");
        let first = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            first_revision,
        );
        assert!(owner.queue(first.clone(), frame(now, false)));
        let first_payload = owner.begin(&first, now).expect("first frame is due");
        let second_revision = owner.next_revision().expect("second revision");
        let second = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            second_revision,
        );
        assert!(owner.queue(second.clone(), frame(now + Duration::from_millis(5), true)));
        assert!(!first_payload.needs_text_caret_animation());
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::None);
        assert!(!complete(&mut owner, &second));
        assert!(complete(&mut owner, &first));
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::None);
        let second_payload = owner
            .begin(&second, now + Duration::from_millis(5))
            .expect("newer pending frame should become ready");
        assert!(second_payload.needs_text_caret_animation());
        assert!(complete(&mut owner, &second));
    }

    #[test]
    fn begin_uses_exact_identity_and_current_time_as_vetoes() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(exact.clone(), frame(now + Duration::from_millis(4), false)));
        let wrong_revision = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision() + 1,
        );
        assert!(!owner.stale(&wrong_revision));
        assert!(
            owner
                .begin(&wrong_revision, now + Duration::from_millis(4))
                .is_none()
        );
        assert!(
            owner
                .begin(&exact, now + Duration::from_millis(3))
                .is_none()
        );
        assert!(
            owner
                .begin(&exact, now + Duration::from_millis(4))
                .is_some()
        );
    }

    #[test]
    fn generation_and_key_fences_reject_unknown_or_mismatched_work() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(!owner.prepare_fence(
            NativeAdapterGeneration::unknown(),
            target(1),
            SchedulerStage::Deadline
        ));
        assert!(!owner.prepare_fence(
            adapter(1),
            NativeTargetGeneration::unknown(),
            SchedulerStage::Deadline
        ));
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let wrong_key = FrameStageIdentity::new(
            FrameScheduleKey::Auxiliary(String::from("auxiliary")),
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision(),
        );
        let wrong_stage = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Layout,
            exact.owner_generation(),
            exact.revision(),
        );
        let wrong_adapter = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(2),
            target(1),
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision(),
        );
        let wrong_target = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(2),
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision(),
        );
        let wrong_owner_generation = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            exact.owner_generation() + 1,
            exact.revision(),
        );
        assert!(!owner.queue(wrong_key.clone(), frame(now, false)));
        assert!(!owner.queue(wrong_stage.clone(), frame(now, false)));
        assert!(!owner.queue(wrong_adapter.clone(), frame(now, false)));
        assert!(!owner.queue(wrong_target.clone(), frame(now, false)));
        assert!(!owner.queue(wrong_owner_generation.clone(), frame(now, false)));
        assert!(owner.stale(&wrong_key));
        assert!(owner.stale(&wrong_stage));
        assert!(owner.stale(&wrong_adapter));
        assert!(owner.stale(&wrong_target));
        assert!(owner.stale(&wrong_owner_generation));
        assert!(owner.queue(exact.clone(), frame(now, false)));
        owner.invalidate();
        assert!(owner.stale(&exact));
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::None);
        assert!(owner.prepare_fence(adapter(2), target(1), SchedulerStage::Deadline));
    }

    #[test]
    fn failed_completion_after_begin_has_no_second_drain_path() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(exact.clone(), frame(now, false)));
        assert!(owner.begin(&exact, now).is_some());

        let mut drain_count = 0;
        drain_count += 1;
        let wrong_completion = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision() + 1,
        );
        assert!(!owner.complete(&wrong_completion, now, now + Duration::from_millis(1)));
        assert_eq!(
            drain_count, 1,
            "a completion-evidence veto must not authorize fallback drainage"
        );
        assert!(owner.has_in_flight());
        assert!(
            owner
                .begin(&exact, now + Duration::from_millis(1))
                .is_none()
        );
    }

    #[test]
    fn known_generation_transitions_after_completion_veto_preserve_begun_owner() {
        let now = Instant::now();
        let adapter_generation = adapter(1);
        let target_generation = target(1);
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline
        ));
        let revision = owner.next_revision().expect("revision");
        let exact = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(exact.clone(), frame(now, false)));
        assert!(owner.begin(&exact, now).is_some());

        let wrong_completion = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter_generation,
            target_generation,
            SchedulerStage::Deadline,
            exact.owner_generation(),
            exact.revision() + 1,
        );
        assert!(!owner.complete(&wrong_completion, now, now + Duration::from_millis(1)));
        assert!(owner.has_in_flight());

        let mut next_adapter_generation = adapter_generation;
        assert!(next_adapter_generation.advance());
        assert!(!owner.prepare_fence(
            next_adapter_generation,
            target_generation,
            SchedulerStage::Deadline
        ));
        assert!(owner.has_in_flight());

        let mut next_target_generation = target_generation;
        assert!(next_target_generation.advance());
        assert!(!owner.prepare_fence(
            adapter_generation,
            next_target_generation,
            SchedulerStage::Deadline
        ));
        assert!(owner.has_in_flight());
        assert!(owner.complete(&exact, now, now + Duration::from_millis(1)));
    }

    #[test]
    fn checked_revision_and_generation_exhaustion_veto_without_payload_release() {
        let now = Instant::now();
        let mut revision_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        revision_owner.next_revision = u64::MAX;
        assert_eq!(revision_owner.next_revision(), None);
        assert!(revision_owner.revision_exhausted);
        assert!(revision_owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let generation_identity = FrameStageIdentity::new(
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision_owner.owner_generation(),
            u64::MAX,
        );
        assert!(!revision_owner.queue(generation_identity, TimedFrame::timed_repaint(now),));
        assert_eq!(revision_owner.next_ready_stage(now), NextReadyStage::None);

        let mut generation_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(generation_owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = generation_owner.next_revision().expect("revision");
        let identity = identity(
            &generation_owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(generation_owner.queue(identity, TimedFrame::timed_repaint(now)));
        generation_owner.owner_generation = u64::MAX;
        generation_owner.invalidate();
        assert!(generation_owner.generation_exhausted);
        assert!(!generation_owner.has_in_flight());
        assert_eq!(generation_owner.next_ready_stage(now), NextReadyStage::None);
        assert!(!generation_owner.prepare_fence(adapter(2), target(2), SchedulerStage::Deadline));
    }

    #[test]
    fn all_scheduler_stage_variants_are_explicit_and_native_stages_are_admitted() {
        for stage in SchedulerStage::ORDER {
            assert!(stage.is_non_preemptive());
        }
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::Lifecycle));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::DiscreteInput));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::ImmediateTransient));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::Deadline));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::Projection));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::Layout));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::PaintPlan));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::EncodePresent));
        assert!(SchedulerStage::ORDER.contains(&SchedulerStage::Maintenance));

        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        for stage in SchedulerStage::ORDER {
            assert_eq!(
                owner.prepare_fence(adapter(1), target(1), stage),
                matches!(
                    stage,
                    SchedulerStage::Deadline
                        | SchedulerStage::Projection
                        | SchedulerStage::Layout
                        | SchedulerStage::PaintPlan
                        | SchedulerStage::EncodePresent
                        | SchedulerStage::Maintenance
                )
            );
        }
        assert_eq!(NextReadyStage::default(), NextReadyStage::None);
        assert_eq!(NextReadyStage::Deadline, NextReadyStage::Deadline);
    }

    #[test]
    fn projection_completion_mismatch_cannot_replay_or_fallback() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let adapter_generation = adapter(1);
        let target_generation = target(1);
        let ticket = owner
            .admit_projection(adapter_generation, target_generation)
            .expect("projection ticket");
        let wrong_ticket = ProjectionStageTicket {
            identity: FrameStageIdentity::new(
                FrameScheduleKey::Primary,
                adapter_generation,
                target_generation,
                SchedulerStage::Projection,
                ticket.identity.owner_generation,
                ticket.identity.revision + 1,
            ),
        };

        assert!(!owner.complete_projection(wrong_ticket));
        assert!(owner.projection_ticket_is_current(&ticket));
        assert!(
            owner
                .admit_projection(adapter_generation, target_generation)
                .is_none()
        );
        assert!(owner.complete_projection(ticket));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn layout_admission_allows_known_generations_after_projection_completion() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.admit_layout(adapter(1), target(1)).is_none());
        assert!(owner.complete_projection(projection));

        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        assert_eq!(layout.identity().key(), &FrameScheduleKey::Primary);
        assert_eq!(layout.identity().adapter_generation(), adapter(1));
        assert_eq!(layout.identity().target_generation(), target(1));
        assert_eq!(layout.identity().stage(), SchedulerStage::Layout);
        assert_eq!(layout.identity().revision(), 2);
        assert!(owner.layout_ticket_is_current(&layout));
        assert!(owner.complete_layout(layout));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn layout_admission_rejects_unknown_generations_and_pending_work() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(
            owner
                .admit_layout(NativeAdapterGeneration::unknown(), target(1))
                .is_none()
        );
        assert!(
            owner
                .admit_layout(adapter(1), NativeTargetGeneration::unknown())
                .is_none()
        );

        let now = Instant::now();
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let pending = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(pending, frame(now, false)));
        assert!(owner.admit_layout(adapter(1), target(1)).is_none());
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::Deadline);
    }

    #[test]
    fn layout_completion_mismatch_leaves_the_exact_ticket_in_flight() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.complete_projection(projection));
        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        let wrong = LayoutStageTicket {
            identity: FrameStageIdentity::new(
                FrameScheduleKey::Primary,
                adapter(1),
                target(1),
                SchedulerStage::Layout,
                layout.identity().owner_generation(),
                layout.identity().revision() + 1,
            ),
        };

        assert!(!owner.complete_layout(wrong));
        assert!(owner.layout_ticket_is_current(&layout));
        assert!(owner.has_in_flight());
        assert!(owner.admit_layout(adapter(1), target(1)).is_none());
        assert!(owner.complete_layout(layout));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn layout_completion_marks_older_revision_stale_and_invalidate_clears_it() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.complete_projection(projection));
        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        let exact = layout.identity().clone();
        assert!(owner.complete_layout(layout));
        assert!(owner.stale(&exact));

        owner.invalidate();
        assert!(!owner.has_in_flight());
        assert!(owner.stale(&exact));

        let next = owner
            .admit_layout(adapter(1), target(1))
            .expect("next layout ticket");
        assert_eq!(next.identity().revision(), exact.revision() + 1);
        assert!(owner.complete_layout(next));
    }

    #[test]
    fn paint_plan_admission_allows_known_generations_after_layout_completion() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.complete_projection(projection));
        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        assert!(owner.complete_layout(layout));

        let paint_plan = owner
            .admit_paint_plan(adapter(1), target(1))
            .expect("paint-plan ticket");
        assert_eq!(paint_plan.identity().key(), &FrameScheduleKey::Primary);
        assert_eq!(paint_plan.identity().adapter_generation(), adapter(1));
        assert_eq!(paint_plan.identity().target_generation(), target(1));
        assert_eq!(paint_plan.identity().stage(), SchedulerStage::PaintPlan);
        assert_eq!(paint_plan.identity().revision(), 3);
        assert!(owner.paint_plan_ticket_is_current(&paint_plan));
        assert!(owner.complete_paint_plan(paint_plan));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn paint_plan_admission_rejects_unknown_generations_and_pending_work() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(
            owner
                .admit_paint_plan(NativeAdapterGeneration::unknown(), target(1))
                .is_none()
        );
        assert!(
            owner
                .admit_paint_plan(adapter(1), NativeTargetGeneration::unknown())
                .is_none()
        );

        let now = Instant::now();
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("revision");
        let pending = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        assert!(owner.queue(pending, frame(now, false)));
        assert!(owner.admit_paint_plan(adapter(1), target(1)).is_none());
        assert_eq!(owner.next_ready_stage(now), NextReadyStage::Deadline);
    }

    #[test]
    fn paint_plan_completion_mismatch_leaves_the_exact_ticket_in_flight() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.complete_projection(projection));
        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        assert!(owner.complete_layout(layout));
        let paint_plan = owner
            .admit_paint_plan(adapter(1), target(1))
            .expect("paint-plan ticket");
        let wrong = PaintPlanStageTicket {
            identity: FrameStageIdentity::new(
                FrameScheduleKey::Primary,
                adapter(1),
                target(1),
                SchedulerStage::PaintPlan,
                paint_plan.identity().owner_generation(),
                paint_plan.identity().revision() + 1,
            ),
        };

        assert!(!owner.complete_paint_plan(wrong));
        assert!(owner.paint_plan_ticket_is_current(&paint_plan));
        assert!(owner.has_in_flight());
        assert!(owner.admit_paint_plan(adapter(1), target(1)).is_none());
        assert!(owner.complete_paint_plan(paint_plan));
        assert!(!owner.has_in_flight());
    }

    #[test]
    fn paint_plan_completion_marks_older_revision_stale_and_invalidate_clears_it() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let projection = owner
            .admit_projection(adapter(1), target(1))
            .expect("projection ticket");
        assert!(owner.complete_projection(projection));
        let layout = owner
            .admit_layout(adapter(1), target(1))
            .expect("layout ticket");
        assert!(owner.complete_layout(layout));
        let paint_plan = owner
            .admit_paint_plan(adapter(1), target(1))
            .expect("paint-plan ticket");
        let exact = paint_plan.identity().clone();
        assert!(owner.complete_paint_plan(paint_plan));
        assert!(owner.stale(&exact));

        owner.invalidate();
        assert!(!owner.has_in_flight());
        assert!(owner.stale(&exact));

        let next = owner
            .admit_paint_plan(adapter(1), target(1))
            .expect("next paint-plan ticket");
        assert_eq!(next.identity().revision(), exact.revision() + 1);
        assert!(owner.complete_paint_plan(next));
    }

    #[test]
    fn maintenance_ticket_is_exact_and_follows_encode_present_stage() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let binding = NativeResourceMaintenanceBinding::new(
            NativeResourceMaintenanceSlot::Quarantine(0),
            adapter(1),
            NativeSubmissionCompletionIdentity::never_submitted(adapter(1)),
        );
        let mismatched_witness = NativeResourceMaintenanceBinding::new(
            NativeResourceMaintenanceSlot::Quarantine(0),
            adapter(1),
            NativeSubmissionCompletionIdentity::never_submitted(adapter(2)),
        );
        assert!(
            owner
                .admit_maintenance(adapter(2), target(1), mismatched_witness)
                .is_none()
        );
        let ticket = owner
            .admit_maintenance(adapter(2), target(1), binding)
            .expect("known maintenance evidence should admit");
        assert_eq!(ticket.identity().stage(), SchedulerStage::Maintenance);
        assert_eq!(ticket.identity().adapter_generation(), adapter(2));
        assert_eq!(ticket.binding(), binding);

        let wrong = MaintenanceStageTicket {
            identity: FrameStageIdentity::new(
                FrameScheduleKey::Primary,
                adapter(2),
                target(1),
                SchedulerStage::Maintenance,
                ticket.identity().owner_generation(),
                ticket.identity().revision() + 1,
            ),
            binding,
            owner_token: ticket.owner_token,
        };
        assert!(!owner.complete_maintenance(wrong));
        assert!(owner.maintenance_ticket_is_current(&ticket));
        assert!(owner.complete_maintenance(ticket));
        assert!(!owner.has_in_flight());
        let owner_generation = owner.owner_generation();
        owner.invalidate();
        assert_eq!(owner.owner_generation(), owner_generation + 1);
    }

    #[test]
    fn invalid_maintenance_witness_preserves_completed_deadline_owner() {
        let now = Instant::now();
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        assert!(owner.prepare_fence(adapter(1), target(1), SchedulerStage::Deadline));
        let revision = owner.next_revision().expect("deadline revision");
        let completed_identity = identity(
            &owner,
            FrameScheduleKey::Primary,
            adapter(1),
            target(1),
            SchedulerStage::Deadline,
            revision,
        );
        let completed_bundle = frame(now, false);
        assert!(owner.queue(completed_identity.clone(), completed_bundle));
        assert_eq!(
            owner.begin(&completed_identity, now),
            Some(completed_bundle)
        );
        assert!(owner.complete(&completed_identity, now, now + Duration::from_millis(1)));

        let owner_generation = owner.owner_generation();
        let fence = owner.fence;
        let completion = owner.last_completion.clone();
        assert_eq!(owner.completion_bundle(), Some(completed_bundle));
        assert!(owner.stale(&completed_identity));

        let invalid_bindings = [
            NativeResourceMaintenanceBinding::new(
                NativeResourceMaintenanceSlot::Quarantine(0),
                NativeAdapterGeneration::unknown(),
                NativeSubmissionCompletionIdentity::never_submitted(
                    NativeAdapterGeneration::unknown(),
                ),
            ),
            NativeResourceMaintenanceBinding::new(
                NativeResourceMaintenanceSlot::Quarantine(0),
                adapter(1),
                NativeSubmissionCompletionIdentity::never_submitted(adapter(2)),
            ),
        ];

        for binding in invalid_bindings {
            assert!(
                owner
                    .admit_maintenance(adapter(2), target(1), binding)
                    .is_none()
            );
            assert_eq!(owner.owner_generation(), owner_generation);
            assert_eq!(owner.fence, fence);
            assert_eq!(owner.last_completion, completion);
            assert_eq!(owner.completion_bundle(), Some(completed_bundle));
            assert!(!owner.has_in_flight());
            assert!(owner.stale(&completed_identity));
        }
    }
}
