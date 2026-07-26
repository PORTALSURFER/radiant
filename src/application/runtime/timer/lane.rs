//! Runtime-facing timer lane API.

use super::queue::{TimerIdentity, TimerSink, TimerState};
use super::worker::timer_loop;
use std::{
    sync::{Arc, Weak},
    thread,
    time::Duration,
};

#[cfg(test)]
#[path = "lane/tests.rs"]
mod tests;

const TIMER_THREAD_NAME: &str = "radiant-timer";

/// Runtime-owned timer lane for delayed UI wakes.
///
/// Delays should not occupy the UI/event/render owner, and they should not
/// create one OS thread per scheduled delay. This lane keeps opaque timer
/// identities on one ordered worker and wakes the runtime only when identities
/// become due; the UI runtime maps and reduces the resulting message.
pub(in crate::application::runtime) struct TimerLane {
    state: Option<Arc<TimerState>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Default for TimerLane {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerLane {
    pub(in crate::application::runtime) fn new() -> Self {
        let state = Arc::new(TimerState::default());
        let worker_state = Arc::clone(&state);
        match thread::Builder::new()
            .name(TIMER_THREAD_NAME.to_string())
            .spawn(move || timer_loop(worker_state))
        {
            Ok(worker) => Self {
                state: Some(state),
                worker: Some(worker),
            },
            Err(error) => {
                tracing::warn!(
                    thread.name = TIMER_THREAD_NAME,
                    error = %error,
                    "Radiant app runtime failed to spawn timer lane"
                );
                Self {
                    state: None,
                    worker: None,
                }
            }
        }
    }

    pub(in crate::application::runtime) fn schedule(
        &self,
        runtime: Weak<dyn TimerSink>,
        delay: Duration,
        identity: TimerIdentity,
    ) -> bool {
        let Some(state) = &self.state else {
            tracing::warn!(
                "Radiant app runtime has no timer lane available for delayed wake; refusing to block the UI path"
            );
            return false;
        };
        state.schedule_once(runtime, delay, identity)
    }

    pub(in crate::application::runtime) fn schedule_interval(
        &self,
        runtime: Weak<dyn TimerSink>,
        every: Duration,
        identity: TimerIdentity,
    ) -> bool {
        let Some(state) = &self.state else {
            tracing::warn!(
                "Radiant app runtime has no timer lane available for interval subscription; refusing to block the UI path"
            );
            return false;
        };
        state.schedule_interval(runtime, every, identity)
    }

    #[cfg(test)]
    pub(super) fn without_worker_for_test() -> Self {
        Self {
            state: None,
            worker: None,
        }
    }

    pub(in crate::application::runtime) fn close(&self) {
        if let Some(state) = &self.state {
            state.close();
        }
    }
}

impl Drop for TimerLane {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            state.close();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
