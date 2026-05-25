use super::*;

#[test]
fn application_builder_lists_keep_row_heights_stable_across_item_counts() {
    use radiant::prelude::{self as ui, IntoView};

    fn surface(count: u64) -> UiSurface<()> {
        ui::column([ui::list(0..count, |index| {
            ui::list_row(
                index,
                [
                    ui::text(format!("Item {index}"))
                        .id(100 + index)
                        .fill_width(),
                    ui::button("Delete").danger().message(()).id(200 + index),
                ],
            )
            .id(10 + index)
        })
        .id(2)])
        .id(1)
        .padding(12.0)
        .into_surface()
    }

    let two = layout_tree(
        &surface(2).layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 200.0)),
    );
    let ten = layout_tree(
        &surface(10).layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 200.0)),
    );

    assert_eq!(two.rects[&10].height(), 44.0);
    assert_eq!(two.rects[&11].height(), 44.0);
    assert_eq!(ten.rects[&10].height(), 44.0);
    assert_eq!(ten.rects[&11].height(), 44.0);
}

#[test]
fn application_builder_default_containers_use_dense_spacing() {
    use radiant::prelude::{self as ui, IntoView};

    let row_surface: UiSurface<()> = ui::row([
        ui::text("Left").id(10).fixed(40.0, 20.0),
        ui::text("Right").id(11).fixed(40.0, 20.0),
    ])
    .id(1)
    .into_surface();
    let row_layout = layout_tree(
        &row_surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 40.0)),
    );
    assert_eq!(
        row_layout.rects[&11].min.x,
        row_layout.rects[&10].max.x + radiant::DEFAULT_ROW_SPACING
    );

    let column_surface: UiSurface<()> = ui::column([
        ui::text("Top").id(20).fixed(40.0, 20.0),
        ui::text("Bottom").id(21).fixed(40.0, 20.0),
    ])
    .id(2)
    .into_surface();
    let column_layout = layout_tree(
        &column_surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 80.0)),
    );
    assert_eq!(
        column_layout.rects[&21].min.y,
        column_layout.rects[&20].max.y + radiant::DEFAULT_COLUMN_SPACING
    );
}

#[test]
fn application_builder_styled_containers_use_default_panel_padding() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([ui::text("Panel").id(10).fixed(40.0, 20.0)])
        .id(1)
        .style(radiant::widgets::WidgetStyle::default())
        .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 60.0)),
    );

    assert_eq!(
        layout.rects[&10].min.x,
        radiant::DEFAULT_STYLED_CONTAINER_PADDING
    );
    assert_eq!(
        layout.rects[&10].min.y,
        radiant::DEFAULT_STYLED_CONTAINER_PADDING
    );
}

#[test]
fn application_builder_virtual_list_records_virtual_window() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::virtual_list(
        0..512_u64,
        |index| {
            ui::list_row(
                index,
                [ui::button(format!("Row {index:03}"))
                    .message(DemoMessage::Increment)
                    .id(1_000 + index)],
            )
            .id(10_000 + index)
        },
        64.0,
    )
    .id(2)
    .into_surface();
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(2, Vector2::new(0.0, 640.0));

    let output = layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&2)
        .expect("virtual_list should lower to a virtualized scroll viewport");

    assert_eq!(window.total_children, 512);
    assert!(window.first_index > 0);
    assert!(window.culled_after > 0);
    assert!(surface.find_widget(1_000).is_some());
}

#[test]
fn application_builder_list_row_id_uses_direct_numeric_identity() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::list_row_id(
        42,
        [ui::button("Open").message(DemoMessage::Increment).id(420)],
    )
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 64.0)),
    );

    assert!(layout.rects.contains_key(&42));
    assert!(surface.find_widget(420).is_some());
}

#[test]
fn tree_list_items_support_named_parts_construction() {
    use radiant::prelude as ui;

    let from_parts = ui::TreeListItem::from_parts(ui::TreeListItemParts {
        id: String::from("arrangement/tracks"),
        depth: 2,
        label: String::from("Tracks"),
    })
    .branch(true)
    .selected(true)
    .draggable(true)
    .drop_target(true);

    let positional = ui::TreeListItem::new("arrangement/tracks", 2, "Tracks")
        .branch(true)
        .selected(true)
        .draggable(true)
        .drop_target(true);

    assert_eq!(from_parts, positional);
    assert_eq!(from_parts.depth, 2);
    assert!(from_parts.has_children);
    assert!(from_parts.selected);
    assert!(from_parts.draggable);
    assert!(from_parts.drop_target);
}

#[test]
fn application_builder_grid_lowers_to_fixed_column_tile_layout() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::grid_with_gaps(
        (0..5).map(|index| {
            ui::text(format!("Tile {index}"))
                .id(100 + index)
                .fill_width()
                .height(28.0)
        }),
        2,
        10.0,
        6.0,
    )
    .id(10)
    .padding(4.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 160.0)),
    );
    let first = layout.rects[&100];
    let second = layout.rects[&101];
    let third = layout.rects[&102];

    assert_eq!(layout.rects[&10].min.x, 0.0);
    assert!(second.min.x > first.max.x);
    assert_eq!(first.min.y, second.min.y);
    assert!(third.min.y > first.min.y);
    assert_eq!(first.height(), 28.0);
}

#[test]
fn application_builder_wrap_flows_fixed_width_items_to_new_rows() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::wrap(
        (0..4).map(|index| {
            ui::text(format!("Tag {index}"))
                .id(200 + index)
                .size(70.0, 20.0)
        }),
        6.0,
        5.0,
    )
    .id(20)
    .padding(4.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 120.0)),
    );
    let first = layout.rects[&200];
    let second = layout.rects[&201];
    let third = layout.rects[&202];

    assert!(second.min.x > first.max.x);
    assert_eq!(first.min.y, second.min.y);
    assert_eq!(third.min.x, first.min.x);
    assert!(third.min.y > first.min.y);
}
