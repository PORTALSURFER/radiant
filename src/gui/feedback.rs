//! Generic user-feedback surface primitives.

/// Progress overlay state for long-running operations.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlay {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
}

/// Drag/drop overlay content for pointer-following feedback.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DragOverlay {
    /// Whether a drag payload is currently active.
    pub active: bool,
    /// Human-friendly payload label.
    pub label: String,
    /// Current hover target label.
    pub target_label: String,
    /// Whether the current target is a valid drop.
    pub valid_target: bool,
    /// Cursor anchor x-coordinate for the floating drag chip, when available.
    pub pointer_x: Option<u16>,
    /// Cursor anchor y-coordinate for the floating drag chip, when available.
    pub pointer_y: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::{DragOverlay, ProgressOverlay};

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
    fn drag_overlay_defaults_to_inactive_and_unanchored() {
        let overlay = DragOverlay::default();

        assert!(!overlay.active);
        assert_eq!(overlay.label, "");
        assert_eq!(overlay.target_label, "");
        assert!(!overlay.valid_target);
        assert_eq!(overlay.pointer_x, None);
        assert_eq!(overlay.pointer_y, None);
    }
}
