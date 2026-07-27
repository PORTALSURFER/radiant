//! Timer worker loop and opaque wake delivery.

use super::queue::{
    TimerEntry, TimerPayload, TimerState, lock_timer_queue, wait_for_timer_work,
    wait_until_timer_due,
};
use super::timing::min_timer_delay;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) fn timer_loop(state: Arc<TimerState>) {
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
                    let Some(entry) = queue.entries.pop() else {
                        continue;
                    };
                    break entry;
                }
                queue = wait_until_timer_due(&state, queue, due.saturating_duration_since(now));
            }
        };
        deliver_timer_wake(&state, entry);
    }
}

fn deliver_timer_wake(state: &TimerState, entry: TimerEntry) {
    let Some(runtime) = entry.runtime.upgrade() else {
        return;
    };
    let (identity, recurring, every) = match entry.payload {
        TimerPayload::Once { identity } => (identity, false, Duration::ZERO),
        TimerPayload::Interval { every, identity } => (identity, true, every),
    };
    if !runtime.admit_timer(identity) {
        // Give one-shot admission a chance to retire its reservation even
        // when the identity was invalidated before the timer became due.
        let _ = runtime.enqueue_timer_wake(identity);
        return;
    }
    let _accepted = runtime.enqueue_timer_wake(identity);
    if recurring && runtime.timer_is_current(identity) {
        let _ = state.schedule_interval(
            Arc::downgrade(&runtime),
            every.max(min_timer_delay()),
            identity,
        );
    }
}
