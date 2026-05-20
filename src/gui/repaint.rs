//! Backend-neutral repaint signaling primitives.

use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};

#[cfg(test)]
#[path = "repaint/tests.rs"]
mod tests;

/// Runtime-provided callback used by background systems to request a UI repaint.
pub trait RepaintSignal: Send + Sync {
    /// Request that the active UI backend schedules a repaint soon.
    fn request_repaint(&self);
}

/// Mark a repaint event as pending if one is not already queued.
///
/// Runtime backends use this as a small lock-free coalescing gate before
/// sending a wakeup event to a platform backend.
pub fn try_mark_repaint_pending(pending: &AtomicBool) -> bool {
    pending
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

/// Repaint signal that coalesces duplicate wakeups while one repaint is pending.
///
/// The runtime owns the `pending` flag and clears it after processing the
/// wakeup. The callback returns whether the wakeup was successfully queued; a
/// failed queue attempt clears the pending flag so later requests can retry.
pub struct CoalescingRepaintSignal {
    pending: Arc<AtomicBool>,
    queue_wakeup: Box<dyn Fn() -> bool + Send + Sync>,
}

impl CoalescingRepaintSignal {
    /// Create a coalescing repaint signal around a backend wakeup callback.
    pub fn new(
        pending: Arc<AtomicBool>,
        queue_wakeup: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            pending,
            queue_wakeup: Box::new(queue_wakeup),
        }
    }
}

impl RepaintSignal for CoalescingRepaintSignal {
    fn request_repaint(&self) {
        if !try_mark_repaint_pending(self.pending.as_ref()) {
            return;
        }
        if !(self.queue_wakeup)() {
            self.pending.store(false, Ordering::Release);
        }
    }
}

/// Shared holder for the current repaint callback.
///
/// The active runtime updates this when UI contexts change, while background
/// workers only call [`Self::request_repaint`].
#[derive(Default)]
pub struct SharedRepaintSignal {
    signal: RwLock<Option<Arc<dyn RepaintSignal>>>,
}

impl SharedRepaintSignal {
    /// Replace the active repaint callback.
    ///
    /// Passing `Some` installs a new callback for subsequent repaint requests.
    /// Passing `None` disables repaint signaling until a new callback is set.
    pub fn set_signal(&self, signal: Option<Arc<dyn RepaintSignal>>) {
        if let Ok(mut lock) = self.signal.write() {
            *lock = signal;
        }
    }

    /// Request a repaint through the active callback, if one is available.
    pub fn request_repaint(&self) {
        if let Ok(lock) = self.signal.read()
            && let Some(signal) = lock.as_ref()
        {
            signal.request_repaint();
        }
    }
}
