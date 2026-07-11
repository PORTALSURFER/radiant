//! Public API coverage for declarative surface node construction helpers.

use radiant::{
    gui::types::ImageRgba,
    layout::{
        Constraints, Point, Rect, SizeModeCross, SizeModeMain, SlotParams, Vector2, layout_tree,
    },
    runtime::{PaintPrimitive, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{ButtonWidget, WidgetSizing},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

#[test]
fn surface_node_row_column_and_fill_helpers_project_layout() {
    let header = radiant::widgets::TextWidget::new(
        20,
        "Header",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    );
    let primary = ButtonWidget::new(21, "Primary", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let secondary = ButtonWidget::new(
        22,
        "Secondary",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );

    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::column(
        2,
        6.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(header)),
            SurfaceChild::fill(SurfaceNode::row(
                3,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(
                        primary,
                        WidgetMessageMapper::button(|_| DemoMessage::Increment),
                    )),
                    SurfaceChild::fill(SurfaceNode::widget(
                        secondary,
                        WidgetMessageMapper::button(|_| DemoMessage::Increment),
                    )),
                ],
            )),
        ],
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 80.0)),
    );

    assert!(output.rects.contains_key(&2));
    assert!(output.rects.contains_key(&3));
    assert!(output.rects.contains_key(&20));
    assert!(output.rects.contains_key(&21));
    assert!(output.rects.contains_key(&22));
}

#[test]
fn surface_node_grid_helper_projects_tile_layout() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::grid(
        28,
        2,
        10.0,
        5.0,
        vec![
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(29, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(30, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(35, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
        ],
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    let first = output.rects.get(&29).expect("first tile");
    let second = output.rects.get(&30).expect("second tile");
    let third = output.rects.get(&35).expect("third tile");

    assert!(second.min.x > first.min.x);
    assert_eq!(first.min.y, second.min.y);
    assert!(third.min.y > first.min.y);
}

#[test]
fn surface_nodes_support_explicit_public_container_construction() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::container(
        40,
        radiant::layout::ContainerPolicy {
            kind: radiant::layout::ContainerKind::Row,
            spacing: 4.0,
            ..Default::default()
        },
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                41,
                "Alpha",
                WidgetSizing::fixed(Vector2::new(40.0, 20.0)),
            )),
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::text(42, "Beta", WidgetSizing::fixed(Vector2::new(40.0, 20.0))),
            ),
        ],
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 32.0)),
    );

    assert!(output.rects.contains_key(&40));
    assert!(output.rects.contains_key(&41));
    assert!(output.rects.contains_key(&42));
}

#[test]
fn surface_node_stack_and_card_helpers_project_grouped_surface() {
    let image = Arc::new(ImageRgba::new(1, 1, vec![0, 128, 255, 255]).unwrap());
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::stack(
        23,
        vec![
            SurfaceChild::fill(SurfaceNode::card(
                24,
                WidgetSizing::fixed(Vector2::new(180.0, 96.0)),
            )),
            SurfaceChild::fill(SurfaceNode::column(
                25,
                4.0,
                vec![SurfaceChild::fill(SurfaceNode::text(
                    26,
                    "Overview",
                    WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
                ))],
            )),
            SurfaceChild::fill(SurfaceNode::image(
                27,
                Arc::clone(&image),
                WidgetSizing::fixed(Vector2::new(16.0, 16.0)),
            )),
        ],
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 96.0)),
    );
    let theme = ThemeTokens::default();
    let plan = surface.paint_plan(&output, &theme);

    assert_eq!(output.rects.get(&24), output.rects.get(&25));
    assert!(surface.find_widget(24).is_some());
    assert_eq!(
        plan.primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill.widget_id),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![24]
    );
    assert_eq!(
        plan.primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Image(draw) => Some((draw.widget_id, draw.image.width())),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(27, 1)]
    );
}
