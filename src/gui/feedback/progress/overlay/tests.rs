use super::ProgressOverlay;

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
