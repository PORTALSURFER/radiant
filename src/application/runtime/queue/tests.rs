use super::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct TestRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for TestRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

#[test]
fn pending_messages_recover_after_poisoned_queue_lock() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let poisoned = Arc::clone(&runtime);
    let _ = thread::spawn(move || {
        let mut pending = poisoned.pending.lock().expect("pending messages lock");
        pending.push(1);
        panic!("poison pending message queue");
    })
    .join();

    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![1, 2]);
}

#[test]
fn commands_recover_after_poisoned_queue_lock() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let poisoned = Arc::clone(&runtime);
    let _ = thread::spawn(move || {
        let mut commands = poisoned.commands.lock().expect("pending commands lock");
        commands.push(Command::message(1));
        panic!("poison pending command queue");
    })
    .join();

    assert!(runtime.enqueue_command(Command::message(2)));

    assert_eq!(runtime.take_commands().len(), 2);
}

#[test]
fn repaint_requests_recover_after_poisoned_signal_lock() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let called = Arc::new(AtomicBool::new(false));
    let poisoned = Arc::clone(&runtime);
    let signal = Arc::clone(&called);
    let _ = thread::spawn(move || {
        let mut repaint = poisoned.repaint.lock().expect("repaint signal lock");
        *repaint = Some(Arc::new(TestRepaintSignal { called: signal }));
        panic!("poison repaint signal lock");
    })
    .join();

    assert!(runtime.enqueue(1));

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn pending_message_queue_retains_capacity_after_drain() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let capacity = runtime.pending.lock().expect("pending lock").capacity();

    let pending = runtime.take_pending();
    assert_eq!(pending.len(), 32);
    assert_eq!(pending.capacity(), capacity);

    let retained_capacity = runtime.pending.lock().expect("pending lock").capacity();
    assert_eq!(retained_capacity, capacity);
}

#[test]
fn command_queue_retains_capacity_after_drain() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue_command(Command::message(message)));
    }
    let capacity = runtime.commands.lock().expect("commands lock").capacity();

    let commands = runtime.take_commands();
    assert_eq!(commands.len(), 32);
    assert_eq!(commands.capacity(), capacity);

    let retained_capacity = runtime.commands.lock().expect("commands lock").capacity();
    assert_eq!(retained_capacity, capacity);
}

#[test]
fn pending_message_queue_drains_into_reused_output_without_replacing_queue_storage() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let queue_capacity = runtime.pending.lock().expect("pending lock").capacity();
    let mut pending = Vec::with_capacity(64);
    let output_capacity = pending.capacity();

    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, (0..32).collect::<Vec<_>>());
    assert_eq!(pending.capacity(), output_capacity);
    let queue = runtime.pending.lock().expect("pending lock");
    assert!(queue.is_empty());
    assert_eq!(queue.capacity(), queue_capacity);
}

#[test]
fn pending_frame_drains_before_regular_messages() {
    let runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![99, 1, 2]);
}

#[test]
fn pending_frame_drains_before_retained_backlog() {
    let runtime = AppRuntime::<u32>::default();
    let mut pending = vec![10, 11];

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, vec![99, 10, 11, 1]);
}

#[test]
fn pending_frame_is_coalesced_until_drained() {
    let runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue_frame(1));
    assert!(!runtime.enqueue_frame(2));
    assert_eq!(runtime.take_pending(), vec![1]);

    assert!(runtime.enqueue_frame(3));
    assert_eq!(runtime.take_pending(), vec![3]);
}

#[test]
fn command_queue_drains_into_reused_output_without_replacing_queue_storage() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue_command(Command::message(message)));
    }
    let queue_capacity = runtime.commands.lock().expect("commands lock").capacity();
    let mut commands = Vec::with_capacity(64);
    let output_capacity = commands.capacity();

    runtime.drain_commands_into(&mut commands);

    assert_eq!(commands.len(), 32);
    assert_eq!(commands.capacity(), output_capacity);
    let queue = runtime.commands.lock().expect("commands lock");
    assert!(queue.is_empty());
    assert_eq!(queue.capacity(), queue_capacity);
}

#[test]
fn delayed_messages_use_runtime_timer_lane() {
    let runtime = Arc::new(AppRuntime::<u32>::default());

    assert!(runtime.schedule_message(Duration::from_millis(1), 7));

    let started = Instant::now();
    let mut delivered = Vec::new();
    while started.elapsed() < Duration::from_secs(1) {
        delivered = runtime.take_pending();
        if !delivered.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(delivered, vec![7]);
}

#[test]
fn delayed_messages_stop_after_runtime_shutdown() {
    let runtime = Arc::new(AppRuntime::<u32>::default());

    runtime.shutdown();

    assert!(!runtime.schedule_message(Duration::ZERO, 7));
    thread::sleep(Duration::from_millis(1));
    assert!(runtime.take_pending().is_empty());
}
