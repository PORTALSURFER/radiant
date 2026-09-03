//! Qualified effect construction.

use super::{BusinessEventSink, Command, TaskPriority};
use crate::application::{
    CancellationToken, DeclarativeEffectOwner, LatestTask, TaskCompletion, TaskTicket,
};
use crate::runtime::command::WorkerEffectSink;
use std::time::Duration;

/// Explicit owner selected for one qualified runtime effect.
///
/// The enum is intentionally qualified: it is exported beside [`Effect`] from
/// [`crate::runtime`], but is not part of the common runtime prelude.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectOwner {
    /// Work remains owned by the application runtime.
    Application,
    /// Work is owned by the accepted live generation for this declarative handle.
    Declarative(DeclarativeEffectOwner),
}

/// Qualified facade for deferred runtime work.
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
    ticket: TaskTicket,
    token: CancellationToken,
}

impl<Message: 'static> Effect<Message> {
    /// Build a UI-owned delayed message effect.
    ///
    /// The timer lane transports only an opaque wake. `message` is retained by
    /// the UI-owned mapper until the wake is admitted on a later UI turn.
    pub fn after(
        latest: &mut LatestTask,
        owner: EffectOwner,
        delay: Duration,
        map: impl FnOnce(TaskCompletion<()>) -> Message + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        let token = CancellationToken::new();
        let identity = crate::runtime::command::EffectId(latest.effect_id());
        let command = Command::after_effect(
            identity,
            owner,
            delay,
            ticket,
            transaction,
            token.clone(),
            map,
        );
        Self {
            command,
            ticket,
            token,
        }
    }

    /// Build a one-shot worker effect.
    ///
    /// `work` runs on the runtime worker lane and returns an owned `Send`
    /// value. `map` runs on the UI owner after the worker completion is
    /// admitted. `priority` is a scheduling hint and does not change the
    /// worker transport lane.
    pub fn worker<Output>(
        latest: &mut LatestTask,
        owner: EffectOwner,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        let token = CancellationToken::new();
        let identity = crate::runtime::command::EffectId(latest.effect_id());
        let command = Command::perform_worker_effect_for_effect(
            identity,
            owner,
            name,
            priority,
            ticket,
            transaction,
            token.clone(),
            work,
            map,
        );
        Self {
            command,
            ticket,
            token,
        }
    }

    /// Build a replaceable, qualified platform-service effect.
    ///
    /// The selected owner is checked against the accepted surface before the
    /// host is invoked. The request result, including synchronous host
    /// failures, is delivered through the existing deferred platform ingress
    /// and mapped on a later UI turn.
    pub fn platform(
        latest: &mut LatestTask,
        owner: EffectOwner,
        request: crate::runtime::PlatformRequest,
        map: impl FnOnce(crate::runtime::PlatformResult) -> Message + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        let token = CancellationToken::new();
        let identity = crate::runtime::command::EffectId(latest.effect_id());
        let command = crate::runtime::Command::platform_effect(
            identity,
            owner,
            ticket,
            transaction,
            token.clone(),
            request,
            map,
        );
        Self {
            command,
            ticket,
            token,
        }
    }

    /// Build an ordered worker stream effect.
    ///
    /// The worker can emit owned `Event` values through `events`. Accepted
    /// events are delivered FIFO before the final output. Both mappers run on
    /// the UI owner and are intentionally not required to be `Send`.
    pub fn ordered_stream<Event, Output>(
        latest: &mut LatestTask,
        owner: EffectOwner,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::stream(
            latest, owner, name, priority, false, work, map_event, map_final,
        )
    }

    /// Build a latest-wins worker stream effect.
    ///
    /// While the UI is behind, pending intermediate events are coalesced to
    /// the newest accepted value. The final output remains ordered and is
    /// delivered once after the retained event.
    pub fn latest_stream<Event, Output>(
        latest: &mut LatestTask,
        owner: EffectOwner,
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        Self::stream(
            latest, owner, name, priority, true, work, map_event, map_final,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stream<Event, Output>(
        latest: &mut LatestTask,
        owner: EffectOwner,
        name: &'static str,
        priority: TaskPriority,
        latest_wins: bool,
        work: impl FnOnce(BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> Self
    where
        Event: Send + 'static,
        Output: Send + 'static,
    {
        let worker = move |sink: WorkerEffectSink| {
            let event_sink = BusinessEventSink::new({
                let sink = sink.clone();
                move |event| {
                    if latest_wins {
                        sink.emit_latest(Box::new(event))
                    } else {
                        sink.emit(Box::new(event))
                    }
                }
            });
            let close_guard = latest_wins.then(|| LatestStreamCloseGuard::new(sink.clone()));
            let output = work(event_sink);
            if let Some(close_guard) = close_guard {
                close_guard.close();
            }
            output
        };

        let transaction = latest.begin_replacement();
        let ticket = transaction.replacement();
        let token = CancellationToken::new();
        let identity = crate::runtime::command::EffectId(latest.effect_id());
        let command = Command::perform_worker_stream_for_effect(
            identity,
            owner,
            name,
            priority,
            ticket,
            transaction,
            token.clone(),
            latest_wins,
            worker,
            map_event,
            map_final,
        );
        Self {
            command,
            ticket,
            token,
        }
    }

    /// Return the reserved generation for this effect.
    pub const fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Clone the per-effect cancellation token before converting the effect.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
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

impl<Message: 'static> From<Effect<Message>> for Command<Message> {
    /// Lower one facade effect into its existing runtime command lane.
    fn from(effect: Effect<Message>) -> Self {
        effect.into_command()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::LatestTask;
    use crate::runtime::command::{EffectGeneration, WorkerEffectMapper, WorkerEffectWork};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn facade_bridge_preserves_timer_and_worker_lanes_and_ui_local_mappers() {
        let mut latest = LatestTask::new();
        let timer = Effect::after(
            &mut latest,
            EffectOwner::Application,
            Duration::from_millis(4),
            |completion| Rc::new((completion.ticket, "timer")),
        );
        let timer_ticket = timer.ticket();
        assert!(matches!(Command::effect(timer), Command::Timer(_)));

        let mut latest = LatestTask::new();
        let mapped = Rc::new(Cell::new(false));
        let worker = Effect::worker(
            &mut latest,
            EffectOwner::Application,
            "facade-worker",
            TaskPriority::Interactive,
            || 7_u8,
            {
                let mapped = Rc::clone(&mapped);
                move |completion| {
                    mapped.set(completion.output == 7 && completion.ticket.id() > 0);
                    Rc::new("worker")
                }
            },
        );
        let worker_ticket = worker.ticket();
        let command: Command<Rc<&str>> = worker.into();
        let Command::PerformWorker(effect) = command else {
            panic!("worker facade must use the worker command lane");
        };
        assert_eq!(effect.name, "facade-worker");
        assert_eq!(effect.priority, TaskPriority::Interactive);
        assert!(effect.owner.is_none());
        assert_eq!(effect.generation, EffectGeneration(worker_ticket.id()));
        assert!(effect.transaction.is_some());
        assert!(effect.lifecycle.is_some());
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
        assert!(timer_ticket.id() > 0);
    }

    #[test]
    fn facade_stream_modes_retain_distinct_fifo_and_latest_policies() {
        let mut ordered_latest = LatestTask::new();
        let ordered: Command<u8> = Effect::ordered_stream(
            &mut ordered_latest,
            EffectOwner::Application,
            "ordered-facade",
            TaskPriority::Background,
            |events: BusinessEventSink<u8>| {
                assert!(events.emit(1));
                2_u8
            },
            |completion| completion.output,
            |completion| completion.output,
        )
        .into();
        let mut latest_latest = LatestTask::new();
        let latest: Command<u8> = Effect::latest_stream(
            &mut latest_latest,
            EffectOwner::Application,
            "latest-facade",
            TaskPriority::Background,
            |events: BusinessEventSink<u8>| {
                assert!(events.emit(1));
                2_u8
            },
            |completion| completion.output,
            |completion| completion.output,
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

    #[test]
    fn facade_malformed_internal_completion_fails_closed_before_ui_mapping() {
        let mut latest = LatestTask::new();
        let mapped = Rc::new(Cell::new(false));
        let effect = Effect::worker(
            &mut latest,
            EffectOwner::Application,
            "malformed-facade",
            TaskPriority::Background,
            || 7_u8,
            {
                let mapped = Rc::clone(&mapped);
                move |_| {
                    mapped.set(true);
                    Rc::from("mapped")
                }
            },
        );
        let command: Command<Rc<str>> = effect.into();
        let Command::PerformWorker(effect) = command else {
            panic!("worker facade must use the worker command lane");
        };
        let WorkerEffectMapper::Once(map) = effect.mapper else {
            panic!("one-shot facade must use one-shot mapping");
        };
        assert!(map(Box::new("unexpected output")).is_none());
        assert!(!mapped.get());
    }
}
