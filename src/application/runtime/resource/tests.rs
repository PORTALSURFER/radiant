use super::*;
use crate::application::runtime::task::resource_operations::{
    ResourceOperationReplaceMode, ResourceOperationReserve,
};
use crate::{
    application::SharedResourceOperation,
    gui::types::Vector2,
    runtime::{
        Command, Effect, RuntimeBridge, SurfaceNode, TaskPriority, UiSurface,
        testing::DeterministicHost,
    },
};
use std::sync::Arc;

#[test]
fn progress_validation_rejects_invalid_finite_ranges() {
    assert_eq!(
        ResourceProgress::determinate(0, 0),
        Err(ResourceProgressError::ZeroTotal)
    );
    assert_eq!(
        ResourceProgress::determinate(3, 2),
        Err(ResourceProgressError::CompletedExceedsTotal)
    );
    assert_eq!(
        ResourceProgress::determinate(2, 2),
        Ok(ResourceProgress::determinate(2, 2).expect("valid progress"))
    );
    assert_eq!(
        ResourceProgress::determinate(2, 2)
            .expect("valid progress")
            .determinate_parts(),
        Some((2, 2))
    );
    assert!(ResourceProgress::indeterminate().is_indeterminate());
}

#[test]
fn idle_snapshots_are_owned_and_do_not_require_payload_clone() {
    struct NonClone;
    let resource = Resource::<NonClone, NonClone>::new("resource");
    let snapshot = resource.snapshot();
    let copy = snapshot.clone();

    assert_eq!(copy.key().as_str(), "resource");
    assert_eq!(copy.phase(), ResourcePhase::Idle);
    assert!(copy.value().is_none());
    assert!(copy.error().is_none());
}

#[test]
fn rekey_clears_local_presentation_state_without_starting_work() {
    let mut resource = Resource::<u8, u8>::new("old");

    assert!(resource.rekey("new").expect("rekey"));
    assert_eq!(resource.key().as_str(), "new");
    assert_eq!(resource.phase(), ResourcePhase::Idle);
    assert_eq!(resource.revision(), 1);
    assert_eq!(resource.generation(), 1);
}

#[test]
fn rekey_detaches_local_state_without_cancelling_shared_work() {
    let tasks = SharedResourceTasks::new();
    let (interest, _effect, operation) = active_operation(&tasks, "old", 44);
    let mut resource = Resource::<u8, u8>::new("old");
    assert!(
        resource
            .begin(operation.clone(), ResourceRefreshPolicy::RetainReady)
            .expect("begin")
    );

    assert!(resource.rekey("new").expect("rekey"));
    assert!(operation.is_current());
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Idle);
    assert!(!resource.apply_progress(
        &operation,
        1,
        ResourceProgress::determinate(1, 1).expect("valid progress"),
    ));
    assert!(interest.is_live());
}

#[test]
fn retry_intents_are_fenced_and_do_not_start_work() {
    let mut resource = Resource::<u8, u8>::new("retry");
    let intent = resource
        .snapshot()
        .retry_intent()
        .expect("idle retry intent");

    assert!(resource.take_retry(&intent));
    assert_eq!(resource.phase(), ResourcePhase::Idle);
    assert_eq!(resource.revision(), 1);
    assert!(!resource.take_retry(&intent));
    assert!(resource.snapshot().cancel_intent().is_none());
}

#[test]
fn retry_intents_cannot_cross_resource_instances_or_recreation() {
    let first = Resource::<u8, u8>::new("same-key");
    let same_live_intent = first.snapshot().retry_intent().expect("retry intent");
    let mut second = Resource::<u8, u8>::new("same-key");
    assert!(!second.take_retry(&same_live_intent));

    let stale_intent = first.snapshot().retry_intent().expect("retry intent");
    drop(first);
    let mut recreated = Resource::<u8, u8>::new("same-key");
    assert!(!recreated.take_retry(&stale_intent));
}

#[test]
fn cancellation_intents_atomically_fence_duplicates() {
    let tasks = SharedResourceTasks::new();
    let (_interest, _effect, operation) = active_operation(&tasks, "cancel-intent", 12);
    let mut resource = Resource::<u8, u8>::new("cancel-intent");
    assert!(
        resource
            .begin(operation, ResourceRefreshPolicy::RetainReady)
            .expect("begin")
    );

    let intent = resource
        .snapshot()
        .cancel_intent()
        .expect("pending cancellation intent");
    assert!(resource.cancel_intent(&tasks, &intent));
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Cancelled);
    assert!(!resource.cancel_intent(&tasks, &intent));
}

fn reserve(
    tasks: &SharedResourceTasks,
    key: &'static str,
    mode: ResourceOperationReplaceMode,
) -> crate::application::runtime::task::resource_operations::ResourceOperationReservation {
    match tasks
        .operations
        .reserve(key.into(), mode)
        .expect("reservation")
    {
        ResourceOperationReserve::Reserved(reservation) => reservation,
        _ => panic!("expected a worker reservation"),
    }
}

#[test]
fn rejected_replacement_restores_and_accepts_the_predecessor_completion() {
    let tasks = SharedResourceTasks::new();
    let interest = active_interest(&tasks, "rollback", 91);
    let first = reserve(&tasks, "rollback", ResourceOperationReplaceMode::Join);
    first.transaction().accept();
    let first_operation = tasks
        .operation(&"rollback".into())
        .expect("first operation");
    let mut resource = Resource::<u8, u8>::new("rollback");
    assert!(
        resource
            .begin(first_operation, ResourceRefreshPolicy::RetainReady)
            .expect("begin first")
    );

    let replacement = reserve(&tasks, "rollback", ResourceOperationReplaceMode::Replace);
    let replacement_operation = tasks.operation(&"rollback".into()).expect("replacement");
    assert!(
        resource
            .begin(replacement_operation, ResourceRefreshPolicy::RetainReady)
            .expect("begin replacement")
    );
    replacement.transaction().reject();

    assert!(resource.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(9),
            current: first.current(),
        },
    ));
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Ready);
    assert_eq!(**resource.snapshot().value().expect("restored value"), 9);
    assert!(interest.is_live());
}

#[test]
fn accepted_replacement_rejects_the_old_completion() {
    let tasks = SharedResourceTasks::new();
    let _interest = active_interest(&tasks, "accepted-replacement", 92);
    let first = reserve(
        &tasks,
        "accepted-replacement",
        ResourceOperationReplaceMode::Join,
    );
    first.transaction().accept();
    let mut resource = Resource::<u8, u8>::new("accepted-replacement");
    assert!(
        resource
            .begin(
                tasks
                    .operation(&"accepted-replacement".into())
                    .expect("first"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin first")
    );
    let replacement = reserve(
        &tasks,
        "accepted-replacement",
        ResourceOperationReplaceMode::Replace,
    );
    assert!(
        resource
            .begin(
                tasks
                    .operation(&"accepted-replacement".into())
                    .expect("replacement"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin replacement")
    );
    replacement.transaction().accept();

    assert!(!resource.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(3),
            current: first.current(),
        },
    ));
}

#[test]
fn rejected_replacement_keeps_predecessor_through_a_newer_pending_reservation() {
    let tasks = SharedResourceTasks::new();
    let _interest = active_interest(&tasks, "rollback-race", 93);
    let first = reserve(&tasks, "rollback-race", ResourceOperationReplaceMode::Join);
    first.transaction().accept();
    let mut resource = Resource::<u8, u8>::new("rollback-race");
    assert!(
        resource
            .begin(
                tasks.operation(&"rollback-race".into()).expect("first"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin first")
    );
    let second = reserve(
        &tasks,
        "rollback-race",
        ResourceOperationReplaceMode::Replace,
    );
    assert!(
        resource
            .begin(
                tasks.operation(&"rollback-race".into()).expect("second"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin second")
    );
    second.transaction().reject();

    let third = reserve(
        &tasks,
        "rollback-race",
        ResourceOperationReplaceMode::Replace,
    );
    assert!(
        resource
            .begin(
                tasks.operation(&"rollback-race".into()).expect("third"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin third")
    );
    third.transaction().reject();

    assert!(resource.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(11),
            current: first.current(),
        },
    ));
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Ready);
}

#[test]
fn failed_completion_schedules_one_due_retry_without_losing_local_error_state() {
    let tasks = SharedResourceTasks::new();
    let _interest = active_interest(&tasks, "retry-backoff", 94);
    let first = reserve(&tasks, "retry-backoff", ResourceOperationReplaceMode::Join);
    first.transaction().accept();
    let mut resource = Resource::<u8, &'static str>::new("retry-backoff");
    assert!(
        resource
            .begin(
                tasks.operation(&"retry-backoff".into()).expect("first"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin first")
    );
    assert!(resource.apply_completion_with_retry(
        &tasks,
        SharedResourceCompletion {
            output: Err("offline"),
            current: first.current(),
        },
        10,
    ));
    let snapshot = resource.snapshot();
    assert_eq!(snapshot.phase(), ResourcePhase::Failed);
    assert_eq!(**snapshot.error().expect("typed error"), "offline");
    assert!(
        crate::runtime::Effect::resource_worker(
            &tasks,
            "retry-backoff",
            crate::application::SharedResourceTaskMode::Join,
            "backoff-join",
            crate::runtime::TaskPriority::Background,
            || (),
            |_| (),
        )
        .expect("join while backoff")
        .is_none()
    );
    assert!(
        crate::runtime::Effect::resource_retry(
            &tasks,
            &"retry-backoff".into(),
            9,
            "not-due",
            crate::runtime::TaskPriority::Background,
            || (),
            |_| (),
        )
        .is_none()
    );
    let retry = crate::runtime::Effect::resource_retry(
        &tasks,
        &"retry-backoff".into(),
        10,
        "due",
        crate::runtime::TaskPriority::Background,
        || (),
        |_| (),
    )
    .expect("due retry");
    assert!(
        resource
            .begin(
                tasks
                    .operation(&"retry-backoff".into())
                    .expect("retry operation"),
                ResourceRefreshPolicy::RetainReady,
            )
            .expect("begin retry")
    );
    assert!(
        crate::runtime::Effect::resource_retry(
            &tasks,
            &"retry-backoff".into(),
            10,
            "already-taken",
            crate::runtime::TaskPriority::Background,
            || (),
            |_| (),
        )
        .is_none()
    );
    drop(retry);
}

#[test]
fn fence_exhaustion_fails_without_wrapping() {
    let mut resource = Resource::<u8, u8>::new("resource");
    resource.revision = u64::MAX;
    resource.generation = u64::MAX;

    assert_eq!(
        resource.advance_fences(),
        Err(ResourceStateError::RevisionExhausted)
    );
    assert!(!resource.can_bump_revision());
    assert_eq!(resource.revision, u64::MAX);
    assert_eq!(resource.generation, u64::MAX);
}

fn active_operation(
    tasks: &SharedResourceTasks,
    key: &'static str,
    interest_id: u64,
) -> (
    crate::application::ResourceInterest,
    crate::runtime::Effect<()>,
    SharedResourceOperation,
) {
    use std::sync::{Arc, atomic::AtomicBool};

    let interest = tasks
        .admit_interest(
            1,
            0,
            interest_id,
            key.into(),
            crate::application::ResourceInterestKind::Visible,
            Arc::new(AtomicBool::new(true)),
        )
        .expect("test interest");
    let effect = crate::runtime::Effect::resource_worker(
        tasks,
        key,
        crate::application::SharedResourceTaskMode::Join,
        "resource-state-test",
        crate::runtime::TaskPriority::Background,
        || (),
        |_| (),
    )
    .expect("reservation")
    .expect("worker effect");
    let operation = tasks.operation(interest.key()).expect("current operation");
    (interest, effect, operation)
}

fn active_interest(
    tasks: &SharedResourceTasks,
    key: &'static str,
    interest_id: u64,
) -> crate::application::ResourceInterest {
    use std::sync::atomic::AtomicBool;

    tasks
        .admit_interest(
            1,
            0,
            interest_id,
            key.into(),
            crate::application::ResourceInterestKind::Visible,
            Arc::new(AtomicBool::new(true)),
        )
        .expect("test interest")
}

enum CompletionMessage {
    Complete(SharedResourceCompletion<Result<u8, &'static str>>),
}

struct CompletionBridge {
    resource: Resource<u8, &'static str>,
    tasks: SharedResourceTasks,
}

impl RuntimeBridge<CompletionMessage> for CompletionBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<CompletionMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, message: CompletionMessage) -> Command<CompletionMessage> {
        let CompletionMessage::Complete(completion) = message;
        assert!(self.resource.apply_completion(&self.tasks, completion));
        Command::none()
    }
}

fn completion_host(
    tasks: SharedResourceTasks,
    key: &'static str,
) -> DeterministicHost<CompletionBridge, CompletionMessage> {
    DeterministicHost::with_default_config(
        CompletionBridge {
            resource: Resource::new(key),
            tasks,
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("deterministic host")
}

fn completion_worker(
    tasks: &SharedResourceTasks,
    key: &'static str,
    result: Result<u8, &'static str>,
) -> (Effect<CompletionMessage>, SharedResourceOperation) {
    let effect = Effect::resource_worker(
        tasks,
        key,
        crate::application::SharedResourceTaskMode::Refresh,
        "resource-completion-test",
        TaskPriority::Background,
        move || result,
        CompletionMessage::Complete,
    )
    .expect("reservation")
    .expect("worker effect");
    let operation = tasks.operation(&key.into()).expect("current operation");
    (effect, operation)
}

fn complete_worker(host: &mut DeterministicHost<CompletionBridge, CompletionMessage>) {
    let worker = host
        .pending_worker_tasks()
        .first()
        .expect("pending worker")
        .id;
    host.complete_worker(worker).expect("worker completion");
    host.turn().expect("completion reducer turn");
}

#[test]
fn begin_is_idempotent_and_progress_requires_exact_newer_current_operation() {
    let tasks = SharedResourceTasks::new();
    let (interest, _effect, operation) = active_operation(&tasks, "progress", 1);
    let mut resource = Resource::<u8, u8>::new("progress");

    assert!(
        resource
            .begin(operation.clone(), ResourceRefreshPolicy::RetainReady)
            .expect("begin")
    );
    let generation = resource.generation();
    assert!(
        !resource
            .begin(operation.clone(), ResourceRefreshPolicy::DiscardReady)
            .expect("same operation")
    );
    assert_eq!(resource.generation(), generation);
    assert!(resource.apply_progress(
        &operation,
        1,
        ResourceProgress::determinate(1, 4).expect("valid progress"),
    ));
    assert!(!resource.apply_progress(
        &operation,
        1,
        ResourceProgress::determinate(2, 4).expect("valid progress"),
    ));
    assert_eq!(
        resource.progress(),
        Some(ResourceProgress::determinate(1, 4).expect("valid progress"))
    );
    assert!(interest.is_live());
}

#[test]
fn foreign_or_cancelled_operation_cannot_leave_a_loading_snapshot() {
    let tasks = SharedResourceTasks::new();
    let (first_interest, _first_effect, first) = active_operation(&tasks, "first", 1);
    let (second_interest, _second_effect, second) = active_operation(&tasks, "second", 2);
    let mut resource = Resource::<u8, u8>::new("first");

    assert_eq!(
        resource.begin(second, ResourceRefreshPolicy::RetainReady),
        Err(ResourceStateError::ForeignOperation)
    );
    assert!(
        resource
            .begin(first, ResourceRefreshPolicy::RetainReady)
            .expect("begin first")
    );
    assert!(resource.cancel(&tasks));
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Cancelled);
    assert!(!resource.cancel(&tasks));
    assert!(first_interest.is_live());
    assert!(second_interest.is_live());
}

#[test]
fn completion_and_refresh_failure_retain_the_last_ready_value() {
    let tasks = SharedResourceTasks::new();
    let interest = active_interest(&tasks, "completion", 1);
    let mut host = completion_host(tasks.clone(), "completion");

    let (effect, operation) = completion_worker(&tasks, "completion", Ok(7));
    assert!(
        host.bridge_mut()
            .resource
            .begin(operation, ResourceRefreshPolicy::RetainReady)
            .expect("begin ready operation")
    );
    host.execute_command(Command::effect(effect))
        .expect("worker admission");
    complete_worker(&mut host);
    assert_eq!(
        host.bridge().resource.snapshot().phase(),
        ResourcePhase::Ready
    );
    assert_eq!(**host.bridge().resource.value().expect("ready value"), 7);

    let (effect, operation) = completion_worker(&tasks, "completion", Err("offline"));
    assert!(
        host.bridge_mut()
            .resource
            .begin(operation, ResourceRefreshPolicy::RetainReady)
            .expect("begin refresh")
    );
    assert_eq!(
        host.bridge().resource.snapshot().phase(),
        ResourcePhase::Refreshing
    );
    host.execute_command(Command::effect(effect))
        .expect("refresh admission");
    complete_worker(&mut host);

    let snapshot = host.bridge().resource.snapshot();
    assert_eq!(snapshot.phase(), ResourcePhase::Failed);
    assert_eq!(**snapshot.value().expect("retained ready value"), 7);
    assert_eq!(**snapshot.error().expect("failure"), "offline");

    let (effect, operation) = completion_worker(&tasks, "completion", Err("discarded"));
    assert!(
        host.bridge_mut()
            .resource
            .begin(operation, ResourceRefreshPolicy::DiscardReady)
            .expect("begin discard refresh")
    );
    assert_eq!(
        host.bridge().resource.snapshot().phase(),
        ResourcePhase::Pending
    );
    host.execute_command(Command::effect(effect))
        .expect("discard refresh admission");
    complete_worker(&mut host);
    let snapshot = host.bridge().resource.snapshot();
    assert_eq!(snapshot.phase(), ResourcePhase::Failed);
    assert!(snapshot.value().is_none());
    assert_eq!(**snapshot.error().expect("failure"), "discarded");
    assert!(interest.is_live());
}

#[test]
fn cancelled_pending_replacement_keeps_rollback_until_rejection() {
    let tasks = SharedResourceTasks::new();
    let _interest = active_interest(&tasks, "cancel-rollback", 95);
    let first = reserve(
        &tasks,
        "cancel-rollback",
        ResourceOperationReplaceMode::Join,
    );
    first.transaction().accept();
    let first_operation = tasks.operation(&"cancel-rollback".into()).expect("first");
    let mut resource = Resource::<u8, u8>::new("cancel-rollback");
    resource
        .begin(first_operation.clone(), ResourceRefreshPolicy::RetainReady)
        .expect("begin first");
    let second = reserve(
        &tasks,
        "cancel-rollback",
        ResourceOperationReplaceMode::Replace,
    );
    let token = crate::application::CancellationToken::new();
    second.attach_cancellation(token.clone());
    resource
        .begin(
            tasks.operation(&"cancel-rollback".into()).expect("second"),
            ResourceRefreshPolicy::RetainReady,
        )
        .expect("begin second");
    token.cancel();
    assert!(!resource.apply_progress(&first_operation, 1, ResourceProgress::indeterminate()));
    assert!(resource.predecessor.is_some());
    second.transaction().reject();
    assert!(resource.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(7),
            current: first.current()
        }
    ));
}

#[test]
fn rejected_discard_refresh_restores_completed_ready_state() {
    let tasks = SharedResourceTasks::new();
    let _interest = active_interest(&tasks, "ready-rollback", 96);
    let first = reserve(&tasks, "ready-rollback", ResourceOperationReplaceMode::Join);
    first.transaction().accept();
    let mut resource = Resource::<u8, u8>::new("ready-rollback");
    resource
        .begin(
            tasks.operation(&"ready-rollback".into()).expect("first"),
            ResourceRefreshPolicy::RetainReady,
        )
        .expect("begin first");
    assert!(resource.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(42),
            current: first.current()
        }
    ));
    let second = reserve(
        &tasks,
        "ready-rollback",
        ResourceOperationReplaceMode::Replace,
    );
    resource
        .begin(
            tasks.operation(&"ready-rollback".into()).expect("second"),
            ResourceRefreshPolicy::DiscardReady,
        )
        .expect("begin second");
    assert!(resource.snapshot().value().is_none());
    drop(second);
    assert_eq!(resource.snapshot().phase(), ResourcePhase::Ready);
    assert_eq!(**resource.snapshot().value().expect("restored ready"), 42);
}
