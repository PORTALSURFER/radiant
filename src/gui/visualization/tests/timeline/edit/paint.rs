use super::*;
use crate::gui::{
    types::Rgba8,
    visualization::{TimelineEditHandle, TimelineEditPaintStyle, TimelineEditRegion},
};

fn fill_rects(primitives: &[crate::runtime::PaintPrimitive]) -> Vec<(u64, Rect, Rgba8)> {
    primitives
        .iter()
        .filter_map(|primitive| primitive.fill_rect())
        .map(|fill| (fill.widget_id, fill.rect, fill.color))
        .collect()
}

#[test]
fn timeline_edit_preview_pushes_standard_region_fills() {
    let mut primitives = Vec::new();
    let region_color = |region| match region {
        TimelineEditRegion::LeadingInner | TimelineEditRegion::TrailingInner => {
            Rgba8::new(20, 40, 60, 180)
        }
        TimelineEditRegion::LeadingOuter | TimelineEditRegion::TrailingOuter => {
            Rgba8::new(20, 40, 60, 96)
        }
    };

    preview().push_standard_region_fills(
        &mut primitives,
        7,
        mapper(),
        region_geometry(),
        region_color,
    );

    let fills = fill_rects(&primitives);
    assert_eq!(fills.len(), 4);
    assert_eq!(fills[0].0, 7);
    assert_rect_near(
        Some(fills[0].1),
        Rect::from_min_max(Point::new(40.0, 0.0), Point::new(60.0, 80.0)),
    );
    assert_eq!(fills[0].2, Rgba8::new(20, 40, 60, 180));
    assert_rect_near(
        Some(fills[2].1),
        Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 80.0)),
    );
    assert_eq!(fills[2].2, Rgba8::new(20, 40, 60, 96));
}

#[test]
fn timeline_edit_paint_style_derives_standard_colors() {
    let style = TimelineEditPaintStyle::new(Rgba8::new(20, 40, 60, 255))
        .region_alphas(120, 64)
        .handle_alpha(210)
        .curve_alpha(230);

    assert_eq!(
        style.region_color(TimelineEditRegion::LeadingInner),
        Rgba8::new(20, 40, 60, 120)
    );
    assert_eq!(
        style.region_color(TimelineEditRegion::TrailingOuter),
        Rgba8::new(20, 40, 60, 64)
    );
    assert_eq!(
        style.handle_color(TimelineEditHandle::LeadingEnd),
        Rgba8::new(20, 40, 60, 210)
    );
    assert_eq!(style.curve_color(), Rgba8::new(20, 40, 60, 230));
}

#[test]
fn timeline_edit_preview_pushes_standard_styled_fills() {
    let style = TimelineEditPaintStyle::new(Rgba8::new(90, 120, 240, 255))
        .region_alphas(180, 96)
        .handle_alpha(220);
    let mut primitives = Vec::new();

    preview().push_standard_styled_region_fills(
        &mut primitives,
        7,
        mapper(),
        region_geometry(),
        style,
    );
    preview().push_standard_styled_handle_fills(&mut primitives, 9, mapper(), geometry(), style);

    let fills = fill_rects(&primitives);
    assert_eq!(fills.len(), 10);
    assert_eq!(fills[0].2, Rgba8::new(90, 120, 240, 180));
    assert_eq!(fills[2].2, Rgba8::new(90, 120, 240, 96));
    assert_eq!(fills[4].0, 9);
    assert_eq!(fills[4].2, Rgba8::new(90, 120, 240, 220));
}

#[test]
fn timeline_edit_preview_pushes_standard_handle_fills() {
    let mut primitives = Vec::new();
    let handle_color = |handle| match handle {
        TimelineEditHandle::LeadingEnd | TimelineEditHandle::TrailingStart => {
            Rgba8::new(90, 120, 240, 220)
        }
        _ => Rgba8::new(90, 120, 240, 160),
    };

    preview().push_standard_handle_fills(&mut primitives, 9, mapper(), geometry(), handle_color);

    let fills = fill_rects(&primitives);
    assert_eq!(fills.len(), 6);
    assert_eq!(fills[0].0, 9);
    assert_rect_near(
        Some(fills[0].1),
        Rect::from_min_max(Point::new(55.0, 0.0), Point::new(65.0, 10.0)),
    );
    assert_eq!(fills[0].2, Rgba8::new(90, 120, 240, 220));
    assert_rect_near(
        Some(fills[4].1),
        Rect::from_min_max(Point::new(15.0, 35.0), Point::new(25.0, 45.0)),
    );
    assert_eq!(fills[4].2, Rgba8::new(90, 120, 240, 160));
}
