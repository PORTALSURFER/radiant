//! Backend-neutral repaint signaling primitives.

use std::sync::{Arc, RwLock};

/// Runtime-provided callback used by background systems to request a UI repaint.
pub trait RepaintSignal: Send + Sync {
    /// Request that the active UI backend schedules a repaint soon.
    fn request_repaint(&self);
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
        if let Ok(lock) = self.signal.read() {
            if let Some(signal) = lock.as_ref() {
                signal.request_repaint();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    #[derive(Default)]
    struct CountingSignal {
        called: Arc<AtomicBool>,
    }

    impl RepaintSignal for CountingSignal {
        fn request_repaint(&self) {
            self.called.store(true, Ordering::Release);
        }
    }

    #[test]
    fn shared_repaint_signal_noop_when_unset() {
        let signal = SharedRepaintSignal::default();
        signal.request_repaint();
    }

    #[test]
    fn shared_repaint_signal_forwards_request_to_active_callback() {
        let called = Arc::new(AtomicBool::new(false));
        let callback = Arc::new(CountingSignal {
            called: Arc::clone(&called),
        });

        let signal = SharedRepaintSignal::default();
        signal.set_signal(Some(callback));
        signal.request_repaint();

        assert!(called.load(Ordering::Acquire));
    }

    #[test]
    fn shared_repaint_signal_replaces_existing_callback() {
        let first_called = Arc::new(AtomicBool::new(false));
        let second_called = Arc::new(AtomicBool::new(false));

        let signal = SharedRepaintSignal::default();
        signal.set_signal(Some(Arc::new(CountingSignal {
            called: Arc::clone(&first_called),
        })));
        signal.set_signal(Some(Arc::new(CountingSignal {
            called: Arc::clone(&second_called),
        })));
        signal.request_repaint();

        assert!(!first_called.load(Ordering::Acquire));
        assert!(second_called.load(Ordering::Acquire));
    }
}
