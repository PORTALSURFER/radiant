use super::*;

#[derive(Default)]
struct CountingSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

#[test]
fn shared_repaint_signal_noop_when_unset() {
    let signal = SharedRepaintSignal::default();
    signal.request_repaint();
}

#[test]
fn shared_repaint_signal_forwards_request_to_active_callback() {
    let called = Arc::new(AtomicBool::new(false));
    let callback = Arc::new(CountingSignal {
        called: Arc::clone(&called),
    });

    let signal = SharedRepaintSignal::default();
    signal.set_signal(Some(callback));
    signal.request_repaint();

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn shared_repaint_signal_replaces_existing_callback() {
    let first_called = Arc::new(AtomicBool::new(false));
    let second_called = Arc::new(AtomicBool::new(false));

    let signal = SharedRepaintSignal::default();
    signal.set_signal(Some(Arc::new(CountingSignal {
        called: Arc::clone(&first_called),
    })));
    signal.set_signal(Some(Arc::new(CountingSignal {
        called: Arc::clone(&second_called),
    })));
    signal.request_repaint();

    assert!(!first_called.load(Ordering::Acquire));
    assert!(second_called.load(Ordering::Acquire));
}

#[test]
fn repaint_pending_gate_coalesces_duplicate_requests() {
    let pending = AtomicBool::new(false);

    assert!(try_mark_repaint_pending(&pending));
    assert!(pending.load(Ordering::Acquire));
    assert!(!try_mark_repaint_pending(&pending));
}

#[test]
fn coalescing_repaint_signal_queues_one_wakeup_while_pending() {
    let pending = Arc::new(AtomicBool::new(false));
    let wakeups = Arc::new(AtomicBool::new(false));
    let wakeups_for_callback = Arc::clone(&wakeups);
    let signal = CoalescingRepaintSignal::new(Arc::clone(&pending), move || {
        wakeups_for_callback.store(true, Ordering::Release);
        true
    });

    signal.request_repaint();
    signal.request_repaint();

    assert!(pending.load(Ordering::Acquire));
    assert!(wakeups.load(Ordering::Acquire));
}

#[test]
fn coalescing_repaint_signal_clears_pending_when_queue_fails() {
    let pending = Arc::new(AtomicBool::new(false));
    let signal = CoalescingRepaintSignal::new(Arc::clone(&pending), || false);

    signal.request_repaint();

    assert!(!pending.load(Ordering::Acquire));
}
