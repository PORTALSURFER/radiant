use super::super::local_drop_marker;
use crate::{
    application::{IntoView, stack, text},
    gui::types::{Point, Rect, Rgba8},
    layout::Vector2,
    runtime::UiSurface,
};

#[test]
fn local_drop_marker_paints_at_local_x() {
    let marker_color = Rgba8::new(255, 160, 82, 230);
    let frame = UiSurface::new(
        stack([
            text("").size(200.0, 24.0),
            local_drop_marker::<()>(42.0, marker_color, 2.0, 20.0)
                .fill_width()
                .height(24.0)
                .padding_y(2.0),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 24.0)),
        &Default::default(),
    );

    let marker = frame
        .paint_plan
        .fill_rects()
        .find(|fill| fill.color == marker_color)
        .expect("local drop marker should paint");

    assert!((marker.rect.min.x - 42.0).abs() < 0.01, "{:?}", marker.rect);
    assert!((marker.rect.min.y - 2.0).abs() < 0.01, "{:?}", marker.rect);
    assert!(
        (marker.rect.width() - 2.0).abs() < 0.01,
        "{:?}",
        marker.rect
    );
    assert!(
        (marker.rect.height() - 20.0).abs() < 0.01,
        "{:?}",
        marker.rect
    );
}

#[test]
fn local_drop_marker_clamps_to_visible_bounds() {
    let marker_color = Rgba8::new(255, 160, 82, 230);
    let frame = UiSurface::new(
        stack([
            text("").size(96.0, 24.0),
            local_drop_marker::<()>(220.0, marker_color, 2.0, 20.0)
                .fill_width()
                .height(24.0)
                .padding_y(2.0),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 24.0)),
        &Default::default(),
    );

    let marker = frame
        .paint_plan
        .fill_rects()
        .find(|fill| fill.color == marker_color)
        .expect("constrained local drop marker should stay visible");

    assert!((marker.rect.min.x - 94.0).abs() < 0.01, "{:?}", marker.rect);
    assert!(
        (marker.rect.width() - 2.0).abs() < 0.01,
        "{:?}",
        marker.rect
    );
}
