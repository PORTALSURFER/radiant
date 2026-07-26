//! Ordered timer queue and shared worker state.

use super::timing::due_in;
use crate::runtime::RuntimeTimerWake;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::{Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

/// Opaque identity carried from the timer worker back to the UI owner.
pub(crate) type TimerIdentity = RuntimeTimerWake;
pub(crate) type TimerWake = RuntimeTimerWake;

/// Runtime ingress implemented by the UI-owned application runtime.
pub(crate) trait TimerSink: Send + Sync {
    fn admit_timer(&self, identity: TimerIdentity) -> bool;
    fn enqueue_timer_wake(&self, wake: TimerWake) -> bool;
    fn timer_is_current(&self, identity: TimerIdentity) -> bool;
}

pub(super) struct TimerState {
    pub(super) queue: Mutex<TimerQueue>,
    pub(super) wake: Condvar,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            queue: Mutex::new(TimerQueue::default()),
            wake: Condvar::new(),
        }
    }
}

impl TimerState {
    pub(super) fn schedule_once(
        &self,
        runtime: Weak<dyn TimerSink>,
        delay: Duration,
        identity: TimerIdentity,
    ) -> bool {
        self.schedule_payload(runtime, delay, TimerPayload::Once { identity })
    }

    pub(super) fn schedule_interval(
        &self,
        runtime: Weak<dyn TimerSink>,
        every: Duration,
        identity: TimerIdentity,
    ) -> bool {
        self.schedule_payload(runtime, every, TimerPayload::Interval { every, identity })
    }

    fn schedule_payload(
        &self,
        runtime: Weak<dyn TimerSink>,
        delay: Duration,
        payload: TimerPayload,
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

    pub(super) fn close(&self) {
        lock_timer_queue(&self.queue).closed = true;
        self.wake.notify_one();
    }
}

#[derive(Default)]
pub(super) struct TimerQueue {
    pub(super) entries: BinaryHeap<TimerEntry>,
    pub(super) next_order: u64,
    pub(super) closed: bool,
}

pub(super) struct TimerEntry {
    pub(super) due: Instant,
    pub(super) order: u64,
    pub(super) runtime: Weak<dyn TimerSink>,
    pub(super) payload: TimerPayload,
}

pub(super) enum TimerPayload {
    Once {
        identity: TimerIdentity,
    },
    Interval {
        every: Duration,
        identity: TimerIdentity,
    },
}

impl Eq for TimerEntry {}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.order == other.order
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .due
            .cmp(&self.due)
            .then_with(|| other.order.cmp(&self.order))
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn lock_timer_queue(queue: &Mutex<TimerQueue>) -> std::sync::MutexGuard<'_, TimerQueue> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn wait_for_timer_work<'a>(
    state: &TimerState,
    queue: std::sync::MutexGuard<'a, TimerQueue>,
) -> std::sync::MutexGuard<'a, TimerQueue> {
    state
        .wake
        .wait(queue)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn wait_until_timer_due<'a>(
    state: &TimerState,
    queue: std::sync::MutexGuard<'a, TimerQueue>,
    duration: Duration,
) -> std::sync::MutexGuard<'a, TimerQueue> {
    state
        .wake
        .wait_timeout(queue, duration)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
}

pub(crate) fn timer_sink<T>(runtime: &Arc<T>) -> Weak<dyn TimerSink>
where
    T: TimerSink + 'static,
{
    let runtime: Arc<dyn TimerSink> = runtime.clone();
    Arc::downgrade(&runtime)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSink;

    impl TimerSink for TestSink {
        fn admit_timer(&self, _: TimerIdentity) -> bool {
            true
        }

        fn enqueue_timer_wake(&self, _: TimerWake) -> bool {
            true
        }

        fn timer_is_current(&self, _: TimerIdentity) -> bool {
            true
        }
    }

    #[test]
    fn equal_due_interval_and_command_wakes_preserve_insertion_order() {
        let sink: Arc<dyn TimerSink> = Arc::new(TestSink);
        let due = Instant::now();
        let application = RuntimeTimerWake::application(1, 0, 1);
        let controller = RuntimeTimerWake::controller(1, 0, 1);
        let mut queue = BinaryHeap::new();
        queue.push(TimerEntry {
            due,
            order: 0,
            runtime: Arc::downgrade(&sink),
            payload: TimerPayload::Interval {
                every: Duration::from_millis(10),
                identity: application,
            },
        });
        queue.push(TimerEntry {
            due,
            order: 1,
            runtime: Arc::downgrade(&sink),
            payload: TimerPayload::Once {
                identity: controller,
            },
        });

        let first = queue.pop().expect("interval wake");
        let second = queue.pop().expect("command wake");
        assert_eq!(first.order, 0);
        assert_eq!(second.order, 1);
        assert!(matches!(
            first.payload,
            TimerPayload::Interval { identity, .. }
                if identity.owner == crate::runtime::RuntimeTimerOwner::Application
        ));
        assert!(matches!(
            second.payload,
            TimerPayload::Once { identity }
                if identity.owner == crate::runtime::RuntimeTimerOwner::Controller
        ));
    }
}
