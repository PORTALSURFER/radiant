use super::super::*;
use radiant::application as app;

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

#[test]
fn application_builder_centered_layer_centers_fixed_size_child() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::centered_layer(
        ui::text("Dialog").key("centered-dialog").id(2),
        Vector2::new(120.0, 80.0),
    )
    .id(1)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 300.0)),
    );
    let child = layout.rects[&2];

    assert_eq!(child.min.x, 140.0);
    assert_eq!(child.min.y, 110.0);
    assert_eq!(child.width(), 120.0);
    assert_eq!(child.height(), 80.0);
}

#[test]
fn centered_layer_parts_support_named_construction() {
    use radiant::prelude as ui;

    let parts: app::CenteredLayerParts<()> =
        app::CenteredLayerParts::new(ui::text("Dialog"), Vector2::new(320.0, 180.0));

    assert_eq!(parts.size, Vector2::new(320.0, 180.0));
}

#[test]
fn floating_layer_anchor_helpers_position_content_around_trigger() {
    use radiant::{prelude as ui, prelude::IntoView, runtime::PaintPrimitive};

    let frame = UiSurface::new(
        ui::stack([
            ui::text("").size(240.0, 140.0),
            ui::floating_layer_above::<()>(
                18.0,
                80.0,
                6.0,
                Vector2::new(90.0, 24.0),
                ui::text("Above").id(71),
            ),
            ui::floating_layer_below::<()>(
                18.0,
                80.0,
                20.0,
                6.0,
                Vector2::new(90.0, 24.0),
                ui::text("Below").id(72),
            ),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0)),
        &Default::default(),
    );

    let text_rect = |widget_id| {
        frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == widget_id => Some(text.rect),
                _ => None,
            })
            .expect("anchored floating-layer text should paint")
    };

    assert_eq!(text_rect(71).min, Point::new(18.0, 50.0));
    assert_eq!(text_rect(72).min, Point::new(18.0, 106.0));
}

#[test]
fn floating_layer_anchor_parts_support_named_interactive_construction() {
    use radiant::prelude as ui;

    let parts: app::FloatingLayerAnchorParts<()> = app::FloatingLayerAnchorParts::new(
        ui::text("Popup"),
        Vector2::new(160.0, 80.0),
        12.0,
        42.0,
        20.0,
        4.0,
        ui::FloatingLayerPlacement::Below,
    )
    .interactive(true);

    assert_eq!(parts.x, 12.0);
    assert_eq!(parts.trigger_y, 42.0);
    assert_eq!(parts.trigger_height, 20.0);
    assert_eq!(parts.gap, 4.0);
    assert_eq!(parts.size, Vector2::new(160.0, 80.0));
    assert_eq!(parts.placement, ui::FloatingLayerPlacement::Below);
    assert!(parts.interactive);
}
