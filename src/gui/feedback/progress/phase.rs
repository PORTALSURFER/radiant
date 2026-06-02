#[cfg(test)]
#[path = "phase/tests.rs"]
mod tests;

/// Maps completed/total work counters into one fractional progress subrange.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressPhase {
    start: f32,
    end: f32,
}

impl ProgressPhase {
    /// Create a progress phase from normalized start and end fractions.
    pub fn new(start: f32, end: f32) -> Self {
        Self {
            start: normalized_fraction(start),
            end: normalized_fraction(end),
        }
    }

    /// Start fraction for this phase.
    pub const fn start(self) -> f32 {
        self.start
    }

    /// End fraction for this phase.
    pub const fn end(self) -> f32 {
        self.end
    }

    /// Return this phase's normalized fraction for completed work counters.
    pub fn fraction(self, completed: usize, total: usize) -> Option<f32> {
        if total == 0 {
            return None;
        }
        let ratio = completed as f32 / total as f32;
        Some(self.start + (self.end - self.start) * ratio.clamp(0.0, 1.0))
    }

    /// Report this phase's fraction through a callback.
    ///
    /// Returns whether the callback was invoked. A zero total cannot produce a
    /// meaningful fraction and is ignored.
    pub fn report(self, completed: usize, total: usize, mut report: impl FnMut(f32)) -> bool {
        let Some(fraction) = self.fraction(completed, total) else {
            return false;
        };
        report(fraction);
        true
    }
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
