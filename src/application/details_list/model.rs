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

/// Named construction inputs for details-list sort state.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DetailsSortParts {
    /// Stable sorted column id.
    pub column_id: String,
    /// Current sort direction.
    pub direction: SortDirection,
}

impl DetailsSort {
    /// Build a current sort descriptor from named construction inputs.
    pub fn from_parts(parts: DetailsSortParts) -> Self {
        Self {
            column_id: parts.column_id,
            direction: parts.direction,
        }
    }

    /// Build a current sort descriptor.
    pub fn new(column_id: impl ToString, direction: SortDirection) -> Self {
        Self::from_parts(DetailsSortParts {
            column_id: column_id.to_string(),
            direction,
        })
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

/// Named construction inputs for one sortable details-list column.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnParts {
    /// Stable caller-owned column id.
    pub id: String,
    /// Header label.
    pub label: String,
    /// Fixed logical width, or `None` for the flexible primary column.
    pub width: Option<f32>,
}

impl DetailsColumn {
    /// Build a details-list column from named construction inputs.
    pub fn from_parts(parts: DetailsColumnParts) -> Self {
        Self {
            id: parts.id,
            label: parts.label,
            width: parts.width,
        }
    }

    /// Build a flexible details-list column.
    pub fn flexible(id: impl ToString, label: impl Into<String>) -> Self {
        Self::from_parts(DetailsColumnParts {
            id: id.to_string(),
            label: label.into(),
            width: None,
        })
    }

    /// Build a fixed-width details-list column.
    pub fn fixed(id: impl ToString, label: impl Into<String>, width: f32) -> Self {
        Self::from_parts(DetailsColumnParts {
            id: id.to_string(),
            label: label.into(),
            width: Some(width),
        })
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

/// Named construction inputs for one compact details-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsRowParts {
    /// Stable caller-owned row id.
    pub id: String,
    /// Cell text in the same order as the columns.
    pub cells: Vec<String>,
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
    pub fn new(id: impl ToString, cells: impl IntoIterator<Item = impl Into<String>>) -> Self {
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
