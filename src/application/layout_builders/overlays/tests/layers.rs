use super::super::{LayerHorizontalAnchor, LayerVerticalAnchor, anchored_layer, centered_layer};
use crate::{
    application::{IntoView, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, UiSurface},
};

#[test]
fn anchored_layer_places_child_at_configured_edges() {
    let frame = UiSurface::new(
        anchored_layer::<()>(
            text("details").id(90).size(80.0, 20.0),
            Vector2::new(80.0, 20.0),
            LayerHorizontalAnchor::End,
            LayerVerticalAnchor::End,
            12.0,
            8.0,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == 90 => Some(text.rect),
            _ => None,
        })
        .expect("anchored layer child should paint");

    assert!((text_rect.min.x - 108.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 72.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn centered_layer_uses_anchored_layer_center_policy() {
    let frame = UiSurface::new(
        centered_layer::<()>(text("center").id(91), Vector2::new(80.0, 20.0)).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == 91 => Some(text.rect),
            _ => None,
        })
        .expect("centered layer child should paint");

    assert!((text_rect.min.x - 60.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 40.0).abs() < 0.01, "{text_rect:?}");
}
