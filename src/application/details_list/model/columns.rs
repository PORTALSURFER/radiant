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

#[cfg(test)]
mod tests {
    use super::{
        DetailsColumnPlacement, details_column_reorder_index, reorder_details_columns_by_id,
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
}
