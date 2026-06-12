use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::{
        PaintClipStart, PaintFillRect, PaintFillRectBatch, PaintPrimitive, PaintStrokePolyline,
        PaintSvg, PaintSvgDocument, SurfacePaintPlan,
    },
    theme::ThemeTokens,
};
use std::sync::Arc;

#[test]
fn shape_and_svg_queries_return_matching_primitives_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    let first_fill = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(10.0, 11.0));
    let stroke = Rect::from_min_size(Point::new(3.0, 4.0), Vector2::new(12.0, 13.0));
    let second_fill = Rect::from_min_size(Point::new(5.0, 6.0), Vector2::new(14.0, 15.0));
    let svg_rect = Rect::from_min_size(Point::new(7.0, 8.0), Vector2::new(16.0, 17.0));
    let svg = PaintSvgDocument::from_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><path d="M0 0h1v1H0z"/></svg>"#,
    )
    .expect("valid test svg");

    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 21,
            rect: first_fill,
            color: theme.accent_mint,
        }));
    plan.primitives.push(PaintPrimitive::StrokeRect(
        crate::runtime::PaintStrokeRect {
            widget_id: 22,
            rect: stroke,
            color: theme.text_primary,
            width: 1.0,
        },
    ));
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 23,
            rect: second_fill,
            color: theme.accent_danger,
        }));
    plan.primitives.push(PaintPrimitive::Svg(PaintSvg {
        widget_id: 24,
        document: svg,
        rect: svg_rect,
    }));

    assert_eq!(
        plan.fill_rects()
            .map(|fill| fill.widget_id)
            .collect::<Vec<_>>(),
        vec![21, 23]
    );
    assert_eq!(
        plan.fill_rects_for_widget(21)
            .map(|fill| fill.rect)
            .collect::<Vec<_>>(),
        vec![first_fill]
    );
    assert_eq!(
        plan.visible_fill_rects_for_widget(21)
            .map(|fill| fill.rect)
            .collect::<Vec<_>>(),
        vec![first_fill]
    );
    assert!(plan.contains_visible_fill_rect_for_widget(21));
    assert!(!plan.contains_visible_fill_rect_for_widget(404));
    assert_eq!(
        plan.stroke_rects()
            .map(|stroke| stroke.widget_id)
            .collect::<Vec<_>>(),
        vec![22]
    );
    assert_eq!(
        plan.stroke_rects_for_widget(22)
            .map(|stroke| stroke.rect)
            .collect::<Vec<_>>(),
        vec![stroke]
    );
    assert_eq!(
        plan.svgs().map(|svg| svg.widget_id).collect::<Vec<_>>(),
        vec![24]
    );
    assert_eq!(
        plan.svgs_for_widget(24)
            .map(|svg| svg.rect)
            .collect::<Vec<_>>(),
        vec![svg_rect]
    );
    assert_eq!(plan.first_svg_rect_for_widget(24), Some(svg_rect));
    assert_eq!(plan.first_svg_rect_for_widget(404), None);
    assert_eq!(
        plan.primitives[0].fill_rect().map(|fill| fill.rect),
        Some(first_fill)
    );
    assert_eq!(
        plan.primitives[1].stroke_rect().map(|stroke| stroke.rect),
        Some(stroke)
    );
    assert_eq!(plan.primitives[3].svg().map(|svg| svg.rect), Some(svg_rect));
}

#[test]
fn visible_fill_queries_ignore_transparent_or_empty_fills() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 91,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            color: theme.accent_mint.with_alpha(0),
        }));
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 91,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 10.0)),
            color: theme.accent_mint,
        }));
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 91,
            rect: Rect::from_min_size(Point::new(f32::NAN, 0.0), Vector2::new(10.0, 10.0)),
            color: theme.accent_mint,
        }));
    plan.primitives
        .push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 91,
            rect: Rect::from_min_size(Point::new(1.0, 1.0), Vector2::new(3.0, 4.0)),
            color: theme.accent_mint,
        }));

    assert_eq!(plan.fill_rects_for_widget(91).count(), 4);
    assert_eq!(
        plan.visible_fill_rects_for_widget(91)
            .map(|fill| fill.rect)
            .collect::<Vec<_>>(),
        vec![Rect::from_min_size(
            Point::new(1.0, 1.0),
            Vector2::new(3.0, 4.0)
        )]
    );
    assert!(plan.contains_visible_fill_rect_for_widget(91));
    assert!(!plan.contains_visible_fill_rect_for_widget(404));
}

#[test]
fn visible_fill_presence_includes_batched_rectangles() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives
        .push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id: 91,
            rects: Arc::from([Rect::from_min_size(
                Point::new(1.0, 1.0),
                Vector2::new(3.0, 4.0),
            )]),
            color: theme.accent_mint.with_alpha(0),
        }));
    plan.primitives
        .push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id: 91,
            rects: Arc::from([Rect::from_min_size(
                Point::new(1.0, 1.0),
                Vector2::new(0.0, 4.0),
            )]),
            color: theme.accent_mint,
        }));

    assert!(!plan.contains_visible_fill_rect_for_widget(91));

    plan.primitives
        .push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id: 91,
            rects: Arc::from([Rect::from_min_size(
                Point::new(2.0, 3.0),
                Vector2::new(5.0, 6.0),
            )]),
            color: theme.accent_mint,
        }));

    assert!(plan.contains_visible_fill_rect_for_widget(91));
    assert!(!plan.contains_visible_fill_rect_for_widget(404));
}

#[test]
fn clip_polygon_and_polyline_queries_return_matching_primitives_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    let clip_rect = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(10.0, 11.0));
    let polygon_points = Arc::from([
        Point::new(0.0, 0.0),
        Point::new(8.0, 0.0),
        Point::new(4.0, 8.0),
    ]);
    let polyline_points = Arc::from([Point::new(0.0, 2.0), Point::new(8.0, 2.0)]);

    plan.primitives
        .push(PaintPrimitive::ClipStart(PaintClipStart {
            node_id: 7,
            rect: clip_rect,
        }));
    plan.primitives.push(PaintPrimitive::FillPolygon(
        crate::runtime::PaintFillPolygon {
            widget_id: 31,
            points: polygon_points,
            color: theme.accent_mint,
        },
    ));
    plan.primitives
        .push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: 32,
            points: polyline_points,
            color: theme.text_primary,
            width: 1.0,
        }));

    assert_eq!(
        plan.clip_starts().map(|clip| clip.rect).collect::<Vec<_>>(),
        vec![clip_rect]
    );
    assert_eq!(
        plan.fill_polygons()
            .map(|fill| fill.widget_id)
            .collect::<Vec<_>>(),
        vec![31]
    );
    assert_eq!(
        plan.fill_polygons_for_widget(31)
            .map(|fill| fill.widget_id)
            .collect::<Vec<_>>(),
        vec![31]
    );
    assert_eq!(
        plan.visible_fill_polygons_for_widget(31)
            .map(|fill| fill.widget_id)
            .collect::<Vec<_>>(),
        vec![31]
    );
    assert!(plan.contains_visible_fill_polygon_for_widget(31));
    assert!(!plan.contains_visible_fill_polygon_for_widget(404));
    assert_eq!(
        plan.stroke_polylines()
            .map(|stroke| stroke.widget_id)
            .collect::<Vec<_>>(),
        vec![32]
    );
    assert_eq!(
        plan.primitives[0].clip_start().map(|clip| clip.rect),
        Some(clip_rect)
    );
    assert_eq!(
        plan.primitives[1].fill_polygon().map(|fill| fill.widget_id),
        Some(31)
    );
    assert_eq!(
        plan.primitives[2]
            .stroke_polyline()
            .map(|stroke| stroke.widget_id),
        Some(32)
    );
}
