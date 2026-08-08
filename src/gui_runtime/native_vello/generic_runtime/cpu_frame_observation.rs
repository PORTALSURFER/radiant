//! Crate-private, bounded CPU-frame evidence for the native application owner.
//!
//! This module records what the existing native frame path did.  It does not
//! select a window, request a redraw, or call a runner.  The parent runtime
//! owns the ledger; child runners only expose an ephemeral capture for the
//! parent to commit after their redraw path returns.

use super::{FrameScheduleKey, FrameWork, FrameWorkReason};
use crate::runtime::{
    NativeCpuFrameCompletionOutcome, NativeCpuFrameObservationDiagnostics, RepaintScope,
    SurfaceInvalidation, SurfaceRefreshDiagnostics,
};
use std::time::Duration;

pub(super) const LATEST_SAMPLE_CAPACITY: usize = 4;

/// The parent keeps a bounded number of stable schedule identities. A key
/// that cannot be admitted because this bound is full is deliberately omitted
/// from shadow evidence; it never affects scheduler operation.
pub(super) const CPU_FRAME_OBSERVATION_KEY_CAPACITY: usize = 16;

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

/// Age evidence for a redraw request. This is kept separate from cadence
/// lateness because a pending redraw can coexist with an idle or waiting
/// timed cadence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFramePendingRedrawAge {
    /// No redraw request was pending at the observation boundary.
    #[default]
    NotRequested,
    /// A request was pending and its age was measured from its exact request
    /// instant.
    Known(Duration),
    /// A request was reported without an exact request instant.
    Unknown,
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

    pub(super) const fn index(self) -> usize {
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
    exact_interaction: bool,
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
        if matches!(
            frame_work.reason(),
            FrameWorkReason::RoutedInput | FrameWorkReason::PointerHover
        ) {
            self.exact_interaction = true;
        }
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

    pub(super) const fn exact_interaction(&self) -> bool {
        self.exact_interaction
    }
}

/// Snapshot captured immediately before an existing native redraw path.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CpuFrameObservationAdmission {
    key: FrameScheduleKey,
    tracked: bool,
    frame_work: FrameWork,
    cadence_target_fps: Option<u32>,
    pending_redraw_age: CpuFramePendingRedrawAge,
}

/// Parent-owned observation boundary borrowed by an auxiliary runner while it
/// performs a synchronous route-time redraw.
///
/// The child owns only the ephemeral capture.  The parent creates this scope
/// for the auxiliary key and commits each completed redraw through its ledger.
pub(super) struct CpuFrameObservationOwner<'a> {
    ledger: &'a mut CpuFrameObservationLedger,
    key: FrameScheduleKey,
}

impl<'a> CpuFrameObservationOwner<'a> {
    pub(super) fn new(ledger: &'a mut CpuFrameObservationLedger, key: FrameScheduleKey) -> Self {
        Self { ledger, key }
    }

    pub(super) fn begin(
        &mut self,
        frame_work: FrameWork,
        cadence_target_fps: Option<u32>,
        pending_redraw_age: CpuFramePendingRedrawAge,
    ) -> CpuFrameObservationAdmission {
        self.ledger.begin(
            self.key.clone(),
            frame_work,
            cadence_target_fps,
            pending_redraw_age,
        )
    }

    pub(super) fn finish(
        &mut self,
        admission: CpuFrameObservationAdmission,
        capture: CpuFrameObservationCapture,
        redraw_failed: bool,
    ) {
        self.ledger.finish(admission, capture, redraw_failed);
    }
}

/// Mutually exclusive completion state for an admitted redraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CpuFrameCompletionOutcome {
    SuccessfulPresentation,
    SkippedOrVetoed,
    Incomplete,
    Failed,
    RecoveryTriggered,
}

/// One bounded latest-sample record retained for a stable schedule key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameObservationSample {
    pub(super) key: FrameScheduleKey,
    pub(super) frame_work: FrameWork,
    pub(super) invalidation: SurfaceInvalidation,
    pub(super) cadence_target_fps: Option<u32>,
    pub(super) pending_redraw_age: CpuFramePendingRedrawAge,
    pub(super) outcome: CpuFrameCompletionOutcome,
    pub(super) recovery_triggered: bool,
    pub(super) exact_interaction: bool,
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
            CpuFrameCompletionOutcome::RecoveryTriggered => {
                self.failed_frames = self.failed_frames.saturating_add(1);
                self.recovery_triggered_frames = self.recovery_triggered_frames.saturating_add(1);
            }
        }
        if recovery && !matches!(outcome, CpuFrameCompletionOutcome::RecoveryTriggered) {
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

    fn latest_sample(&self) -> Option<&CpuFrameObservationSample> {
        if self.sample_count == 0 {
            return None;
        }
        let index = (self.next_sample + LATEST_SAMPLE_CAPACITY - 1) % LATEST_SAMPLE_CAPACITY;
        self.samples[index].as_ref()
    }
}

/// Borrowed, read-only view of the bounded observation ledger.
pub(super) struct CpuFrameObservationProjection<'a> {
    states: &'a [CpuFrameObservationState],
}

impl<'a> CpuFrameObservationProjection<'a> {
    pub(super) fn window(
        &self,
        key: &FrameScheduleKey,
    ) -> Option<CpuFrameObservationWindowProjection<'a>> {
        self.states
            .iter()
            .find(|state| &state.key == key)
            .map(|state| CpuFrameObservationWindowProjection { state })
    }
}

/// Read-only projection of one stable schedule key's bounded evidence.
pub(super) struct CpuFrameObservationWindowProjection<'a> {
    state: &'a CpuFrameObservationState,
}

impl<'a> CpuFrameObservationWindowProjection<'a> {
    pub(super) const fn counters(&self) -> CpuFrameObservationCounters {
        self.state.counters
    }

    pub(super) fn latest_sample(&self) -> Option<&'a CpuFrameObservationSample> {
        self.state.latest_sample()
    }

    pub(super) fn samples(&self) -> &[Option<CpuFrameObservationSample>; LATEST_SAMPLE_CAPACITY] {
        &self.state.samples
    }

    pub(super) fn sample_count(&self) -> usize {
        self.state.sample_count
    }
}

/// Application-level owner of bounded CPU-frame evidence for all windows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameObservationLedger {
    states: Vec<CpuFrameObservationState>,
}

impl CpuFrameObservationLedger {
    pub(super) fn projection(&self) -> CpuFrameObservationProjection<'_> {
        CpuFrameObservationProjection {
            states: &self.states,
        }
    }

    /// Project one stable schedule key into the public, observational summary
    /// without exposing the private ledger, sample, or stage types.
    pub(super) fn project_frame_diagnostics(
        &self,
        key: &FrameScheduleKey,
    ) -> NativeCpuFrameObservationDiagnostics {
        let Some(window) = self.projection().window(key) else {
            return NativeCpuFrameObservationDiagnostics::default();
        };
        let Some(sample) = window.latest_sample() else {
            return NativeCpuFrameObservationDiagnostics::default();
        };
        let counters = window.counters();
        NativeCpuFrameObservationDiagnostics {
            available: true,
            latest_outcome: public_completion_outcome(sample.outcome),
            latest_exact_interaction: sample.exact_interaction,
            admitted_redraws: counters.admitted_redraws,
            successful_presentations: counters.successful_presentations,
            skipped_or_vetoed_redraws: counters.skipped_or_vetoed_redraws,
            incomplete_frames: counters.incomplete_frames,
            failed_frames: counters.failed_frames,
            recovery_triggered_frames: counters.recovery_triggered_frames,
        }
    }

    pub(super) fn clear(&mut self) {
        self.states.clear();
    }

    pub(super) fn begin(
        &mut self,
        key: FrameScheduleKey,
        frame_work: FrameWork,
        cadence_target_fps: Option<u32>,
        pending_redraw_age: CpuFramePendingRedrawAge,
    ) -> CpuFrameObservationAdmission {
        let tracked = if let Some(state) = self.state_or_insert(key.clone()) {
            state.counters.admitted_redraws = state.counters.admitted_redraws.saturating_add(1);
            true
        } else {
            false
        };
        CpuFrameObservationAdmission {
            key,
            tracked,
            frame_work,
            cadence_target_fps,
            pending_redraw_age,
        }
    }

    pub(super) fn finish(
        &mut self,
        admission: CpuFrameObservationAdmission,
        capture: CpuFrameObservationCapture,
        redraw_failed: bool,
    ) {
        if !admission.tracked {
            return;
        }
        let recovery_triggered = capture.recovery_triggered();
        if recovery_triggered {
            // A recovery transition establishes a new evidence epoch. Keep
            // the triggering sample, but do not let pre-recovery history
            // influence its replacement window.
            self.remove(&admission.key);
        }
        let Some(state) = self.state_or_insert(admission.key.clone()) else {
            return;
        };
        if recovery_triggered {
            state.counters.admitted_redraws = state.counters.admitted_redraws.saturating_add(1);
        }
        let outcome = if recovery_triggered {
            CpuFrameCompletionOutcome::RecoveryTriggered
        } else if capture.successful_presentation() && !redraw_failed {
            CpuFrameCompletionOutcome::SuccessfulPresentation
        } else if redraw_failed {
            CpuFrameCompletionOutcome::Failed
        } else if capture.has_completed_stage() || capture.frame_path_started() {
            CpuFrameCompletionOutcome::Incomplete
        } else {
            CpuFrameCompletionOutcome::SkippedOrVetoed
        };
        state
            .counters
            .record_completion(outcome, recovery_triggered);
        state.append(CpuFrameObservationSample {
            key: admission.key,
            frame_work: capture.frame_work.unwrap_or(admission.frame_work),
            invalidation: capture.invalidation.unwrap_or(SurfaceInvalidation::None),
            cadence_target_fps: admission.cadence_target_fps,
            pending_redraw_age: admission.pending_redraw_age,
            outcome,
            recovery_triggered,
            exact_interaction: capture.exact_interaction(),
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

    #[cfg(test)]
    pub(super) fn sample_count_for_test(&self, key: &FrameScheduleKey) -> Option<usize> {
        self.state(key).map(|state| state.sample_count)
    }

    fn state_or_insert(&mut self, key: FrameScheduleKey) -> Option<&mut CpuFrameObservationState> {
        if let Some(index) = self.states.iter().position(|state| state.key == key) {
            return Some(&mut self.states[index]);
        }
        if self.states.len() >= CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            return None;
        }
        self.states.push(CpuFrameObservationState::new(key));
        let index = self.states.len() - 1;
        Some(&mut self.states[index])
    }
}

fn public_completion_outcome(
    outcome: CpuFrameCompletionOutcome,
) -> NativeCpuFrameCompletionOutcome {
    match outcome {
        CpuFrameCompletionOutcome::SuccessfulPresentation => {
            NativeCpuFrameCompletionOutcome::SuccessfulPresentation
        }
        CpuFrameCompletionOutcome::SkippedOrVetoed => {
            NativeCpuFrameCompletionOutcome::SkippedOrVetoed
        }
        CpuFrameCompletionOutcome::Incomplete => NativeCpuFrameCompletionOutcome::Incomplete,
        CpuFrameCompletionOutcome::Failed => NativeCpuFrameCompletionOutcome::Failed,
        CpuFrameCompletionOutcome::RecoveryTriggered => {
            NativeCpuFrameCompletionOutcome::RecoveryTriggered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::{FrameWorkReason, SceneRebuildMode};
    use crate::runtime::{NativeCpuFrameCompletionOutcome, NativeCpuFrameObservationDiagnostics};
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
            CpuFramePendingRedrawAge::Known(Duration::from_millis(2)),
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

        let projection = ledger.projection();
        assert_eq!(
            projection
                .window(&first_key)
                .unwrap()
                .counters()
                .admitted_redraws,
            1
        );
        assert_eq!(projection.window(&second_key).unwrap().sample_count(), 1);
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
    fn public_projection_is_unavailable_without_a_completed_sample() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        assert_eq!(
            ledger.project_frame_diagnostics(&stable_key),
            NativeCpuFrameObservationDiagnostics::default()
        );

        let _admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFramePendingRedrawAge::Unknown,
        );
        assert_eq!(
            ledger.project_frame_diagnostics(&stable_key),
            NativeCpuFrameObservationDiagnostics::default()
        );
    }

    #[test]
    fn public_projection_maps_each_completion_outcome_and_exact_interaction() {
        for (private_outcome, public_outcome, redraw_failed, exact_interaction) in [
            (
                CpuFrameCompletionOutcome::SuccessfulPresentation,
                NativeCpuFrameCompletionOutcome::SuccessfulPresentation,
                false,
                true,
            ),
            (
                CpuFrameCompletionOutcome::SkippedOrVetoed,
                NativeCpuFrameCompletionOutcome::SkippedOrVetoed,
                false,
                false,
            ),
            (
                CpuFrameCompletionOutcome::Incomplete,
                NativeCpuFrameCompletionOutcome::Incomplete,
                false,
                false,
            ),
            (
                CpuFrameCompletionOutcome::Failed,
                NativeCpuFrameCompletionOutcome::Failed,
                true,
                false,
            ),
            (
                CpuFrameCompletionOutcome::RecoveryTriggered,
                NativeCpuFrameCompletionOutcome::RecoveryTriggered,
                true,
                false,
            ),
        ] {
            let stable_key = key("settings");
            let mut ledger = CpuFrameObservationLedger::default();
            let admission = ledger.begin(
                stable_key.clone(),
                FrameWork::None,
                None,
                CpuFramePendingRedrawAge::Unknown,
            );
            let mut capture = CpuFrameObservationCapture::default();
            if exact_interaction {
                capture.record_frame_work(FrameWork::PaintOnly {
                    reason: FrameWorkReason::RoutedInput,
                });
            }
            match private_outcome {
                CpuFrameCompletionOutcome::SuccessfulPresentation => {
                    capture.mark_successful_presentation();
                }
                CpuFrameCompletionOutcome::SkippedOrVetoed => {}
                CpuFrameCompletionOutcome::Incomplete => capture.mark_frame_path_started(),
                CpuFrameCompletionOutcome::Failed => {}
                CpuFrameCompletionOutcome::RecoveryTriggered => {
                    capture.mark_recovery_triggered();
                }
            }
            ledger.finish(admission, capture, redraw_failed);

            let projected = ledger.project_frame_diagnostics(&stable_key);
            assert!(projected.available);
            assert_eq!(projected.latest_outcome, public_outcome);
            assert_eq!(projected.latest_exact_interaction, exact_interaction);
        }
    }

    #[test]
    fn observation_key_capacity_omits_overflow_without_projection() {
        let mut ledger = CpuFrameObservationLedger::default();
        for index in 0..=CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            admitted(&mut ledger, key(&format!("window-{index}")));
        }

        assert_eq!(ledger.len(), CPU_FRAME_OBSERVATION_KEY_CAPACITY);
        for index in 0..CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            let stable_key = key(&format!("window-{index}"));
            assert!(ledger.state(&stable_key).is_some());
            assert!(ledger.projection().window(&stable_key).is_some());
        }
        let overflow_key = key(&format!("window-{CPU_FRAME_OBSERVATION_KEY_CAPACITY}"));
        assert!(ledger.state(&overflow_key).is_none());
        assert!(ledger.projection().window(&overflow_key).is_none());
    }

    #[test]
    fn tracked_key_records_while_observation_key_capacity_is_full() {
        let mut ledger = CpuFrameObservationLedger::default();
        for index in 0..CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            admitted(&mut ledger, key(&format!("window-{index}")));
        }

        let tracked_key = key("window-0");
        admitted(&mut ledger, tracked_key.clone());

        let state = ledger
            .state(&tracked_key)
            .expect("tracked key should remain");
        assert_eq!(state.counters.admitted_redraws, 2);
        assert_eq!(state.sample_count, 2);
        assert_eq!(ledger.len(), CPU_FRAME_OBSERVATION_KEY_CAPACITY);
    }

    #[test]
    fn removing_a_key_releases_a_slot_without_transferring_history() {
        let mut ledger = CpuFrameObservationLedger::default();
        for index in 0..CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            admitted(&mut ledger, key(&format!("window-{index}")));
        }

        let removed_key = key("window-0");
        ledger.remove(&removed_key);
        let replacement_key = key("replacement");
        admitted(&mut ledger, replacement_key.clone());

        assert_eq!(ledger.len(), CPU_FRAME_OBSERVATION_KEY_CAPACITY);
        assert!(ledger.state(&removed_key).is_none());
        let replacement = ledger
            .state(&replacement_key)
            .expect("replacement key should use the released slot");
        assert_eq!(replacement.counters.admitted_redraws, 1);
        assert_eq!(replacement.sample_count, 1);
        assert_eq!(
            replacement.samples[0].as_ref().unwrap().key,
            replacement_key
        );
    }

    #[test]
    fn overflow_admissions_remain_untracked_through_every_finish_outcome() {
        let mut ledger = CpuFrameObservationLedger::default();
        for index in 0..CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            admitted(&mut ledger, key(&format!("window-{index}")));
        }

        for (index, expected_outcome) in [
            CpuFrameCompletionOutcome::SuccessfulPresentation,
            CpuFrameCompletionOutcome::Failed,
            CpuFrameCompletionOutcome::Incomplete,
            CpuFrameCompletionOutcome::SkippedOrVetoed,
            CpuFrameCompletionOutcome::RecoveryTriggered,
        ]
        .into_iter()
        .enumerate()
        {
            let overflow_key = key(&format!("overflow-{index}"));
            let admission = ledger.begin(
                overflow_key.clone(),
                FrameWork::None,
                None,
                CpuFramePendingRedrawAge::Unknown,
            );
            assert!(!admission.tracked);

            ledger.remove(&key(&format!("window-{index}")));
            let mut capture = CpuFrameObservationCapture::default();
            let redraw_failed = match expected_outcome {
                CpuFrameCompletionOutcome::SuccessfulPresentation => {
                    capture.mark_successful_presentation();
                    false
                }
                CpuFrameCompletionOutcome::Failed => true,
                CpuFrameCompletionOutcome::Incomplete => {
                    capture.mark_frame_path_started();
                    false
                }
                CpuFrameCompletionOutcome::RecoveryTriggered => {
                    capture.mark_recovery_triggered();
                    true
                }
                CpuFrameCompletionOutcome::SkippedOrVetoed => false,
            };
            ledger.finish(admission, capture, redraw_failed);

            assert!(ledger.state(&overflow_key).is_none());
            assert!(ledger.projection().window(&overflow_key).is_none());
            assert_eq!(ledger.len(), CPU_FRAME_OBSERVATION_KEY_CAPACITY - 1);

            admitted(&mut ledger, key(&format!("replacement-{index}")));
            assert_eq!(ledger.len(), CPU_FRAME_OBSERVATION_KEY_CAPACITY);
        }
    }

    #[test]
    fn clear_reuses_only_bounded_key_allocation() {
        let mut ledger = CpuFrameObservationLedger::default();
        for index in 0..CPU_FRAME_OBSERVATION_KEY_CAPACITY {
            admitted(&mut ledger, key(&format!("window-{index}")));
        }
        let capacity_before_clear = ledger.states.capacity();

        assert!(capacity_before_clear <= CPU_FRAME_OBSERVATION_KEY_CAPACITY);
        ledger.clear();
        assert_eq!(ledger.len(), 0);
        assert_eq!(ledger.states.capacity(), capacity_before_clear);

        admitted(&mut ledger, key("reused"));
        assert_eq!(ledger.state(&key("reused")).unwrap().sample_count, 1);
    }

    #[test]
    fn counters_saturate_without_wrapping() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let state = ledger.state_or_insert(stable_key.clone()).unwrap();
        state.counters.admitted_redraws = u64::MAX;
        state.counters.successful_presentations = u64::MAX;
        state.counters.skipped_or_vetoed_redraws = u64::MAX;
        state.counters.incomplete_frames = u64::MAX;
        state.counters.failed_frames = u64::MAX;
        state.counters.recovery_triggered_frames = u64::MAX;

        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.mark_successful_presentation();
        ledger.finish(admission, capture, false);

        let counters = ledger.state(&stable_key).unwrap().counters;
        assert_eq!(counters.admitted_redraws, u64::MAX);
        assert_eq!(counters.successful_presentations, u64::MAX);
        assert_eq!(counters.skipped_or_vetoed_redraws, u64::MAX);
        assert_eq!(counters.incomplete_frames, u64::MAX);
        assert_eq!(counters.failed_frames, u64::MAX);
        assert_eq!(counters.recovery_triggered_frames, u64::MAX);

        let projected = ledger.project_frame_diagnostics(&stable_key);
        assert!(projected.available);
        assert_eq!(projected.admitted_redraws, u64::MAX);
        assert_eq!(projected.successful_presentations, u64::MAX);
        assert_eq!(projected.skipped_or_vetoed_redraws, u64::MAX);
        assert_eq!(projected.incomplete_frames, u64::MAX);
        assert_eq!(projected.failed_frames, u64::MAX);
        assert_eq!(projected.recovery_triggered_frames, u64::MAX);
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
            CpuFramePendingRedrawAge::Unknown,
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
            CpuFrameCompletionOutcome::RecoveryTriggered
        );
    }

    #[test]
    fn recovery_starts_a_fresh_epoch_but_keeps_the_triggering_outcome() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        admitted(&mut ledger, stable_key.clone());
        let admission = ledger.begin(
            stable_key.clone(),
            FrameWork::None,
            None,
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.mark_recovery_triggered();
        ledger.finish(admission, capture, true);

        let state = ledger.state(&stable_key).unwrap();
        assert_eq!(state.counters.admitted_redraws, 1);
        assert_eq!(state.sample_count, 1);
        assert_eq!(
            state.samples[0].as_ref().unwrap().outcome,
            CpuFrameCompletionOutcome::RecoveryTriggered
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
            CpuFramePendingRedrawAge::Unknown,
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
            CpuFramePendingRedrawAge::Unknown,
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
                    due_at: now,
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
                    due_at: now,
                    next_wake: now + Duration::from_millis(16),
                },
                60,
                RuntimeAnimationActivity::paint_only(),
                false,
                FrameScheduleRedrawEvidence::default(),
            ),
        ];
        let mut scheduler = NativeFrameScheduler::default();
        let before = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            before.selected.clone().unwrap(),
            FrameWork::None,
            Some(60),
            CpuFramePendingRedrawAge::Known(Duration::ZERO),
        );
        let after = scheduler.observe(now, &demands, FrameScheduleDeadlines::default());
        ledger.finish(admission, CpuFrameObservationCapture::default(), false);

        assert_eq!(before.selected, after.selected);
    }
}
