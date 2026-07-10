/// One compact details-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsRow {
    /// Stable caller-owned row id.
    pub id: String,
    /// Cell text in the same order as the columns.
    pub cells: Vec<TextContent>,
    /// Whether this row is currently selected.
    pub selected: bool,
}

/// Named construction inputs for one compact details-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsRowParts {
    /// Stable caller-owned row id.
    pub id: String,
    /// Cell text in the same order as the columns.
    pub cells: Vec<TextContent>,
}

impl DetailsRow {
    /// Build one details-list row from named construction inputs.
    pub fn from_parts(parts: DetailsRowParts) -> Self {
        Self {
            id: parts.id,
            cells: parts.cells,
            selected: false,
        }
    }

    /// Build one details-list row.
    pub fn new(id: impl ToString, cells: impl IntoIterator<Item = impl Into<TextContent>>) -> Self {
        Self::from_parts(DetailsRowParts {
            id: id.to_string(),
            cells: cells.into_iter().map(Into::into).collect(),
        })
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}
use crate::application::TextContent;
