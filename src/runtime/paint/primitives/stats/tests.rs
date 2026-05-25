use super::*;
use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::{
        PaintFillRect, PaintFillRectBatch, PaintImage, PaintStrokeRectBatch, PaintText,
        PaintTextAlign, PaintTextRun,
    },
    theme::ThemeTokens,
    widgets::TextWrap,
};
use std::sync::Arc;

#[test]
fn surface_paint_plan_stats_count_core_primitive_groups() {
    let theme = ThemeTokens::default();
    let image = crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
        .expect("valid test image");
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 1,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(4.0, 4.0)),
            color: theme.accent_mint,
        }));
    plan.primitives
        .push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id: 4,
            rects: Arc::from(vec![
                Rect::from_min_size(Point::new(8.0, 0.0), Vector2::new(4.0, 4.0)),
                Rect::from_min_size(Point::new(16.0, 0.0), Vector2::new(4.0, 4.0)),
            ]),
            color: theme.accent_mint,
        }));
    plan.primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id: 2,
        text: PaintText::from("ready"),
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 12.0)),
        font_size: 12.0,
        baseline: None,
        color: theme.text_primary,
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
    }));
    plan.primitives
        .push(PaintPrimitive::StrokeRectBatch(PaintStrokeRectBatch {
            widget_id: 5,
            rects: Arc::from(vec![Rect::from_min_size(
                Point::new(8.0, 8.0),
                Vector2::new(4.0, 4.0),
            )]),
            color: theme.accent_mint,
            width: 1.0,
        }));
    plan.primitives.push(PaintPrimitive::Image(PaintImage {
        widget_id: 3,
        source_rect: None,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 8.0)),
        image: Arc::new(image),
    }));

    let stats = plan.stats();

    assert_eq!(stats.total, 5);
    assert_eq!(stats.fills, 2);
    assert_eq!(stats.strokes, 1);
    assert_eq!(stats.text, 1);
    assert_eq!(stats.images, 1);
}
