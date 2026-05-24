use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    runtime::{PaintTextAlign, PaintTextRun},
};

use super::{OverlayVertex, push_rect_vertices};

pub(super) fn push_text_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    text: &PaintTextRun,
) {
    let Some(layout) = BitmapTextLayout::for_run(text) else {
        return;
    };
    let mut x = layout.start_x;
    for ch in text.text.chars().take(layout.max_chars) {
        if let Some(rows) = glyph_rows(ch) {
            push_glyph_vertices(
                vertices,
                target_size,
                rows,
                Point::new(x, layout.y),
                layout.scale,
                text.color,
            );
        }
        x += layout.advance;
        if x >= text.rect.max.x {
            break;
        }
    }
}

struct BitmapTextLayout {
    start_x: f32,
    y: f32,
    advance: f32,
    scale: f32,
    max_chars: usize,
}

impl BitmapTextLayout {
    fn for_run(text: &PaintTextRun) -> Option<Self> {
        if !text_is_renderable(text) {
            return None;
        }
        let scale = (text.font_size / GLYPH_HEIGHT).clamp(1.0, 3.0);
        let advance = GLYPH_ADVANCE * scale;
        let max_chars = (text.rect.width() / advance).floor().max(0.0) as usize;
        if max_chars == 0 {
            return None;
        }
        let text_width = text.text.chars().take(max_chars).count() as f32 * advance;
        Some(Self {
            start_x: aligned_start_x(text, text_width),
            y: text.rect.min.y + ((text.rect.height() - GLYPH_HEIGHT * scale) * 0.5).max(0.0),
            advance,
            scale,
            max_chars,
        })
    }
}

const GLYPH_HEIGHT: f32 = 7.0;
const GLYPH_ADVANCE: f32 = 6.0;

fn text_is_renderable(text: &PaintTextRun) -> bool {
    !text.text.is_empty()
        && text.rect.has_finite_positive_area()
        && text.color.a != 0
        && text.font_size > 0.0
        && text.font_size.is_finite()
}

fn aligned_start_x(text: &PaintTextRun, text_width: f32) -> f32 {
    match text.align {
        PaintTextAlign::Left => text.rect.min.x,
        PaintTextAlign::Center => text.rect.min.x + (text.rect.width() - text_width) * 0.5,
        PaintTextAlign::Right => text.rect.max.x - text_width,
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
                glyph_pixel(origin, row_index, col, scale),
                color,
            );
        }
    }
}

fn glyph_pixel(origin: Point, row_index: usize, col: usize, scale: f32) -> UiRect {
    UiRect::from_min_size(
        Point::new(
            origin.x + col as f32 * scale,
            origin.y + row_index as f32 * scale,
        ),
        Vector2::new(scale, scale),
    )
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
