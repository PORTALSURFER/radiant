//! Shadow-only CPU scheduling-pressure evidence for the native frame owner.
//!
//! This module translates the current scheduler snapshot and the bounded raw
//! observation ledger into typed evidence. It never selects work, advances a
//! cursor, composes deadlines, requests redraw, or mutates a runner.

use super::cpu_frame_observation::{
    CpuFrameCompletionOutcome, CpuFrameObservationCounters, CpuFrameObservationLedger,
    CpuFrameObservationWindowProjection, CpuFramePendingRedrawAge, CpuFrameStage,
    CpuFrameStageObservation, LATEST_SAMPLE_CAPACITY,
};
use super::{
    FrameScheduleDemand, FrameScheduleKey, FrameScheduleWork, FrameSchedulerPlan, TimedFrameCadence,
};
use std::time::{Duration, Instant};

/// The parent keeps a bounded number of stable schedule identities. A key that
/// cannot be admitted because this bound is full is deliberately omitted from
/// shadow evidence; it never affects scheduler operation.
pub(super) const CPU_FRAME_FAIRNESS_KEY_CAPACITY: usize = 16;

/// Each stable key retains only its latest bounded scheduler-turn samples.
pub(super) const CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY: usize = 8;

/// The existing scheduler's disposition for one demand in one observation
/// turn. Admission is recorded separately at the scheduler cursor boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFrameTurnDisposition {
    #[default]
    NotDue,
    Selected,
    DueButDeferred,
}

/// One bounded scheduler-turn sample. The stable key lives on the owning
/// state, rather than being cloned into every ring entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameFairnessSample {
    pub(super) disposition: CpuFrameTurnDisposition,
    pub(super) work: FrameScheduleWork,
    pub(super) cursor_admitted: bool,
}

/// Saturating scheduler-turn totals for one stable schedule key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CpuFrameFairnessCounters {
    pub(super) not_due_turns: u64,
    pub(super) selected_turns: u64,
    pub(super) due_but_deferred_turns: u64,
    pub(super) cursor_admissions: u64,
}

impl CpuFrameFairnessCounters {
    fn record_disposition(&mut self, disposition: CpuFrameTurnDisposition) {
        match disposition {
            CpuFrameTurnDisposition::NotDue => {
                self.not_due_turns = self.not_due_turns.saturating_add(1);
            }
            CpuFrameTurnDisposition::Selected => {
                self.selected_turns = self.selected_turns.saturating_add(1);
            }
            CpuFrameTurnDisposition::DueButDeferred => {
                self.due_but_deferred_turns = self.due_but_deferred_turns.saturating_add(1);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CpuFrameFairnessState {
    key: FrameScheduleKey,
    counters: CpuFrameFairnessCounters,
    samples: [Option<CpuFrameFairnessSample>; CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY],
    next_sample: usize,
    sample_count: usize,
}

impl CpuFrameFairnessState {
    fn new(key: FrameScheduleKey) -> Self {
        Self {
            key,
            counters: CpuFrameFairnessCounters::default(),
            samples: std::array::from_fn(|_| None),
            next_sample: 0,
            sample_count: 0,
        }
    }

    fn record(&mut self, disposition: CpuFrameTurnDisposition, work: FrameScheduleWork) {
        self.counters.record_disposition(disposition);
        self.samples[self.next_sample] = Some(CpuFrameFairnessSample {
            disposition,
            work,
            cursor_admitted: false,
        });
        self.next_sample = (self.next_sample + 1) % CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY;
        self.sample_count = self
            .sample_count
            .saturating_add(1)
            .min(CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY);
    }

    fn latest_sample(&self) -> Option<&CpuFrameFairnessSample> {
        if self.sample_count == 0 {
            return None;
        }
        let index = (self.next_sample + CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY - 1)
            % CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY;
        self.samples[index].as_ref()
    }

    fn mark_latest_admitted(&mut self) -> bool {
        let latest_is_unadmitted_selection = self.latest_sample().is_some_and(|sample| {
            matches!(sample.disposition, CpuFrameTurnDisposition::Selected)
                && !sample.cursor_admitted
        });
        if !latest_is_unadmitted_selection {
            return false;
        }
        let index = (self.next_sample + CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY - 1)
            % CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY;
        let Some(sample) = self.samples[index].as_mut() else {
            return false;
        };
        sample.cursor_admitted = true;
        self.counters.cursor_admissions = self.counters.cursor_admissions.saturating_add(1);
        true
    }
}

/// Parent-owned fixed-capacity scheduler-turn ledger. Auxiliary runners do
/// not own a copy; they borrow only the existing raw observation owner while
/// the parent performs their synchronous route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameFairnessLedger {
    states: [Option<CpuFrameFairnessState>; CPU_FRAME_FAIRNESS_KEY_CAPACITY],
}

impl Default for CpuFrameFairnessLedger {
    fn default() -> Self {
        Self {
            states: std::array::from_fn(|_| None),
        }
    }
}

impl CpuFrameFairnessLedger {
    pub(super) fn projection(&self) -> CpuFrameFairnessProjection<'_> {
        CpuFrameFairnessProjection {
            states: &self.states,
        }
    }

    pub(super) fn clear(&mut self) {
        self.states.fill(None);
    }

    pub(super) fn remove(&mut self, key: &FrameScheduleKey) {
        if let Some(state) = self
            .states
            .iter_mut()
            .find(|state| state.as_ref().is_some_and(|state| &state.key == key))
        {
            *state = None;
        }
    }

    fn record_turn(
        &mut self,
        assessment: &CpuFrameFairnessAssessment<'_, '_>,
        plan: &FrameSchedulerPlan,
    ) {
        for demand in assessment.demands {
            let evidence = assessment.evidence_for_demand(demand);
            let disposition = assessment.disposition_for(demand, plan);
            let state = if self
                .projection()
                .window(demand.key())
                .is_some_and(|window| window.has_samples())
            {
                self.state_mut(demand.key())
            } else {
                self.state_or_insert(demand.key())
            };
            if let Some(state) = state {
                state.record(disposition, evidence.work);
            }
        }
    }

    pub(super) fn mark_admitted(&mut self, key: &FrameScheduleKey) {
        if let Some(state) = self.state_mut(key) {
            let _ = state.mark_latest_admitted();
        }
    }

    fn state_mut(&mut self, key: &FrameScheduleKey) -> Option<&mut CpuFrameFairnessState> {
        self.states
            .iter_mut()
            .find(|state| state.as_ref().is_some_and(|state| &state.key == key))
            .and_then(Option::as_mut)
    }

    fn state_or_insert(&mut self, key: &FrameScheduleKey) -> Option<&mut CpuFrameFairnessState> {
        if let Some(index) = self
            .states
            .iter()
            .position(|state| state.as_ref().is_some_and(|state| &state.key == key))
        {
            return self.states[index].as_mut();
        }
        let index = self.states.iter().position(Option::is_none)?;
        self.states[index] = Some(CpuFrameFairnessState::new(key.clone()));
        self.states[index].as_mut()
    }

    #[cfg(test)]
    fn state(&self, key: &FrameScheduleKey) -> Option<&CpuFrameFairnessState> {
        self.states
            .iter()
            .find_map(|state| state.as_ref().filter(|state| &state.key == key))
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.states.iter().filter(|state| state.is_some()).count()
    }
}

/// Borrowed, read-only projection of the parent-owned scheduler-turn ledger.
pub(super) struct CpuFrameFairnessProjection<'a> {
    states: &'a [Option<CpuFrameFairnessState>; CPU_FRAME_FAIRNESS_KEY_CAPACITY],
}

impl<'a> CpuFrameFairnessProjection<'a> {
    pub(super) fn window(
        &self,
        key: &FrameScheduleKey,
    ) -> Option<CpuFrameFairnessWindowProjection<'a>> {
        self.states
            .iter()
            .find_map(|state| state.as_ref().filter(|state| &state.key == key))
            .map(|state| CpuFrameFairnessWindowProjection { state })
    }
}

/// Read-only projection for one stable schedule key.
pub(super) struct CpuFrameFairnessWindowProjection<'a> {
    state: &'a CpuFrameFairnessState,
}

impl<'a> CpuFrameFairnessWindowProjection<'a> {
    fn has_samples(&self) -> bool {
        self.state.sample_count != 0
    }

    #[cfg(test)]
    pub(super) fn key(&self) -> &'a FrameScheduleKey {
        &self.state.key
    }

    #[cfg(test)]
    pub(super) const fn counters(&self) -> CpuFrameFairnessCounters {
        self.state.counters
    }

    #[cfg(test)]
    pub(super) fn latest_sample(&self) -> Option<&'a CpuFrameFairnessSample> {
        self.state.latest_sample()
    }

    #[cfg(test)]
    pub(super) fn samples(
        &self,
    ) -> &[Option<CpuFrameFairnessSample>; CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY] {
        &self.state.samples
    }

    #[cfg(test)]
    pub(super) const fn sample_count(&self) -> usize {
        self.state.sample_count
    }
}

/// The requested native cadence and the effective cadence after runtime
/// activity and caret caps are applied remain separate evidence fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFrameCadenceRate {
    Known(u32),
    #[default]
    Unknown,
}

/// Typed cadence pressure. The due lateness is measured from the original
/// cadence boundary, never from the next wake opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CpuFrameCadencePressure {
    NotApplicable,
    Waiting { until: Instant },
    Due { lateness: Duration },
    Unknown,
}

/// Positive interaction evidence is deliberately opt-in to an exact existing
/// frame-work reason. No event history is inferred from timing or stage data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CpuFrameInteractionEvidence {
    #[default]
    Unknown,
    Exact,
}

/// Recent completion outcomes plus bounded cumulative counters for one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameRecentOutcomeEvidence {
    pub(super) outcomes: [Option<CpuFrameCompletionOutcome>; LATEST_SAMPLE_CAPACITY],
    pub(super) latest: Option<CpuFrameCompletionOutcome>,
    pub(super) counters: CpuFrameObservationCounters,
    pub(super) sample_count: usize,
}

impl Default for CpuFrameRecentOutcomeEvidence {
    fn default() -> Self {
        Self {
            outcomes: [None; LATEST_SAMPLE_CAPACITY],
            latest: None,
            counters: CpuFrameObservationCounters::default(),
            sample_count: 0,
        }
    }
}

/// Per-stage completion evidence. Stage durations remain individually typed;
/// this boundary intentionally has no aggregate CPU-duration field because
/// refresh/projection/layout stages can overlap or be hierarchical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameStageCompletionEvidence {
    pub(super) available: bool,
    pub(super) stages: [CpuFrameStageObservation; CpuFrameStage::COUNT],
}

impl Default for CpuFrameStageCompletionEvidence {
    fn default() -> Self {
        Self {
            available: false,
            stages: [CpuFrameStageObservation::NotCompleted; CpuFrameStage::COUNT],
        }
    }
}

/// Conservative, per-window scheduling-pressure evidence for one stable key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CpuFrameFairnessEvidence<'a> {
    pub(super) key: &'a FrameScheduleKey,
    pub(super) requested_cadence: CpuFrameCadenceRate,
    pub(super) effective_cadence: CpuFrameCadenceRate,
    pub(super) cadence: CpuFrameCadencePressure,
    pub(super) work: FrameScheduleWork,
    pub(super) pending_redraw_age: CpuFramePendingRedrawAge,
    pub(super) interaction: CpuFrameInteractionEvidence,
    pub(super) recent_outcomes: CpuFrameRecentOutcomeEvidence,
    pub(super) stage_completion: CpuFrameStageCompletionEvidence,
}

/// Borrowed assessment view over the current scheduler demand snapshot and
/// bounded observation ledger. It is intentionally not a scheduler input.
pub(super) struct CpuFrameFairnessAssessment<'demands, 'observations> {
    now: Instant,
    demands: &'demands [FrameScheduleDemand],
    ledger: Option<&'observations CpuFrameObservationLedger>,
}

pub(super) fn assess_cpu_frame_fairness<'demands, 'observations>(
    now: Instant,
    demands: &'demands [FrameScheduleDemand],
    ledger: Option<&'observations CpuFrameObservationLedger>,
) -> CpuFrameFairnessAssessment<'demands, 'observations> {
    CpuFrameFairnessAssessment {
        now,
        demands,
        ledger,
    }
}

impl<'demands, 'observations> CpuFrameFairnessAssessment<'demands, 'observations> {
    #[cfg(test)]
    pub(super) fn evidence_for(
        &self,
        key: &FrameScheduleKey,
    ) -> Option<CpuFrameFairnessEvidence<'demands>> {
        self.demands
            .iter()
            .find(|demand| demand.key() == key)
            .map(|demand| self.evidence_for_demand(demand))
    }

    pub(super) fn record_turn(
        &self,
        ledger: &mut CpuFrameFairnessLedger,
        plan: &FrameSchedulerPlan,
    ) {
        ledger.record_turn(self, plan);
    }

    fn disposition_for(
        &self,
        demand: &FrameScheduleDemand,
        plan: &FrameSchedulerPlan,
    ) -> CpuFrameTurnDisposition {
        if !demand.has_due_work(self.now) {
            CpuFrameTurnDisposition::NotDue
        } else if plan.selected.as_ref() == Some(demand.key()) {
            CpuFrameTurnDisposition::Selected
        } else {
            CpuFrameTurnDisposition::DueButDeferred
        }
    }

    fn evidence_for_demand(
        &self,
        demand: &'demands FrameScheduleDemand,
    ) -> CpuFrameFairnessEvidence<'demands> {
        let observation = self
            .ledger
            .map(CpuFrameObservationLedger::projection)
            .and_then(|projection| projection.window(demand.key()));
        CpuFrameFairnessEvidence {
            key: demand.key(),
            requested_cadence: CpuFrameCadenceRate::Known(demand.requested_target_fps()),
            effective_cadence: CpuFrameCadenceRate::Known(demand.frame_target_fps()),
            cadence: cadence_pressure(self.now, Some(demand.cadence())),
            work: demand.work(self.now),
            pending_redraw_age: pending_redraw_age(demand),
            interaction: interaction_evidence(observation.as_ref()),
            recent_outcomes: recent_outcomes(observation.as_ref()),
            stage_completion: stage_completion(observation.as_ref()),
        }
    }
}

fn cadence_pressure(now: Instant, cadence: Option<TimedFrameCadence>) -> CpuFrameCadencePressure {
    match cadence {
        None => CpuFrameCadencePressure::Unknown,
        Some(TimedFrameCadence::Idle) => CpuFrameCadencePressure::NotApplicable,
        Some(TimedFrameCadence::WaitUntil(until)) => CpuFrameCadencePressure::Waiting { until },
        Some(TimedFrameCadence::DrainNow { due_at, .. }) => CpuFrameCadencePressure::Due {
            lateness: now.saturating_duration_since(due_at),
        },
    }
}

fn pending_redraw_age(demand: &FrameScheduleDemand) -> CpuFramePendingRedrawAge {
    if !demand.pending_redraw_requested() {
        return CpuFramePendingRedrawAge::NotRequested;
    }
    match demand.pending_redraw_age() {
        CpuFramePendingRedrawAge::NotRequested => CpuFramePendingRedrawAge::Unknown,
        age => age,
    }
}

fn interaction_evidence(
    observation: Option<&CpuFrameObservationWindowProjection<'_>>,
) -> CpuFrameInteractionEvidence {
    observation
        .and_then(CpuFrameObservationWindowProjection::latest_sample)
        .filter(|sample| {
            sample.exact_interaction
                && matches!(
                    sample.outcome,
                    CpuFrameCompletionOutcome::SuccessfulPresentation
                )
        })
        .map_or(CpuFrameInteractionEvidence::Unknown, |_| {
            CpuFrameInteractionEvidence::Exact
        })
}

fn recent_outcomes(
    observation: Option<&CpuFrameObservationWindowProjection<'_>>,
) -> CpuFrameRecentOutcomeEvidence {
    let Some(observation) = observation else {
        return CpuFrameRecentOutcomeEvidence::default();
    };
    let samples = observation.samples();
    CpuFrameRecentOutcomeEvidence {
        outcomes: std::array::from_fn(|index| samples[index].as_ref().map(|sample| sample.outcome)),
        latest: observation.latest_sample().map(|sample| sample.outcome),
        counters: observation.counters(),
        sample_count: observation.sample_count(),
    }
}

fn stage_completion(
    observation: Option<&CpuFrameObservationWindowProjection<'_>>,
) -> CpuFrameStageCompletionEvidence {
    let Some(sample) = observation.and_then(CpuFrameObservationWindowProjection::latest_sample)
    else {
        return CpuFrameStageCompletionEvidence::default();
    };
    CpuFrameStageCompletionEvidence {
        available: true,
        stages: sample.stages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::{
        CpuFrameObservationCapture, CpuFrameStage, FrameScheduleDeadlines,
        FrameScheduleRedrawEvidence, FrameWork, FrameWorkReason, NativeFrameScheduler,
        SceneRebuildMode,
    };
    use crate::runtime::RuntimeAnimationActivity;

    fn key(name: &str) -> FrameScheduleKey {
        FrameScheduleKey::Auxiliary(name.to_owned())
    }

    fn demand(key: FrameScheduleKey, cadence: TimedFrameCadence) -> FrameScheduleDemand {
        FrameScheduleDemand::from_cadence(
            key,
            cadence,
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence::default(),
        )
    }

    #[test]
    fn cadence_pressure_keeps_idle_waiting_due_and_unknown_distinct() {
        let now = Instant::now();
        assert_eq!(
            cadence_pressure(now, Some(TimedFrameCadence::Idle)),
            CpuFrameCadencePressure::NotApplicable
        );
        let until = now + Duration::from_millis(4);
        assert_eq!(
            cadence_pressure(now, Some(TimedFrameCadence::WaitUntil(until))),
            CpuFrameCadencePressure::Waiting { until }
        );
        assert_eq!(
            cadence_pressure(
                now,
                Some(TimedFrameCadence::DrainNow {
                    due_at: now - Duration::from_millis(7),
                    next_wake: now + Duration::from_millis(9),
                }),
            ),
            CpuFrameCadencePressure::Due {
                lateness: Duration::from_millis(7)
            }
        );
        assert_eq!(
            cadence_pressure(now, None),
            CpuFrameCadencePressure::Unknown
        );
    }

    #[test]
    fn due_lateness_uses_due_boundary_and_not_next_wake() {
        let now = Instant::now();
        let stable_key = key("settings");
        let evidence = CpuFrameFairnessEvidence {
            key: &stable_key,
            requested_cadence: CpuFrameCadenceRate::Known(60),
            effective_cadence: CpuFrameCadenceRate::Known(60),
            cadence: cadence_pressure(
                now,
                Some(TimedFrameCadence::DrainNow {
                    due_at: now - Duration::from_millis(5),
                    next_wake: now + Duration::from_secs(1),
                }),
            ),
            work: FrameScheduleWork::default(),
            pending_redraw_age: CpuFramePendingRedrawAge::NotRequested,
            interaction: CpuFrameInteractionEvidence::Unknown,
            recent_outcomes: CpuFrameRecentOutcomeEvidence::default(),
            stage_completion: CpuFrameStageCompletionEvidence::default(),
        };
        assert_eq!(
            evidence.cadence,
            CpuFrameCadencePressure::Due {
                lateness: Duration::from_millis(5)
            }
        );
    }

    #[test]
    fn pending_redraw_age_stays_separate_from_cadence_pressure() {
        let now = Instant::now();
        let mut redraw = FrameScheduleRedrawEvidence {
            pending_redraw_requested: true,
            pending_redraw_age: CpuFramePendingRedrawAge::Known(Duration::from_millis(23)),
            ..FrameScheduleRedrawEvidence::default()
        };
        let idle = demand(key("idle"), TimedFrameCadence::Idle);
        let idle_demands = [idle];
        let assessment = assess_cpu_frame_fairness(now, &idle_demands, None);
        let idle_evidence = assessment.evidence_for(&key("idle")).unwrap();
        assert_eq!(
            idle_evidence.pending_redraw_age,
            CpuFramePendingRedrawAge::NotRequested
        );
        redraw.pending_redraw_requested = true;
        let due = FrameScheduleDemand::from_cadence(
            key("due"),
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            redraw,
        );
        let due_demands = [due];
        let due_assessment = assess_cpu_frame_fairness(now, &due_demands, None);
        let due_evidence = due_assessment.evidence_for(&key("due")).unwrap();
        assert_eq!(
            due_evidence.cadence,
            CpuFrameCadencePressure::Due {
                lateness: Duration::ZERO
            }
        );
        assert_eq!(
            due_evidence.pending_redraw_age,
            CpuFramePendingRedrawAge::Known(Duration::from_millis(23))
        );

        let mut ambiguous_redraw = FrameScheduleRedrawEvidence {
            pending_redraw_requested: true,
            ..FrameScheduleRedrawEvidence::default()
        };
        ambiguous_redraw.pending_redraw_age = CpuFramePendingRedrawAge::NotRequested;
        let ambiguous = FrameScheduleDemand::from_cadence(
            key("ambiguous"),
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            ambiguous_redraw,
        );
        let ambiguous_demands = [ambiguous];
        let ambiguous_assessment = assess_cpu_frame_fairness(now, &ambiguous_demands, None);
        assert_eq!(
            ambiguous_assessment
                .evidence_for(&key("ambiguous"))
                .unwrap()
                .pending_redraw_age,
            CpuFramePendingRedrawAge::Unknown
        );
    }

    #[test]
    fn requested_and_effective_cadences_remain_separate() {
        let now = Instant::now();
        let stable_key = key("settings");
        let demand = FrameScheduleDemand::observe_runtime(
            stable_key.clone(),
            now - Duration::from_secs(1),
            now - Duration::from_secs(1),
            120,
            RuntimeAnimationActivity::paint_only_at(24),
            false,
            FrameScheduleRedrawEvidence::default(),
        );
        let demands = [demand];
        let assessment = assess_cpu_frame_fairness(now, &demands, None);
        let evidence = assessment.evidence_for(&stable_key).unwrap();

        assert_eq!(evidence.requested_cadence, CpuFrameCadenceRate::Known(120));
        assert_eq!(evidence.effective_cadence, CpuFrameCadenceRate::Known(24));
    }

    #[test]
    fn absent_timing_and_interaction_remain_unknown() {
        let now = Instant::now();
        let demand = demand(key("settings"), TimedFrameCadence::Idle);
        let demands = [demand];
        let assessment = assess_cpu_frame_fairness(now, &demands, None);
        let evidence = assessment.evidence_for(&key("settings")).unwrap();

        assert_eq!(evidence.requested_cadence, CpuFrameCadenceRate::Known(60));
        assert_eq!(evidence.effective_cadence, CpuFrameCadenceRate::Known(60));
        assert_eq!(evidence.interaction, CpuFrameInteractionEvidence::Unknown);
        assert!(!evidence.stage_completion.available);
        assert_eq!(evidence.recent_outcomes.sample_count, 0);
    }

    #[test]
    fn exact_interaction_and_stage_completion_are_projected_without_a_cpu_sum() {
        let stable_key = key("settings");
        let mut ledger = CpuFrameObservationLedger::default();
        let frame_work = FrameWork::RebuildScene {
            reason: FrameWorkReason::RoutedInput,
            mode: SceneRebuildMode::Immediate,
        };
        let admission = ledger.begin(
            stable_key.clone(),
            frame_work,
            Some(60),
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_frame_work(frame_work);
        capture.record_stage(
            CpuFrameStage::ApplicationProjection,
            true,
            super::super::CpuFrameDuration::Known(Duration::from_millis(2)),
        );
        capture.record_stage(
            CpuFrameStage::Layout,
            true,
            super::super::CpuFrameDuration::Known(Duration::from_millis(3)),
        );
        capture.mark_successful_presentation();
        ledger.finish(admission, capture, false);

        let demand = demand(stable_key.clone(), TimedFrameCadence::Idle);
        let demands = [demand];
        let assessment = assess_cpu_frame_fairness(now(), &demands, Some(&ledger));
        let evidence = assessment.evidence_for(&stable_key).unwrap();
        assert_eq!(evidence.interaction, CpuFrameInteractionEvidence::Exact);
        assert!(evidence.stage_completion.available);
        assert!(matches!(
            evidence.stage_completion.stages[CpuFrameStage::ApplicationProjection.index()],
            CpuFrameStageObservation::Completed(_)
        ));
        assert!(matches!(
            evidence.stage_completion.stages[CpuFrameStage::Layout.index()],
            CpuFrameStageObservation::Completed(_)
        ));
    }

    #[test]
    fn non_successful_interaction_outcomes_remain_unknown() {
        for outcome in [
            CpuFrameCompletionOutcome::Incomplete,
            CpuFrameCompletionOutcome::Failed,
            CpuFrameCompletionOutcome::RecoveryTriggered,
            CpuFrameCompletionOutcome::SkippedOrVetoed,
        ] {
            let (interaction, projected_outcome) = project_interaction_outcome(outcome);
            assert_eq!(projected_outcome, outcome);
            assert_eq!(interaction, CpuFrameInteractionEvidence::Unknown);
        }
    }

    #[test]
    fn assessment_does_not_change_scheduler_selection_or_composed_deadlines() {
        let now = Instant::now();
        let demands = [
            demand(
                FrameScheduleKey::Primary,
                TimedFrameCadence::DrainNow {
                    due_at: now,
                    next_wake: now + Duration::from_millis(16),
                },
            ),
            demand(
                key("settings"),
                TimedFrameCadence::DrainNow {
                    due_at: now,
                    next_wake: now + Duration::from_millis(16),
                },
            ),
        ];
        let deadlines = FrameScheduleDeadlines {
            activation: Some(now + Duration::from_millis(3)),
            maintenance: Some(now + Duration::from_millis(5)),
            ..FrameScheduleDeadlines::default()
        };
        let scheduler = NativeFrameScheduler::default();
        let before = scheduler.observe(now, &demands, deadlines);
        let assessment = assess_cpu_frame_fairness(now, &demands, None);
        let mut ledger = CpuFrameFairnessLedger::default();
        assessment.record_turn(&mut ledger, &before);
        let after = scheduler.observe(now, &demands, deadlines);

        assert_eq!(before.selected, after.selected);
        assert_eq!(before.deadlines, after.deadlines);
    }

    fn due_demand(key: FrameScheduleKey, now: Instant) -> FrameScheduleDemand {
        demand(
            key,
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + Duration::from_millis(16),
            },
        )
    }

    fn record_turn(
        ledger: &mut CpuFrameFairnessLedger,
        scheduler: &NativeFrameScheduler,
        now: Instant,
        demands: &[FrameScheduleDemand],
    ) -> FrameSchedulerPlan {
        let plan = scheduler.observe(now, demands, FrameScheduleDeadlines::default());
        assess_cpu_frame_fairness(now, demands, None).record_turn(ledger, &plan);
        plan
    }

    fn latest_sample(
        ledger: &CpuFrameFairnessLedger,
        key: &FrameScheduleKey,
    ) -> CpuFrameFairnessSample {
        ledger
            .projection()
            .window(key)
            .and_then(|window| window.latest_sample().copied())
            .expect("fairness turn should retain a latest sample")
    }

    #[test]
    fn simultaneous_due_windows_project_selected_deferred_and_admitted_separately() {
        let now = Instant::now();
        let primary_key = FrameScheduleKey::Primary;
        let auxiliary_key = key("settings");
        let demands = [
            due_demand(primary_key.clone(), now),
            due_demand(auxiliary_key.clone(), now),
        ];
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();
        let plan = record_turn(&mut ledger, &scheduler, now, &demands);

        assert_eq!(plan.selected, Some(primary_key.clone()));
        assert_eq!(
            latest_sample(&ledger, &primary_key).disposition,
            CpuFrameTurnDisposition::Selected
        );
        assert!(!latest_sample(&ledger, &primary_key).cursor_admitted);
        assert_eq!(
            latest_sample(&ledger, &auxiliary_key).disposition,
            CpuFrameTurnDisposition::DueButDeferred
        );
        assert!(!latest_sample(&ledger, &auxiliary_key).cursor_admitted);

        ledger.mark_admitted(&primary_key);
        assert!(latest_sample(&ledger, &primary_key).cursor_admitted);
        assert!(!latest_sample(&ledger, &auxiliary_key).cursor_admitted);
        let primary_counters = ledger.projection().window(&primary_key).unwrap().counters();
        let auxiliary_counters = ledger
            .projection()
            .window(&auxiliary_key)
            .unwrap()
            .counters();
        assert_eq!(primary_counters.selected_turns, 1);
        assert_eq!(primary_counters.cursor_admissions, 1);
        assert_eq!(auxiliary_counters.due_but_deferred_turns, 1);
        assert_eq!(auxiliary_counters.cursor_admissions, 0);
        assert_eq!(
            primary_counters.cursor_admissions + auxiliary_counters.cursor_admissions,
            1,
            "one scheduler turn admits at most one cursor key"
        );
    }

    #[test]
    fn selected_but_not_admitted_remains_unadmitted_after_a_veto() {
        let now = Instant::now();
        let stable_key = key("vetoed");
        let demands = [due_demand(stable_key.clone(), now)];
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();

        let plan = record_turn(&mut ledger, &scheduler, now, &demands);
        assert_eq!(plan.selected, Some(stable_key.clone()));
        assert_eq!(
            latest_sample(&ledger, &stable_key).disposition,
            CpuFrameTurnDisposition::Selected
        );
        assert_eq!(
            ledger
                .projection()
                .window(&stable_key)
                .unwrap()
                .counters()
                .cursor_admissions,
            0
        );
    }

    #[test]
    fn primary_and_auxiliary_due_combinations_remain_distinct() {
        let now = Instant::now();
        let auxiliary_key = key("settings");

        let primary_due = [
            due_demand(FrameScheduleKey::Primary, now),
            demand(auxiliary_key.clone(), TimedFrameCadence::Idle),
        ];
        let mut primary_ledger = CpuFrameFairnessLedger::default();
        let primary_scheduler = NativeFrameScheduler::default();
        let primary_plan = record_turn(&mut primary_ledger, &primary_scheduler, now, &primary_due);
        assert_eq!(primary_plan.selected, Some(FrameScheduleKey::Primary));
        assert_eq!(
            latest_sample(&primary_ledger, &FrameScheduleKey::Primary).disposition,
            CpuFrameTurnDisposition::Selected
        );
        assert_eq!(
            latest_sample(&primary_ledger, &auxiliary_key).disposition,
            CpuFrameTurnDisposition::NotDue
        );

        let auxiliary_due = [
            demand(FrameScheduleKey::Primary, TimedFrameCadence::Idle),
            due_demand(auxiliary_key.clone(), now),
        ];
        let mut auxiliary_ledger = CpuFrameFairnessLedger::default();
        let auxiliary_scheduler = NativeFrameScheduler::default();
        let auxiliary_plan = record_turn(
            &mut auxiliary_ledger,
            &auxiliary_scheduler,
            now,
            &auxiliary_due,
        );
        assert_eq!(
            auxiliary_plan.selected,
            Some(FrameScheduleKey::Auxiliary("settings".to_owned()))
        );
        assert_eq!(
            latest_sample(&auxiliary_ledger, &FrameScheduleKey::Primary).disposition,
            CpuFrameTurnDisposition::NotDue
        );
        assert_eq!(
            latest_sample(&auxiliary_ledger, &auxiliary_key).disposition,
            CpuFrameTurnDisposition::Selected
        );
    }

    #[test]
    fn timed_and_pending_redraw_work_flags_remain_distinct_in_samples() {
        let now = Instant::now();
        let timed_key = key("timed");
        let pending_key = key("pending");
        let timed = due_demand(timed_key.clone(), now);
        let pending = FrameScheduleDemand::from_cadence(
            pending_key.clone(),
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
        let demands = [timed, pending];
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();
        let plan = record_turn(&mut ledger, &scheduler, now, &demands);

        assert_eq!(plan.selected, Some(timed_key.clone()));
        let timed_work = latest_sample(&ledger, &timed_key).work;
        assert!(timed_work.drain_timed_frame);
        assert!(!timed_work.reissue_pending_redraw);
        let pending_work = latest_sample(&ledger, &pending_key).work;
        assert!(!pending_work.drain_timed_frame);
        assert!(pending_work.reissue_pending_redraw);
        assert_eq!(
            latest_sample(&ledger, &pending_key).disposition,
            CpuFrameTurnDisposition::DueButDeferred
        );
    }

    #[test]
    fn stable_key_reorder_preserves_turn_identity() {
        let now = Instant::now();
        let first_key = key("first");
        let second_key = key("second");
        let first_turn = [
            demand(first_key.clone(), TimedFrameCadence::Idle),
            due_demand(second_key.clone(), now),
        ];
        let second_turn = [
            due_demand(second_key.clone(), now),
            demand(first_key.clone(), TimedFrameCadence::Idle),
        ];
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();
        record_turn(&mut ledger, &scheduler, now, &first_turn);
        record_turn(&mut ledger, &scheduler, now, &second_turn);

        let first = ledger.projection().window(&first_key).unwrap();
        assert_eq!(first.key(), &first_key);
        assert_eq!(first.counters().not_due_turns, 2);
        let second = ledger.projection().window(&second_key).unwrap();
        assert_eq!(second.key(), &second_key);
        assert_eq!(second.counters().selected_turns, 2);
    }

    #[test]
    fn fixed_capacity_samples_and_key_slots_do_not_grow() {
        let now = Instant::now();
        let stable_key = key("bounded");
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();
        for _ in 0..(CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY + 3) {
            let demands = [demand(stable_key.clone(), TimedFrameCadence::Idle)];
            record_turn(&mut ledger, &scheduler, now, &demands);
        }
        let window = ledger.projection().window(&stable_key).unwrap();
        assert_eq!(window.sample_count(), CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY);
        assert_eq!(window.samples().len(), CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY);
        assert_eq!(
            window.counters().not_due_turns,
            (CPU_FRAME_FAIRNESS_SAMPLE_CAPACITY + 3) as u64
        );

        for index in 0..CPU_FRAME_FAIRNESS_KEY_CAPACITY {
            let key = key(&format!("window-{index}"));
            let demands = [demand(key, TimedFrameCadence::Idle)];
            record_turn(&mut ledger, &scheduler, now, &demands);
        }
        let overflow_key = key("overflow");
        let demands = [demand(overflow_key.clone(), TimedFrameCadence::Idle)];
        record_turn(&mut ledger, &scheduler, now, &demands);
        assert_eq!(ledger.len(), CPU_FRAME_FAIRNESS_KEY_CAPACITY);
        assert!(ledger.state(&overflow_key).is_none());
    }

    #[test]
    fn fairness_counters_saturate_without_wrapping() {
        let now = Instant::now();
        let stable_key = key("saturating");
        let mut ledger = CpuFrameFairnessLedger::default();
        let state = ledger.state_or_insert(&stable_key).unwrap();
        state.counters = CpuFrameFairnessCounters {
            not_due_turns: u64::MAX,
            selected_turns: u64::MAX,
            due_but_deferred_turns: u64::MAX,
            cursor_admissions: u64::MAX,
        };
        let demands = [due_demand(stable_key.clone(), now)];
        let scheduler = NativeFrameScheduler::default();
        record_turn(&mut ledger, &scheduler, now, &demands);
        ledger.mark_admitted(&stable_key);
        assert_eq!(
            ledger.projection().window(&stable_key).unwrap().counters(),
            CpuFrameFairnessCounters {
                not_due_turns: u64::MAX,
                selected_turns: u64::MAX,
                due_but_deferred_turns: u64::MAX,
                cursor_admissions: u64::MAX,
            }
        );
    }

    #[test]
    fn removal_and_clear_fence_history_before_reinsertion() {
        let now = Instant::now();
        let stable_key = key("settings");
        let demands = [due_demand(stable_key.clone(), now)];
        let mut ledger = CpuFrameFairnessLedger::default();
        let scheduler = NativeFrameScheduler::default();
        record_turn(&mut ledger, &scheduler, now, &demands);
        ledger.mark_admitted(&stable_key);
        ledger.remove(&stable_key);
        assert!(ledger.projection().window(&stable_key).is_none());

        let reinserted = [demand(stable_key.clone(), TimedFrameCadence::Idle)];
        record_turn(&mut ledger, &scheduler, now, &reinserted);
        let counters = ledger.projection().window(&stable_key).unwrap().counters();
        assert_eq!(counters.selected_turns, 0);
        assert_eq!(counters.not_due_turns, 1);
        assert_eq!(counters.cursor_admissions, 0);

        ledger.clear();
        assert_eq!(ledger.len(), 0);
        assert!(ledger.projection().window(&stable_key).is_none());
    }

    #[derive(Debug, PartialEq, Eq)]
    struct SchedulerTrace {
        plans: Vec<FrameSchedulerPlan>,
        admissions: Vec<FrameScheduleKey>,
        observe_calls: usize,
        admission_calls: usize,
    }

    fn scheduler_trace(now: Instant, with_observation: bool) -> SchedulerTrace {
        let primary = due_demand(FrameScheduleKey::Primary, now);
        let auxiliary_key = key("settings");
        let auxiliary = due_demand(auxiliary_key, now);
        let demands = [primary, auxiliary];
        let external_deadlines = FrameScheduleDeadlines {
            activation: Some(now + Duration::from_millis(3)),
            maintenance: Some(now + Duration::from_millis(5)),
            ..FrameScheduleDeadlines::default()
        };
        let mut scheduler = NativeFrameScheduler::default();
        let mut ledger = with_observation.then(CpuFrameFairnessLedger::default);
        let mut trace = SchedulerTrace {
            plans: Vec::new(),
            admissions: Vec::new(),
            observe_calls: 0,
            admission_calls: 0,
        };
        for _ in 0..4 {
            let plan = scheduler.observe(now, &demands, external_deadlines);
            trace.observe_calls += 1;
            if let Some(ledger) = ledger.as_mut() {
                assess_cpu_frame_fairness(now, &demands, None).record_turn(ledger, &plan);
            }
            trace.plans.push(plan.clone());
            if let Some(key) = plan.selected.clone() {
                if let Some(ledger) = ledger.as_mut() {
                    ledger.mark_admitted(&key);
                }
                scheduler.record_admission(key.clone());
                trace.admissions.push(key);
                trace.admission_calls += 1;
            }
        }
        trace
    }

    #[test]
    fn observation_enabled_and_disabled_scheduler_traces_are_identical() {
        let now = Instant::now();
        let enabled = scheduler_trace(now, true);
        let disabled = scheduler_trace(now, false);
        assert_eq!(enabled, disabled);
        assert_eq!(enabled.observe_calls, 4);
        assert_eq!(enabled.admission_calls, 4);
        assert_eq!(
            enabled.admissions,
            vec![
                FrameScheduleKey::Primary,
                FrameScheduleKey::Auxiliary("settings".to_owned()),
                FrameScheduleKey::Primary,
                FrameScheduleKey::Auxiliary("settings".to_owned()),
            ]
        );
    }

    fn now() -> Instant {
        Instant::now()
    }

    fn project_interaction_outcome(
        expected: CpuFrameCompletionOutcome,
    ) -> (CpuFrameInteractionEvidence, CpuFrameCompletionOutcome) {
        let stable_key = key("interaction");
        let frame_work = FrameWork::RebuildScene {
            reason: FrameWorkReason::RoutedInput,
            mode: SceneRebuildMode::Immediate,
        };
        let mut ledger = CpuFrameObservationLedger::default();
        let admission = ledger.begin(
            stable_key.clone(),
            frame_work,
            Some(60),
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_frame_work(frame_work);
        let redraw_failed = match expected {
            CpuFrameCompletionOutcome::SuccessfulPresentation => {
                capture.mark_successful_presentation();
                false
            }
            CpuFrameCompletionOutcome::SkippedOrVetoed => false,
            CpuFrameCompletionOutcome::Incomplete => {
                capture.mark_frame_path_started();
                false
            }
            CpuFrameCompletionOutcome::Failed => true,
            CpuFrameCompletionOutcome::RecoveryTriggered => {
                capture.mark_recovery_triggered();
                false
            }
        };
        ledger.finish(admission, capture, redraw_failed);

        let demand = demand(stable_key.clone(), TimedFrameCadence::Idle);
        let demands = [demand];
        let evidence = assess_cpu_frame_fairness(now(), &demands, Some(&ledger))
            .evidence_for(&stable_key)
            .expect("interaction demand should project evidence");
        (
            evidence.interaction,
            evidence
                .recent_outcomes
                .latest
                .expect("finished interaction should have an outcome"),
        )
    }
}
