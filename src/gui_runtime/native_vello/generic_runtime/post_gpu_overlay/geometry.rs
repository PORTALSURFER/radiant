use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    runtime::{PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun},
};

use super::vertex::OverlayVertex;

pub(super) fn replayable_suffix(primitives: &[PaintPrimitive]) -> Option<&[PaintPrimitive]> {
    primitives
        .iter()
        .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
        .and_then(|index| primitives.get(index + 1..))
}

#[cfg(test)]
pub(super) fn replayable_vertices_into(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    vertices: &mut Vec<OverlayVertex>,
) {
    vertices.clear();
    append_replayable_vertices(primitives, target_size, vertices);
}

pub(super) fn replayable_vertices_in_regions_into(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    regions: &[UiRect],
    vertices: &mut Vec<OverlayVertex>,
) {
    vertices.clear();
    append_replayable_vertices_in_regions(primitives, target_size, regions, vertices);
}

pub(super) fn append_replayable_vertices(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    vertices: &mut Vec<OverlayVertex>,
) {
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => {
                push_rect_vertices(vertices, target_size, fill.rect, fill.color);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                push_stroke_vertices(vertices, target_size, stroke);
            }
            PaintPrimitive::Text(text) => {
                push_text_vertices(vertices, target_size, text);
            }
            _ => {}
        }
    }
}

pub(super) fn append_replayable_vertices_in_regions(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    regions: &[UiRect],
    vertices: &mut Vec<OverlayVertex>,
) {
    if regions.is_empty() {
        return;
    }
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) if fill.color.a >= OPAQUE_REVEALED_FILL_ALPHA => {}
            PaintPrimitive::FillRect(fill) => {
                for region in regions {
                    if let Some(rect) = intersect_rect(fill.rect, *region) {
                        push_rect_vertices(vertices, target_size, rect, fill.color);
                    }
                }
            }
            PaintPrimitive::StrokeRect(stroke) => {
                for edge in stroke_rect_edges(stroke.rect, stroke.width) {
                    for region in regions {
                        if let Some(rect) = intersect_rect(edge, *region) {
                            push_rect_vertices(vertices, target_size, rect, stroke.color);
                        }
                    }
                }
            }
            PaintPrimitive::Text(text) => {
                for region in regions {
                    if let Some(rect) = intersect_rect(text.rect, *region) {
                        let mut clipped = text.clone();
                        clipped.rect = rect;
                        push_text_vertices(vertices, target_size, &clipped);
                    }
                }
            }
            _ => {}
        }
    }
}

const OPAQUE_REVEALED_FILL_ALPHA: u8 = 240;

fn intersect_rect(a: UiRect, b: UiRect) -> Option<UiRect> {
    if !a.has_finite_positive_area() || !b.has_finite_positive_area() {
        return None;
    }
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    let intersection = UiRect::from_min_max(min, max);
    intersection
        .has_finite_positive_area()
        .then_some(intersection)
}

fn push_stroke_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    stroke: &PaintStrokeRect,
) {
    for rect in stroke_rect_edges(stroke.rect, stroke.width) {
        push_rect_vertices(vertices, target_size, rect, stroke.color);
    }
}

fn push_rect_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    rect: UiRect,
    color: Rgba8,
) {
    if !rect.has_finite_positive_area()
        || !target_has_finite_positive_size(target_size)
        || color.a == 0
        || rect_is_outside_target(rect, target_size)
    {
        return;
    }
    let color = rgba_to_float(color);
    let left = clip_x(rect.min.x, target_size);
    let right = clip_x(rect.max.x, target_size);
    let top = clip_y(rect.min.y, target_size);
    let bottom = clip_y(rect.max.y, target_size);
    vertices.extend_from_slice(&[
        vertex(left, top, color),
        vertex(right, top, color),
        vertex(left, bottom, color),
        vertex(left, bottom, color),
        vertex(right, top, color),
        vertex(right, bottom, color),
    ]);
}

fn push_text_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    text: &PaintTextRun,
) {
    if text.text.is_empty()
        || !text.rect.has_finite_positive_area()
        || !target_has_finite_positive_size(target_size)
        || text.color.a == 0
        || text.font_size <= 0.0
        || !text.font_size.is_finite()
    {
        return;
    }
    let scale = (text.font_size / 7.0).clamp(1.0, 3.0);
    let glyph_height = 7.0 * scale;
    let advance = 6.0 * scale;
    let max_chars = (text.rect.width() / advance).floor().max(0.0) as usize;
    if max_chars == 0 {
        return;
    }
    let chars = text.text.chars().take(max_chars).collect::<Vec<_>>();
    let text_width = chars.len() as f32 * advance;
    let mut x = match text.align {
        PaintTextAlign::Left => text.rect.min.x,
        PaintTextAlign::Center => text.rect.min.x + (text.rect.width() - text_width) * 0.5,
        PaintTextAlign::Right => text.rect.max.x - text_width,
    };
    let y = text.rect.min.y + ((text.rect.height() - glyph_height) * 0.5).max(0.0);
    for ch in chars {
        if let Some(rows) = glyph_rows(ch) {
            push_glyph_vertices(
                vertices,
                target_size,
                rows,
                Point::new(x, y),
                scale,
                text.color,
            );
        }
        x += advance;
        if x >= text.rect.max.x {
            break;
        }
    }
}

fn push_glyph_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    rows: [u8; 7],
    origin: Point,
    scale: f32,
    color: Rgba8,
) {
    for (row_index, row) in rows.iter().enumerate() {
        for col in 0..5 {
            if row & (1 << (4 - col)) == 0 {
                continue;
            }
            push_rect_vertices(
                vertices,
                target_size,
                UiRect::from_min_size(
                    Point::new(
                        origin.x + col as f32 * scale,
                        origin.y + row_index as f32 * scale,
                    ),
                    Vector2::new(scale, scale),
                ),
                color,
            );
        }
    }
}

fn glyph_rows(ch: char) -> Option<[u8; 7]> {
    Some(match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b01010, 0b01010, 0b10001,
        ],
        'Y' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11110, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        ' ' => [0; 7],
        _ => return None,
    })
}

fn vertex(x: f32, y: f32, color: [f32; 4]) -> OverlayVertex {
    OverlayVertex::new([x, y], color)
}

fn clip_x(x: f32, target_size: Vector2) -> f32 {
    x / target_size.x.max(1.0) * 2.0 - 1.0
}

fn clip_y(y: f32, target_size: Vector2) -> f32 {
    1.0 - y / target_size.y.max(1.0) * 2.0
}

fn rect_is_outside_target(rect: UiRect, target_size: Vector2) -> bool {
    let target_width = target_size.x.max(0.0);
    let target_height = target_size.y.max(0.0);
    rect.max.x <= 0.0
        || rect.min.x >= target_width
        || rect.max.y <= 0.0
        || rect.min.y >= target_height
}

fn stroke_rect_edges(rect: UiRect, width: f32) -> [UiRect; 4] {
    let width = if width.is_finite() && width > 0.0 {
        width
    } else {
        1.0
    };
    [
        UiRect::from_min_size(rect.min, Vector2::new(rect.width(), width)),
        UiRect::from_min_size(
            Point::new(rect.min.x, rect.max.y - width),
            Vector2::new(rect.width(), width),
        ),
        UiRect::from_min_size(rect.min, Vector2::new(width, rect.height())),
        UiRect::from_min_size(
            Point::new(rect.max.x - width, rect.min.y),
            Vector2::new(width, rect.height()),
        ),
    ]
}

fn target_has_finite_positive_size(target_size: Vector2) -> bool {
    target_size.x.is_finite()
        && target_size.y.is_finite()
        && target_size.x > 0.0
        && target_size.y > 0.0
}

fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

#[cfg(test)]
#[path = "geometry/tests.rs"]
mod tests;
