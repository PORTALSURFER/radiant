//! Logical native residency accounting for prepared raw-signal GPU resources.
//!
//! This deliberately accounts retained buffer and body-texture handles, not
//! driver allocation size, submission completion, or GPU fences.

use crate::gui::repaint::RepaintSignal;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

const MAX_LOGICAL_SIGNAL_GPU_BYTES: usize = 128 * 1024 * 1024;

pub(in crate::gui_runtime::native_vello::generic_runtime) struct SignalGpuBudget {
    limit: usize,
    used: AtomicUsize,
    waiting: AtomicBool,
    retry_ready: AtomicBool,
    wake: Option<Arc<dyn RepaintSignal>>,
}

impl SignalGpuBudget {
    fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            used: AtomicUsize::new(0),
            waiting: AtomicBool::new(false),
            retry_ready: AtomicBool::new(false),
            wake: None,
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn with_wake(
        wake: Arc<dyn RepaintSignal>,
    ) -> Self {
        Self {
            wake: Some(wake),
            ..Self::default()
        }
    }

    /// Reserve a logical resident handle before creating its native resource.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn reserve(
        self: &Arc<Self>,
        bytes: usize,
    ) -> Option<SignalGpuLease> {
        // A single handle larger than the full budget can never be admitted;
        // it must not arm a release-triggered retry loop.
        if bytes > self.limit {
            return None;
        }
        let mut used = self.used.load(Ordering::Acquire);
        loop {
            let Some(next) = used.checked_add(bytes) else {
                return None;
            };
            if next > self.limit {
                // Close the release/store race: if a lease released capacity
                // while this flag was being armed, retry against its new use.
                self.waiting.store(true, Ordering::Release);
                let refreshed = self.used.load(Ordering::Acquire);
                if refreshed != used {
                    used = refreshed;
                    continue;
                }
                return None;
            }
            match self
                .used
                .compare_exchange_weak(used, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    return Some(SignalGpuLease {
                        budget: Arc::clone(self),
                        bytes,
                    });
                }
                Err(observed) => used = observed,
            }
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn take_retry(&self) -> bool {
        self.retry_ready.swap(false, Ordering::AcqRel)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn logical_bytes(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn with_limit_for_test(
        limit: usize,
    ) -> Arc<Self> {
        Arc::new(Self::with_limit(limit))
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn used_for_test(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }

    #[cfg(test)]
    fn with_limit_and_wake_for_test(limit: usize, wake: Arc<dyn RepaintSignal>) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
            waiting: AtomicBool::new(false),
            retry_ready: AtomicBool::new(false),
            wake: Some(wake),
        })
    }
}

impl Default for SignalGpuBudget {
    fn default() -> Self {
        Self::with_limit(MAX_LOGICAL_SIGNAL_GPU_BYTES)
    }
}

/// RAII ownership of one logical native handle reservation.
pub(in crate::gui_runtime::native_vello::generic_runtime) struct SignalGpuLease {
    budget: Arc<SignalGpuBudget>,
    bytes: usize,
}

impl Drop for SignalGpuLease {
    fn drop(&mut self) {
        let mut used = self.budget.used.load(Ordering::Acquire);
        loop {
            let Some(next) = used.checked_sub(self.bytes) else {
                debug_assert!(false, "signal GPU budget underflow");
                return;
            };
            match self.budget.used.compare_exchange_weak(
                used,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    if self.budget.waiting.swap(false, Ordering::AcqRel) {
                        self.budget.retry_ready.store(true, Ordering::Release);
                        if let Some(wake) = &self.budget.wake {
                            wake.request_repaint();
                        }
                    }
                    return;
                }
                Err(observed) => used = observed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Wake(AtomicUsize);

    impl RepaintSignal for Wake {
        fn request_repaint(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn leases_release_logical_residency_only_after_the_last_handle_drops() {
        let budget = SignalGpuBudget::with_limit_for_test(12);
        let first = budget.reserve(8).expect("first reservation");
        assert_eq!(budget.used_for_test(), 8);
        assert!(budget.reserve(5).is_none());
        let second = budget.reserve(4).expect("second reservation");
        assert_eq!(budget.used_for_test(), 12);
        drop(first);
        assert_eq!(budget.used_for_test(), 4);
        drop(second);
        assert_eq!(budget.used_for_test(), 0);
    }

    #[test]
    fn overflowing_reservation_is_denied_without_changing_usage() {
        let budget = SignalGpuBudget::with_limit_for_test(usize::MAX);
        let held = budget.reserve(1).expect("small reservation");
        assert!(budget.reserve(usize::MAX).is_none());
        assert_eq!(budget.used_for_test(), 1);
        drop(held);
    }

    #[test]
    fn capacity_denial_wakes_once_after_another_handle_releases() {
        let wake = Arc::new(Wake(AtomicUsize::new(0)));
        let budget = SignalGpuBudget::with_limit_and_wake_for_test(8, wake.clone());
        let held = budget.reserve(8).expect("full reservation");

        assert!(budget.reserve(1).is_none());
        assert_eq!(wake.0.load(Ordering::Relaxed), 0);
        drop(held);

        assert_eq!(budget.used_for_test(), 0);
        assert_eq!(wake.0.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn permanently_oversized_denial_does_not_arm_a_release_retry() {
        let wake = Arc::new(Wake(AtomicUsize::new(0)));
        let budget = SignalGpuBudget::with_limit_and_wake_for_test(8, wake.clone());
        let held = budget.reserve(4).expect("held reservation");

        assert!(budget.reserve(9).is_none());
        drop(held);

        assert_eq!(wake.0.load(Ordering::Relaxed), 0);
    }
}
