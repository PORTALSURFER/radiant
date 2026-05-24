use super::super::*;

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
fn property_rows_support_named_parts_construction() {
    use radiant::prelude as ui;

    let from_parts = ui::PropertyRow::from_parts(ui::PropertyRowParts {
        id: String::from("kind"),
        label: String::from("Kind"),
        value: String::from("Signal track"),
    })
    .selected(true);

    let positional = ui::PropertyRow::new("kind", "Kind", "Signal track").selected(true);

    assert_eq!(from_parts, positional);
    assert!(from_parts.selected);
}
