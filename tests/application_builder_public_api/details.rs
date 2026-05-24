#[test]
fn details_columns_use_logical_widths() {
    use radiant::prelude::{DetailsColumn, DetailsColumnParts};

    assert_eq!(
        DetailsColumn::fixed("kind", "Kind", 120.5),
        DetailsColumn {
            id: String::from("kind"),
            label: String::from("Kind"),
            width: Some(120.5),
        }
    );
    assert_eq!(
        DetailsColumn::from_parts(DetailsColumnParts {
            id: String::from("state"),
            label: String::from("State"),
            width: Some(96.0),
        }),
        DetailsColumn::fixed("state", "State", 96.0)
    );
    assert_eq!(
        DetailsColumn::from_parts(DetailsColumnParts {
            id: String::from("name"),
            label: String::from("Name"),
            width: None,
        }),
        DetailsColumn::flexible("name", "Name")
    );
    assert_eq!(DetailsColumn::flexible("name", "Name").width, None);
}

#[test]
fn details_rows_support_named_parts_construction() {
    use radiant::prelude as ui;

    let from_parts = ui::DetailsRow::from_parts(ui::DetailsRowParts {
        id: String::from("timeline"),
        cells: vec![
            String::from("timeline.rs"),
            String::from("Rust"),
            String::from("Ready"),
        ],
    })
    .selected(true);

    let positional =
        ui::DetailsRow::new("timeline", ["timeline.rs", "Rust", "Ready"]).selected(true);

    assert_eq!(from_parts, positional);
    assert_eq!(from_parts.cells.len(), 3);
    assert!(from_parts.selected);
}

#[test]
fn details_sort_supports_named_parts_construction() {
    use radiant::prelude as ui;

    let from_parts = ui::DetailsSort::from_parts(ui::DetailsSortParts {
        column_id: String::from("kind"),
        direction: ui::SortDirection::Descending,
    });
    let positional = ui::DetailsSort::new("kind", ui::SortDirection::Descending);

    assert_eq!(from_parts, positional);
    assert_eq!(from_parts.column_id, "kind");
    assert_eq!(from_parts.direction.toggled(), ui::SortDirection::Ascending);
}
