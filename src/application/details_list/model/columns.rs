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

/// Move one visible details-list column to a visible target index.
///
/// Hosts often keep durable column preferences for both visible and hidden
/// columns. The `target_visible_index` value is resolved against the visible
/// subset only, while hidden columns remain in their existing relative order.
pub fn reorder_visible_details_columns_by_id<T>(
    columns: &mut Vec<T>,
    dragged_id: &str,
    target_visible_index: usize,
    id: impl Fn(&T) -> &str,
    is_visible: impl Fn(&T) -> bool,
) -> bool {
    let Some(from_index) = columns.iter().position(|column| id(column) == dragged_id) else {
        return false;
    };
    if !is_visible(&columns[from_index]) {
        return false;
    }

    let mut visible_index = 0usize;
    let mut from_visible_index = None;
    for column in columns.iter() {
        if !is_visible(column) {
            continue;
        }
        if id(column) == dragged_id {
            from_visible_index = Some(visible_index);
            break;
        }
        visible_index += 1;
    }

    let visible_count = columns.iter().filter(|column| is_visible(column)).count();
    if visible_count == 0 {
        return false;
    }
    let target_visible_index = target_visible_index.min(visible_count.saturating_sub(1));
    if from_visible_index == Some(target_visible_index) {
        return false;
    }

    let column = columns.remove(from_index);
    let insert_index =
        visible_details_column_insert_index(columns, target_visible_index, is_visible);
    columns.insert(insert_index, column);
    true
}

fn visible_details_column_insert_index<T>(
    columns: &[T],
    target_visible_index: usize,
    is_visible: impl Fn(&T) -> bool,
) -> usize {
    let mut visible_seen = 0usize;
    let mut after_last_visible = 0usize;
    for (index, column) in columns.iter().enumerate() {
        if !is_visible(column) {
            continue;
        }
        if visible_seen == target_visible_index {
            return index;
        }
        visible_seen += 1;
        after_last_visible = index + 1;
    }
    after_last_visible
}

#[cfg(test)]
mod tests {
    use super::{
        DetailsColumnPlacement, details_column_reorder_index, reorder_details_columns_by_id,
        reorder_visible_details_columns_by_id,
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
    fn reorder_visible_details_columns_by_id_targets_visible_subset() {
        let mut columns = vec![
            String::from("name"),
            String::from("source_folder"),
            String::from("rating"),
            String::from("extension"),
            String::from("path"),
        ];
        let visible =
            |column: &String| column.as_str() != "source_folder" && column.as_str() != "path";

        assert!(reorder_visible_details_columns_by_id(
            &mut columns,
            "rating",
            0,
            String::as_str,
            visible,
        ));

        assert_eq!(
            columns,
            ["rating", "name", "source_folder", "extension", "path"]
        );

        assert!(reorder_visible_details_columns_by_id(
            &mut columns,
            "rating",
            usize::MAX,
            String::as_str,
            visible,
        ));

        assert_eq!(
            columns,
            ["name", "source_folder", "extension", "rating", "path"]
        );
    }

    #[test]
    fn reorder_visible_details_columns_by_id_ignores_hidden_dragged_column() {
        let mut columns = vec![
            String::from("name"),
            String::from("source_folder"),
            String::from("rating"),
        ];

        assert!(!reorder_visible_details_columns_by_id(
            &mut columns,
            "source_folder",
            0,
            String::as_str,
            |column| column.as_str() != "source_folder",
        ));

        assert_eq!(columns, ["name", "source_folder", "rating"]);
    }
}
