use super::*;
use crate::{
    gui::types::{Point, Vector2},
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintFillRectBatch,
        PaintGpuSurface, PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintTextAlign,
        PaintTextRun,
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
fn text_queries_return_runs_and_inputs_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id: 11,
        text: "Status".into(),
        rect: Rect::from_min_size(Point::new(4.0, 5.0), Vector2::new(80.0, 16.0)),
        font_size: 12.0,
        baseline: None,
        color: theme.text_primary,
        align: PaintTextAlign::Left,
        wrap: crate::widgets::TextWrap::None,
    }));
    plan.primitives
        .push(PaintPrimitive::TextInput(crate::runtime::PaintTextInput {
            widget_id: 12,
            rect: Rect::from_min_size(Point::new(8.0, 24.0), Vector2::new(96.0, 18.0)),
            placeholder: Some("Search".into()),
            completion_suffix: None,
            state: crate::widgets::TextInputState::from_value(String::from("ki")),
            font_size: 12.0,
            baseline: None,
            color: theme.text_primary,
            placeholder_color: theme.text_muted,
            completion_color: theme.text_muted,
            selection_color: theme.accent_mint,
            caret_color: theme.text_primary,
            focused: true,
        }));

    let run = plan.first_text_run("Status").expect("status text run");
    assert_eq!(run.widget_id, 11);
    assert_eq!(run.rect.min, Point::new(4.0, 5.0));
    assert!(plan.contains_text("Status"));
    assert!(!plan.contains_text("Missing"));
    assert_eq!(
        plan.text_runs()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Status"]
    );
    assert_eq!(
        plan.text_inputs()
            .map(|input| input.widget_id)
            .collect::<Vec<_>>(),
        vec![12]
    );
    assert_eq!(
        plan.primitives[0].text_run().map(|run| run.widget_id),
        Some(11)
    );
    assert_eq!(
        plan.primitives[1].text_input().map(|input| input.widget_id),
        Some(12)
    );
}

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
        plan.stroke_rects()
            .map(|stroke| stroke.widget_id)
            .collect::<Vec<_>>(),
        vec![22]
    );
    assert_eq!(
        plan.svgs().map(|svg| svg.widget_id).collect::<Vec<_>>(),
        vec![24]
    );
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
