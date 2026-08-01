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
use super::{FrameScheduleDemand, FrameScheduleKey, TimedFrameCadence};
use std::time::{Duration, Instant};

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

    pub(super) fn for_each(&self, mut visit: impl FnMut(CpuFrameFairnessEvidence<'demands>)) {
        for demand in self.demands {
            visit(self.evidence_for_demand(demand));
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
        .filter(|sample| sample.exact_interaction)
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
        assessment.for_each(|_| {});
        let after = scheduler.observe(now, &demands, deadlines);

        assert_eq!(before.selected, after.selected);
        assert_eq!(before.deadlines, after.deadlines);
    }

    fn now() -> Instant {
        Instant::now()
    }
}
