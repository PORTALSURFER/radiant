//! Timer worker loop and message delivery.

use super::queue::{
    TimerEntry, TimerPayload, TimerState, lock_timer_queue, wait_for_timer_work,
    wait_until_timer_due,
};
use super::timing::min_timer_delay;
use std::{sync::Arc, time::Instant};

pub(super) fn timer_loop<Message>(state: Arc<TimerState<Message>>)
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
                    let Some(entry) = queue.entries.pop() else {
                        continue;
                    };
                    break entry;
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
