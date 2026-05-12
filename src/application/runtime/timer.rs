use super::queue::AppRuntime;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::{Arc, Condvar, Mutex, Weak},
    thread,
    time::{Duration, Instant},
};

const TIMER_THREAD_NAME: &str = "radiant-timer";

/// Runtime-owned timer lane for delayed UI messages.
///
/// Delays should not occupy the UI/event/render owner, and they should not
/// create one OS thread per scheduled message. This lane keeps delayed messages
/// on one ordered worker and wakes the runtime only when messages become due.
pub(super) struct TimerLane<Message> {
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
    pub(super) fn new() -> Self {
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

    pub(super) fn schedule(
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

    pub(super) fn schedule_interval(
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

struct TimerState<Message> {
    queue: Mutex<TimerQueue<Message>>,
    wake: Condvar,
}

impl<Message> Default for TimerState<Message> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(TimerQueue::default()),
            wake: Condvar::new(),
        }
    }
}

impl<Message> TimerState<Message> {
    fn schedule_once(
        &self,
        runtime: Weak<AppRuntime<Message>>,
        delay: Duration,
        message: Message,
    ) -> bool {
        self.schedule_payload(runtime, delay, TimerPayload::Once(Some(message)))
    }

    fn schedule_interval(
        &self,
        runtime: Weak<AppRuntime<Message>>,
        every: Duration,
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    ) -> bool {
        self.schedule_payload(runtime, every, TimerPayload::Interval { every, message })
    }

    fn schedule_payload(
        &self,
        runtime: Weak<AppRuntime<Message>>,
        delay: Duration,
        payload: TimerPayload<Message>,
    ) -> bool {
        let mut queue = lock_timer_queue(&self.queue);
        if queue.closed {
            return false;
        }
        let order = queue.next_order;
        queue.next_order = queue.next_order.wrapping_add(1);
        queue.entries.push(TimerEntry {
            due: due_in(delay),
            order,
            runtime,
            payload,
        });
        self.wake.notify_one();
        true
    }

    fn close(&self) {
        lock_timer_queue(&self.queue).closed = true;
        self.wake.notify_one();
    }
}

struct TimerQueue<Message> {
    entries: BinaryHeap<TimerEntry<Message>>,
    next_order: u64,
    closed: bool,
}

impl<Message> Default for TimerQueue<Message> {
    fn default() -> Self {
        Self {
            entries: BinaryHeap::new(),
            next_order: 0,
            closed: false,
        }
    }
}

struct TimerEntry<Message> {
    due: Instant,
    order: u64,
    runtime: Weak<AppRuntime<Message>>,
    payload: TimerPayload<Message>,
}

enum TimerPayload<Message> {
    Once(Option<Message>),
    Interval {
        every: Duration,
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    },
}

impl<Message> Eq for TimerEntry<Message> {}

impl<Message> PartialEq for TimerEntry<Message> {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.order == other.order
    }
}

impl<Message> Ord for TimerEntry<Message> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .due
            .cmp(&self.due)
            .then_with(|| other.order.cmp(&self.order))
    }
}

impl<Message> PartialOrd for TimerEntry<Message> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn timer_loop<Message>(state: Arc<TimerState<Message>>)
where
    Message: Send + 'static,
{
    loop {
        let entry = {
            let mut queue = lock_timer_queue(&state.queue);
            loop {
                if queue.closed {
                    return;
                }
                let Some(due) = queue.entries.peek().map(|entry| entry.due) else {
                    queue = wait_for_timer_work(&state, queue);
                    continue;
                };
                let now = Instant::now();
                if due <= now {
                    break queue.entries.pop().expect("peeked timer entry exists");
                }
                queue = wait_until_timer_due(&state, queue, due.saturating_duration_since(now));
            }
        };
        deliver_timer_message(&state, entry);
    }
}

fn deliver_timer_message<Message>(state: &TimerState<Message>, mut entry: TimerEntry<Message>)
where
    Message: Send + 'static,
{
    let Some(runtime) = entry.runtime.upgrade() else {
        return;
    };
    if !runtime.is_alive() {
        return;
    }
    match entry.payload {
        TimerPayload::Once(ref mut message) => {
            if let Some(message) = message.take() {
                let _ = runtime.enqueue(message);
            }
        }
        TimerPayload::Interval { every, message } => {
            if runtime.enqueue(message()) {
                let _ = state.schedule_interval(
                    Arc::downgrade(&runtime),
                    every.max(min_timer_delay()),
                    message,
                );
            }
        }
    }
}

fn due_in(delay: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(delay).unwrap_or(now)
}

pub(super) fn min_timer_delay() -> Duration {
    Duration::from_millis(1)
}

fn lock_timer_queue<Message>(
    queue: &Mutex<TimerQueue<Message>>,
) -> std::sync::MutexGuard<'_, TimerQueue<Message>> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_for_timer_work<'a, Message>(
    state: &TimerState<Message>,
    queue: std::sync::MutexGuard<'a, TimerQueue<Message>>,
) -> std::sync::MutexGuard<'a, TimerQueue<Message>> {
    state
        .wake
        .wait(queue)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_until_timer_due<'a, Message>(
    state: &TimerState<Message>,
    queue: std::sync::MutexGuard<'a, TimerQueue<Message>>,
    duration: Duration,
) -> std::sync::MutexGuard<'a, TimerQueue<Message>> {
    state
        .wake
        .wait_timeout(queue, duration)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
}

#[cfg(test)]
mod tests {
    use super::TimerLane;

    #[test]
    fn timer_lane_rejects_work_when_worker_is_unavailable() {
        let lane = TimerLane::<u32>::without_worker_for_test();

        assert!(!lane.schedule(std::sync::Weak::new(), std::time::Duration::ZERO, 1));
    }
}
