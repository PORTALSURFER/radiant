/// Render summary for one titled list or table column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSummary {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

/// Named fields for constructing a list or table column summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSummaryParts {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

impl ColumnSummary {
    /// Build a column summary from named parts.
    pub fn from_parts(parts: ColumnSummaryParts) -> Self {
        Self {
            title: parts.title,
            item_count: parts.item_count,
        }
    }

    /// Build a new column summary.
    pub fn new(title: impl Into<String>, item_count: usize) -> Self {
        Self::from_parts(ColumnSummaryParts {
            title: title.into(),
            item_count,
        })
    }
}
