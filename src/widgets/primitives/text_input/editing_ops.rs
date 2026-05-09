//! Pure text-input editing helpers shared by event routing and command dispatch.

use crate::gui::types::Rect;

pub(super) fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

pub(super) fn caret_for_pointer_x(bounds: Rect, x: f32) -> usize {
    let text_x = (x - bounds.min.x - 16.0).max(0.0);
    let font_size: f32 = if bounds.height() >= 42.0 { 15.0 } else { 13.0 };
    let char_width = (font_size * 0.58_f32).max(1.0_f32);
    (text_x / char_width).floor().max(0.0) as usize
}

pub(super) fn sanitize_single_line_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\r' | '\n' => {}
            '\t' => sanitized.push(' '),
            _ if ch.is_control() => {}
            _ => sanitized.push(ch),
        }
    }
    sanitized
}
