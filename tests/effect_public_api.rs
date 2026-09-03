//! Public API and deterministic production-path coverage for `runtime::Effect`.

use radiant::{
    application::{
        CancellationToken, DeclarativeEffectOwner, IntoView, LatestTask, TaskCompletion,
        TaskTicket, column, text,
    },
    gui::types::Vector2,
    runtime::{
        BusinessEventSink, Command, Effect, EffectOwner, RepaintScope, RuntimeBridge, SurfaceNode,
        TaskPriority, UiSurface,
        testing::{DeterministicHost, DeterministicHostConfig, DeterministicHostError},
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Timer(u64),
    Worker(u8, Rc<str>),
    Event(u8, Rc<str>),
    Final(u8, Rc<str>),
    Replace,
}

#[derive(Default)]
struct RecordingBridge {
    messages: Vec<Message>,
}

impl RuntimeBridge<Message> for RecordingBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.messages.push(message);
        Command::none()
    }
}

fn new_host() -> DeterministicHost<RecordingBridge, Message> {
    DeterministicHost::with_default_config(RecordingBridge::default(), Vector2::new(160.0, 80.0))
        .expect("deterministic host construction")
}

#[test]
fn facade_is_qualified_and_keeps_completion_mappers_ui_local() {
    let mut latest = LatestTask::new();
    let timer = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::from_millis(1),
        |completion: TaskCompletion<()>| Message::Timer(completion.ticket.id()),
    );
    let timer_ticket = timer.ticket();
    let timer_token = timer.token();
    assert!(!timer_token.is_cancelled());
    assert!(timer_ticket.id() > 0);
    assert!(matches!(Command::effect(timer), Command::Timer(_)));

    let mut latest = LatestTask::new();
    let worker = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "public-worker",
        TaskPriority::Interactive,
        || 7_u8,
        |completion: TaskCompletion<u8>| {
            Rc::from(if completion.output == 7 {
                "worker"
            } else {
                "wrong"
            })
        },
    );
    let worker_ticket = worker.ticket();
    let command: Command<Rc<str>> = worker.into();
    assert!(matches!(command, Command::PerformWorker(_)));
    assert!(worker_ticket.id() > 0);
}

#[test]
fn facade_timer_and_worker_use_production_lanes_and_defer_mapping() {
    let mut host = new_host();
    let mut timer_latest = LatestTask::new();
    let timer = Effect::after(
        &mut timer_latest,
        EffectOwner::Application,
        Duration::from_millis(5),
        |completion| Message::Timer(completion.ticket.id()),
    );
    let mapper_seen = Rc::new(Cell::new(false));
    let mut worker_latest = LatestTask::new();
    let worker = Effect::worker(
        &mut worker_latest,
        EffectOwner::Application,
        "explicit-worker",
        TaskPriority::Interactive,
        || 9_u8,
        {
            let mapper_seen = Rc::clone(&mapper_seen);
            move |completion| {
                mapper_seen.set(true);
                Message::Worker(completion.output, Rc::from("worker"))
            }
        },
    );

    host.execute_command(Command::effect(timer))
        .expect("timer admission");
    host.execute_command(Command::effect(worker))
        .expect("worker admission");
    assert_eq!(host.pending_timer_count(), 1);
    assert_eq!(host.pending_worker_tasks().len(), 1);
    assert!(host.bridge().messages.is_empty());

    host.advance_time(Duration::from_millis(5))
        .expect("exact-deadline advance");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("worker completion");
    assert!(!mapper_seen.get());
    assert!(host.bridge().messages.is_empty());

    host.turn().expect("later runtime turn");
    assert!(mapper_seen.get());
    assert_eq!(host.bridge().messages.len(), 2);
    assert!(host.bridge().messages.iter().any(|message| matches!(
        message,
        Message::Timer(ticket) if *ticket > 0
    )));
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Worker(9, Rc::from("worker"),))
    );
    assert_eq!(
        host.complete_worker(worker_id),
        Err(DeterministicHostError::DuplicateWorkerCompletion(worker_id))
    );
}

#[test]
fn facade_latest_replacement_fences_all_timer_and_worker_lane_pairs() {
    let mut host = new_host();
    let mut latest = LatestTask::new();
    let first = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "worker-worker-first",
        TaskPriority::Background,
        || 1_u8,
        |completion| Message::Worker(completion.output, Rc::from("first")),
    );
    host.execute_command(Command::effect(first))
        .expect("first worker admission");
    let second = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "worker-worker-second",
        TaskPriority::Background,
        || 2_u8,
        |completion| Message::Worker(completion.output, Rc::from("second")),
    );
    host.execute_command(Command::effect(second))
        .expect("second worker admission");
    for worker in host.pending_worker_tasks() {
        host.complete_worker(worker.id)
            .expect("worker-worker completion");
    }
    host.turn().expect("worker-worker turn");
    assert_eq!(
        host.bridge().messages,
        vec![Message::Worker(2, Rc::from("second"))]
    );

    let mut host = new_host();
    let mut latest = LatestTask::new();
    let first = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        |_| Message::Timer(1),
    );
    host.execute_command(Command::effect(first))
        .expect("first timer admission");
    let second = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        |_| Message::Timer(2),
    );
    host.execute_command(Command::effect(second))
        .expect("second timer admission");
    host.advance_time(Duration::ZERO)
        .expect("timer-timer release");
    host.turn().expect("timer-timer turn");
    assert_eq!(host.bridge().messages, vec![Message::Timer(2)]);

    let mut host = new_host();
    let mut latest = LatestTask::new();
    let first = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        |_| Message::Timer(1),
    );
    host.execute_command(Command::effect(first))
        .expect("timer-worker timer admission");
    let second = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "timer-worker-second",
        TaskPriority::Background,
        || 2_u8,
        |completion| Message::Worker(completion.output, Rc::from("second")),
    );
    host.execute_command(Command::effect(second))
        .expect("timer-worker worker admission");
    host.advance_time(Duration::ZERO)
        .expect("timer-worker release");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("timer-worker completion");
    host.turn().expect("timer-worker turn");
    assert_eq!(
        host.bridge().messages,
        vec![Message::Worker(2, Rc::from("second"))]
    );

    let mut host = new_host();
    let mut latest = LatestTask::new();
    let first = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "worker-timer-first",
        TaskPriority::Background,
        || 1_u8,
        |completion| Message::Worker(completion.output, Rc::from("first")),
    );
    host.execute_command(Command::effect(first))
        .expect("worker-timer worker admission");
    let second = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        |_| Message::Timer(2),
    );
    host.execute_command(Command::effect(second))
        .expect("worker-timer timer admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("worker-timer completion");
    host.advance_time(Duration::ZERO)
        .expect("worker-timer release");
    host.turn().expect("worker-timer turn");
    assert_eq!(host.bridge().messages, vec![Message::Timer(2)]);
}

#[test]
fn facade_streams_preserve_typed_fifo_and_latest_final_delivery() {
    let mut host = new_host();
    let mut ordered_latest = LatestTask::new();
    host.execute_command(Command::effect(Effect::ordered_stream(
        &mut ordered_latest,
        EffectOwner::Application,
        "ordered-stream",
        TaskPriority::Background,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            assert!(events.emit(2));
            3_u8
        },
        |completion| Message::Event(completion.output, Rc::from("ordered")),
        |completion| Message::Final(completion.output, Rc::from("ordered")),
    )))
    .expect("ordered stream admission");
    let ordered_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(ordered_id)
        .expect("ordered stream completion");
    host.turn().expect("ordered stream turn");
    assert_eq!(
        host.bridge().messages,
        vec![
            Message::Event(1, Rc::from("ordered")),
            Message::Event(2, Rc::from("ordered")),
            Message::Final(3, Rc::from("ordered")),
        ]
    );

    let mut host = new_host();
    let mut latest_latest = LatestTask::new();
    host.execute_command(Command::effect(Effect::latest_stream(
        &mut latest_latest,
        EffectOwner::Application,
        "latest-stream",
        TaskPriority::Idle,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            assert!(events.emit(2));
            3_u8
        },
        |completion| Message::Event(completion.output, Rc::from("latest")),
        |completion| Message::Final(completion.output, Rc::from("latest")),
    )))
    .expect("latest stream admission");
    let latest_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(latest_id)
        .expect("latest stream completion");
    host.turn().expect("latest stream turn");
    assert_eq!(
        host.bridge().messages,
        vec![
            Message::Event(2, Rc::from("latest")),
            Message::Final(3, Rc::from("latest")),
        ]
    );
}

#[test]
fn facade_mappers_receive_the_reserved_ticket_for_timer_worker_event_and_final() {
    let mut host = new_host();

    let timer_ticket = Rc::new(Cell::new(None::<TaskTicket>));
    let timer_ticket_for_mapper = Rc::clone(&timer_ticket);
    let mut timer_latest = LatestTask::new();
    let timer = Effect::after(
        &mut timer_latest,
        EffectOwner::Application,
        Duration::ZERO,
        move |completion| {
            assert_eq!(Some(completion.ticket), timer_ticket_for_mapper.get());
            Message::Timer(completion.ticket.id())
        },
    );
    timer_ticket.set(Some(timer.ticket()));
    host.execute_command(Command::effect(timer))
        .expect("timer admission");

    let worker_ticket = Rc::new(Cell::new(None::<TaskTicket>));
    let worker_ticket_for_mapper = Rc::clone(&worker_ticket);
    let mut worker_latest = LatestTask::new();
    let worker = Effect::worker(
        &mut worker_latest,
        EffectOwner::Application,
        "ticketed-worker",
        TaskPriority::Background,
        || 4_u8,
        move |completion| {
            assert_eq!(Some(completion.ticket), worker_ticket_for_mapper.get());
            Message::Worker(completion.output, Rc::from("ticketed"))
        },
    );
    worker_ticket.set(Some(worker.ticket()));
    host.execute_command(Command::effect(worker))
        .expect("worker admission");

    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("ticketed worker completion");
    host.advance_time(Duration::ZERO)
        .expect("ticketed timer release");
    host.turn().expect("ticketed timer and worker turn");
    assert_eq!(host.bridge().messages.len(), 2);

    let event_ticket = Rc::new(Cell::new(None::<TaskTicket>));
    let event_ticket_for_mapper = Rc::clone(&event_ticket);
    let final_ticket = Rc::new(Cell::new(None::<TaskTicket>));
    let final_ticket_for_mapper = Rc::clone(&final_ticket);
    let mut stream_latest = LatestTask::new();
    let stream = Effect::ordered_stream(
        &mut stream_latest,
        EffectOwner::Application,
        "ticketed-stream",
        TaskPriority::Background,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(5));
            6_u8
        },
        move |completion| {
            assert_eq!(Some(completion.ticket), event_ticket_for_mapper.get());
            Message::Event(completion.output, Rc::from("ticketed"))
        },
        move |completion| {
            assert_eq!(Some(completion.ticket), final_ticket_for_mapper.get());
            Message::Final(completion.output, Rc::from("ticketed"))
        },
    );
    let stream_ticket = stream.ticket();
    event_ticket.set(Some(stream_ticket));
    final_ticket.set(Some(stream_ticket));
    host.execute_command(Command::effect(stream))
        .expect("ticketed stream admission");
    let stream_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(stream_id)
        .expect("ticketed stream completion");
    host.turn().expect("ticketed stream turn");
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Event(5, Rc::from("ticketed"),))
    );
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Final(6, Rc::from("ticketed"),))
    );
}

#[test]
fn facade_cancellation_is_idempotent_before_work_and_after_enqueue() {
    let mut host = new_host();
    let mut latest = LatestTask::new();
    let mapped = Rc::new(Cell::new(0));
    let effect = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "cancel-before-work",
        TaskPriority::Background,
        || 1_u8,
        {
            let mapped = Rc::clone(&mapped);
            move |completion| {
                mapped.set(mapped.get() + completion.output as usize);
                Message::Worker(completion.output, Rc::from("cancelled"))
            }
        },
    );
    let token = effect.token();
    host.execute_command(Command::effect(effect))
        .expect("worker admission");
    token.cancel();
    token.cancel();
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("cancelled worker completion");
    host.turn().expect("cancelled worker turn");
    assert_eq!(mapped.get(), 0);
    assert!(host.bridge().messages.is_empty());

    let mut latest = LatestTask::new();
    let mapped = Rc::new(Cell::new(0));
    let effect = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "cancel-after-enqueue",
        TaskPriority::Background,
        || 2_u8,
        {
            let mapped = Rc::clone(&mapped);
            move |completion| {
                mapped.set(mapped.get() + completion.output as usize);
                Message::Worker(completion.output, Rc::from("late-cancel"))
            }
        },
    );
    let token = effect.token();
    host.execute_command(Command::effect(effect))
        .expect("worker admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("worker completion");
    token.cancel();
    token.cancel();
    host.turn().expect("late cancellation turn");
    assert_eq!(mapped.get(), 0);

    let mut latest = LatestTask::new();
    let timer = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        |_| Message::Timer(7),
    );
    let token = timer.token();
    host.execute_command(Command::effect(timer))
        .expect("timer admission");
    token.cancel();
    token.cancel();
    host.advance_time(Duration::ZERO)
        .expect("cancelled timer release");
    host.turn().expect("cancelled timer turn");
    assert!(!host.bridge().messages.contains(&Message::Timer(7)));

    let mapper_token = Rc::new(RefCell::new(None::<CancellationToken>));
    let mapper_token_for_mapper = Rc::clone(&mapper_token);
    let mut latest = LatestTask::new();
    let timer = Effect::after(
        &mut latest,
        EffectOwner::Application,
        Duration::ZERO,
        move |_| {
            mapper_token_for_mapper
                .borrow()
                .as_ref()
                .expect("timer token installed")
                .cancel();
            Message::Timer(8)
        },
    );
    *mapper_token.borrow_mut() = Some(timer.token());
    host.execute_command(Command::effect(timer))
        .expect("mapper-cancelling timer admission");
    host.advance_time(Duration::ZERO)
        .expect("mapper-cancelling timer release");
    host.turn().expect("mapper-cancelling timer turn");
    assert!(!host.bridge().messages.contains(&Message::Timer(8)));
}

#[test]
fn facade_cancellation_from_a_preceding_mapper_fences_a_later_same_drain() {
    let mut host = new_host();
    let later_mapper_calls = Rc::new(Cell::new(0));
    let mut later_latest = LatestTask::new();
    let later = Effect::worker(
        &mut later_latest,
        EffectOwner::Application,
        "later-cancelled-worker",
        TaskPriority::Background,
        || 2_u8,
        {
            let later_mapper_calls = Rc::clone(&later_mapper_calls);
            move |completion| {
                later_mapper_calls.set(later_mapper_calls.get() + 1);
                Message::Worker(completion.output, Rc::from("later"))
            }
        },
    );
    let later_token = later.token();

    let mut first_latest = LatestTask::new();
    let first = Effect::worker(
        &mut first_latest,
        EffectOwner::Application,
        "cancelling-worker",
        TaskPriority::Background,
        || 1_u8,
        move |completion| {
            later_token.cancel();
            Message::Worker(completion.output, Rc::from("first"))
        },
    );
    host.execute_command(Command::effect(first))
        .expect("preceding worker admission");
    host.execute_command(Command::effect(later))
        .expect("later worker admission");
    let workers = host.pending_worker_tasks();
    host.complete_worker(workers[0].id)
        .expect("preceding worker completion");
    host.complete_worker(workers[1].id)
        .expect("later worker completion");
    host.turn().expect("same-drain cancellation turn");
    assert_eq!(
        host.bridge().messages,
        vec![Message::Worker(1, Rc::from("first"))]
    );
    assert_eq!(later_mapper_calls.get(), 0);
}

struct ReplacementBridge {
    latest: LatestTask,
    messages: Vec<Message>,
    replacement_mapper_calls: Rc<Cell<usize>>,
    replace_once: bool,
}

impl ReplacementBridge {
    fn new(replacement_mapper_calls: Rc<Cell<usize>>) -> Self {
        Self {
            latest: LatestTask::new(),
            messages: Vec::new(),
            replacement_mapper_calls,
            replace_once: true,
        }
    }

    fn effect(
        &mut self,
        output: u8,
        map: impl FnOnce(TaskCompletion<u8>) -> Message + 'static,
    ) -> Effect<Message> {
        Effect::worker(
            &mut self.latest,
            EffectOwner::Application,
            "same-key-worker",
            TaskPriority::Background,
            move || output,
            map,
        )
    }
}

impl RuntimeBridge<Message> for ReplacementBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        let replace = matches!(message, Message::Replace) && self.replace_once;
        self.messages.push(message);
        if !replace {
            return Command::none();
        }
        self.replace_once = false;
        let calls = Rc::clone(&self.replacement_mapper_calls);
        Command::effect(self.effect(3, move |completion| {
            calls.set(calls.get() + 1);
            Message::Worker(completion.output, Rc::from("replacement"))
        }))
    }
}

#[test]
fn facade_same_key_reducer_fences_later_mapping_in_the_same_drain() {
    let calls = Rc::new(Cell::new(0));
    let bridge = ReplacementBridge::new(Rc::clone(&calls));
    let mut host = DeterministicHost::with_default_config(bridge, Vector2::new(160.0, 80.0))
        .expect("deterministic host construction");
    let mut trigger_latest = LatestTask::new();
    let first = Effect::worker(
        &mut trigger_latest,
        EffectOwner::Application,
        "replacement-trigger",
        TaskPriority::Background,
        || 1_u8,
        |_| Message::Replace,
    );
    host.execute_command(Command::effect(first))
        .expect("first worker admission");
    let second = host.bridge_mut().effect(2, |completion| {
        Message::Worker(completion.output, Rc::from("second"))
    });
    host.execute_command(Command::effect(second))
        .expect("replacement worker admission");
    let workers = host.pending_worker_tasks();
    assert_eq!(workers.len(), 2);
    host.complete_worker(workers[0].id)
        .expect("first worker completion");
    host.complete_worker(workers[1].id)
        .expect("second worker completion");
    host.turn().expect("same-drain replacement turn");
    assert_eq!(host.bridge().messages, vec![Message::Replace]);
    assert_eq!(calls.get(), 0);
    assert_eq!(host.pending_worker_tasks().len(), 1);
}

#[test]
fn facade_invalid_declarative_owner_rejects_atomically_and_restores_predecessor() {
    let mut host = new_host();
    let mut latest = LatestTask::new();
    let first = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "predecessor",
        TaskPriority::Background,
        || 1_u8,
        |completion| Message::Worker(completion.output, Rc::from("predecessor")),
    );
    let first_ticket = first.ticket();
    host.execute_command(Command::effect(first))
        .expect("predecessor admission");

    let invalid = Effect::worker(
        &mut latest,
        EffectOwner::Declarative(DeclarativeEffectOwner::new()),
        "invalid-owner",
        TaskPriority::Background,
        || 2_u8,
        |completion| Message::Worker(completion.output, Rc::from("invalid")),
    );
    host.execute_command(Command::effect(invalid))
        .expect("invalid owner rejection");
    assert_eq!(latest.active(), Some(first_ticket));
    assert_eq!(host.pending_worker_tasks().len(), 1);

    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("predecessor completion");
    host.turn().expect("predecessor turn");
    assert_eq!(
        host.bridge().messages,
        vec![Message::Worker(1, Rc::from("predecessor"))]
    );
}

#[test]
fn facade_host_admission_rejection_restores_predecessor() {
    let config =
        DeterministicHostConfig::new(Vector2::new(160.0, 80.0)).with_max_pending_workers(1);
    let mut host = DeterministicHost::new(RecordingBridge::default(), config)
        .expect("deterministic host construction");
    let mut latest = LatestTask::new();
    let first = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "capacity-predecessor",
        TaskPriority::Background,
        || 1_u8,
        |completion| Message::Worker(completion.output, Rc::from("predecessor")),
    );
    let first_ticket = first.ticket();
    host.execute_command(Command::effect(first))
        .expect("predecessor admission");

    let rejected = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "capacity-rejected",
        TaskPriority::Background,
        || 2_u8,
        |completion| Message::Worker(completion.output, Rc::from("rejected")),
    );
    let error = host
        .execute_command(Command::effect(rejected))
        .expect_err("worker capacity must reject the replacement");
    assert_eq!(
        error,
        DeterministicHostError::Capacity {
            lane: radiant::runtime::testing::DeterministicLane::Workers,
            limit: 1,
        }
    );
    assert_eq!(latest.active(), Some(first_ticket));
    assert_eq!(host.pending_worker_tasks().len(), 1);

    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("predecessor completion");
    host.turn().expect("predecessor turn");
    assert_eq!(
        host.bridge().messages,
        vec![Message::Worker(1, Rc::from("predecessor"))]
    );
}

struct OwnerBridge {
    owner: DeclarativeEffectOwner,
    sibling: DeclarativeEffectOwner,
    show_owner: bool,
    retire_on_replace: bool,
    messages: Vec<Message>,
}

impl OwnerBridge {
    fn surface(&self) -> UiSurface<Message> {
        if self.show_owner {
            column([
                text::<Message>("owner")
                    .key("owner")
                    .effect_owner(self.owner),
                text::<Message>("sibling")
                    .key("sibling")
                    .effect_owner(self.sibling),
            ])
            .into_surface()
        } else {
            UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new()))
        }
    }
}

impl RuntimeBridge<Message> for OwnerBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(self.surface())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        let retire = self.retire_on_replace && matches!(message, Message::Replace);
        self.messages.push(message);
        if retire {
            self.show_owner = false;
            return Command::repaint(RepaintScope::Projection);
        }
        Command::none()
    }
}

#[test]
fn facade_declarative_siblings_are_selected_exactly_and_retirement_fences_work() {
    let owner = DeclarativeEffectOwner::new();
    let sibling = DeclarativeEffectOwner::new();
    let mut host = DeterministicHost::with_default_config(
        OwnerBridge {
            owner,
            sibling,
            show_owner: true,
            retire_on_replace: false,
            messages: Vec::new(),
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("owner host construction");
    let mut owner_latest = LatestTask::new();
    let owner_effect = Effect::worker(
        &mut owner_latest,
        EffectOwner::Declarative(owner),
        "owner-worker",
        TaskPriority::Background,
        || 1_u8,
        |completion| Message::Worker(completion.output, Rc::from("owner")),
    );
    let mut sibling_latest = LatestTask::new();
    let sibling_effect = Effect::worker(
        &mut sibling_latest,
        EffectOwner::Declarative(sibling),
        "sibling-worker",
        TaskPriority::Background,
        || 2_u8,
        |completion| Message::Worker(completion.output, Rc::from("sibling")),
    );
    host.execute_command(Command::effect(owner_effect))
        .expect("owner admission");
    host.execute_command(Command::effect(sibling_effect))
        .expect("sibling admission");
    let workers = host.pending_worker_tasks();
    assert_eq!(workers.len(), 2);
    for worker in workers {
        host.complete_worker(worker.id)
            .expect("sibling worker completion");
    }
    host.turn().expect("owner worker turn");
    assert_eq!(host.bridge().messages.len(), 2);
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Worker(1, Rc::from("owner"),))
    );
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Worker(2, Rc::from("sibling"),))
    );

    let mut owner_latest = LatestTask::new();
    let effect = Effect::worker(
        &mut owner_latest,
        EffectOwner::Declarative(owner),
        "retired-owner",
        TaskPriority::Background,
        || 3_u8,
        |completion| Message::Worker(completion.output, Rc::from("retired")),
    );
    host.execute_command(Command::effect(effect))
        .expect("retirement candidate admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.bridge_mut().show_owner = false;
    host.execute_command(Command::repaint(RepaintScope::Projection))
        .expect("owner retirement refresh");
    host.complete_worker(worker_id)
        .expect("retired worker completion");
    host.turn().expect("retired worker turn");
    assert!(
        !host
            .bridge()
            .messages
            .contains(&Message::Worker(3, Rc::from("retired")))
    );

    host.bridge_mut().show_owner = true;
    host.execute_command(Command::repaint(RepaintScope::Projection))
        .expect("owner reinstallation refresh");
    let mut owner_latest = LatestTask::new();
    let effect = Effect::worker(
        &mut owner_latest,
        EffectOwner::Declarative(owner),
        "retired-after-enqueue",
        TaskPriority::Background,
        || 4_u8,
        |completion| Message::Worker(completion.output, Rc::from("retired-after")),
    );
    host.execute_command(Command::effect(effect))
        .expect("post-enqueue retirement candidate admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("post-enqueue retirement completion");
    host.bridge_mut().show_owner = false;
    host.execute_command(Command::repaint(RepaintScope::Projection))
        .expect("post-enqueue owner retirement refresh");
    host.turn().expect("post-enqueue retired worker turn");
    assert!(
        !host
            .bridge()
            .messages
            .contains(&Message::Worker(4, Rc::from("retired-after"),))
    );
}

#[test]
fn facade_owner_retirement_during_a_drain_fences_later_owner_mapping() {
    let owner = DeclarativeEffectOwner::new();
    let sibling = DeclarativeEffectOwner::new();
    let later_mapper_calls = Rc::new(Cell::new(0));
    let mut host = DeterministicHost::with_default_config(
        OwnerBridge {
            owner,
            sibling,
            show_owner: true,
            retire_on_replace: true,
            messages: Vec::new(),
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("owner host construction");
    let mut trigger_latest = LatestTask::new();
    let trigger = Effect::worker(
        &mut trigger_latest,
        EffectOwner::Declarative(owner),
        "retirement-trigger",
        TaskPriority::Background,
        || 1_u8,
        |_| Message::Replace,
    );
    host.execute_command(Command::effect(trigger))
        .expect("retirement trigger admission");
    let mut later_latest = LatestTask::new();
    let later = Effect::worker(
        &mut later_latest,
        EffectOwner::Declarative(owner),
        "retirement-later",
        TaskPriority::Background,
        || 2_u8,
        {
            let later_mapper_calls = Rc::clone(&later_mapper_calls);
            move |completion| {
                later_mapper_calls.set(later_mapper_calls.get() + 1);
                Message::Worker(completion.output, Rc::from("retired-during"))
            }
        },
    );
    host.execute_command(Command::effect(later))
        .expect("later owner admission");
    let workers = host.pending_worker_tasks();
    host.complete_worker(workers[0].id)
        .expect("retirement trigger completion");
    host.complete_worker(workers[1].id)
        .expect("later owner completion");
    host.turn().expect("same-drain owner retirement");
    assert_eq!(host.bridge().messages, vec![Message::Replace]);
    assert_eq!(later_mapper_calls.get(), 0);
}

#[test]
fn facade_shutdown_fences_enqueued_work() {
    let mut host = new_host();
    let mut latest = LatestTask::new();
    let effect = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "shutdown-worker",
        TaskPriority::Background,
        || 4_u8,
        |completion| Message::Worker(completion.output, Rc::from("shutdown")),
    );
    host.execute_command(Command::effect(effect))
        .expect("shutdown candidate admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.execute_command(Command::Exit)
        .expect("runtime shutdown");
    assert_eq!(
        host.complete_worker(worker_id),
        Err(DeterministicHostError::RuntimeNotAcceptingWork)
    );
    assert!(host.bridge().messages.is_empty());

    let mut host = new_host();
    let mut latest = LatestTask::new();
    let effect = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "shutdown-after-enqueue",
        TaskPriority::Background,
        || 5_u8,
        |completion| Message::Worker(completion.output, Rc::from("shutdown-after")),
    );
    host.execute_command(Command::effect(effect))
        .expect("post-enqueue shutdown candidate admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("post-enqueue shutdown completion");
    host.execute_command(Command::Exit)
        .expect("post-enqueue runtime shutdown");
    assert!(host.bridge().messages.is_empty());

    let mut host = new_host();
    let mut latest = LatestTask::new();
    let effect = Effect::worker(
        &mut latest,
        EffectOwner::Application,
        "shutdown-before-admission",
        TaskPriority::Background,
        || 6_u8,
        |completion| Message::Worker(completion.output, Rc::from("shutdown-before")),
    );
    host.enqueue_command(Command::effect(effect))
        .expect("queue effect before shutdown");
    host.execute_command(Command::Exit)
        .expect("queued-effect runtime shutdown");
    assert!(host.pending_worker_tasks().is_empty());
    assert!(host.bridge().messages.is_empty());
}
