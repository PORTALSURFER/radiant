//! Bounded native whole-run closing policy.

use std::time::{Duration, Instant};

pub(super) const NATIVE_CLOSING_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(16);
pub(super) const NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES: u8 = 16;
pub(super) const NATIVE_CLOSING_MAX_DURATION: Duration = Duration::from_millis(250);
pub(super) const NATIVE_RECOVERY_MAX_DURATION: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeClosingBudget {
    first_admitted_at: Instant,
    maintenance_opportunities: u8,
}

impl NativeClosingBudget {
    fn new(first_admitted_at: Instant) -> Self {
        Self {
            first_admitted_at,
            maintenance_opportunities: 0,
        }
    }

    pub(super) fn deadline(self) -> Instant {
        self.first_admitted_at + NATIVE_CLOSING_MAX_DURATION
    }

    #[cfg(test)]
    pub(super) fn maintenance_opportunities(self) -> u8 {
        self.maintenance_opportunities
    }

    pub(super) fn next_opportunity_deadline(self, now: Instant) -> Instant {
        (now + NATIVE_CLOSING_MAINTENANCE_INTERVAL).min(self.deadline())
    }

    fn observe_opportunity(
        &mut self,
        now: Instant,
        native_ownership_empty: bool,
    ) -> NativeClosingProgress {
        if native_ownership_empty {
            return NativeClosingProgress::Complete;
        }
        if now >= self.deadline()
            || self.maintenance_opportunities >= NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES
        {
            return NativeClosingProgress::Cutoff;
        }
        self.maintenance_opportunities = self.maintenance_opportunities.saturating_add(1);
        if self.maintenance_opportunities >= NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES
            || now >= self.deadline()
        {
            NativeClosingProgress::Cutoff
        } else {
            NativeClosingProgress::Continue
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeRecoveryBudget {
    first_admitted_at: Instant,
}

impl NativeRecoveryBudget {
    fn new(first_admitted_at: Instant) -> Self {
        Self { first_admitted_at }
    }

    fn deadline(self) -> Instant {
        self.first_admitted_at + NATIVE_RECOVERY_MAX_DURATION
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeClosingProgress {
    Continue,
    Complete,
    Cutoff,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum NativeLifecycle {
    #[default]
    Running,
    Recovering(NativeRecoveryBudget),
    Closing(NativeClosingBudget),
    Stopped,
}

impl NativeLifecycle {
    pub(super) const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    pub(super) const fn is_closing(self) -> bool {
        matches!(self, Self::Closing(_))
    }

    pub(super) const fn is_recovering(self) -> bool {
        matches!(self, Self::Recovering(_))
    }

    pub(super) const fn is_stopped(self) -> bool {
        matches!(self, Self::Stopped)
    }

    pub(super) fn admit_closing(&mut self, first_admitted_at: Instant) -> bool {
        if !matches!(self, Self::Running | Self::Recovering(_)) {
            return false;
        }
        *self = Self::Closing(NativeClosingBudget::new(first_admitted_at));
        true
    }

    pub(super) fn admit_recovery(&mut self) -> bool {
        self.admit_recovery_at(Instant::now())
    }

    fn admit_recovery_at(&mut self, first_admitted_at: Instant) -> bool {
        if !self.is_running() {
            return false;
        }
        *self = Self::Recovering(NativeRecoveryBudget::new(first_admitted_at));
        true
    }

    pub(super) fn finish_recovery(&mut self) -> bool {
        if !self.is_recovering() {
            return false;
        }
        *self = Self::Running;
        true
    }

    pub(super) fn observe_closing_opportunity(
        &mut self,
        now: Instant,
        native_ownership_empty: bool,
    ) -> Option<NativeClosingProgress> {
        let Self::Closing(budget) = self else {
            return None;
        };
        Some(budget.observe_opportunity(now, native_ownership_empty))
    }

    pub(super) fn closing_deadline(&self, now: Instant) -> Option<Instant> {
        match self {
            Self::Closing(budget) => Some(budget.next_opportunity_deadline(now)),
            Self::Running | Self::Recovering(_) | Self::Stopped => None,
        }
    }

    pub(super) fn recovery_deadline(&self) -> Option<Instant> {
        match self {
            Self::Recovering(budget) => Some(budget.deadline()),
            Self::Running | Self::Closing(_) | Self::Stopped => None,
        }
    }

    pub(super) fn recovery_expired(&self, now: Instant) -> bool {
        self.recovery_deadline()
            .is_some_and(|deadline| now >= deadline)
    }

    pub(super) fn finish_closing(&mut self) -> bool {
        if !self.is_closing() {
            return false;
        }
        *self = Self::Stopped;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NATIVE_CLOSING_MAX_DURATION, NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES,
        NATIVE_RECOVERY_MAX_DURATION, NativeClosingProgress, NativeLifecycle,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn first_and_duplicate_admission_is_monotonic() {
        let first_admitted_at = Instant::now();
        let mut lifecycle = NativeLifecycle::default();

        assert!(lifecycle.admit_closing(first_admitted_at));
        assert!(!lifecycle.admit_closing(first_admitted_at + Duration::from_millis(1)));
        assert!(lifecycle.is_closing());
    }

    #[test]
    fn immediate_completion_does_not_consume_a_maintenance_opportunity() {
        let mut lifecycle = NativeLifecycle::default();
        lifecycle.admit_closing(Instant::now());

        assert_eq!(
            lifecycle.observe_closing_opportunity(Instant::now(), true),
            Some(NativeClosingProgress::Complete)
        );
    }

    #[test]
    fn pending_retirement_consumes_one_bounded_opportunity() {
        let first_admitted_at = Instant::now();
        let mut lifecycle = NativeLifecycle::default();
        lifecycle.admit_closing(first_admitted_at);

        assert_eq!(
            lifecycle.observe_closing_opportunity(first_admitted_at, false),
            Some(NativeClosingProgress::Continue)
        );
        let NativeLifecycle::Closing(budget) = lifecycle else {
            panic!("closing admission should retain its budget");
        };
        assert_eq!(budget.maintenance_opportunities(), 1);
    }

    #[test]
    fn deadline_cutoff_is_absolute_from_first_admission() {
        let first_admitted_at = Instant::now();
        let mut lifecycle = NativeLifecycle::default();
        lifecycle.admit_closing(first_admitted_at);

        assert_eq!(
            lifecycle.observe_closing_opportunity(
                first_admitted_at + NATIVE_CLOSING_MAX_DURATION,
                false,
            ),
            Some(NativeClosingProgress::Cutoff)
        );
    }

    #[test]
    fn turn_budget_cutoff_is_at_most_sixteen_opportunities() {
        let first_admitted_at = Instant::now();
        let mut lifecycle = NativeLifecycle::default();

        lifecycle.admit_closing(first_admitted_at);
        for opportunity in 1..NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES {
            assert_eq!(
                lifecycle.observe_closing_opportunity(
                    first_admitted_at + Duration::from_millis(u64::from(opportunity)),
                    false,
                ),
                Some(NativeClosingProgress::Continue)
            );
        }
        assert_eq!(
            lifecycle.observe_closing_opportunity(
                first_admitted_at
                    + Duration::from_millis(u64::from(
                        NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES,
                    )),
                false,
            ),
            Some(NativeClosingProgress::Cutoff)
        );
    }

    #[test]
    fn stopped_transition_is_idempotent_and_cannot_reopen() {
        let mut lifecycle = NativeLifecycle::default();
        lifecycle.admit_closing(Instant::now());

        assert!(lifecycle.finish_closing());
        assert!(!lifecycle.finish_closing());
        assert!(!lifecycle.admit_closing(Instant::now()));
        assert!(lifecycle.is_stopped());
    }

    #[test]
    fn recovery_round_trip_preserves_running_lifecycle_without_closing() {
        let mut lifecycle = NativeLifecycle::default();

        assert!(lifecycle.admit_recovery_at(Instant::now()));
        assert!(lifecycle.is_recovering());
        assert!(!lifecycle.is_closing());
        assert!(!lifecycle.admit_recovery_at(Instant::now()));
        assert!(lifecycle.finish_recovery());
        assert!(lifecycle.is_running());
    }

    #[test]
    fn recovery_failure_can_enter_the_existing_closing_budget() {
        let mut lifecycle = NativeLifecycle::default();

        assert!(lifecycle.admit_recovery_at(Instant::now()));
        assert!(lifecycle.admit_closing(Instant::now()));
        assert!(lifecycle.is_closing());
        assert!(!lifecycle.is_recovering());
    }

    #[test]
    fn recovery_deadline_is_absolute_from_first_admission() {
        let first_admitted_at = Instant::now();
        let mut lifecycle = NativeLifecycle::default();

        assert!(lifecycle.admit_recovery_at(first_admitted_at));
        assert_eq!(
            lifecycle.recovery_deadline(),
            Some(first_admitted_at + NATIVE_RECOVERY_MAX_DURATION)
        );
        assert!(!lifecycle.recovery_expired(
            first_admitted_at + NATIVE_RECOVERY_MAX_DURATION - Duration::from_millis(1)
        ));
        assert!(lifecycle.recovery_expired(first_admitted_at + NATIVE_RECOVERY_MAX_DURATION));
    }
}
