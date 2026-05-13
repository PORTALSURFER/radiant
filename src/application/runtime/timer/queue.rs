//! Ordered timer queue and shared worker state.

use super::timing::due_in;
use crate::application::runtime::queue::AppRuntime;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::{Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

pub(super) struct TimerState<Message> {
    pub(super) queue: Mutex<TimerQueue<Message>>,
    pub(super) wake: Condvar,
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
    pub(super) fn schedule_once(
        &self,
        runtime: Weak<AppRuntime<Message>>,
        delay: Duration,
        message: Message,
    ) -> bool {
        self.schedule_payload(runtime, delay, TimerPayload::Once(Some(message)))
    }

    pub(super) fn schedule_interval(
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

    pub(super) fn close(&self) {
        lock_timer_queue(&self.queue).closed = true;
        self.wake.notify_one();
    }
}

pub(super) struct TimerQueue<Message> {
    pub(super) entries: BinaryHeap<TimerEntry<Message>>,
    pub(super) next_order: u64,
    pub(super) closed: bool,
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

pub(super) struct TimerEntry<Message> {
    pub(super) due: Instant,
    pub(super) order: u64,
    pub(super) runtime: Weak<AppRuntime<Message>>,
    pub(super) payload: TimerPayload<Message>,
}

pub(super) enum TimerPayload<Message> {
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

pub(super) fn lock_timer_queue<Message>(
    queue: &Mutex<TimerQueue<Message>>,
) -> std::sync::MutexGuard<'_, TimerQueue<Message>> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn wait_for_timer_work<'a, Message>(
    state: &TimerState<Message>,
    queue: std::sync::MutexGuard<'a, TimerQueue<Message>>,
) -> std::sync::MutexGuard<'a, TimerQueue<Message>> {
    state
        .wake
        .wait(queue)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn wait_until_timer_due<'a, Message>(
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
