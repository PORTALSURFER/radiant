//! Logical native residency accounting for prepared raw-signal GPU resources.
//!
//! This deliberately accounts retained buffer and body-texture handles, not
//! driver allocation size, submission completion, or GPU fences.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const MAX_LOGICAL_SIGNAL_GPU_BYTES: usize = 128 * 1024 * 1024;

pub(super) struct SignalGpuBudget {
    limit: usize,
    used: AtomicUsize,
}

impl SignalGpuBudget {
    fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            used: AtomicUsize::new(0),
        }
    }

    /// Reserve a logical resident handle before creating its native resource.
    pub(super) fn reserve(self: &Arc<Self>, bytes: usize) -> Option<SignalGpuLease> {
        let mut used = self.used.load(Ordering::Acquire);
        loop {
            let next = used.checked_add(bytes)?;
            if next > self.limit {
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

    #[cfg(test)]
    pub(super) fn with_limit_for_test(limit: usize) -> Arc<Self> {
        Arc::new(Self::with_limit(limit))
    }

    #[cfg(test)]
    pub(super) fn used_for_test(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
}

impl Default for SignalGpuBudget {
    fn default() -> Self {
        Self::with_limit(MAX_LOGICAL_SIGNAL_GPU_BYTES)
    }
}

/// RAII ownership of one logical native handle reservation.
pub(super) struct SignalGpuLease {
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
                Ok(_) => return,
                Err(observed) => used = observed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
