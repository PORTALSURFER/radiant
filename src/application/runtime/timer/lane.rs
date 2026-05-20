//! Runtime-facing timer lane API.

use super::queue::TimerState;
use super::worker::timer_loop;
use crate::application::runtime::queue::AppRuntime;
use std::{
    sync::{Arc, Weak},
    thread,
    time::Duration,
};

#[cfg(test)]
#[path = "lane/tests.rs"]
mod tests;

const TIMER_THREAD_NAME: &str = "radiant-timer";

/// Runtime-owned timer lane for delayed UI messages.
///
/// Delays should not occupy the UI/event/render owner, and they should not
/// create one OS thread per scheduled message. This lane keeps delayed messages
/// on one ordered worker and wakes the runtime only when messages become due.
pub(in crate::application::runtime) struct TimerLane<Message> {
    state: Option<Arc<TimerState<Message>>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl<Message> Default for TimerLane<Message>
where
    Message: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> TimerLane<Message>
where
    Message: Send + 'static,
{
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
        runtime: Weak<AppRuntime<Message>>,
        delay: Duration,
        message: Message,
    ) -> bool {
        let Some(state) = &self.state else {
            tracing::warn!(
                "Radiant app runtime has no timer lane available for delayed message; refusing to block the UI path"
            );
            return false;
        };
        state.schedule_once(runtime, delay, message)
    }

    pub(in crate::application::runtime) fn schedule_interval(
        &self,
        runtime: Weak<AppRuntime<Message>>,
        every: Duration,
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    ) -> bool {
        let Some(state) = &self.state else {
            tracing::warn!(
                "Radiant app runtime has no timer lane available for interval subscription; refusing to block the UI path"
            );
            return false;
        };
        state.schedule_interval(runtime, every, message)
    }

    #[cfg(test)]
    pub(super) fn without_worker_for_test() -> Self {
        Self {
            state: None,
            worker: None,
        }
    }
}

impl<Message> Drop for TimerLane<Message> {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            state.close();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
