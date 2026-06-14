use std::cmp::Ordering;

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

#[cfg(test)]
mod tests {
    use super::{DetailsSort, SortDirection, details_sort_label};

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
