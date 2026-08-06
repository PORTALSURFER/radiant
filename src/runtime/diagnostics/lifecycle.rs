use std::collections::VecDeque;

/// Maximum number of lifecycle transitions retained in one runtime snapshot.
pub const RUNTIME_LIFECYCLE_HISTORY_CAPACITY: usize = 8;

/// Typed lifecycle phase for a generic [`SurfaceRuntime`](crate::runtime::SurfaceRuntime).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeLifecyclePhase {
    /// No controller-owned lifecycle evidence is available.
    #[default]
    Unknown,
    /// The controller is being constructed and is not yet ready for work.
    Starting,
    /// The controller is ready to accept runtime work.
    Running,
    /// The controller is rebuilding or otherwise preparing to resume work.
    Recovering,
    /// The controller is closing and no longer accepts new work.
    Closing,
    /// The controller has completed its terminal exit transition.
    Stopped,
}

impl RuntimeLifecyclePhase {
    pub(crate) const fn accepts_work(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Recovering)
    }
}

/// One accepted generic runtime lifecycle transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeLifecycleTransition {
    /// Saturating one-based sequence number assigned to this transition.
    pub sequence: u64,
    /// Phase before the accepted transition.
    pub from: RuntimeLifecyclePhase,
    /// Phase after the accepted transition.
    pub to: RuntimeLifecyclePhase,
}

/// Bounded controller-owned lifecycle evidence for a generic runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLifecycleDiagnostics {
    /// Whether controller-owned lifecycle evidence is available.
    pub available: bool,
    /// Current controller-owned lifecycle phase.
    pub phase: RuntimeLifecyclePhase,
    /// Saturating count of all accepted lifecycle transitions.
    pub transition_count: u64,
    /// The most recent bounded transitions, ordered oldest to newest.
    pub history: Vec<RuntimeLifecycleTransition>,
}

impl Default for RuntimeLifecycleDiagnostics {
    fn default() -> Self {
        Self {
            available: false,
            phase: RuntimeLifecyclePhase::Unknown,
            transition_count: 0,
            history: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeLifecycleController {
    phase: RuntimeLifecyclePhase,
    transition_count: u64,
    history: VecDeque<RuntimeLifecycleTransition>,
}

impl RuntimeLifecycleController {
    pub(crate) fn starting() -> Self {
        Self {
            phase: RuntimeLifecyclePhase::Starting,
            transition_count: 0,
            history: VecDeque::with_capacity(RUNTIME_LIFECYCLE_HISTORY_CAPACITY),
        }
    }

    pub(crate) fn phase(&self) -> RuntimeLifecyclePhase {
        self.phase
    }

    pub(crate) fn accepts_work(&self) -> bool {
        self.phase.accepts_work()
    }

    pub(crate) fn transition(&mut self, next: RuntimeLifecyclePhase) -> bool {
        if !is_legal_transition(self.phase, next) {
            return false;
        }

        let sequence = self.transition_count.saturating_add(1);
        self.transition_count = sequence;
        self.history.push_back(RuntimeLifecycleTransition {
            sequence,
            from: self.phase,
            to: next,
        });
        if self.history.len() > RUNTIME_LIFECYCLE_HISTORY_CAPACITY {
            let _ = self.history.pop_front();
        }
        self.phase = next;
        true
    }

    pub(crate) fn diagnostics(&self) -> RuntimeLifecycleDiagnostics {
        RuntimeLifecycleDiagnostics {
            available: self.transition_count != 0,
            phase: self.phase,
            transition_count: self.transition_count,
            history: self.history.iter().copied().collect(),
        }
    }
}

impl Default for RuntimeLifecycleController {
    fn default() -> Self {
        Self {
            phase: RuntimeLifecyclePhase::Unknown,
            transition_count: 0,
            history: VecDeque::with_capacity(RUNTIME_LIFECYCLE_HISTORY_CAPACITY),
        }
    }
}

fn is_legal_transition(from: RuntimeLifecyclePhase, to: RuntimeLifecyclePhase) -> bool {
    matches!(
        (from, to),
        (
            RuntimeLifecyclePhase::Starting,
            RuntimeLifecyclePhase::Running
        ) | (
            RuntimeLifecyclePhase::Starting,
            RuntimeLifecyclePhase::Closing
        ) | (
            RuntimeLifecyclePhase::Running,
            RuntimeLifecyclePhase::Recovering
        ) | (
            RuntimeLifecyclePhase::Running,
            RuntimeLifecyclePhase::Closing
        ) | (
            RuntimeLifecyclePhase::Recovering,
            RuntimeLifecyclePhase::Running
        ) | (
            RuntimeLifecyclePhase::Recovering,
            RuntimeLifecyclePhase::Closing
        ) | (
            RuntimeLifecyclePhase::Closing,
            RuntimeLifecyclePhase::Stopped
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_diagnostics_are_unavailable() {
        assert_eq!(
            RuntimeLifecycleDiagnostics::default(),
            RuntimeLifecycleDiagnostics {
                available: false,
                phase: RuntimeLifecyclePhase::Unknown,
                transition_count: 0,
                history: Vec::new(),
            }
        );
    }

    #[test]
    fn duplicate_and_illegal_transitions_are_vetoed_without_evidence() {
        let mut controller = RuntimeLifecycleController::starting();
        assert!(!controller.transition(RuntimeLifecyclePhase::Starting));
        assert!(!controller.transition(RuntimeLifecyclePhase::Recovering));
        assert_eq!(controller.diagnostics().transition_count, 0);
        assert_eq!(controller.phase(), RuntimeLifecyclePhase::Starting);
        assert!(controller.diagnostics().history.is_empty());

        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        let before = controller.diagnostics();
        assert!(!controller.transition(RuntimeLifecyclePhase::Running));
        assert!(!controller.transition(RuntimeLifecyclePhase::Starting));
        assert_eq!(controller.diagnostics(), before);
    }

    #[test]
    fn recovery_requires_running_and_returns_to_running() {
        let mut controller = RuntimeLifecycleController::starting();
        assert!(!controller.transition(RuntimeLifecyclePhase::Recovering));
        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        assert!(controller.transition(RuntimeLifecyclePhase::Recovering));
        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        assert_eq!(
            controller.diagnostics().history,
            vec![
                RuntimeLifecycleTransition {
                    sequence: 1,
                    from: RuntimeLifecyclePhase::Starting,
                    to: RuntimeLifecyclePhase::Running,
                },
                RuntimeLifecycleTransition {
                    sequence: 2,
                    from: RuntimeLifecyclePhase::Running,
                    to: RuntimeLifecyclePhase::Recovering,
                },
                RuntimeLifecycleTransition {
                    sequence: 3,
                    from: RuntimeLifecyclePhase::Recovering,
                    to: RuntimeLifecyclePhase::Running,
                },
            ]
        );
    }

    #[test]
    fn history_is_bounded_oldest_to_newest() {
        let mut controller = RuntimeLifecycleController::starting();
        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        for _ in 0..(RUNTIME_LIFECYCLE_HISTORY_CAPACITY + 2) {
            assert!(controller.transition(RuntimeLifecyclePhase::Recovering));
            assert!(controller.transition(RuntimeLifecyclePhase::Running));
        }

        let diagnostics = controller.diagnostics();
        assert_eq!(
            diagnostics.history.len(),
            RUNTIME_LIFECYCLE_HISTORY_CAPACITY
        );
        assert!(
            diagnostics
                .history
                .windows(2)
                .all(|window| window[0].sequence <= window[1].sequence)
        );
        assert_eq!(
            diagnostics
                .history
                .first()
                .map(|transition| transition.sequence),
            Some(diagnostics.transition_count - RUNTIME_LIFECYCLE_HISTORY_CAPACITY as u64 + 1)
        );
        assert_eq!(
            diagnostics
                .history
                .last()
                .map(|transition| transition.sequence),
            Some(diagnostics.transition_count)
        );
    }

    #[test]
    fn sequence_and_count_saturate_without_wrapping() {
        let mut controller = RuntimeLifecycleController::starting();
        controller.transition_count = u64::MAX - 1;
        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        assert!(controller.transition(RuntimeLifecyclePhase::Recovering));

        let diagnostics = controller.diagnostics();
        assert_eq!(diagnostics.transition_count, u64::MAX);
        assert_eq!(
            diagnostics
                .history
                .iter()
                .map(|transition| transition.sequence)
                .collect::<Vec<_>>(),
            vec![u64::MAX, u64::MAX]
        );
    }

    #[test]
    fn stopped_is_terminal() {
        let mut controller = RuntimeLifecycleController::starting();
        assert!(controller.transition(RuntimeLifecyclePhase::Running));
        assert!(controller.transition(RuntimeLifecyclePhase::Closing));
        assert!(controller.transition(RuntimeLifecyclePhase::Stopped));
        let before = controller.diagnostics();
        assert!(!controller.transition(RuntimeLifecyclePhase::Running));
        assert!(!controller.transition(RuntimeLifecyclePhase::Stopped));
        assert_eq!(controller.diagnostics(), before);
    }
}
