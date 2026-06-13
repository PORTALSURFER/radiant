use super::{
    AppRuntime, BusinessThreadPool, business_thread_name, default_business_worker_count,
    sleep_while_runtime_alive, spawn_business_thread,
};
use crate::runtime::TaskPriority;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[test]
fn spawn_business_thread_reports_success_for_started_work() {
    let (sender, receiver) = mpsc::channel();

    assert!(spawn_business_thread("test-thread", move || {
        let _ = sender.send(());
    }));
    assert!(receiver.recv().is_ok());
}

#[test]
fn business_thread_names_make_offloaded_work_explicit() {
    assert_eq!(
        business_thread_name("asset-load"),
        "radiant-business-asset-load"
    );
}

#[test]
fn default_business_worker_count_keeps_ui_capacity_available() {
    assert!((1..=2).contains(&default_business_worker_count()));
}

#[test]
fn business_thread_pool_runs_queued_work_on_named_workers() {
    let pool = BusinessThreadPool::new(2);
    let (sender, receiver) = mpsc::channel();
    for index in 0..4 {
        let sender = sender.clone();
        assert!(
            pool.spawn("test-job", TaskPriority::Background, None, move || {
                let thread_name = thread::current().name().unwrap_or_default().to_string();
                sender.send((index, thread_name)).expect("send work result");
            })
        );
    }
    drop(sender);

    let mut completed = receiver.iter().collect::<Vec<_>>();
    completed.sort_by_key(|(index, _)| *index);

    assert_eq!(completed.len(), 4);
    assert!(
        completed
            .iter()
            .all(|(_, name)| name.starts_with("radiant-business-worker-"))
    );
}

#[test]
fn business_thread_pool_rejects_work_when_no_workers_are_available() {
    let pool = BusinessThreadPool::without_workers_for_test();
    let ran = Arc::new(AtomicBool::new(false));
    let ran_task = Arc::clone(&ran);

    assert!(
        !pool.spawn("test-job", TaskPriority::Background, None, move || {
            ran_task.store(true, Ordering::Release);
        })
    );

    assert!(!ran.load(Ordering::Acquire));
    let diagnostics = pool.diagnostics.snapshot();
    assert_eq!(diagnostics.business.rejected, 1);
    assert_eq!(diagnostics.business.failed, 1);
    assert_eq!(
        diagnostics.business.recent.last().map(|event| event.state),
        Some(crate::runtime::BusinessTaskDiagnosticState::Rejected)
    );
}

#[test]
fn business_thread_pool_records_lifecycle_diagnostics() {
    let pool = BusinessThreadPool::new(1);
    let (sender, receiver) = mpsc::channel();

    assert!(pool.spawn(
        "diagnostic-job",
        TaskPriority::Interactive,
        None,
        move || {
            sender.send(()).expect("send work completion");
        }
    ));
    receiver.recv().expect("work should run");

    let diagnostics = wait_for_business_completion(&pool, 1);
    assert_eq!(diagnostics.business.queued, 1);
    assert_eq!(diagnostics.business.started, 1);
    assert_eq!(diagnostics.business.completed, 1);
    assert_eq!(diagnostics.business.running, 0);
    assert!(
        diagnostics
            .business
            .recent
            .iter()
            .any(|event| event.name == "diagnostic-job")
    );
}

#[test]
fn business_thread_pool_records_cancelled_completion_diagnostics() {
    let pool = BusinessThreadPool::new(1);
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_probe = Arc::clone(&cancelled);
    let cancelled_for_work = Arc::clone(&cancelled);
    let (sender, receiver) = mpsc::channel();

    assert!(pool.spawn(
        "cancelled-job",
        TaskPriority::Background,
        Some(Box::new(move || {
            cancelled_for_probe.load(Ordering::Acquire)
        })),
        move || {
            cancelled_for_work.store(true, Ordering::Release);
            sender.send(()).expect("send cancelled work completion");
        }
    ));
    receiver.recv().expect("work should run");

    let diagnostics = wait_for_business_completion(&pool, 1);
    assert_eq!(diagnostics.business.cancelled, 1);
    assert_eq!(diagnostics.business.completed, 0);
}

#[test]
fn runtime_sleep_stops_promptly_after_shutdown() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let weak = Arc::downgrade(&runtime);
    let stopper = Arc::clone(&runtime);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        stopper.shutdown();
    });

    let started = Instant::now();
    assert!(!sleep_while_runtime_alive(&weak, Duration::from_secs(60)));

    assert!(started.elapsed() < Duration::from_secs(1));
}

fn wait_for_business_completion(
    pool: &BusinessThreadPool,
    expected_terminal: usize,
) -> crate::runtime::RuntimeDiagnostics {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let diagnostics = pool.diagnostics.snapshot();
        let terminal = diagnostics.business.completed + diagnostics.business.cancelled;
        if terminal >= expected_terminal || Instant::now() >= deadline {
            return diagnostics;
        }
        thread::sleep(Duration::from_millis(1));
    }
}
