use super::*;
use crate::gui::{
    paint::{
        DrawImage, FillCircle, FillLinearGradient, FillRect, PaintFrame, Primitive, TextAlign,
        TextRun,
    },
    types::{ImageRgba, Point, Rect, Rgba8, Vector2},
};
use std::sync::Arc;

#[test]
fn visual_snapshot_from_paint_frame_serializes_primitives_and_text() {
    let image = Arc::new(ImageRgba::new(1, 1, vec![1, 2, 3, 4]).unwrap());
    let frame = PaintFrame {
        clear_color: Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        },
        primitives: vec![
            Primitive::Rect(FillRect {
                rect: Rect::from_min_size(Point::new(1.1114, 2.2225), Vector2::new(10.0, 20.0)),
                color: Rgba8 {
                    r: 4,
                    g: 5,
                    b: 6,
                    a: 255,
                },
            }),
            Primitive::Circle(FillCircle {
                center: Point::new(3.0, 4.0),
                radius: 5.5555,
                color: Rgba8 {
                    r: 7,
                    g: 8,
                    b: 9,
                    a: 255,
                },
            }),
            Primitive::LinearGradient(FillLinearGradient {
                rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 9.0)),
                start: Point::new(0.0, 0.0),
                end: Point::new(8.0, 9.0),
                start_color: Rgba8 {
                    r: 10,
                    g: 11,
                    b: 12,
                    a: 255,
                },
                end_color: Rgba8 {
                    r: 13,
                    g: 14,
                    b: 15,
                    a: 255,
                },
            }),
            Primitive::Image(DrawImage {
                rect: Rect::from_min_size(Point::new(6.0, 7.0), Vector2::new(1.0, 1.0)),
                image,
            }),
        ],
        text_runs: vec![TextRun {
            text: String::from("Frame"),
            position: Point::new(9.0, 10.0),
            font_size: 12.3456,
            color: Rgba8 {
                r: 16,
                g: 17,
                b: 18,
                a: 255,
            },
            max_width: Some(101.0104),
            align: TextAlign::Center,
        }],
    };

    let snapshot = visual_snapshot_from_paint_frame("fixture", [640.4, 480.6], &frame);

    assert_eq!(snapshot.name, "fixture");
    assert_eq!(snapshot.viewport_width, 640);
    assert_eq!(snapshot.viewport_height, 481);
    assert_eq!(snapshot.primitive_count, 4);
    assert_eq!(snapshot.text_run_count, 1);
    assert_eq!(snapshot.primitives.len(), 4);
    assert_eq!(snapshot.text_runs[0].font_size, 12.346);
    assert_eq!(snapshot.text_runs[0].max_width, Some(101.01));
}
