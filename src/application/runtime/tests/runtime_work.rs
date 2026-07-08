use super::LatestStreamGate;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

#[test]
fn latest_stream_gate_rejects_emits_after_close() {
    let gate = LatestStreamGate::new();
    let stale = Arc::new(AtomicBool::new(false));
    let stale_for_emit = Arc::clone(&stale);

    gate.close();
    let accepted = gate.emit_latest(
        1,
        |_| true,
        || {
            stale_for_emit.store(true, Ordering::Release);
        },
    );

    assert!(!accepted);
    assert!(stale.load(Ordering::Acquire));
}

#[test]
fn latest_stream_gate_serializes_close_until_enqueue_finishes() {
    let gate = Arc::new(LatestStreamGate::new());
    let close_gate = Arc::clone(&gate);
    let emit_gate = Arc::clone(&gate);
    let (enqueue_entered_tx, enqueue_entered_rx) = mpsc::channel();
    let (release_enqueue_tx, release_enqueue_rx) = mpsc::channel();
    let (close_attempted_tx, close_attempted_rx) = mpsc::channel();
    let (close_done_tx, close_done_rx) = mpsc::channel();

    let emitter = thread::spawn(move || {
        emit_gate.emit_latest(
            1,
            |_| {
                enqueue_entered_tx.send(()).expect("enqueue entered");
                release_enqueue_rx.recv().expect("release enqueue");
                true
            },
            || {},
        )
    });
    enqueue_entered_rx.recv().expect("enqueue entered");

    let closer = thread::spawn(move || {
        close_attempted_tx.send(()).expect("close attempted");
        close_gate.close();
        close_done_tx.send(()).expect("close done");
    });
    close_attempted_rx.recv().expect("close attempted");
    assert!(
        close_done_rx
            .recv_timeout(Duration::from_millis(20))
            .is_err()
    );

    release_enqueue_tx.send(()).expect("release enqueue");
    assert!(emitter.join().expect("emitter joins"));
    close_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("close finishes after enqueue");
    closer.join().expect("closer joins");
}
