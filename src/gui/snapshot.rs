//! Serializable visual snapshot primitives for deterministic GUI fixtures.

use crate::gui::{
    paint::{PaintFrame, Primitive, TextAlign},
    types::{Point, Rect, Rgba8},
};
use serde::Serialize;

/// Serializable color captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

/// Serializable point captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotPoint {
    /// X coordinate in logical window space.
    pub x: f32,
    /// Y coordinate in logical window space.
    pub y: f32,
}

/// Serializable rectangle captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRect {
    /// Minimum X coordinate in logical window space.
    pub x: f32,
    /// Minimum Y coordinate in logical window space.
    pub y: f32,
    /// Width in logical points.
    pub width: f32,
    /// Height in logical points.
    pub height: f32,
}

/// Serializable primitive captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SnapshotPrimitive {
    /// Filled rectangle primitive.
    Rect {
        /// Primitive bounds.
        rect: SnapshotRect,
        /// Fill color.
        color: SnapshotColor,
    },
    /// Filled circle primitive.
    Circle {
        /// Circle center.
        center: SnapshotPoint,
        /// Circle radius.
        radius: f32,
        /// Fill color.
        color: SnapshotColor,
    },
    /// Filled linear-gradient primitive.
    LinearGradient {
        /// Primitive bounds.
        rect: SnapshotRect,
        /// Gradient start point.
        start: SnapshotPoint,
        /// Gradient end point.
        end: SnapshotPoint,
        /// Gradient start color.
        start_color: SnapshotColor,
        /// Gradient end color.
        end_color: SnapshotColor,
    },
    /// RGBA image primitive.
    Image {
        /// Image placement bounds.
        rect: SnapshotRect,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Image RGBA pixels.
        pixels: Vec<u8>,
    },
}

/// Text alignment captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotTextAlign {
    /// Left-aligned text.
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
}

/// Serializable text run captured from one rendered frame.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotTextRun {
    /// Text content.
    pub text: String,
    /// Text anchor position.
    pub position: SnapshotPoint,
    /// Font size in logical points.
    pub font_size: f32,
    /// Text color.
    pub color: SnapshotColor,
    /// Optional max width for text layout.
    pub max_width: Option<f32>,
    /// Text alignment.
    pub align: SnapshotTextAlign,
}

/// Deterministic snapshot of one rendered GUI frame.
#[derive(Debug, Clone, Serialize)]
pub struct VisualSnapshot {
    /// Fixture name.
    pub name: String,
    /// Viewport width in logical pixels.
    pub viewport_width: u32,
    /// Viewport height in logical pixels.
    pub viewport_height: u32,
    /// Frame clear color.
    pub clear_color: SnapshotColor,
    /// Number of captured primitives.
    pub primitive_count: usize,
    /// Number of captured text runs.
    pub text_run_count: usize,
    /// Captured paint primitives.
    pub primitives: Vec<SnapshotPrimitive>,
    /// Captured text runs.
    pub text_runs: Vec<SnapshotTextRun>,
}

/// Convert a backend-neutral paint frame into a deterministic visual snapshot.
///
/// This helper is generic over the host that produced the frame. Compatibility
/// shells and new declarative hosts can share the same fixture serialization
/// without duplicating primitive quantization or paint payload conversion.
pub fn visual_snapshot_from_paint_frame(
    name: impl Into<String>,
    viewport: [f32; 2],
    frame: &PaintFrame,
) -> VisualSnapshot {
    let viewport_width = u32::try_from(viewport[0].round().max(1.0) as i64).unwrap_or(1);
    let viewport_height = u32::try_from(viewport[1].round().max(1.0) as i64).unwrap_or(1);
    let primitives = frame.primitives.iter().map(snap_primitive).collect();
    let text_runs = frame
        .text_runs
        .iter()
        .map(|run| SnapshotTextRun {
            text: run.text.clone(),
            position: snap_point(run.position),
            font_size: quantize(run.font_size),
            color: snap_color(run.color),
            max_width: run.max_width.map(quantize),
            align: snap_align(run.align),
        })
        .collect();

    VisualSnapshot {
        name: name.into(),
        viewport_width,
        viewport_height,
        clear_color: snap_color(frame.clear_color),
        primitive_count: frame.primitives.len(),
        text_run_count: frame.text_runs.len(),
        primitives,
        text_runs,
    }
}

fn snap_primitive(primitive: &Primitive) -> SnapshotPrimitive {
    match primitive {
        Primitive::Rect(fill_rect) => SnapshotPrimitive::Rect {
            rect: snap_rect(fill_rect.rect),
            color: snap_color(fill_rect.color),
        },
        Primitive::Circle(fill_circle) => SnapshotPrimitive::Circle {
            center: snap_point(fill_circle.center),
            radius: quantize(fill_circle.radius),
            color: snap_color(fill_circle.color),
        },
        Primitive::LinearGradient(fill_gradient) => SnapshotPrimitive::LinearGradient {
            rect: snap_rect(fill_gradient.rect),
            start: snap_point(fill_gradient.start),
            end: snap_point(fill_gradient.end),
            start_color: snap_color(fill_gradient.start_color),
            end_color: snap_color(fill_gradient.end_color),
        },
        Primitive::Image(draw_image) => SnapshotPrimitive::Image {
            rect: snap_rect(draw_image.rect),
            width: u32::try_from(draw_image.image.width).unwrap_or(0),
            height: u32::try_from(draw_image.image.height).unwrap_or(0),
            pixels: draw_image.image.pixels.as_ref().to_vec(),
        },
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn snap_color(color: Rgba8) -> SnapshotColor {
    SnapshotColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn snap_point(point: Point) -> SnapshotPoint {
    SnapshotPoint {
        x: quantize(point.x),
        y: quantize(point.y),
    }
}

fn snap_rect(rect: Rect) -> SnapshotRect {
    SnapshotRect {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn snap_align(align: TextAlign) -> SnapshotTextAlign {
    match align {
        TextAlign::Left => SnapshotTextAlign::Left,
        TextAlign::Center => SnapshotTextAlign::Center,
        TextAlign::Right => SnapshotTextAlign::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::{
        paint::{DrawImage, FillCircle, FillLinearGradient, FillRect, TextRun},
        types::ImageRgba,
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
                    rect: Rect::from_min_size(
                        Point::new(1.1114, 2.2225),
                        crate::gui::types::Vector2::new(10.0, 20.0),
                    ),
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
                    rect: Rect::from_min_size(
                        Point::new(0.0, 0.0),
                        crate::gui::types::Vector2::new(8.0, 9.0),
                    ),
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
                    rect: Rect::from_min_size(
                        Point::new(6.0, 7.0),
                        crate::gui::types::Vector2::new(1.0, 1.0),
                    ),
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
}
