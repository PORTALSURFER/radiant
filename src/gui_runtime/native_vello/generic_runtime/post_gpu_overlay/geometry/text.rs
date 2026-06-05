use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    runtime::{PaintTextAlign, PaintTextRun},
};

use super::{OverlayVertex, push_rect_vertices};
use glyphs::glyph_rows;

mod glyphs;

pub(super) fn push_text_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    text: &PaintTextRun,
) {
    push_text_vertices_in_rect(vertices, target_size, text, text.rect);
}

pub(super) fn push_text_vertices_in_rect(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    text: &PaintTextRun,
    rect: UiRect,
) {
    let Some(layout) = BitmapTextLayout::for_run(text, rect) else {
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
        if x >= rect.max.x {
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
    fn for_run(text: &PaintTextRun, rect: UiRect) -> Option<Self> {
        if !text_is_renderable(text, rect) {
            return None;
        }
        let scale = (text.font_size / GLYPH_HEIGHT).clamp(1.0, 3.0);
        let advance = GLYPH_ADVANCE * scale;
        let max_chars = (rect.width() / advance).floor().max(0.0) as usize;
        if max_chars == 0 {
            return None;
        }
        let text_width = visible_text_char_count(text.text.as_ref(), max_chars) as f32 * advance;
        Some(Self {
            start_x: aligned_start_x(text, rect, text_width),
            y: rect.min.y + ((rect.height() - GLYPH_HEIGHT * scale) * 0.5).max(0.0),
            advance,
            scale,
            max_chars,
        })
    }
}

fn visible_text_char_count(text: &str, max_chars: usize) -> usize {
    if text.is_ascii() {
        text.len().min(max_chars)
    } else {
        text.chars().take(max_chars).count()
    }
}

const GLYPH_HEIGHT: f32 = 7.0;
const GLYPH_ADVANCE: f32 = 6.0;

fn text_is_renderable(text: &PaintTextRun, rect: UiRect) -> bool {
    !text.text.is_empty()
        && rect.has_finite_positive_area()
        && text.color.a != 0
        && text.font_size > 0.0
        && text.font_size.is_finite()
}

fn aligned_start_x(text: &PaintTextRun, rect: UiRect, text_width: f32) -> f32 {
    match text.align {
        PaintTextAlign::Left => rect.min.x,
        PaintTextAlign::Center => rect.min.x + (rect.width() - text_width) * 0.5,
        PaintTextAlign::Right => rect.max.x - text_width,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_text_char_count_uses_ascii_width_without_unicode_byte_counting() {
        assert_eq!(visible_text_char_count("tempo", 12), 5);
        assert_eq!(visible_text_char_count("tempo", 3), 3);
        assert_eq!(visible_text_char_count("øß猫", 12), 3);
        assert_eq!("øß猫".len(), 7);
    }
}
