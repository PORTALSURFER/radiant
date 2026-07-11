use crate::gui::{
    paint::{PaintFrame, Primitive, TextAlign},
    types::{Point, Rect, Rgba8},
};

use super::{
    SnapshotColor, SnapshotPoint, SnapshotPrimitive, SnapshotRect, SnapshotTextAlign,
    SnapshotTextRun, VisualSnapshot,
};

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
            width: u32::try_from(draw_image.image.width()).unwrap_or(0),
            height: u32::try_from(draw_image.image.height()).unwrap_or(0),
            pixels: draw_image.image.pixels().to_vec(),
        },
        Primitive::Svg(draw_svg) => SnapshotPrimitive::Svg {
            rect: snap_rect(draw_svg.rect),
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
