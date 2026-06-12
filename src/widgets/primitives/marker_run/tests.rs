use crate::{
    gui::types::{Point, Rect, Rgba8},
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::Widget,
};

use super::{
    geometry::{for_each_marker_rect, marker_geometry},
    model::{MarkerRunAlign, MarkerRunProps},
    widget::{ColorMarkerRunWidget, MarkerRunWidget},
};

const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

fn bounds() -> Rect {
    Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0))
}

fn marker_rects(bounds: Rect, props: MarkerRunProps) -> Vec<Rect> {
    let mut rects = Vec::new();
    for_each_marker_rect(
        bounds,
        props.count as usize,
        marker_geometry(props.side, props.gap, props.inset, props.align),
        |_, rect| rects.push(rect),
    );
    rects
}

#[test]
fn right_aligned_marker_run_respects_gap_and_inset() {
    let rects = marker_rects(
        bounds(),
        MarkerRunProps {
            count: 3,
            side: 5,
            gap: 4,
            inset: 4,
            align: MarkerRunAlign::Right,
            color: Some(WHITE),
        },
    );

    assert_eq!(
        rects,
        vec![
            Rect::from_min_max(Point::new(83.0, 27.5), Point::new(88.0, 32.5)),
            Rect::from_min_max(Point::new(92.0, 27.5), Point::new(97.0, 32.5)),
            Rect::from_min_max(Point::new(101.0, 27.5), Point::new(106.0, 32.5)),
        ]
    );
}

#[test]
fn empty_or_transparent_marker_runs_paint_no_rects() {
    assert!(marker_rects(bounds(), MarkerRunProps::new(Some(WHITE), 0)).is_empty());

    let widget = MarkerRunWidget::new(None, 3);
    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        bounds(),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(primitives.is_empty());
}

#[test]
fn same_color_marker_run_batches_multiple_rects() {
    let widget = MarkerRunWidget::new(Some(WHITE), 3)
        .with_side(5)
        .with_gap(4)
        .with_inset(4);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds(),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let [PaintPrimitive::FillRectBatch(batch)] = primitives.as_slice() else {
        panic!("expected one batched fill primitive");
    };
    assert_eq!(batch.widget_id, widget.common.id);
    assert_eq!(batch.color, WHITE);
    assert_eq!(batch.rects.len(), 3);
}

#[test]
fn single_marker_run_stays_as_single_fill_rect() {
    let widget = MarkerRunWidget::new(Some(WHITE), 1)
        .with_side(5)
        .with_gap(4)
        .with_inset(4);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds(),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let [PaintPrimitive::FillRect(fill)] = primitives.as_slice() else {
        panic!("expected one fill primitive");
    };
    assert_eq!(fill.widget_id, widget.common.id);
    assert_eq!(fill.color, WHITE);
}

#[test]
fn color_marker_run_can_paint_one_marker_per_color() {
    const RED: Rgba8 = Rgba8 {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    const BLUE: Rgba8 = Rgba8 {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    let widget = ColorMarkerRunWidget::new(vec![RED, BLUE])
        .with_side(5)
        .with_gap(4)
        .with_inset(4);
    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        bounds(),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let colors = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, vec![RED, BLUE]);
}
