use super::super::*;

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
fn application_builder_menus_support_named_parts_construction() {
    use radiant::prelude as ui;
    use std::sync::Arc;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::stack([
                ui::text(format!("Selected: {}", state.name)).id(31),
                ui::context_menu_overlay_from_parts(ui::ContextMenuOverlayParts {
                    bounds: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
                    anchor: Point::new(12.0, 12.0),
                    size: Vector2::new(132.0, 88.0),
                    title: String::from("Actions"),
                    items: vec![
                        ui::MenuItem::from_parts(ui::MenuItemParts {
                            label: String::from("Inspect"),
                            style: radiant::widgets::WidgetStyle::default(),
                            on_select: Arc::new(|state: &mut DemoState| {
                                state.name = String::from("inspect")
                            }),
                        }),
                        ui::MenuItem::new("Delete", |state: &mut DemoState| {
                            state.name = String::from("delete")
                        })
                        .danger(),
                    ],
                })
                .id(30),
            ])
        })
        .into_bridge();

    let surface = bridge.project_surface();
    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 2);

    let message = surface
        .dispatch_widget_output(
            focus_order[0],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("named-parts menu item should emit a state action");
    let command = bridge.update(message);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 31, "text").text,
        "Selected: inspect"
    );
}

#[test]
fn application_builder_menu_height_matches_compact_menu_layout() {
    use radiant::prelude as ui;

    assert_eq!(ui::message_menu_height(0), 44.0);
    assert_eq!(ui::message_menu_height(1), 72.0);
    assert_eq!(ui::message_menu_height(2), 104.0);
    assert_eq!(ui::message_menu_height(3), 136.0);
    assert_eq!(ui::menu_height(2), ui::message_menu_height(2));
}

#[test]
fn application_builder_drag_preview_paints_as_non_widget_overlay() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::stack([
        ui::text("Content").id(10).size(120.0, 24.0),
        ui::drag_preview_sized("kicks", Point::new(12.0, 18.0), Vector2::new(120.0, 24.0)).id(20),
    ])
    .into_surface();
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 96.0)),
    );
    let plan = surface.paint_plan(&output, &radiant::theme::ThemeTokens::default());

    assert!(surface.find_widget(20).is_none());
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        radiant::runtime::PaintPrimitive::Text(text) if text.widget_id == 20 && text.text == "kicks"
    )));
}

#[test]
fn application_builder_local_drop_marker_paints_at_local_offset() {
    use radiant::prelude::{self as ui, IntoView};

    let marker_color = ui::Rgba8::new(255, 160, 82, 230);
    let surface: UiSurface<()> = ui::stack([
        ui::text("Content").id(10).size(160.0, 24.0),
        ui::local_drop_marker(36.0, marker_color, 2.0, 18.0)
            .id(21)
            .fill_width()
            .height(24.0)
            .padding_y(3.0),
    ])
    .into_surface();
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 24.0)),
    );
    let plan = surface.paint_plan(&output, &radiant::theme::ThemeTokens::default());

    let marker = plan
        .fill_rects()
        .find(|fill| fill.color == marker_color)
        .expect("local drop marker should paint");
    assert!((marker.rect.min.x - 36.0).abs() < 0.01, "{:?}", marker.rect);
    assert!((marker.rect.min.y - 3.0).abs() < 0.01, "{:?}", marker.rect);
    assert!(surface.find_widget(21).is_none());
}
