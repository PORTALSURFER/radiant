//! Generic list and row state primitives.

use serde::{Deserialize, Serialize};

/// Transient state for row-scoped batch operations.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RowProcessingState {
    /// The row is not part of an active row-scoped operation.
    #[default]
    None,
    /// The row is waiting in the current batch.
    Queued,
    /// The row is currently being processed.
    Active,
    /// The row completed successfully.
    Completed,
    /// The row was skipped by the batch.
    Skipped,
    /// The row failed during processing.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::RowProcessingState;

    #[test]
    fn row_processing_state_defaults_to_none() {
        assert_eq!(RowProcessingState::default(), RowProcessingState::None);
    }
}
