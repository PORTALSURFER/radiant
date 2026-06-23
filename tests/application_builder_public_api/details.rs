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
    assert_eq!(from_parts.label_for("kind", "Kind"), "Kind v");
    assert_eq!(
        ui::details_sort_label("Name", "name", Some(&from_parts)),
        "Name"
    );
}

#[test]
fn details_column_drag_helpers_cover_resize_and_reorder_state() {
    use radiant::prelude as ui;
    use radiant::prelude::Point;

    let resize = ui::DetailsColumnResizeDrag::new("name", 100.0, 240.0);
    assert_eq!(resize.width_at(130.0, 48.0, 420.0), 270.0);
    assert_eq!(resize.width_at(-500.0, 48.0, 420.0), 48.0);

    let placements = vec![
        ui::DetailsColumnPlacement::new("name", 240.0),
        ui::DetailsColumnPlacement::new("rating", 68.0),
        ui::DetailsColumnPlacement::new("extension", 54.0),
    ];
    let content_left = ui::details_column_drag_content_left(&placements, "rating", 300.0, 10.0)
        .expect("rating column should exist");
    let reorder = ui::DetailsColumnReorderDrag::new("rating", content_left);
    let reorder_with_pointer =
        ui::DetailsColumnReorderDrag::from_start("rating", content_left, Point::new(300.0, 0.0));

    assert_eq!(content_left, 16.0);
    assert_eq!(reorder.target_index(&placements, 410.0, 10.0), Some(2));
    assert_eq!(reorder.pointer, Point::new(0.0, 0.0));
    assert_eq!(reorder_with_pointer.pointer, Point::new(300.0, 0.0));
    assert_eq!(
        reorder_with_pointer.current_target_index(&placements, 10.0),
        Some(1)
    );
}

#[test]
fn compact_details_row_exposes_details_list_density() {
    use super::*;
    use radiant::prelude as ui;
    use radiant::prelude::IntoView;

    let surface: UiSurface<DemoState> = ui::column([ui::compact_details_row([
        ui::text("Name").id(10).fixed(40.0, 20.0),
        ui::text("Size").id(11).fixed(40.0, 20.0),
    ])
    .id(1)])
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 40.0)),
    );

    assert_eq!(layout.rects[&1].height(), 22.0);
    assert_eq!(layout.rects[&10].min.x, 8.0);
    assert_eq!(layout.rects[&10].min.y, 1.0);
    assert_eq!(layout.rects[&11].min.x, layout.rects[&10].max.x + 10.0);
}

#[test]
fn compact_details_cell_exposes_details_list_cell_sizing() {
    use super::*;
    use radiant::prelude as ui;
    use radiant::prelude::IntoView;

    let surface: UiSurface<DemoState> = ui::column([ui::compact_details_row([
        ui::compact_details_cell(ui::text("Name").id(10), Some(64.0)),
        ui::compact_details_cell(ui::text("Kind").id(11), None),
    ])
    .id(1)])
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 40.0)),
    );

    assert_eq!(layout.rects[&10].width(), 64.0);
    assert_eq!(layout.rects[&10].height(), 20.0);
    assert_eq!(layout.rects[&11].height(), 20.0);
    assert!(layout.rects[&11].width() > 64.0);
}

#[test]
fn compact_details_header_row_exposes_details_list_header_chrome() {
    use super::*;
    use radiant::prelude as ui;
    use radiant::prelude::IntoView;

    let surface: UiSurface<DemoState> = ui::column([ui::compact_details_header_row([
        ui::text("Name").id(10).fixed(40.0, 20.0),
        ui::text("Size").id(11).fixed(40.0, 20.0),
    ])
    .id(1)])
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 40.0)),
    );

    assert_eq!(layout.rects[&1].height(), 24.0);
    assert_eq!(layout.rects[&10].min.x, 8.0);
    assert_eq!(layout.rects[&10].min.y, 2.0);
    assert_eq!(layout.rects[&11].min.x, layout.rects[&10].max.x + 10.0);
}

#[test]
fn compact_resizable_details_header_cell_builds_standard_interactive_cell() {
    use super::*;
    use radiant::{prelude as ui, prelude::IntoView, runtime::PaintPrimitive};

    #[derive(Clone)]
    enum HeaderMessage {
        Sort,
        Drag,
        Resize,
    }

    let surface: UiSurface<HeaderMessage> = ui::row([
        ui::compact_resizable_details_header_cell(
            "name-header",
            "Name v",
            120.0,
            HeaderMessage::Sort,
            |_| HeaderMessage::Drag,
            |_| HeaderMessage::Resize,
        )
        .id(1),
        ui::text("Tail").id(2).fixed(20.0, 20.0),
    ])
    .spacing(0.0)
    .height(20.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 40.0)),
    );
    let sort_drag_id = ui::compact_details_header_sort_drag_id(1);
    let resize_id = ui::compact_details_header_resize_id(1);
    let frame = radiant::runtime::UiSurface::new(surface.root().clone()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 40.0)),
        &Default::default(),
    );

    assert_eq!(layout.rects[&1].width(), 120.0);
    assert!(layout.rects.contains_key(&sort_drag_id));
    assert_eq!(layout.rects[&resize_id].width(), 4.0);
    assert_eq!(layout.rects[&2].min.x, 120.0);
    assert!(
        frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Name v")
        )
    );
}
