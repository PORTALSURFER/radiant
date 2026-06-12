use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::{
        PaintClipEnd, PaintClipStart, PaintFillRect, PaintFillRectBatch, PaintPrimitive,
        PaintStrokeRectBatch, SurfacePaintPlan,
    },
    theme::ThemeTokens,
};
use std::sync::Arc;

#[test]
fn plan_level_paint_queries_ignore_clip_bookkeeping() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    let clip_rect = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(10.0, 11.0));

    plan.primitives
        .push(PaintPrimitive::ClipStart(PaintClipStart {
            node_id: 7,
            rect: clip_rect,
        }));
    plan.primitives
        .push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 7 }));

    assert!(!plan.contains_paint_primitives());
    assert_eq!(plan.paint_primitives().count(), 0);
    assert!(
        plan.primitives
            .iter()
            .all(|primitive| !primitive.is_paint())
    );
    assert_eq!(plan.rects().collect::<Vec<_>>(), vec![clip_rect]);
    assert!(plan.contains_rect_matching(|rect| rect == clip_rect));
    assert!(!plan.contains_rect_matching(|rect| rect.width() > 20.0));
    assert_eq!(plan.paint_rects().collect::<Vec<_>>(), Vec::<Rect>::new());
    assert!(!plan.contains_paint_rect_matching(|rect| rect == clip_rect));

    let fill_rect = Rect::from_min_size(Point::new(3.0, 4.0), Vector2::new(12.0, 13.0));
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 21,
            rect: fill_rect,
            color: theme.accent_mint,
        }));

    assert!(plan.contains_paint_primitives());
    assert_eq!(plan.paint_primitives().count(), 1);
    assert!(plan.primitives[2].is_paint());
    assert!(plan.contains_rect_matching(|rect| rect == fill_rect));
    assert_eq!(plan.paint_rects().collect::<Vec<_>>(), vec![fill_rect]);
    assert!(plan.contains_paint_rect_matching(|rect| rect == fill_rect));
}

#[test]
fn plan_level_rect_queries_flatten_batched_rectangles() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    let clip_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 30.0));
    let fill_first = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(3.0, 4.0));
    let fill_second = Rect::from_min_size(Point::new(5.0, 6.0), Vector2::new(7.0, 8.0));
    let stroke_first = Rect::from_min_size(Point::new(9.0, 10.0), Vector2::new(11.0, 12.0));
    let stroke_second = Rect::from_min_size(Point::new(13.0, 14.0), Vector2::new(15.0, 16.0));

    plan.primitives
        .push(PaintPrimitive::ClipStart(PaintClipStart {
            node_id: 7,
            rect: clip_rect,
        }));
    plan.primitives
        .push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id: 21,
            rects: Arc::from([fill_first, fill_second]),
            color: theme.accent_mint,
        }));
    plan.primitives
        .push(PaintPrimitive::StrokeRectBatch(PaintStrokeRectBatch {
            widget_id: 22,
            rects: Arc::from([stroke_first, stroke_second]),
            color: theme.text_primary,
            width: 1.0,
        }));

    assert_eq!(
        plan.rects().collect::<Vec<_>>(),
        vec![
            clip_rect,
            fill_first,
            fill_second,
            stroke_first,
            stroke_second
        ]
    );
    assert_eq!(
        plan.paint_rects().collect::<Vec<_>>(),
        vec![fill_first, fill_second, stroke_first, stroke_second]
    );
    assert!(plan.contains_rect_matching(|rect| rect == clip_rect));
    assert!(plan.contains_paint_rect_matching(|rect| rect == fill_second));
    assert!(plan.contains_paint_rect_matching(|rect| rect == stroke_second));
    assert!(!plan.contains_paint_rect_matching(|rect| rect == clip_rect));
}
