//! Deterministic production-path coverage for shared resource interests and work.

use radiant::{
    application::{
        ResourceInterest, ResourceInterestError, ResourceInterestKind, SharedResourceCompletion,
        SharedResourceTaskMode, SharedResourceTasks,
    },
    gui::types::Vector2,
    runtime::{
        testing::{DeterministicHost, DeterministicHostConfig},
        Command, Effect, EffectOwner, RuntimeBridge, SurfaceNode, TaskPriority, UiSurface,
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc};

enum Message {
    Interest(Result<ResourceInterest, ResourceInterestError>),
    Ready(SharedResourceCompletion<u8>),
    Retry(SharedResourceCompletion<u8>),
}

struct ResourceBridge {
    tasks: SharedResourceTasks,
    interests: Vec<ResourceInterest>,
    ready: Vec<u8>,
    retries: usize,
}

impl ResourceBridge {
    fn new(tasks: SharedResourceTasks) -> Self {
        Self {
            tasks,
            interests: Vec::new(),
            ready: Vec::new(),
            retries: 0,
        }
    }
}

impl RuntimeBridge<Message> for ResourceBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Interest(Ok(interest)) => self.interests.push(interest),
            Message::Interest(Err(error)) => panic!("unexpected interest rejection: {error:?}"),
            Message::Ready(completion) => {
                if let Some(value) = self.tasks.finish_ready(completion) {
                    self.ready.push(value);
                }
            }
            Message::Retry(completion) => {
                assert!(self.tasks.schedule_retry(&completion, 10));
                self.retries += 1;
            }
        }
        Command::none()
    }
}

fn host(tasks: SharedResourceTasks) -> DeterministicHost<ResourceBridge, Message> {
    DeterministicHost::with_default_config(ResourceBridge::new(tasks), Vector2::new(160.0, 80.0))
        .expect("deterministic host")
}

fn acquire(
    host: &mut DeterministicHost<ResourceBridge, Message>,
    tasks: &SharedResourceTasks,
    key: &'static str,
    interest_id: u64,
) {
    host.execute_command(tasks.interest(
        key,
        EffectOwner::Application,
        interest_id,
        ResourceInterestKind::Visible,
        Message::Interest,
    ))
    .expect("interest admission");
}

fn worker(
    tasks: &SharedResourceTasks,
    key: &'static str,
    mode: SharedResourceTaskMode,
    value: u8,
) -> Option<Effect<Message>> {
    Effect::resource_worker(
        tasks,
        key,
        mode,
        "shared-resource-test",
        TaskPriority::Background,
        move || value,
        Message::Ready,
    )
    .expect("resource operation admission")
}

#[test]
fn two_interests_share_one_worker_and_completion_reduces_on_later_turn() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "shared", 1);
    acquire(&mut host, &tasks, "shared", 2);

    let mapper_ran = Rc::new(Cell::new(false));
    let mapper_probe = Rc::clone(&mapper_ran);
    let effect = Effect::resource_worker(
        &tasks,
        "shared",
        SharedResourceTaskMode::Join,
        "shared-resource-worker",
        TaskPriority::Background,
        || 7_u8,
        move |completion| {
            mapper_probe.set(true);
            Message::Ready(completion)
        },
    )
    .expect("first reservation")
    .expect("first worker starts");
    assert!(worker(&tasks, "shared", SharedResourceTaskMode::Join, 9).is_none());
    host.execute_command(Command::effect(effect))
        .expect("worker admission");
    assert_eq!(host.pending_worker_tasks().len(), 1);

    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("worker completion");
    assert!(!mapper_ran.get());
    assert!(host.bridge().ready.is_empty());
    host.turn().expect("deferred mapper turn");
    assert!(mapper_ran.get());
    assert_eq!(host.bridge().ready, vec![7]);
}

#[test]
fn final_release_before_terminal_skips_mapper_but_other_interest_keeps_work_live() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "lifetime", 1);
    acquire(&mut host, &tasks, "lifetime", 2);
    let first = host.bridge().interests[0].clone();
    let second = host.bridge().interests[1].clone();
    let calls = Rc::new(Cell::new(0));
    let calls_for_map = Rc::clone(&calls);
    let effect = Effect::resource_worker(
        &tasks,
        "lifetime",
        SharedResourceTaskMode::Join,
        "lifetime-worker",
        TaskPriority::Background,
        || 3_u8,
        move |completion| {
            calls_for_map.set(calls_for_map.get() + 1);
            Message::Ready(completion)
        },
    )
    .expect("reservation")
    .expect("worker");
    host.execute_command(Command::effect(effect))
        .expect("admission");

    assert!(first.release());
    assert!(second.is_live());
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("completion");
    host.turn().expect("completion turn");
    assert_eq!(calls.get(), 1);
    assert_eq!(host.bridge().ready, vec![3]);

    assert!(second.release());
    let no_mapper_calls = Rc::new(Cell::new(0));
    // A fresh demand starts work, then its final release fences the terminal.
    acquire(&mut host, &tasks, "lifetime", 3);
    let probe = Rc::clone(&no_mapper_calls);
    let effect = Effect::resource_worker(
        &tasks,
        "lifetime",
        SharedResourceTaskMode::Refresh,
        "fenced-worker",
        TaskPriority::Background,
        || 4_u8,
        move |completion| {
            probe.set(probe.get() + 1);
            Message::Ready(completion)
        },
    )
    .expect("refresh reservation")
    .expect("refresh worker");
    host.execute_command(Command::effect(effect))
        .expect("admission");
    let final_interest = host
        .bridge()
        .interests
        .last()
        .expect("new interest")
        .clone();
    assert!(final_interest.release());
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("terminal");
    host.turn().expect("fenced terminal turn");
    assert_eq!(no_mapper_calls.get(), 0);
}

#[test]
fn release_inside_mapper_fences_post_map_reducer_application() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "post-map", 1);
    let release_in_mapper = host.bridge().interests[0].clone();
    let effect = Effect::resource_worker(
        &tasks,
        "post-map",
        SharedResourceTaskMode::Join,
        "post-map-worker",
        TaskPriority::Background,
        || 5_u8,
        move |completion| {
            assert!(release_in_mapper.release());
            Message::Ready(completion)
        },
    )
    .expect("reservation")
    .expect("worker");
    host.execute_command(Command::effect(effect))
        .expect("admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("completion");
    host.turn().expect("post-map fence turn");
    assert!(host.bridge().ready.is_empty());
}

#[test]
fn dropped_effect_rolls_back_and_ready_retention_reuses_without_new_worker() {
    let tasks = SharedResourceTasks::new();
    tasks
        .retain_ready("ready", true)
        .expect("retention metadata");
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "ready", 1);

    let dropped = worker(&tasks, "ready", SharedResourceTaskMode::Join, 1).expect("reservation");
    drop(dropped);
    let effect =
        worker(&tasks, "ready", SharedResourceTaskMode::Join, 2).expect("rollback restored idle");
    host.execute_command(Command::effect(effect))
        .expect("admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("completion");
    host.turn().expect("ready reducer turn");
    assert_eq!(host.bridge().ready, vec![2]);

    let interest = host.bridge().interests[0].clone();
    assert!(interest.release());
    acquire(&mut host, &tasks, "ready", 1);
    assert!(worker(&tasks, "ready", SharedResourceTaskMode::Join, 9).is_none());
    assert!(host.pending_worker_tasks().is_empty());
}

#[test]
fn scheduled_retry_respects_deadline_and_is_taken_once_without_join_bypass() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "retry", 1);
    let effect = Effect::resource_worker(
        &tasks,
        "retry",
        SharedResourceTaskMode::Join,
        "retry-source",
        TaskPriority::Background,
        || 1_u8,
        Message::Retry,
    )
    .expect("reservation")
    .expect("worker");
    host.execute_command(Command::effect(effect))
        .expect("admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("failure completion");
    host.turn().expect("retry scheduling reducer");
    assert_eq!(host.bridge().retries, 1);

    let key = "retry".into();
    assert!(Effect::resource_retry(
        &tasks,
        &key,
        9,
        "retry",
        TaskPriority::Background,
        || 2_u8,
        Message::Ready,
    )
    .is_none());
    assert!(worker(&tasks, "retry", SharedResourceTaskMode::Join, 3).is_none());
    let retry = Effect::resource_retry(
        &tasks,
        &key,
        10,
        "retry",
        TaskPriority::Background,
        || 4_u8,
        Message::Ready,
    )
    .expect("due retry starts once");
    assert!(Effect::resource_retry(
        &tasks,
        &key,
        10,
        "retry",
        TaskPriority::Background,
        || 5_u8,
        Message::Ready,
    )
    .is_none());
    host.execute_command(Command::effect(retry))
        .expect("retry admission");
    assert_eq!(host.pending_worker_tasks().len(), 1);
}

#[test]
fn rejected_host_admission_restores_the_existing_shared_worker() {
    let tasks = SharedResourceTasks::new();
    let config =
        DeterministicHostConfig::new(Vector2::new(160.0, 80.0)).with_max_pending_workers(1);
    let mut host =
        DeterministicHost::new(ResourceBridge::new(tasks.clone()), config).expect("host");
    acquire(&mut host, &tasks, "capacity", 1);
    let first = worker(&tasks, "capacity", SharedResourceTaskMode::Join, 1).expect("first worker");
    host.execute_command(Command::effect(first))
        .expect("first admission");
    let rejected =
        worker(&tasks, "capacity", SharedResourceTaskMode::Refresh, 2).expect("replacement worker");
    assert!(host.execute_command(Command::effect(rejected)).is_err());
    assert_eq!(host.pending_worker_tasks().len(), 1);
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id)
        .expect("predecessor completion");
    host.turn().expect("predecessor reducer");
    assert_eq!(host.bridge().ready, vec![1]);
}

#[test]
fn effect_token_cancellation_settles_then_allows_a_fresh_same_key_join() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "token-cancel", 1);
    let mapper_calls = Rc::new(Cell::new(0));
    let mapper_probe = Rc::clone(&mapper_calls);
    let effect = Effect::resource_worker(
        &tasks,
        "token-cancel",
        SharedResourceTaskMode::Join,
        "token-cancel-worker",
        TaskPriority::Background,
        || 1_u8,
        move |completion| {
            mapper_probe.set(mapper_probe.get() + 1);
            Message::Ready(completion)
        },
    )
    .expect("reservation")
    .expect("worker");
    let token = effect.token();
    host.execute_command(Command::effect(effect))
        .expect("admission");
    token.cancel();
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("cancelled terminal");
    host.turn().expect("cancelled terminal turn");
    assert_eq!(mapper_calls.get(), 0);
    assert!(host.bridge().ready.is_empty());

    let fresh = worker(&tasks, "token-cancel", SharedResourceTaskMode::Join, 2)
        .expect("cancelled operation settled to idle");
    host.execute_command(Command::effect(fresh))
        .expect("fresh admission");
    assert_eq!(host.pending_worker_tasks().len(), 1);
}

#[test]
fn shared_task_cancellation_fences_stale_completion_and_permits_fresh_work() {
    let tasks = SharedResourceTasks::new();
    let mut host = host(tasks.clone());
    acquire(&mut host, &tasks, "broker-cancel", 1);
    let key: radiant::runtime::ResourceKey = "broker-cancel".into();
    let effect = worker(&tasks, "broker-cancel", SharedResourceTaskMode::Join, 1).expect("worker");
    host.execute_command(Command::effect(effect))
        .expect("admission");
    assert!(tasks.cancel(&key));
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("stale terminal");
    host.turn().expect("stale terminal turn");
    assert!(host.bridge().ready.is_empty());

    let fresh = worker(&tasks, "broker-cancel", SharedResourceTaskMode::Join, 2)
        .expect("cancelled broker operation settled to idle");
    host.execute_command(Command::effect(fresh))
        .expect("fresh admission");
    let worker_id = host.pending_worker_tasks()[0].id;
    host.complete_worker(worker_id).expect("fresh terminal");
    host.turn().expect("fresh terminal turn");
    assert_eq!(host.bridge().ready, vec![2]);
}
