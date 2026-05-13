/// Sort direction displayed by a sortable details-list column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending sort.
    Ascending,
    /// Descending sort.
    Descending,
}

impl SortDirection {
    pub(super) fn marker(self) -> &'static str {
        match self {
            Self::Ascending => " ^",
            Self::Descending => " v",
        }
    }

    /// Return the opposite sort direction.
    pub fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// Current sort state for a details list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DetailsSort {
    /// Stable sorted column id.
    pub column_id: String,
    /// Current sort direction.
    pub direction: SortDirection,
}

impl DetailsSort {
    /// Build a current sort descriptor.
    pub fn new(column_id: impl ToString, direction: SortDirection) -> Self {
        Self {
            column_id: column_id.to_string(),
            direction,
        }
    }
}

/// One sortable details-list column.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumn {
    /// Stable caller-owned column id.
    pub id: String,
    /// Header label.
    pub label: String,
    /// Fixed logical width, or `None` for the flexible primary column.
    pub width: Option<f32>,
}

impl DetailsColumn {
    /// Build a flexible details-list column.
    pub fn flexible(id: impl ToString, label: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.into(),
            width: None,
        }
    }

    /// Build a fixed-width details-list column.
    pub fn fixed(id: impl ToString, label: impl Into<String>, width: f32) -> Self {
        Self {
            id: id.to_string(),
            label: label.into(),
            width: Some(width),
        }
    }
}

/// One compact details-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsRow {
    /// Stable caller-owned row id.
    pub id: String,
    /// Cell text in the same order as the columns.
    pub cells: Vec<String>,
    /// Whether this row is currently selected.
    pub selected: bool,
}

impl DetailsRow {
    /// Build one details-list row.
    pub fn new(id: impl ToString, cells: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            id: id.to_string(),
            cells: cells.into_iter().map(Into::into).collect(),
            selected: false,
        }
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}
