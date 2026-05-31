/// Small revision counter for GUI cache, identity, and retained-state refreshes.
///
/// Use this when application code needs a cheap value that changes after a
/// GUI-only state transition, such as clearing hover/drag state or invalidating
/// app-owned retained projections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RevisionCounter {
    revision: u64,
}

impl RevisionCounter {
    /// Build a counter from an explicit revision value.
    pub const fn new(revision: u64) -> Self {
        Self { revision }
    }

    /// Return the current revision value.
    pub const fn get(self) -> u64 {
        self.revision
    }

    /// Advance the revision and return the new value.
    pub fn bump(&mut self) -> u64 {
        self.revision = self.revision.wrapping_add(1);
        self.revision
    }

    /// Advance the revision when `condition` is true and return the current value.
    pub fn bump_if(&mut self, condition: bool) -> u64 {
        if condition {
            self.bump();
        }
        self.revision
    }
}
