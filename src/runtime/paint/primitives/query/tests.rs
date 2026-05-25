use super::*;
use crate::{
    gui::types::{Point, Vector2},
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintFillRectBatch,
        PaintGpuSurface, PaintStrokeRectBatch,
    },
    theme::ThemeTokens,
};
use std::sync::Arc;

#[test]
fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 7,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 9.0)),
            color: theme.accent_mint,
        }));
    plan.primitives.push(PaintPrimitive::StrokeRect(
        crate::runtime::PaintStrokeRect {
            widget_id: 7,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 11.0)),
            color: theme.accent_mint,
            width: 1.0,
        },
    ));

    assert_eq!(
        plan.first_widget_rect(7),
        Some(Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(8.0, 9.0)
        ))
    );
    assert_eq!(plan.first_widget_rect(404), None);
}

#[test]
fn paint_primitive_reports_widget_id_and_rect_for_anchor_primitives() {
    let atlas = crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
        .expect("valid test atlas");
    let primitive = PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 42,
        key: 1,
        revision: 0,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(64.0, 32.0)),
        content: GpuSurfaceContent::RgbaAtlas {
            atlas: std::sync::Arc::new(atlas),
            source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    });

    assert_eq!(primitive.widget_id(), Some(42));
    assert_eq!(
        primitive.rect(),
        Some(Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(64.0, 32.0)
        ))
    );
}

#[test]
fn batched_fill_rect_reports_widget_id_and_first_rect_anchor() {
    let first = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(8.0, 9.0));
    let second = Rect::from_min_size(Point::new(16.0, 3.0), Vector2::new(4.0, 5.0));
    let primitive = PaintPrimitive::FillRectBatch(PaintFillRectBatch {
        widget_id: 77,
        rects: Arc::from(vec![first, second]),
        color: ThemeTokens::default().accent_mint,
    });

    assert_eq!(primitive.widget_id(), Some(77));
    assert_eq!(primitive.rect(), Some(first));
}

#[test]
fn batched_stroke_rect_reports_widget_id_and_first_rect_anchor() {
    let first = Rect::from_min_size(Point::new(2.0, 3.0), Vector2::new(8.0, 9.0));
    let second = Rect::from_min_size(Point::new(18.0, 4.0), Vector2::new(4.0, 5.0));
    let primitive = PaintPrimitive::StrokeRectBatch(PaintStrokeRectBatch {
        widget_id: 78,
        rects: Arc::from(vec![first, second]),
        color: ThemeTokens::default().accent_mint,
        width: 1.0,
    });

    assert_eq!(primitive.widget_id(), Some(78));
    assert_eq!(primitive.rect(), Some(first));
}
