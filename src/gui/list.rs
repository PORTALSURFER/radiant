//! Generic list and row state primitives.

use serde::{Deserialize, Serialize};

/// Render summary for one titled list or table column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSummary {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

impl ColumnSummary {
    /// Build a new column summary.
    pub fn new(title: impl Into<String>, item_count: usize) -> Self {
        Self {
            title: title.into(),
            item_count,
        }
    }
}

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
    use super::{ColumnSummary, RowProcessingState};

    #[test]
    fn column_summary_preserves_title_and_count() {
        let column = ColumnSummary::new("Inbox", 42);

        assert_eq!(column.title, "Inbox");
        assert_eq!(column.item_count, 42);
    }

    #[test]
    fn row_processing_state_defaults_to_none() {
        assert_eq!(RowProcessingState::default(), RowProcessingState::None);
    }
}
