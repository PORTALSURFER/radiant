#[cfg(test)]
#[path = "overlay/tests.rs"]
mod tests;

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

/// Domain-neutral progress counters for long-running UI work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProgressSnapshot {
    /// Completed units.
    pub completed: usize,
    /// Total units, or zero when the total is not yet known.
    pub total: usize,
}

impl ProgressSnapshot {
    /// Build progress counters from completed and total units.
    pub const fn new(completed: usize, total: usize) -> Self {
        Self { completed, total }
    }

    /// Return whether this progress is indeterminate.
    pub const fn is_indeterminate(self) -> bool {
        self.total == 0
    }

    /// Completed units clamped to the known total.
    pub const fn clamped_completed(self) -> usize {
        if self.total == 0 {
            self.completed
        } else if self.completed > self.total {
            self.total
        } else {
            self.completed
        }
    }

    /// Determinate progress fraction, when a total is known.
    pub fn fraction(self) -> Option<f32> {
        if self.total == 0 {
            None
        } else {
            Some(self.clamped_completed() as f32 / self.total.max(1) as f32)
        }
    }

    /// Format a compact count label.
    pub fn count_label(self, indeterminate_suffix: &str) -> String {
        if self.total == 0 {
            format!("{} {indeterminate_suffix}", self.completed)
        } else {
            format!("{}/{}", self.clamped_completed(), self.total)
        }
    }
}
