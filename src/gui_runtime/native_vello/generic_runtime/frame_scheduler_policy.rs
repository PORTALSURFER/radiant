//! Private executable contract for the next application-level frame policy.
//!
//! The native selector consumes the private demand, fairness, and stable-key
//! admission pieces. Cadence caps, diagnostic budgets, and coalescing rules
//! remain private policy until their corresponding runtime consumers exist.

#![allow(dead_code)]

use super::frame_scheduler::FrameScheduleKey;
use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

/// The safe-boundary order for one non-preemptive stage bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchedulerStage {
    Lifecycle,
    DiscreteInput,
    ImmediateTransient,
    Deadline,
    Projection,
    Layout,
    PaintPlan,
    EncodePresent,
    Maintenance,
}

impl SchedulerStage {
    /// The normative order used when a frame boundary admits a stage.
    pub(super) const ORDER: [Self; 9] = [
        Self::Lifecycle,
        Self::DiscreteInput,
        Self::ImmediateTransient,
        Self::Deadline,
        Self::Projection,
        Self::Layout,
        Self::PaintPlan,
        Self::EncodePresent,
        Self::Maintenance,
    ];

    /// Selected stages run to a safe completion boundary.
    pub(super) const fn is_non_preemptive(self) -> bool {
        let _ = self;
        true
    }
}

/// Priority classes used by cross-window admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchedulerWorkClass {
    Lifecycle,
    DiscreteInput,
    Deadline,
    Transient,
    Animation,
    Projection,
    Layout,
    Paint,
    Maintenance,
}

impl SchedulerWorkClass {
    const fn rank(self) -> u8 {
        match self {
            Self::Lifecycle => 0,
            Self::DiscreteInput => 1,
            Self::Transient => 2,
            Self::Deadline => 3,
            Self::Animation => 4,
            Self::Projection => 5,
            Self::Layout => 6,
            Self::Paint => 7,
            Self::Maintenance => 8,
        }
    }

    pub(super) const fn outranks(self, other: Self) -> bool {
        self.rank() < other.rank()
    }

    const fn cannot_be_preempted_by_promotion(self) -> bool {
        matches!(self, Self::Lifecycle | Self::DiscreteInput)
    }
}

/// Explicit fences used by the fairness guarantee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SchedulerFairnessEligibility {
    pub(super) due_work: bool,
    pub(super) passes_lifecycle_native_generation_fences: bool,
    pub(super) blocked_by_admitted_lifecycle: bool,
    pub(super) blocked_by_admitted_discrete_input: bool,
    pub(super) continuous_coalescible_work: bool,
}

impl SchedulerFairnessEligibility {
    /// Determine whether the two-complete-epoch guarantee applies.
    pub(super) const fn is_fairness_eligible(self) -> bool {
        self.due_work
            && self.passes_lifecycle_native_generation_fences
            && !self.blocked_by_admitted_lifecycle
            && !self.blocked_by_admitted_discrete_input
    }
}

/// One demand supplied to the private policy evaluator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SchedulerDemand {
    key: FrameScheduleKey,
    class: SchedulerWorkClass,
    deadline: Instant,
    eligibility: SchedulerFairnessEligibility,
}

impl SchedulerDemand {
    pub(super) const fn new(
        key: FrameScheduleKey,
        class: SchedulerWorkClass,
        deadline: Instant,
        eligibility: SchedulerFairnessEligibility,
    ) -> Self {
        Self {
            key,
            class,
            deadline,
            eligibility,
        }
    }

    pub(super) fn key(&self) -> &FrameScheduleKey {
        &self.key
    }

    pub(super) const fn class(&self) -> SchedulerWorkClass {
        self.class
    }

    pub(super) const fn deadline(&self) -> Instant {
        self.deadline
    }

    pub(super) const fn eligibility(&self) -> SchedulerFairnessEligibility {
        self.eligibility
    }
}

/// Bounded fairness bookkeeping for one parent event-loop scheduler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SchedulerFairnessLedger {
    epoch: u64,
    overflow_deferrals: u64,
    states: [Option<SchedulerFairnessState>; MAX_SCHEDULER_KEYS],
}

pub(super) const MAX_SCHEDULER_KEYS: usize = 16;

/// Explicit result for a bounded fairness-ledger admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchedulerAdmissionResult {
    Tracked,
    OverflowDeferred,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SchedulerFairnessState {
    key: FrameScheduleKey,
    admitted_this_epoch: bool,
    missed_epochs: u8,
}

impl Default for SchedulerFairnessLedger {
    fn default() -> Self {
        Self {
            epoch: 0,
            overflow_deferrals: 0,
            states: std::array::from_fn(|_| None),
        }
    }
}

impl SchedulerFairnessLedger {
    /// Return the current complete-epoch number.
    pub(super) const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Record one complete stage bundle for the current epoch.
    pub(super) fn record_admission(&mut self, key: &FrameScheduleKey) -> SchedulerAdmissionResult {
        if let Some(state) = self.state_or_insert(key) {
            state.admitted_this_epoch = true;
            SchedulerAdmissionResult::Tracked
        } else {
            self.overflow_deferrals = self.overflow_deferrals.saturating_add(1);
            SchedulerAdmissionResult::OverflowDeferred
        }
    }

    /// Number of admissions or epoch observations explicitly deferred because
    /// the bounded key ledger was full.
    pub(super) const fn overflow_deferrals(&self) -> u64 {
        self.overflow_deferrals
    }

    /// Release a retired key's slot for a later stable-key admission.
    pub(super) fn remove(&mut self, key: &FrameScheduleKey) {
        if let Some(state) = self
            .states
            .iter_mut()
            .find(|state| state.as_ref().is_some_and(|state| &state.key == key))
        {
            *state = None;
        }
    }

    /// Retire stable keys that are absent from the current parent snapshot.
    pub(super) fn remove_absent(&mut self, demands: &[SchedulerDemand]) {
        for state in &mut self.states {
            if state
                .as_ref()
                .is_some_and(|state| !demands.iter().any(|demand| demand.key() == &state.key))
            {
                *state = None;
            }
        }
    }

    /// Whether an eligible key has already received its one bundle this epoch.
    pub(super) fn can_admit(&self, demand: &SchedulerDemand) -> bool {
        if !demand.eligibility.is_fairness_eligible() {
            return true;
        }
        match self.state(&demand.key) {
            Some(state) => !state.admitted_this_epoch,
            None => self.has_capacity(),
        }
    }

    /// Close one complete epoch and update the bounded promotion counter.
    pub(super) fn complete_epoch(&mut self, demands: &[SchedulerDemand]) {
        for demand in demands {
            let Some(state) = self.state_or_insert(&demand.key) else {
                self.overflow_deferrals = self.overflow_deferrals.saturating_add(1);
                continue;
            };
            if demand.eligibility.is_fairness_eligible() {
                if state.admitted_this_epoch {
                    state.missed_epochs = 0;
                } else {
                    state.missed_epochs = state.missed_epochs.saturating_add(1);
                }
            } else {
                state.missed_epochs = 0;
            }
            state.admitted_this_epoch = false;
        }
        self.epoch = self.epoch.saturating_add(1);
    }

    /// Return the policy class after applying two-epoch promotion.
    pub(super) fn effective_class(&self, demand: &SchedulerDemand) -> SchedulerWorkClass {
        let promoted =
            demand.eligibility.is_fairness_eligible() && self.missed_epochs(&demand.key) >= 2;
        if promoted && !demand.class.cannot_be_preempted_by_promotion() {
            SchedulerWorkClass::Deadline
        } else {
            demand.class
        }
    }

    pub(super) fn missed_epochs(&self, key: &FrameScheduleKey) -> u8 {
        self.state(key).map_or(0, |state| state.missed_epochs)
    }

    /// Select one eligible demand using promotion, class priority, deadline,
    /// and finally stable-key round robin.
    pub(super) fn select_candidate(
        &self,
        demands: &[SchedulerDemand],
        last_admitted: Option<&FrameScheduleKey>,
    ) -> Option<FrameScheduleKey> {
        let has_unserved = demands
            .iter()
            .any(|demand| demand.eligibility.is_fairness_eligible() && self.can_admit(demand));
        let mut best: Option<(usize, SchedulerWorkClass, Instant)> = None;

        for (index, demand) in demands.iter().enumerate() {
            if !demand.eligibility.is_fairness_eligible()
                || self.is_overflow(demand)
                || (has_unserved && !self.can_admit(demand))
            {
                continue;
            }
            let class = self.effective_class(demand);
            let candidate = (index, class, demand.deadline);
            let is_better = best.is_none_or(|current| {
                class.rank() < current.1.rank()
                    || (class.rank() == current.1.rank()
                        && (demand.deadline < current.2
                            || (demand.deadline == current.2
                                && stable_key_precedes(
                                    demand.key(),
                                    demands[current.0].key(),
                                    last_admitted,
                                ))))
            });
            if is_better {
                best = Some(candidate);
            }
        }

        best.map(|(index, _, _)| demands[index].key.clone())
    }

    fn state(&self, key: &FrameScheduleKey) -> Option<&SchedulerFairnessState> {
        self.states
            .iter()
            .find_map(|state| state.as_ref().filter(|state| &state.key == key))
    }

    fn has_capacity(&self) -> bool {
        self.states.iter().any(Option::is_none)
    }

    fn is_overflow(&self, demand: &SchedulerDemand) -> bool {
        demand.eligibility.is_fairness_eligible()
            && self.state(&demand.key).is_none()
            && !self.has_capacity()
    }

    fn state_or_insert(&mut self, key: &FrameScheduleKey) -> Option<&mut SchedulerFairnessState> {
        if let Some(index) = self
            .states
            .iter()
            .position(|state| state.as_ref().is_some_and(|state| &state.key == key))
        {
            return self.states[index].as_mut();
        }
        let index = self.states.iter().position(Option::is_none)?;
        self.states[index] = Some(SchedulerFairnessState {
            key: key.clone(),
            admitted_this_epoch: false,
            missed_epochs: 0,
        });
        self.states[index].as_mut()
    }
}

fn stable_key_precedes(
    candidate: &FrameScheduleKey,
    current: &FrameScheduleKey,
    last_admitted: Option<&FrameScheduleKey>,
) -> bool {
    let Some(last_admitted) = last_admitted else {
        return compare_keys(candidate, current).is_lt();
    };

    let candidate_after_cursor = compare_keys(candidate, last_admitted).is_gt();
    let current_after_cursor = compare_keys(current, last_admitted).is_gt();
    match (candidate_after_cursor, current_after_cursor) {
        (true, false) => true,
        (false, true) => false,
        _ => compare_keys(candidate, current).is_lt(),
    }
}

fn compare_keys(left: &FrameScheduleKey, right: &FrameScheduleKey) -> Ordering {
    match (left, right) {
        (FrameScheduleKey::Primary, FrameScheduleKey::Primary) => Ordering::Equal,
        (FrameScheduleKey::Primary, FrameScheduleKey::Auxiliary(_)) => Ordering::Less,
        (FrameScheduleKey::Auxiliary(_), FrameScheduleKey::Primary) => Ordering::Greater,
        (FrameScheduleKey::Auxiliary(left), FrameScheduleKey::Auxiliary(right)) => left.cmp(right),
    }
}

/// Normalize the requested cadence against host/display and activity caps.
pub(super) fn effective_frame_rate(
    requested_fps: u32,
    host_display_cap_fps: u32,
    activity_cap_fps: Option<u32>,
) -> u32 {
    let requested = crate::gui_runtime::options::normalize_native_target_fps(requested_fps);
    let host_cap = crate::gui_runtime::options::normalize_native_target_fps(host_display_cap_fps);
    let activity_cap = activity_cap_fps
        .map(crate::gui_runtime::options::normalize_native_target_fps)
        .unwrap_or(u32::MAX);
    requested.min(host_cap).min(activity_cap)
}

/// Standalone caret animation uses the explicit 30 Hz activity cap.
pub(super) fn standalone_caret_frame_rate(requested_fps: u32, host_display_cap_fps: u32) -> u32 {
    effective_frame_rate(requested_fps, host_display_cap_fps, Some(30))
}

/// Diagnostic safe-boundary targets derived from one effective cadence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SchedulerSoftBudgets {
    pub(super) input_transient: Duration,
    pub(super) projection_reconciliation: Duration,
    pub(super) layout_paint_plan: Duration,
    pub(super) encode_present: Duration,
    pub(super) maintenance_background: Duration,
}

impl SchedulerSoftBudgets {
    pub(super) fn for_effective_fps(effective_fps: u32) -> Self {
        let fps = crate::gui_runtime::options::normalize_native_target_fps(effective_fps);
        let frame_period = Duration::from_secs_f64(1.0 / f64::from(fps));
        Self {
            input_transient: bounded_budget(frame_period, 8, Duration::from_millis(2)),
            projection_reconciliation: bounded_budget(frame_period, 4, Duration::from_millis(4)),
            layout_paint_plan: bounded_budget(frame_period, 4, Duration::from_millis(4)),
            encode_present: bounded_budget(frame_period, 2, Duration::from_millis(6)),
            maintenance_background: bounded_budget(frame_period, 8, Duration::from_millis(2)),
        }
    }
}

fn bounded_budget(frame_period: Duration, divisor: u32, cap: Duration) -> Duration {
    Duration::from_secs_f64(frame_period.as_secs_f64() / f64::from(divisor)).min(cap)
}

/// Work categories with the target's latest-wins policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchedulerWorkKind {
    Animation,
    PointerMotion,
    Scroll,
    Redraw,
    StaleVisual,
    Background,
    DiscreteInput,
    Lifecycle,
    EditTerminal,
    PlatformCompletion,
}

impl SchedulerWorkKind {
    pub(super) const fn is_latest_wins(self) -> bool {
        matches!(
            self,
            Self::Animation
                | Self::PointerMotion
                | Self::Scroll
                | Self::Redraw
                | Self::StaleVisual
                | Self::Background
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eligible() -> SchedulerFairnessEligibility {
        SchedulerFairnessEligibility {
            due_work: true,
            passes_lifecycle_native_generation_fences: true,
            ..SchedulerFairnessEligibility::default()
        }
    }

    fn demand_at(key: &str, class: SchedulerWorkClass, deadline: Instant) -> SchedulerDemand {
        SchedulerDemand::new(
            FrameScheduleKey::Auxiliary(key.to_owned()),
            class,
            deadline,
            eligible(),
        )
    }

    fn demand(key: &str, class: SchedulerWorkClass) -> SchedulerDemand {
        demand_at(key, class, Instant::now())
    }

    #[test]
    fn stage_order_is_explicit_and_non_preemptive() {
        assert_eq!(
            SchedulerStage::ORDER,
            [
                SchedulerStage::Lifecycle,
                SchedulerStage::DiscreteInput,
                SchedulerStage::ImmediateTransient,
                SchedulerStage::Deadline,
                SchedulerStage::Projection,
                SchedulerStage::Layout,
                SchedulerStage::PaintPlan,
                SchedulerStage::EncodePresent,
                SchedulerStage::Maintenance,
            ]
        );
        assert!(
            SchedulerStage::ORDER
                .into_iter()
                .all(SchedulerStage::is_non_preemptive)
        );
        assert!(SchedulerWorkClass::Lifecycle.outranks(SchedulerWorkClass::Maintenance));
        assert!(SchedulerWorkClass::Deadline.outranks(SchedulerWorkClass::Projection));
    }

    #[test]
    fn effective_rate_uses_the_lowest_normalized_cap_and_caret_is_30_hz() {
        assert_eq!(effective_frame_rate(120, 60, Some(48)), 48);
        assert_eq!(effective_frame_rate(24, 120, None), 24);
        assert_eq!(standalone_caret_frame_rate(120, 120), 30);
        assert_eq!(standalone_caret_frame_rate(24, 120), 24);
    }

    #[test]
    fn soft_budgets_follow_the_bounded_formula() {
        let budgets = SchedulerSoftBudgets::for_effective_fps(60);

        assert_eq!(budgets.input_transient, Duration::from_millis(2));
        assert_eq!(budgets.projection_reconciliation, Duration::from_millis(4));
        assert_eq!(budgets.layout_paint_plan, Duration::from_millis(4));
        assert_eq!(budgets.encode_present, Duration::from_millis(6));
        assert_eq!(budgets.maintenance_background, Duration::from_millis(2));
    }

    #[test]
    fn continuous_work_does_not_block_fairness_eligibility() {
        let mut eligibility = eligible();
        eligibility.continuous_coalescible_work = true;
        assert!(eligibility.is_fairness_eligible());

        eligibility.blocked_by_admitted_discrete_input = true;
        assert!(!eligibility.is_fairness_eligible());
    }

    #[test]
    fn fences_and_due_work_are_required_for_the_two_epoch_guarantee() {
        let mut eligibility = eligible();
        eligibility.due_work = false;
        assert!(!eligibility.is_fairness_eligible());

        eligibility.due_work = true;
        eligibility.passes_lifecycle_native_generation_fences = false;
        assert!(!eligibility.is_fairness_eligible());
    }

    #[test]
    fn one_admission_per_epoch_is_tracked_before_a_second_bundle() {
        let first = demand("first", SchedulerWorkClass::Animation);
        let mut ledger = SchedulerFairnessLedger::default();

        assert!(ledger.can_admit(&first));
        ledger.record_admission(first.key());
        assert!(!ledger.can_admit(&first));
        ledger.complete_epoch(std::slice::from_ref(&first));
        assert_eq!(ledger.epoch(), 1);
        assert!(ledger.can_admit(&first));
    }

    #[test]
    fn two_missed_eligible_epochs_promote_ordinary_work_to_deadline() {
        let starved = demand("starved", SchedulerWorkClass::Animation);
        let mut ledger = SchedulerFairnessLedger::default();

        ledger.complete_epoch(std::slice::from_ref(&starved));
        ledger.complete_epoch(std::slice::from_ref(&starved));

        assert_eq!(ledger.missed_epochs(starved.key()), 2);
        assert_eq!(
            ledger.effective_class(&starved),
            SchedulerWorkClass::Deadline
        );
    }

    #[test]
    fn promotion_does_not_preempt_lifecycle_or_discrete_input() {
        let lifecycle = demand("lifecycle", SchedulerWorkClass::Lifecycle);
        let discrete = demand("discrete", SchedulerWorkClass::DiscreteInput);
        let mut ledger = SchedulerFairnessLedger::default();

        ledger.complete_epoch(&[lifecycle.clone(), discrete.clone()]);
        ledger.complete_epoch(&[lifecycle.clone(), discrete.clone()]);

        assert_eq!(
            ledger.effective_class(&lifecycle),
            SchedulerWorkClass::Lifecycle
        );
        assert_eq!(
            ledger.effective_class(&discrete),
            SchedulerWorkClass::DiscreteInput
        );
    }

    #[test]
    fn ineligible_work_does_not_accumulate_starvation_promotion() {
        let mut blocked = demand("blocked", SchedulerWorkClass::Animation);
        blocked.eligibility.blocked_by_admitted_lifecycle = true;
        let mut ledger = SchedulerFairnessLedger::default();

        ledger.complete_epoch(&[blocked.clone()]);
        ledger.complete_epoch(&[blocked.clone()]);

        assert_eq!(ledger.missed_epochs(blocked.key()), 0);
        assert_eq!(
            ledger.effective_class(&blocked),
            SchedulerWorkClass::Animation
        );
    }

    #[test]
    fn immediate_transient_feedback_precedes_due_deadline_work() {
        let now = Instant::now();
        let transient = demand_at(
            "pointer",
            SchedulerWorkClass::Transient,
            now + Duration::from_millis(10),
        );
        let deadline = demand_at("animation", SchedulerWorkClass::Deadline, now);

        assert_eq!(
            SchedulerFairnessLedger::default()
                .select_candidate(&[deadline, transient.clone()], None,),
            Some(transient.key().clone())
        );
    }

    #[test]
    fn selection_uses_promotion_priority_deadline_then_stable_round_robin() {
        let now = Instant::now();
        let starved = demand_at(
            "starved",
            SchedulerWorkClass::Animation,
            now + Duration::from_millis(10),
        );
        let projection = demand_at(
            "projection",
            SchedulerWorkClass::Projection,
            now + Duration::from_millis(1),
        );
        let mut ledger = SchedulerFairnessLedger::default();
        ledger.complete_epoch(std::slice::from_ref(&starved));
        ledger.complete_epoch(std::slice::from_ref(&starved));

        assert_eq!(
            ledger.select_candidate(&[projection, starved.clone()], None),
            Some(starved.key().clone())
        );

        let earlier = demand_at(
            "earlier",
            SchedulerWorkClass::Projection,
            now + Duration::from_millis(2),
        );
        let later = demand_at(
            "later",
            SchedulerWorkClass::Projection,
            now + Duration::from_millis(3),
        );
        assert_eq!(
            ledger.select_candidate(&[later, earlier.clone()], None),
            Some(earlier.key().clone())
        );

        let tied_first = demand_at("tied-first", SchedulerWorkClass::Paint, now);
        let tied_second = demand_at("tied-second", SchedulerWorkClass::Paint, now);
        assert_eq!(
            ledger.select_candidate(
                &[tied_first.clone(), tied_second.clone()],
                Some(tied_first.key()),
            ),
            Some(tied_second.key().clone())
        );

        let tied_third = demand_at("tied-third", SchedulerWorkClass::Paint, now);
        assert_eq!(
            ledger.select_candidate(
                &[tied_first, tied_third.clone(), tied_second.clone()],
                Some(tied_second.key()),
            ),
            Some(tied_third.key().clone())
        );
    }

    #[test]
    fn canonical_tie_breaking_is_permutation_independent_and_wraps() {
        let now = Instant::now();
        let primary = SchedulerDemand::new(
            FrameScheduleKey::Primary,
            SchedulerWorkClass::Paint,
            now,
            eligible(),
        );
        let alpha = demand_at("alpha", SchedulerWorkClass::Paint, now);
        let beta = demand_at("beta", SchedulerWorkClass::Paint, now);
        let ledger = SchedulerFairnessLedger::default();

        let forward = ledger.select_candidate(
            &[primary.clone(), alpha.clone(), beta.clone()],
            Some(primary.key()),
        );
        let permuted = ledger.select_candidate(
            &[beta.clone(), primary.clone(), alpha.clone()],
            Some(primary.key()),
        );
        assert_eq!(forward, Some(alpha.key().clone()));
        assert_eq!(permuted, forward);

        let wrapped =
            ledger.select_candidate(&[alpha, beta.clone(), primary.clone()], Some(beta.key()));
        assert_eq!(wrapped, Some(primary.key().clone()));
    }

    #[test]
    fn ledger_overflow_is_explicitly_deferred_and_retires_cleanly() {
        let mut ledger = SchedulerFairnessLedger::default();
        let tracked: Vec<_> = (0..MAX_SCHEDULER_KEYS)
            .map(|index| demand(&format!("tracked-{index}"), SchedulerWorkClass::Animation))
            .collect();

        for demand in &tracked {
            assert_eq!(
                ledger.record_admission(demand.key()),
                SchedulerAdmissionResult::Tracked
            );
        }

        let overflow = demand("overflow", SchedulerWorkClass::Animation);
        assert!(!ledger.can_admit(&overflow));
        assert_eq!(
            ledger.record_admission(overflow.key()),
            SchedulerAdmissionResult::OverflowDeferred
        );
        let before_epoch_observation = ledger.overflow_deferrals();
        ledger.complete_epoch(std::slice::from_ref(&overflow));
        assert_eq!(ledger.overflow_deferrals(), before_epoch_observation + 1);
        assert!(
            ledger
                .select_candidate(std::slice::from_ref(&overflow), None)
                .is_none()
        );

        ledger.remove(tracked[0].key());
        assert_eq!(
            ledger.record_admission(overflow.key()),
            SchedulerAdmissionResult::Tracked
        );
    }

    #[test]
    fn coalescing_policy_keeps_discrete_boundaries_and_terminal_events() {
        for kind in [
            SchedulerWorkKind::Animation,
            SchedulerWorkKind::PointerMotion,
            SchedulerWorkKind::Scroll,
            SchedulerWorkKind::Redraw,
            SchedulerWorkKind::StaleVisual,
            SchedulerWorkKind::Background,
        ] {
            assert!(kind.is_latest_wins());
        }
        for kind in [
            SchedulerWorkKind::DiscreteInput,
            SchedulerWorkKind::Lifecycle,
            SchedulerWorkKind::EditTerminal,
            SchedulerWorkKind::PlatformCompletion,
        ] {
            assert!(!kind.is_latest_wins());
        }
    }
}
