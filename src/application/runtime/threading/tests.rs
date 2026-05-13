use super::{
    AppRuntime, BusinessThreadPool, business_thread_name, default_business_worker_count,
    sleep_while_runtime_alive, spawn_business_thread,
};
use std::{
    sync::{Arc, mpsc},
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
        assert!(pool.spawn("test-job", move || {
            let thread_name = thread::current().name().unwrap_or_default().to_string();
            sender.send((index, thread_name)).expect("send work result");
        }));
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
    let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ran_task = Arc::clone(&ran);

    assert!(!pool.spawn("test-job", move || {
        ran_task.store(true, std::sync::atomic::Ordering::Release);
    }));

    assert!(!ran.load(std::sync::atomic::Ordering::Acquire));
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
