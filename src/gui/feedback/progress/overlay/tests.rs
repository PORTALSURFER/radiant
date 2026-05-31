use super::{ProgressOverlay, ProgressSnapshot};

#[test]
fn progress_overlay_defaults_to_hidden_and_empty() {
    let overlay = ProgressOverlay::default();

    assert!(!overlay.visible);
    assert!(!overlay.modal);
    assert_eq!(overlay.title, "");
    assert_eq!(overlay.detail, None);
    assert_eq!(overlay.completed, 0);
    assert_eq!(overlay.total, 0);
    assert!(!overlay.cancelable);
    assert!(!overlay.cancel_requested);
}

#[test]
fn progress_snapshot_reports_indeterminate_progress() {
    let progress = ProgressSnapshot::new(12, 0);

    assert!(progress.is_indeterminate());
    assert_eq!(progress.clamped_completed(), 12);
    assert_eq!(progress.fraction(), None);
    assert_eq!(progress.count_label("found"), "12 found");
}

#[test]
fn progress_snapshot_clamps_known_totals() {
    let progress = ProgressSnapshot::new(14, 10);

    assert!(!progress.is_indeterminate());
    assert_eq!(progress.clamped_completed(), 10);
    assert_eq!(progress.fraction(), Some(1.0));
    assert_eq!(progress.count_label("found"), "10/10");
}
