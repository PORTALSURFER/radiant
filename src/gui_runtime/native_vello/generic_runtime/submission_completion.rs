//! Completion witnessing for native WGPU submissions.

use super::{RuntimeUserEvent, adapter::NativeAdapterGeneration};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vello::wgpu;
use winit::event_loop::EventLoopProxy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeSubmissionCallbackId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeSubmissionCompletionPhase {
    NeverSubmitted,
    Pending {
        callback_id: NativeSubmissionCallbackId,
        rearm_required: bool,
    },
    Completed {
        rearm_required: bool,
    },
    Exhausted,
}

/// Pure state transitions for one native submission completion witness.
///
/// A callback covers the queue work that preceded its registration. If more
/// work is submitted while that callback is pending, the completion is not
/// sufficient for retirement: a second callback must be registered after the
/// first one completes. This keeps the state conservative without registering
/// an unbounded callback stream for a busy queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeSubmissionCompletionState {
    phase: NativeSubmissionCompletionPhase,
    next_callback_id: u64,
}

impl Default for NativeSubmissionCompletionState {
    fn default() -> Self {
        Self {
            phase: NativeSubmissionCompletionPhase::NeverSubmitted,
            next_callback_id: 0,
        }
    }
}

impl NativeSubmissionCompletionState {
    pub(super) fn record_successful_submission(&mut self) -> Option<NativeSubmissionCallbackId> {
        match self.phase {
            NativeSubmissionCompletionPhase::NeverSubmitted
            | NativeSubmissionCompletionPhase::Completed {
                rearm_required: false,
            } => self.allocate_callback(),
            NativeSubmissionCompletionPhase::Pending {
                callback_id,
                rearm_required: _,
            } => {
                self.phase = NativeSubmissionCompletionPhase::Pending {
                    callback_id,
                    rearm_required: true,
                };
                None
            }
            NativeSubmissionCompletionPhase::Completed {
                rearm_required: true,
            }
            | NativeSubmissionCompletionPhase::Exhausted => None,
        }
    }

    pub(super) fn observe_callback_completion(
        &mut self,
        callback_id: NativeSubmissionCallbackId,
    ) -> bool {
        let NativeSubmissionCompletionPhase::Pending {
            callback_id: expected,
            rearm_required,
        } = self.phase
        else {
            return false;
        };
        if expected != callback_id {
            return false;
        }
        self.phase = NativeSubmissionCompletionPhase::Completed { rearm_required };
        true
    }

    pub(super) fn prepare_rearm(&mut self) -> Option<NativeSubmissionCallbackId> {
        if !matches!(
            self.phase,
            NativeSubmissionCompletionPhase::Completed {
                rearm_required: true,
            }
        ) {
            return None;
        }
        self.allocate_callback()
    }

    pub(super) const fn retirement_eligible(self) -> bool {
        matches!(
            self.phase,
            NativeSubmissionCompletionPhase::NeverSubmitted
                | NativeSubmissionCompletionPhase::Completed {
                    rearm_required: false,
                }
        )
    }

    pub(super) const fn callback_pending(self) -> bool {
        matches!(self.phase, NativeSubmissionCompletionPhase::Pending { .. })
    }

    pub(super) const fn rearm_required(self) -> bool {
        matches!(
            self.phase,
            NativeSubmissionCompletionPhase::Pending {
                rearm_required: true,
                ..
            } | NativeSubmissionCompletionPhase::Completed {
                rearm_required: true,
            }
        )
    }

    fn allocate_callback(&mut self) -> Option<NativeSubmissionCallbackId> {
        let Some(callback_id) = self.next_callback_id.checked_add(1) else {
            self.phase = NativeSubmissionCompletionPhase::Exhausted;
            return None;
        };
        self.next_callback_id = callback_id;
        let callback_id = NativeSubmissionCallbackId(callback_id);
        self.phase = NativeSubmissionCompletionPhase::Pending {
            callback_id,
            rearm_required: false,
        };
        Some(callback_id)
    }
}

struct NativeSubmissionCompletionSignal {
    completed_callback_id: AtomicU64,
    proxy: EventLoopProxy<RuntimeUserEvent>,
}

impl NativeSubmissionCompletionSignal {
    fn mark_completed(&self, callback_id: NativeSubmissionCallbackId) {
        let callback_id = callback_id.0;
        let mut observed = self.completed_callback_id.load(Ordering::Acquire);
        loop {
            if observed >= callback_id {
                return;
            }
            match self.completed_callback_id.compare_exchange_weak(
                observed,
                callback_id,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let _ = self
                        .proxy
                        .send_event(RuntimeUserEvent::NativeResourceMaintenanceRequested);
                    return;
                }
                Err(next) => observed = next,
            }
        }
    }
}

/// Exact-generation lifetime and progress capability retained by one bundle.
///
/// The adapter remains the logical generation owner. These cloned handles are
/// retained only so a quarantined bundle can poll and witness its own queue
/// after the adapter has moved on to a newer generation.
struct NativeSubmissionCompletionCapability {
    generation: NativeAdapterGeneration,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

pub(super) struct NativeSubmissionCompletionWitness {
    capability: NativeSubmissionCompletionCapability,
    state: NativeSubmissionCompletionState,
    signal: Arc<NativeSubmissionCompletionSignal>,
}

impl NativeSubmissionCompletionWitness {
    pub(super) fn new(
        generation: NativeAdapterGeneration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Self {
        Self {
            capability: NativeSubmissionCompletionCapability {
                generation,
                device: device.clone(),
                queue: queue.clone(),
            },
            state: NativeSubmissionCompletionState::default(),
            signal: Arc::new(NativeSubmissionCompletionSignal {
                completed_callback_id: AtomicU64::new(0),
                proxy,
            }),
        }
    }

    pub(super) fn record_successful_submission(&mut self) {
        if let Some(callback_id) = self.state.record_successful_submission() {
            self.register_callback(callback_id);
        }
    }

    /// Poll one exact-generation device without waiting, consume callback
    /// progress, and rearm only after a coalesced submission has completed.
    pub(super) fn maintain(&mut self) -> bool {
        if self.state.callback_pending() {
            let _ = self.capability.device.poll(wgpu::PollType::Poll);
        }
        let completed_callback_id =
            NativeSubmissionCallbackId(self.signal.completed_callback_id.load(Ordering::Acquire));
        let _ = self
            .state
            .observe_callback_completion(completed_callback_id);
        if self.state.rearm_required()
            && let Some(callback_id) = self.state.prepare_rearm()
        {
            self.register_callback(callback_id);
        }
        self.state.callback_pending()
    }

    pub(super) const fn retirement_eligible(&self) -> bool {
        self.state.retirement_eligible()
    }

    fn register_callback(&self, callback_id: NativeSubmissionCallbackId) {
        let signal = Arc::clone(&self.signal);
        self.capability
            .queue
            .on_submitted_work_done(move || signal.mark_completed(callback_id));
    }

    pub(super) fn generation(&self) -> NativeAdapterGeneration {
        self.capability.generation
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeSubmissionCallbackId, NativeSubmissionCompletionState};

    #[test]
    fn never_submitted_witness_is_immediately_eligible() {
        assert!(NativeSubmissionCompletionState::default().retirement_eligible());
    }

    #[test]
    fn first_submission_registers_one_callback() {
        let mut state = NativeSubmissionCompletionState::default();

        let callback = state
            .record_successful_submission()
            .expect("the first successful submission should arm a callback");

        assert!(state.callback_pending());
        assert!(!state.retirement_eligible());
        assert!(state.observe_callback_completion(callback));
        assert!(state.retirement_eligible());
    }

    #[test]
    fn submissions_coalesce_while_pending_and_require_rearm() {
        let mut state = NativeSubmissionCompletionState::default();
        let first = state.record_successful_submission().unwrap();

        assert!(state.record_successful_submission().is_none());
        assert!(state.rearm_required());
        assert!(state.observe_callback_completion(first));
        assert!(!state.retirement_eligible());

        let rearm = state
            .prepare_rearm()
            .expect("coalesced work should require a second callback");
        assert!(state.callback_pending());
        assert!(state.observe_callback_completion(rearm));
        assert!(state.retirement_eligible());
    }

    #[test]
    fn stale_and_duplicate_callback_completion_is_rejected() {
        let mut state = NativeSubmissionCompletionState::default();
        let first = state.record_successful_submission().unwrap();
        let stale = NativeSubmissionCallbackId(first.0.saturating_sub(1));

        assert!(!state.observe_callback_completion(stale));
        assert!(state.observe_callback_completion(first));
        assert!(!state.observe_callback_completion(first));

        let second = state.record_successful_submission().unwrap();
        assert!(!state.observe_callback_completion(first));
        assert!(state.observe_callback_completion(second));
        assert!(!state.observe_callback_completion(second));
    }

    #[test]
    fn completed_submission_can_arm_the_next_callback() {
        let mut state = NativeSubmissionCompletionState::default();
        let first = state.record_successful_submission().unwrap();
        assert!(state.observe_callback_completion(first));
        assert!(state.retirement_eligible());

        let second = state
            .record_successful_submission()
            .expect("a later submission should arm a fresh callback");
        assert_ne!(first, second);
        assert!(!state.retirement_eligible());
    }
}
