//! Qualified application-owned effect construction.

use super::{
    BusinessEventSink, Command, TaskPriority,
    command::{WorkerEffectSink, WorkerStreamOptions},
};
use std::time::Duration;

/// Additive facade for application-owned deferred runtime work.
///
/// `Effect` is a qualified runtime API. Convert it with [`Command::effect`]
/// or [`Into<Command<Message>>`](From::from) before returning or queueing it.
/// The facade deliberately lowers to the existing timer and worker command
/// lanes; the controller remains responsible for the private owner,
/// generation, cancellation, admission, rollback, and late-result policy.
///
/// Worker closures may cross a worker boundary only with owned `Send` output
/// values. The `Message` value and every mapper stay on the UI owner, so they
/// do not require `Send` or `Sync`.
#[must_use = "an Effect must be converted to a Command before it is executed"]
pub struct Effect<Message> {
    command: Command<Message>,
}

impl<Message> Effect<Message> {
    /// Build a UI-owned delayed message effect.
    ///
    /// The timer lane transports only an opaque wake. `message` is retained by
    /// the UI-owned mapper until the wake is admitted on a later UI turn.
    pub fn after(delay: Duration, message: Message) -> Self
    where
        Message: 'static,
    {
        Self {
            command: Command::after(delay, message),
        }
    }

    /// Build a one-shot worker effect.
    ///
    /// `work` runs on the runtime worker lane and returns an owned `Send`
    /// value. `map` runs on the UI owner after the worker completion is
    /// admitted. `priority` is a scheduling hint and does not change the
    /// worker transport lane.
    pub fn worker<Output>(
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self {
            command: Command::perform_worker_effect_with_priority(
                name, priority, None, 0, work, map,
            ),
        }
    }

    /// Build an ordered worker stream effect.
    ///
    /// The worker can emit owned `Event` values through `events`. Accepted
    /// events are delivered FIFO before the final output. Both mappers run on
    /// the UI owner and are intentionally not required to be `Send`.
    pub fn ordered_stream<Event, Output>(
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::stream(name, priority, false, work, map_event, map_final)
    }

    /// Build a latest-wins worker stream effect.
    ///
    /// While the UI is behind, pending intermediate events are coalesced to
    /// the newest accepted value. The final output remains ordered and is
    /// delivered once after the retained event.
    pub fn latest_stream<Event, Output>(
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::stream(name, priority, true, work, map_event, map_final)
    }

    fn stream<Event, Output>(
        name: &'static str,
        priority: TaskPriority,
        latest: bool,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        let worker = move |sink: WorkerEffectSink| {
            let event_sink = BusinessEventSink::new({
                let sink = sink.clone();
                move |event| {
                    if latest {
                        sink.emit_latest(Box::new(event))
                    } else {
                        sink.emit(Box::new(event))
                    }
                }
            });
            let close_guard = latest.then(|| LatestStreamCloseGuard::new(sink.clone()));
            let output = work(event_sink);
            if let Some(close_guard) = close_guard {
                close_guard.close();
            }
            output
        };

        Self {
            command: Command::perform_worker_stream_with_priority(
                name,
                priority,
                WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest,
                },
                worker,
                map_event,
                map_final,
            ),
        }
    }

    fn into_command(self) -> Command<Message> {
        self.command
    }
}

struct LatestStreamCloseGuard(Option<WorkerEffectSink>);

impl LatestStreamCloseGuard {
    fn new(sink: WorkerEffectSink) -> Self {
        Self(Some(sink))
    }

    fn close(mut self) {
        self.close_inner();
    }

    fn close_inner(&mut self) {
        if let Some(sink) = self.0.take() {
            sink.close_latest();
        }
    }
}

impl Drop for LatestStreamCloseGuard {
    fn drop(&mut self) {
        self.close_inner();
    }
}

impl<Message> From<Effect<Message>> for Command<Message> {
    /// Lower one facade effect into its existing runtime command lane.
    fn from(effect: Effect<Message>) -> Self {
        effect.into_command()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::command::{EffectGeneration, WorkerEffectMapper, WorkerEffectWork};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn facade_bridge_preserves_timer_and_worker_lanes_and_ui_local_mappers() {
        let timer = Effect::after(Duration::from_millis(4), Rc::new("timer"));
        assert!(matches!(Command::effect(timer), Command::Timer(_)));

        let mapped = Rc::new(Cell::new(false));
        let worker = Effect::worker("facade-worker", TaskPriority::Interactive, || 7_u8, {
            let mapped = Rc::clone(&mapped);
            move |value| {
                mapped.set(value == 7);
                Rc::new("worker")
            }
        });
        let command: Command<Rc<&str>> = worker.into();
        let Command::PerformWorker(effect) = command else {
            panic!("worker facade must use the worker command lane");
        };
        assert_eq!(effect.name, "facade-worker");
        assert_eq!(effect.priority, TaskPriority::Interactive);
        assert!(effect.owner.is_none());
        assert_eq!(effect.generation, EffectGeneration(0));
        assert!(effect.transaction.is_none());
        assert!(effect.admission_receipt.is_none());
        assert!(effect.is_cancelled.is_none());
        match effect.mapper {
            WorkerEffectMapper::Once(map) => {
                let message = map(Box::new(7_u8)).expect("matching output maps");
                assert_eq!(*message, "worker");
            }
            WorkerEffectMapper::Stream { .. } => panic!("one-shot facade must map once"),
        }
        assert!(mapped.get());
    }

    #[test]
    fn facade_stream_modes_retain_distinct_fifo_and_latest_policies() {
        let ordered: Command<u8> = Effect::ordered_stream(
            "ordered-facade",
            TaskPriority::Background,
            |events: BusinessEventSink<u8>| {
                assert!(events.emit(1));
                2_u8
            },
            |event| event,
            |output| output,
        )
        .into();
        let latest: Command<u8> = Effect::latest_stream(
            "latest-facade",
            TaskPriority::Background,
            |events: BusinessEventSink<u8>| {
                assert!(events.emit(1));
                2_u8
            },
            |event| event,
            |output| output,
        )
        .into();

        for (command, expected_latest) in [(ordered, false), (latest, true)] {
            let Command::PerformWorker(effect) = command else {
                panic!("stream facade must use the worker command lane");
            };
            let WorkerEffectMapper::Stream { latest, .. } = effect.mapper else {
                panic!("stream facade must retain stream mapping");
            };
            assert_eq!(latest, expected_latest);
            assert!(matches!(effect.work, WorkerEffectWork::Stream(_)));
        }
    }
}
