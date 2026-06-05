use std::cmp::Ordering;

#[path = "model/column_drag.rs"]
mod column_drag;
pub use column_drag::{
    DetailsColumnDragFeedback, DetailsColumnReorderDrag, DetailsColumnResizeDrag,
    DetailsColumnWidthUpdate, details_column_drag_content_left, details_column_drag_feedback,
    update_details_column_reorder_drag, update_details_column_resize_drag,
};

/// Sort direction displayed by a sortable details-list column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending sort.
    Ascending,
    /// Descending sort.
    Descending,
}

impl SortDirection {
    /// Return the compact suffix used for sorted details-list headers.
    pub const fn marker(self) -> &'static str {
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

    /// Apply this direction to an already-computed ascending ordering.
    pub fn apply_ordering(self, ordering: Ordering) -> Ordering {
        match self {
            Self::Ascending => ordering,
            Self::Descending => ordering.reverse(),
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

    /// Return the compact display label for a details-list column header.
    ///
    /// The sort marker is appended only when `column_id` matches this sort
    /// state. Applications that build custom details headers can use this to
    /// keep marker copy consistent with Radiant's built-in details lists.
    pub fn label_for(&self, column_id: &str, label: &str) -> String {
        if self.column_id == column_id {
            format!("{}{}", label, self.direction.marker())
        } else {
            label.to_string()
        }
    }
}

/// Return a compact details-list header label with a sort marker when active.
pub fn details_sort_label(label: &str, column_id: &str, sort: Option<&DetailsSort>) -> String {
    sort.map(|sort| sort.label_for(column_id, label))
        .unwrap_or_else(|| label.to_string())
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

/// Minimal column geometry used to resolve pointer-driven column reordering.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnPlacement {
    /// Stable caller-owned column id.
    pub id: String,
    /// Current rendered width.
    pub width: f32,
}

impl DetailsColumnPlacement {
    /// Build a column placement descriptor.
    pub fn new(id: impl ToString, width: f32) -> Self {
        Self {
            id: id.to_string(),
            width,
        }
    }
}

/// Return the insertion index for a dragged details-list column.
///
/// The `pointer_x` value is compared against the midpoint of every non-dragged
/// column in the current visual order. `content_left` is the x-coordinate where
/// the first column starts, and `column_gap` is the spacing between columns.
pub fn details_column_reorder_index(
    placements: &[DetailsColumnPlacement],
    dragged_id: &str,
    pointer_x: f32,
    content_left: f32,
    column_gap: f32,
) -> Option<usize> {
    if !placements
        .iter()
        .any(|placement| placement.id == dragged_id)
    {
        return None;
    }

    let mut x = content_left;
    let mut target = 0usize;
    for placement in placements {
        let midpoint = x + placement.width * 0.5;
        if placement.id != dragged_id && pointer_x > midpoint {
            target += 1;
        }
        x += placement.width + column_gap.max(0.0);
    }
    Some(target.min(placements.len().saturating_sub(1)))
}

/// Move the dragged item to `target_index`, preserving all other item order.
pub fn reorder_details_columns_by_id<T>(
    columns: &mut Vec<T>,
    dragged_id: &str,
    target_index: usize,
    id: impl Fn(&T) -> &str,
) -> bool {
    let Some(from_index) = columns.iter().position(|column| id(column) == dragged_id) else {
        return false;
    };
    let target_index = target_index.min(columns.len().saturating_sub(1));
    if from_index == target_index {
        return false;
    }
    let column = columns.remove(from_index);
    columns.insert(target_index, column);
    true
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

#[cfg(test)]
mod tests {
    use super::{
        DetailsColumnPlacement, DetailsSort, SortDirection, details_column_reorder_index,
        details_sort_label, reorder_details_columns_by_id,
    };

    #[test]
    fn details_column_reorder_index_uses_non_dragged_midpoints() {
        let placements = vec![
            DetailsColumnPlacement::new("name", 240.0),
            DetailsColumnPlacement::new("rating", 68.0),
            DetailsColumnPlacement::new("extension", 54.0),
            DetailsColumnPlacement::new("size", 78.0),
        ];

        assert_eq!(
            details_column_reorder_index(&placements, "rating", 410.0, 8.0, 10.0),
            Some(2)
        );
        assert_eq!(
            details_column_reorder_index(&placements, "size", 16.0, 8.0, 10.0),
            Some(0)
        );
        assert_eq!(
            details_column_reorder_index(&placements, "missing", 16.0, 8.0, 10.0),
            None
        );
    }

    #[test]
    fn reorder_details_columns_by_id_preserves_other_column_order() {
        let mut columns = vec![
            String::from("name"),
            String::from("rating"),
            String::from("extension"),
            String::from("size"),
        ];

        assert!(reorder_details_columns_by_id(
            &mut columns,
            "rating",
            3,
            String::as_str
        ));

        assert_eq!(columns, ["name", "extension", "size", "rating"]);
    }

    #[test]
    fn details_sort_label_marks_only_sorted_column() {
        let sort = DetailsSort::new("name", SortDirection::Descending);

        assert_eq!(details_sort_label("Name", "name", Some(&sort)), "Name v");
        assert_eq!(details_sort_label("Size", "size", Some(&sort)), "Size");
        assert_eq!(details_sort_label("Name", "name", None), "Name");
        assert_eq!(SortDirection::Ascending.marker(), " ^");
    }

    #[test]
    fn sort_direction_applies_to_ascending_ordering() {
        assert_eq!(
            SortDirection::Ascending.apply_ordering(std::cmp::Ordering::Less),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            SortDirection::Descending.apply_ordering(std::cmp::Ordering::Less),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            SortDirection::Descending.apply_ordering(std::cmp::Ordering::Equal),
            std::cmp::Ordering::Equal
        );
    }
}
