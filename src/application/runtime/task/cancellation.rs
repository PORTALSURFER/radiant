use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(test)]
#[path = "cancellation/tests.rs"]
mod tests;

/// Cloneable cooperative cancellation token for host-owned background work.
///
/// Radiant never force-stops running closures. Hosts pass this token into work
/// that can periodically check [`Self::is_cancelled`] and return early.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Build a fresh non-cancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation for every clone of this token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Return whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
