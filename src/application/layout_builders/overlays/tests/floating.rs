use super::super::{floating_layer_above, floating_layer_below};
use crate::{
    application::{IntoView, stack, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, UiSurface},
};

#[test]
fn floating_layer_above_positions_child_before_trigger_gap() {
    let frame = UiSurface::new(
        stack([
            text("").size(200.0, 100.0),
            floating_layer_above::<()>(
                12.0,
                60.0,
                4.0,
                Vector2::new(80.0, 20.0),
                text("popup").id(92),
            ),
        ])
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
            PaintPrimitive::Text(text) if text.widget_id == 92 => Some(text.rect),
            _ => None,
        })
        .expect("floating layer child should paint");

    assert!((text_rect.min.x - 12.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 36.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn floating_layer_below_positions_child_after_trigger_gap() {
    let frame = UiSurface::new(
        stack([
            text("").size(200.0, 100.0),
            floating_layer_below::<()>(
                12.0,
                20.0,
                18.0,
                4.0,
                Vector2::new(80.0, 20.0),
                text("popup").id(93),
            ),
        ])
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
            PaintPrimitive::Text(text) if text.widget_id == 93 => Some(text.rect),
            _ => None,
        })
        .expect("floating layer child should paint");

    assert!((text_rect.min.x - 12.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 42.0).abs() < 0.01, "{text_rect:?}");
}
