use super::*;

#[test]
fn application_builder_property_panel_routes_row_selection() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::selectable_property_panel(
                "Inspector",
                [
                    ui::PropertyRow::new("name", "Name", state.name.clone())
                        .selected(state.name == "name"),
                    ui::PropertyRow::new("count", "Count", state.count.to_string())
                        .selected(state.name == "count"),
                ],
                Some(|state: &mut DemoState, id| state.name = id),
            )
        })
        .into_bridge();

    let surface = bridge.project_surface();
    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 2);

    let message = surface
        .dispatch_widget_output(
            focus_order[1],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("property value button should emit a state action");
    let command = bridge.update(message);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<ButtonWidget, _>(&after, focus_order[0], "button")
            .props
            .label,
        "count"
    );
}

#[test]
fn application_builder_property_panel_read_only_rows_do_not_join_focus_order() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<ui::StateAction<DemoState>> = ui::property_panel(
        "Inspector",
        [
            ui::PropertyRow::new("name", "Name", "Layer 12"),
            ui::PropertyRow::new("kind", "Kind", "Signal track").selected(true),
        ],
    )
    .id(1)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(280.0, 120.0)),
    );

    assert!(surface.keyboard_focus_order().is_empty());
    assert_eq!(layout.rects[&1].min.x, 0.0);
    assert!(layout.rects[&1].height() <= 120.0);
}

#[test]
fn application_builder_context_menu_overlay_routes_items() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::stack([
                ui::text(format!("Selected: {}", state.name))
                    .id(10)
                    .height(24.0)
                    .fill_width(),
                ui::context_menu_overlay(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
                    Point::new(260.0, 150.0),
                    Vector2::new(140.0, 92.0),
                    "Actions",
                    [
                        ui::MenuItem::new("Inspect", |state: &mut DemoState| {
                            state.name = "inspect".to_string()
                        })
                        .primary(),
                        ui::MenuItem::new("Delete", |state: &mut DemoState| {
                            state.name = "delete".to_string()
                        })
                        .danger(),
                    ],
                )
                .id(20),
            ])
        })
        .into_bridge();

    let surface = bridge.project_surface();
    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 2);

    let message = surface
        .dispatch_widget_output(
            focus_order[1],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("context menu item should emit a state action");
    let command = bridge.update(message);
    assert!(command.requests_repaint());

    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Selected: delete"
    );

    let layout = layout_tree(
        &after.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
    );
    assert_eq!(layout.rects[&20].min.x, 0.0);
}

#[test]
fn application_builder_todo_layout_does_not_overlap_header_input_and_list() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::row([
            ui::text("Todos").id(10).size(140.0, 28.0),
            ui::text("1/3 done").id(11).size(120.0, 28.0),
        ])
        .id(2)
        .fill_width(),
        ui::row([
            ui::text_input("Review public API")
                .message(|_| ())
                .id(12)
                .min_size(260.0, 32.0)
                .preferred_size(420.0, 32.0)
                .fill_width(),
            ui::button("Add")
                .primary()
                .message(())
                .id(13)
                .size(80.0, 32.0),
        ])
        .id(3)
        .fill_width(),
        ui::list(0..3, |index| {
            ui::list_row(
                index,
                [
                    ui::checkbox(false)
                        .message(|_| ())
                        .id(20 + index)
                        .size(24.0, 24.0),
                    ui::text(format!("Item {index}"))
                        .id(60 + index)
                        .fill_width(),
                    ui::button("Delete")
                        .danger()
                        .message(())
                        .id(30 + index)
                        .size(84.0, 30.0),
                ],
            )
            .id(40 + index)
        })
        .id(4),
    ])
    .id(1)
    .padding(16.0)
    .spacing(12.0)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(560.0, 360.0)),
    );

    let header = layout.rects[&2];
    let input = layout.rects[&3];
    let list = layout.rects[&4];
    let first_row = layout.rects[&40];

    assert_eq!(header.height(), 28.0);
    assert_eq!(input.height(), 32.0);
    assert!(input.min.y >= header.max.y + 12.0);
    assert!(list.min.y >= input.max.y + 12.0);
    assert!(first_row.min.y >= list.min.y);
    assert_eq!(first_row.height(), 44.0);
}
