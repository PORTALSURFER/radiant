//! Checked usage of backend-neutral paint helpers.

use radiant::gui::{
    paint::{BorderSides, TextFieldPaint, border_fill_rects, text_field_paint},
    types::{Point, Rect, Rgba8},
};

fn main() {
    let color = Rgba8 {
        r: 98,
        g: 128,
        b: 255,
        a: 255,
    };
    let border = border_fill_rects(
        Rect::from_min_max(Point::new(8.0, 8.0), Point::new(168.0, 48.0)),
        color,
        2.0,
        BorderSides::ALL,
    );
    let field = text_field_paint(TextFieldPaint {
        field_rect: Rect::from_min_max(Point::new(8.0, 64.0), Point::new(240.0, 96.0)),
        text_rect: Rect::from_min_max(Point::new(18.0, 72.0), Point::new(230.0, 90.0)),
        text: "selected value".to_string(),
        caret_offset: 92.0,
        selection_offsets: Some((0.0, 58.0)),
        font_size: 13.0,
        fill_color: Rgba8 {
            r: 24,
            g: 26,
            b: 30,
            a: 255,
        },
        border_color: color,
        selection_color: Rgba8 {
            r: 98,
            g: 128,
            b: 255,
            a: 90,
        },
        caret_color: color,
        text_color: Rgba8 {
            r: 240,
            g: 244,
            b: 250,
            a: 255,
        },
        stroke_width: 2.0,
    });

    let primitive_count = border.len() + field.primitives.len();

    println!(
        "built {} primitives and text: {}",
        primitive_count,
        field
            .text_run
            .as_ref()
            .map_or("none", |run| run.text.as_str())
    );
}
