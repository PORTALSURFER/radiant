use super::*;
use crate::gui::types::{Point, Rect, Rgba8};

fn gradient_with_end_alpha(end_alpha: u8) -> Primitive {
    Primitive::LinearGradient(FillLinearGradient {
        rect: Rect::from_min_max(Point::new(1.0, 2.0), Point::new(11.0, 22.0)),
        start: Point::new(1.0, 2.0),
        end: Point::new(11.0, 2.0),
        start_color: Rgba8 {
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        },
        end_color: Rgba8 {
            r: 50,
            g: 60,
            b: 70,
            a: end_alpha,
        },
    })
}

#[test]
fn linear_gradient_primitive_has_stable_equality() {
    let first = gradient_with_end_alpha(80);
    let same = gradient_with_end_alpha(80);
    let changed = gradient_with_end_alpha(81);

    assert_eq!(first, same);
    assert_ne!(first, changed);
}

#[test]
fn paint_frame_equality_includes_gradient_primitives() {
    let first = PaintFrame {
        primitives: vec![gradient_with_end_alpha(80)],
        ..PaintFrame::default()
    };
    let same = PaintFrame {
        primitives: vec![gradient_with_end_alpha(80)],
        ..PaintFrame::default()
    };
    let changed = PaintFrame {
        primitives: vec![gradient_with_end_alpha(81)],
        ..PaintFrame::default()
    };

    assert_eq!(first, same);
    assert_ne!(first, changed);
}

#[test]
fn border_fill_rects_returns_requested_edges() {
    let color = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    };
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 80.0));

    let fills = border_fill_rects(
        rect,
        color,
        2.0,
        BorderSides {
            top: true,
            bottom: false,
            left: false,
            right: true,
        },
    );

    assert_eq!(fills.len(), 2);
    assert_eq!(
        fills[0].rect,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 22.0))
    );
    assert_eq!(
        fills[1].rect,
        Rect::from_min_max(Point::new(48.0, 20.0), Point::new(50.0, 80.0))
    );
    assert!(fills.iter().all(|fill| fill.color == color));
}

#[test]
fn border_fill_rects_omits_degenerate_rectangles() {
    let color = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    };
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(3.0, 12.0));

    assert!(border_fill_rects(rect, color, 2.0, BorderSides::ALL).is_empty());
}

#[test]
fn text_field_paint_emits_chrome_selection_text_and_caret() {
    let color = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    };
    let output = text_field_paint(TextFieldPaint {
        field_rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 24.0)),
        text_rect: Rect::from_min_max(Point::new(8.0, 4.0), Point::new(112.0, 20.0)),
        text: "query".to_string(),
        caret_offset: 36.0,
        selection_offsets: Some((8.0, 24.0)),
        font_size: 12.0,
        fill_color: color,
        border_color: color,
        selection_color: color,
        caret_color: color,
        text_color: color,
        stroke_width: 2.0,
    });

    assert_eq!(output.primitives.len(), 7);
    assert_eq!(
        output.text_run.as_ref().map(|run| run.text.as_str()),
        Some("query")
    );
}
