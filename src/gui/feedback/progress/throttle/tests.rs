use super::{ProgressUpdateGate, ThrottledProgressReporter};
use std::time::{Duration, Instant};

#[test]
fn progress_update_gate_coalesces_tight_fraction_updates() {
    let start = Instant::now();
    let mut gate =
        ProgressUpdateGate::new(Duration::from_millis(50), 0.01).with_max_fraction(0.995);

    assert_eq!(gate.accept_at(0.001, start), Some(0.001));
    assert_eq!(
        gate.accept_at(0.002, start + Duration::from_millis(1)),
        None
    );
    assert_eq!(
        gate.accept_at(0.003, start + Duration::from_millis(2)),
        None
    );
    assert_eq!(
        gate.accept_at(0.012, start + Duration::from_millis(3)),
        None
    );
    assert_eq!(
        gate.accept_at(0.014, start + Duration::from_millis(60)),
        Some(0.014)
    );
}

#[test]
fn progress_update_gate_accepts_terminal_fraction_immediately() {
    let start = Instant::now();
    let mut gate =
        ProgressUpdateGate::new(Duration::from_millis(50), 0.01).with_max_fraction(0.995);

    assert_eq!(gate.accept_at(0.2, start), Some(0.2));
    assert_eq!(
        gate.accept_at(1.0, start + Duration::from_millis(1)),
        Some(0.995)
    );
}

#[test]
fn progress_update_gate_rejects_backward_non_terminal_updates() {
    let start = Instant::now();
    let mut gate = ProgressUpdateGate::new(Duration::from_millis(50), 0.01);

    assert_eq!(gate.accept_at(0.2, start), Some(0.2));
    assert_eq!(
        gate.accept_at(0.19, start + Duration::from_millis(60)),
        None
    );
}

#[test]
fn throttled_progress_reporter_calls_callback_for_accepted_updates() {
    let start = Instant::now();
    let gate = ProgressUpdateGate::new(Duration::from_millis(50), 0.01).with_max_fraction(0.995);
    let mut reports = Vec::new();
    let mut reporter = ThrottledProgressReporter::new(gate, |progress| reports.push(progress));

    reporter.report_at(0.001, start);
    reporter.report_at(0.002, start + Duration::from_millis(1));
    reporter.report_at(0.014, start + Duration::from_millis(60));
    reporter.report_at(1.0, start + Duration::from_millis(61));

    assert_eq!(reports, vec![0.001, 0.014, 0.995]);
}
