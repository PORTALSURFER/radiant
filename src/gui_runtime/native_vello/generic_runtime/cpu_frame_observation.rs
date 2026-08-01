//! Crate-private, bounded CPU-frame evidence for the native application owner.
//!
//! This module records what the existing native frame path did.  It does not
//! select a window, request a redraw, or call a runner.  The parent runtime
//! owns the ledger; child runners only expose an ephemeral capture for the
//! parent to commit after their redraw path returns.

use super::{FrameScheduleKey, FrameWork};
use crate::runtime::{RepaintScope, SurfaceInvalidation, SurfaceRefreshDiagnostics};
use std::time::Duration;

const LATEST_SAMPLE_CAPACITY: usize = 4;

/// A duration whose absence cannot be confused with a zero-length stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFrameDuration {
    /// A duration measured by an existing conditional timing path.
    Known(Duration),
    /// Timing was not available for this completed observation.
    #[default]
    Unknown,
}

impl CpuFrameDuration {
    fn from_recording(record_timings: bool, duration: Duration) -> Self {
        if record_timings {
            Self::Known(duration)
        } else {
            Self::Unknown
        }
    }
}

/// Closed vocabulary for the CPU stages already represented by native frame
/// diagnostics and the render profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CpuFrameStage {
    CoalescedWheelRoute,
    RefreshSurface,
    ApplicationProjection,
    RuntimeProjection,
    WidgetStateSync,
    Layout,
    PaintPlan,
    DeferredSceneRebuild,
    RenderToTexture,
    CompositedBaseRefresh,
    TransientOverlayPaint,
    FullScreenBlit,
    SubmitPresent,
}

impl CpuFrameStage {
    pub(super) const COUNT: usize = 13;

    const fn index(self) -> usize {
        match self {
            Self::CoalescedWheelRoute => 0,
            Self::RefreshSurface => 1,
            Self::ApplicationProjection => 2,
            Self::RuntimeProjection => 3,
            Self::WidgetStateSync => 4,
            Self::Layout => 5,
            Self::PaintPlan => 6,
            Self::DeferredSceneRebuild => 7,
            Self::RenderToTexture => 8,
            Self::CompositedBaseRefresh => 9,
            Self::TransientOverlayPaint => 10,
            Self::FullScreenBlit => 11,
            Self::SubmitPresent => 12,
        }
    }
}

/// Completion and timing evidence for one closed CPU stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFrameStageObservation {
    #[default]
    NotCompleted,
    Completed(CpuFrameDuration),
}

/// Ephemeral evidence collected while one runner is inside `redraw`.
///
/// This is deliberately not a history or an owner.  The application-level
/// parent takes it after the redraw returns and commits it to the bounded
/// ledger.  A child runner therefore cannot retain observations across
/// retirement or reinsertion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameObservationCapture {
    stages: [CpuFrameStageObservation; CpuFrameStage::COUNT],
    frame_work: Option<FrameWork>,
    invalidation: Option<SurfaceInvalidation>,
    frame_path_started: bool,
    successful_presentation: bool,
    recovery_triggered: bool,
}

impl CpuFrameObservationCapture {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn record_stage(
        &mut self,
        stage: CpuFrameStage,
        completed: bool,
        duration: CpuFrameDuration,
    ) {
        if completed {
            self.stages[stage.index()] = CpuFrameStageObservation::Completed(duration);
        }
    }

    pub(super) fn record_profile_stage(
        &mut self,
        stage: CpuFrameStage,
        completed: bool,
        record_timings: bool,
        duration: Duration,
    ) {
        self.record_stage(
            stage,
            completed,
            CpuFrameDuration::from_recording(record_timings, duration),
        );
    }

    pub(super) fn record_refresh_diagnostics(
        &mut self,
        refresh: SurfaceRefreshDiagnostics,
        total: Duration,
        effective_scope: RepaintScope,
    ) {
        if refresh.invalidation == SurfaceInvalidation::None {
            return;
        }

        self.invalidation = Some(refresh.invalidation);
        self.record_stage(
            CpuFrameStage::RefreshSurface,
            true,
            CpuFrameDuration::Known(total),
        );

        if refresh.invalidation == SurfaceInvalidation::PaintOnly {
            return;
        }

        self.record_stage(
            CpuFrameStage::ApplicationProjection,
            true,
            CpuFrameDuration::Known(refresh.timings.application_projection),
        );
        self.record_stage(
            CpuFrameStage::RuntimeProjection,
            true,
            CpuFrameDuration::Known(refresh.timings.runtime_projection),
        );
        self.record_stage(
            CpuFrameStage::WidgetStateSync,
            true,
            CpuFrameDuration::Known(refresh.timings.widget_state_sync),
        );

        let layout_completed =
            effective_scope.refreshes_layout() && !refresh.timings.layout.is_zero();
        self.record_stage(
            CpuFrameStage::Layout,
            layout_completed,
            CpuFrameDuration::Known(refresh.timings.layout),
        );
    }

    pub(super) fn record_frame_work(&mut self, frame_work: FrameWork) {
        self.frame_work = Some(frame_work);
    }

    pub(super) fn mark_frame_path_started(&mut self) {
        self.frame_path_started = true;
    }

    pub(super) fn mark_successful_presentation(&mut self) {
        self.successful_presentation = true;
    }

    pub(super) fn mark_recovery_triggered(&mut self) {
        self.recovery_triggered = true;
    }

    pub(super) const fn has_completed_stage(&self) -> bool {
        let mut index = 0;
        while index < self.stages.len() {
            if matches!(self.stages[index], CpuFrameStageObservation::Completed(_)) {
                return true;
            }
            index += 1;
        }
        false
    }

    pub(super) const fn successful_presentation(&self) -> bool {
        self.successful_presentation
    }

    pub(super) const fn frame_path_started(&self) -> bool {
        self.frame_path_started
    }

    pub(super) const fn recovery_triggered(&self) -> bool {
        self.recovery_triggered
    }
}

/// Snapshot captured immediately before an existing native redraw path.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CpuFrameObservationAdmission {
    key: FrameScheduleKey,
    frame_work: FrameWork,
    cadence_target_fps: Option<u32>,
    deadline_age: CpuFrameDuration,
}

/// Mutually exclusive completion state for an admitted redraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CpuFrameCompletionOutcome {
    SuccessfulPresentation,
    SkippedOrVetoed,
    Incomplete,
    Failed,
}

/// One bounded latest-sample record retained for a stable schedule key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameObservationSample {
    pub(super) key: FrameScheduleKey,
    pub(super) frame_work: FrameWork,
    pub(super) invalidation: SurfaceInvalidation,
    pub(super) cadence_target_fps: Option<u32>,
    pub(super) deadline_age: CpuFrameDuration,
    pub(super) outcome: CpuFrameCompletionOutcome,
    pub(super) recovery_triggered: bool,
    pub(super) stages: [CpuFrameStageObservation; CpuFrameStage::COUNT],
}

/// Saturating per-key totals kept alongside the latest bounded samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameObservationCounters {
    pub(super) admitted_redraws: u64,
    pub(super) successful_presentations: u64,
    pub(super) skipped_or_vetoed_redraws: u64,
    pub(super) incomplete_frames: u64,
    pub(super) failed_frames: u64,
    pub(super) recovery_triggered_frames: u64,
}

impl CpuFrameObservationCounters {
    fn record_completion(&mut self, outcome: CpuFrameCompletionOutcome, recovery: bool) {
        match outcome {
            CpuFrameCompletionOutcome::SuccessfulPresentation => {
                self.successful_presentations = self.successful_presentations.saturating_add(1);
            }
            CpuFrameCompletionOutcome::SkippedOrVetoed => {
                self.skipped_or_vetoed_redraws = self.skipped_or_vetoed_redraws.saturating_add(1);
            }
            CpuFrameCompletionOutcome::Incomplete => {
                self.incomplete_frames = self.incomplete_frames.saturating_add(1);
            }
            CpuFrameCompletionOutcome::Failed => {
                self.failed_frames = self.failed_frames.saturating_add(1);
            }
        }
        if recovery {
            self.recovery_triggered_frames = self.recovery_triggered_frames.saturating_add(1);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CpuFrameObservationState {
    key: FrameScheduleKey,
    counters: CpuFrameObservationCounters,
    samples: [Option<CpuFrameObservationSample>; LATEST_SAMPLE_CAPACITY],
    next_sample: usize,
    sample_count: usize,
}

impl CpuFrameObservationState {
    fn new(key: FrameScheduleKey) -> Self {
        Self {
            key,
            counters: CpuFrameObservationCounters::default(),
            samples: std::array::from_fn(|_| None),
            next_sample: 0,
            sample_count: 0,
        }
    }

    fn append(&mut self, sample: CpuFrameObservationSample) {
        self.samples[self.next_sample] = Some(sample);
        self.next_sample = (self.next_sample + 1) % LATEST_SAMPLE_CAPACITY;
        self.sample_count = self
            .sample_count
            .saturating_add(1)
            .min(LATEST_SAMPLE_CAPACITY);
    }
}

/// Application-level owner of bounded CPU-frame evidence for all windows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameObservationLedger {
    states: Vec<CpuFrameObservationState>,
}

impl CpuFrameObservationLedger {
    pub(super) fn begin(
        &mut self,
        key: FrameScheduleKey,
        frame_work: FrameWork,
        cadence_target_fps: Option<u32>,
        deadline_age: CpuFrameDuration,
    ) -> CpuFrameObservationAdmission {
        let state = self.state_or_insert(key.clone());
        state.counters.admitted_redraws = state.counters.admitted_redraws.saturating_add(1);
        CpuFrameObservationAdmission {
            key,
            frame_work,
            cadence_target_fps,
            deadline_age,
        }
    }

    pub(super) fn finish(
        &mut self,
        admission: CpuFrameObservationAdmission,
        capture: CpuFrameObservationCapture,
        redraw_failed: bool,
    ) {
        let Some(state) = self
            .states
            .iter_mut()
            .find(|state| state.key == admission.key)
        else {
            return;
        };
        let outcome =
            if capture.successful_presentation() && !redraw_failed && !capture.recovery_triggered()
            {
                CpuFrameCompletionOutcome::SuccessfulPresentation
            } else if redraw_failed || capture.recovery_triggered() {
                CpuFrameCompletionOutcome::Failed
            } else if capture.has_completed_stage() || capture.frame_path_started() {
                CpuFrameCompletionOutcome::Incomplete
            } else {
                CpuFrameCompletionOutcome::SkippedOrVetoed
            };
        let recovery_triggered = capture.recovery_triggered();
        state
            .counters
            .record_completion(outcome, recovery_triggered);
        state.append(CpuFrameObservationSample {
            key: admission.key,
            frame_work: capture.frame_work.unwrap_or(admission.frame_work),
            invalidation: capture.invalidation.unwrap_or(SurfaceInvalidation::None),
            cadence_target_fps: admission.cadence_target_fps,
            deadline_age: admission.deadline_age,
            outcome,
            recovery_triggered,
            stages: capture.stages,
        });
    }

    pub(super) fn remove(&mut self, key: &FrameScheduleKey) {
        self.states.retain(|state| &state.key != key);
    }

    #[cfg(test)]
    fn state(&self, key: &FrameScheduleKey) -> Option<&CpuFrameObservationState> {
        self.states.iter().find(|state| &state.key == key)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.states.len()
    }

    #[cfg(test)]
    pub(super) fn counters_for_test(
        &self,
        key: &FrameScheduleKey,
    ) -> Option<CpuFrameObservationCounters> {
        self.state(key).map(|state| state.counters)
    }

    fn state_or_insert(&mut self, key: FrameScheduleKey) -> &mut CpuFrameObservationState {
        if let Some(index) = self.states.iter().position(|state| state.key == key) {
            return &mut self.states[index];
        }
        self.states.push(CpuFrameObservationState::new(key));
        let index = self.states.len() - 1;
        &mut self.states[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::{FrameWorkReason, SceneRebuildMode};
    use std::time::Duration;

    fn key(name: &str) -> FrameScheduleKey {
        FrameScheduleKey::Auxiliary(name.to_owned())
    }

    fn admitted(ledger: &mut CpuFrameObservationLedger, key: FrameScheduleKey) {
        let admission = ledger.begin(
            key,
            FrameWork::PaintOnly {
                reason: FrameWorkReason::PointerHover,
            },
            Some(60),
            CpuFrameDuration::Known(Duration::from_millis(2)),
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_frame_work(admission.frame_work);
        capture.mark_successful_presentation();
        capture.record_stage(
            CpuFrameStage::SubmitPresent,
            true,
            CpuFrameDuration::Known(Duration::from_micros(1)),
        );
        ledger.finish(admission, capture, false);
    }

    #[test]
    fn latest_samples_are_bounded_per_stable_key() {
        let mut ledger = CpuFrameObservationLedger::default();
        for _ in 0..(LATEST_SAMPLE_CAPACITY + 3) {
            admitted(&mut ledger, key("settings"));
        }

        let state = ledger
            .state(&key("settings"))
            .expect("admitted key should have state");
        assert_eq!(state.key, key("settings"));
        assert_eq!(state.sample_count, LATEST_SAMPLE_CAPACITY);
        assert_eq!(state.samples.len(), LATEST_SAMPLE_CAPACITY);
    }

    #[test]
    fn reorder_does_not_move_history_between_stable_keys() {
        let mut ledger = CpuFrameObservationLedger::default();
        admitted(&mut ledger, key("first"));
        admitted(&mut ledger, key("second"));

        let first_key = key("first");
        let second_key = key("second");
        assert_eq!(ledger.state(&first_key).unwrap().key, first_key);
        assert_eq!(ledger.state(&second_key).unwrap().key, second_key);
        assert_eq!(
            ledger.state(&first_key).unwrap().counters.admitted_redraws,
            1
        );
        assert_eq!(
            ledger.state(&second_key).unwrap().counters.admitted_redraws,
            1
        );
    }

    #[test]
    fn removal_drops_history_and_reinsertion_starts_fresh() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        admitted(&mut ledger, stable_key.clone());
        ledger.remove(&stable_key);
        assert_eq!(ledger.len(), 0);

        admitted(&mut ledger, stable_key.clone());
        let state = ledger
            .state(&stable_key)
            .expect("reinsertion should be fresh");
        assert_eq!(state.counters.admitted_redraws, 1);
        assert_eq!(state.sample_count, 1);
    }

    #[test]
    fn counters_saturate_without_wrapping() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let state = ledger.state_or_insert(stable_key.clone());
        state.counters.admitted_redraws = u64::MAX;
        state.counters.successful_presentations = u64::MAX;
        state.counters.recovery_triggered_frames = u64::MAX;

        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFrameDuration::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.mark_successful_presentation();
        capture.mark_recovery_triggered();
        ledger.finish(admission, capture, false);

        let counters = ledger.state(&stable_key).unwrap().counters;
        assert_eq!(counters.admitted_redraws, u64::MAX);
        assert_eq!(counters.successful_presentations, u64::MAX);
        assert_eq!(counters.recovery_triggered_frames, u64::MAX);
    }

    #[test]
    fn unavailable_timing_is_not_encoded_as_zero() {
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_profile_stage(CpuFrameStage::Layout, true, false, Duration::ZERO);

        assert_eq!(
            capture.stages[CpuFrameStage::Layout.index()],
            CpuFrameStageObservation::Completed(CpuFrameDuration::Unknown)
        );
        assert_ne!(
            CpuFrameDuration::Unknown,
            CpuFrameDuration::Known(Duration::ZERO)
        );
    }

    #[test]
    fn failed_and_recovery_frames_never_count_as_presented() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            },
            Some(60),
            CpuFrameDuration::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.mark_successful_presentation();
        capture.mark_recovery_triggered();
        ledger.finish(admission, capture, true);

        let state = ledger.state(&stable_key).unwrap();
        assert_eq!(state.counters.successful_presentations, 0);
        assert_eq!(state.counters.failed_frames, 1);
        assert_eq!(state.counters.recovery_triggered_frames, 1);
        assert_eq!(
            state.samples[0].as_ref().unwrap().outcome,
            CpuFrameCompletionOutcome::Failed
        );
    }

    #[test]
    fn admission_accounting_is_one_record_per_finished_token() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFrameDuration::Unknown,
        );
        ledger.finish(admission, CpuFrameObservationCapture::default(), false);

        let state = ledger.state(&stable_key).unwrap();
        assert_eq!(state.counters.admitted_redraws, 1);
        assert_eq!(state.counters.skipped_or_vetoed_redraws, 1);
        assert_eq!(state.sample_count, 1);
    }

    #[test]
    fn started_but_unpresented_frame_is_incomplete_not_skipped() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFrameDuration::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.mark_frame_path_started();
        ledger.finish(admission, capture, false);

        let counters = ledger.state(&stable_key).unwrap().counters;
        assert_eq!(counters.incomplete_frames, 1);
        assert_eq!(counters.skipped_or_vetoed_redraws, 0);
    }

    #[test]
    fn observation_metadata_does_not_select_or_reorder_scheduler_work() {
        use super::super::{
            FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleRedrawEvidence,
            NativeFrameScheduler,
        };
        use crate::runtime::RuntimeAnimationActivity;
        use std::time::Instant;

        let now = Instant::now();
        let demands = [
            FrameScheduleDemand::from_cadence(
                FrameScheduleKey::Primary,
                super::super::TimedFrameCadence::DrainNow {
                    next_wake: now + Duration::from_millis(16),
                },
                60,
                RuntimeAnimationActivity::paint_only(),
                false,
                FrameScheduleRedrawEvidence::default(),
            ),
            FrameScheduleDemand::from_cadence(
                key("settings"),
                super::super::TimedFrameCadence::DrainNow {
                    next_wake: now + Duration::from_millis(16),
                },
                60,
                RuntimeAnimationActivity::paint_only(),
                false,
                FrameScheduleRedrawEvidence::default(),
            ),
        ];
        let scheduler = NativeFrameScheduler::default();
        let before = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            before.selected.clone().unwrap(),
            FrameWork::None,
            Some(60),
            CpuFrameDuration::Known(Duration::ZERO),
        );
        let after = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        ledger.finish(admission, CpuFrameObservationCapture::default(), false);

        assert_eq!(before.selected, after.selected);
    }
}
