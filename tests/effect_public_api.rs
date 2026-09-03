//! Public API and deterministic transport coverage for `runtime::Effect`.

use radiant::{
    layout::Vector2,
    runtime::{
        BusinessEventSink, Command, Effect, RuntimeBridge, SurfaceNode, TaskPriority, UiSurface,
        testing::{DeterministicHost, DeterministicHostError},
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Timer(Rc<str>),
    Worker(u8, Rc<str>),
    Event(u8, Rc<str>),
    Final(u8, Rc<str>),
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
fn facade_is_qualified_and_keeps_messages_and_mappers_ui_local() {
    let label = Rc::<str>::from("local");
    let after: Effect<Message> =
        Effect::after(Duration::from_millis(1), Message::Timer(Rc::clone(&label)));
    assert!(matches!(Command::effect(after), Command::Timer(_)));

    let mapped = Rc::new(Cell::new(false));
    let worker: Effect<Message> =
        Effect::worker("public-worker", TaskPriority::Interactive, || 7_u8, {
            let mapped = Rc::clone(&mapped);
            let label = Rc::clone(&label);
            move |value| {
                mapped.set(value == 7);
                Message::Worker(value, label)
            }
        });
    let command: Command<Message> = worker.into();
    assert!(matches!(command, Command::PerformWorker(_)));
    assert!(!mapped.get());

    let ordered: Effect<Message> = Effect::ordered_stream(
        "public-ordered-stream",
        TaskPriority::Background,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            2_u8
        },
        {
            let label = Rc::clone(&label);
            move |event| Message::Event(event, Rc::clone(&label))
        },
        {
            let label = Rc::clone(&label);
            move |output| Message::Final(output, label)
        },
    );
    assert!(matches!(
        Command::effect(ordered),
        Command::PerformWorker(_)
    ));

    let latest: Effect<Message> = Effect::latest_stream(
        "public-latest-stream",
        TaskPriority::Idle,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            2_u8
        },
        {
            let label = Rc::clone(&label);
            move |event| Message::Event(event, Rc::clone(&label))
        },
        move |output| Message::Final(output, label),
    );
    let _: Command<Message> = latest.into();
}

#[test]
fn facade_race_matrix_keeps_virtual_time_worker_completion_and_separate_lanes() {
    let mut host = new_host();
    let mapper_seen = Rc::new(Cell::new(false));

    host.execute_command(Command::effect(Effect::after(
        Duration::from_millis(5),
        Message::Timer(Rc::from("timer")),
    )))
    .expect("timer admission");

    host.execute_command(Command::effect(Effect::worker(
        "explicit-worker",
        TaskPriority::Interactive,
        || 9_u8,
        {
            let mapper_seen = Rc::clone(&mapper_seen);
            move |value| {
                mapper_seen.set(true);
                Message::Worker(value, Rc::from("worker"))
            }
        },
    )))
    .expect("worker admission");

    assert_eq!(host.pending_timer_count(), 1);
    assert_eq!(host.pending_worker_tasks().len(), 1);
    assert!(host.bridge().messages.is_empty());

    host.advance_time(Duration::from_millis(4))
        .expect("pre-deadline advance");
    assert_eq!(host.pending_timer_count(), 1);
    assert!(host.bridge().messages.is_empty());

    host.advance_time(Duration::from_millis(1))
        .expect("exact-deadline advance");
    assert_eq!(host.pending_timer_count(), 0);
    assert!(host.bridge().messages.is_empty());

    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("explicit worker completion");
    assert!(host.bridge().messages.is_empty());
    assert!(!mapper_seen.get());

    host.turn().expect("later runtime turn");
    assert_eq!(host.bridge().messages.len(), 2);
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Timer(Rc::from("timer")))
    );
    assert!(
        host.bridge()
            .messages
            .contains(&Message::Worker(9, Rc::from("worker")))
    );
    assert!(mapper_seen.get());

    assert_eq!(
        host.complete_worker(worker_id),
        Err(DeterministicHostError::DuplicateWorkerCompletion(worker_id))
    );
}

#[test]
fn facade_stream_matrix_preserves_fifo_or_coalesces_only_intermediate_events() {
    let mut host = new_host();
    host.execute_command(Command::effect(Effect::ordered_stream(
        "ordered-stream",
        TaskPriority::Background,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            assert!(events.emit(2));
            3_u8
        },
        |event| Message::Event(event, Rc::from("ordered")),
        |output| Message::Final(output, Rc::from("ordered")),
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
    host.execute_command(Command::effect(Effect::latest_stream(
        "latest-stream",
        TaskPriority::Background,
        |events: BusinessEventSink<u8>| {
            assert!(events.emit(1));
            assert!(events.emit(2));
            3_u8
        },
        |event| Message::Event(event, Rc::from("latest")),
        |output| Message::Final(output, Rc::from("latest")),
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
