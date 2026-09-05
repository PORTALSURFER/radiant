//! Pure text-input editing helpers shared by event routing and command dispatch.

use crate::{
    gui::types::Rect,
    runtime::ResolvedEnvironment,
    widgets::{DeclaredTextMetrics, TextAlign, TextScaleParticipation},
};

pub(super) fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

pub(super) fn caret_for_pointer_x_with_environment(
    bounds: Rect,
    x: f32,
    text: &str,
    declared: DeclaredTextMetrics,
    align: TextAlign,
    environment: &ResolvedEnvironment,
) -> usize {
    let resolved = declared.resolve(environment, TextScaleParticipation::Scaled);
    let char_width = (resolved.font_size * 0.58).max(1.0);
    let text_len = text.chars().count();
    let text_rect_min = bounds.min.x + resolved.insets.x;
    let text_rect_max = (bounds.max.x - resolved.insets.x).max(text_rect_min);
    let text_width = text_len as f32 * char_width;
    let physical = align.resolve(environment.writing_direction());
    let content_width = (text_rect_max - text_rect_min).max(0.0);
    let slack = (content_width - text_width).max(0.0);
    let alignment_offset = match physical {
        crate::runtime::PaintTextAlign::Left => 0.0,
        crate::runtime::PaintTextAlign::Center => slack * 0.5,
        crate::runtime::PaintTextAlign::Right => slack,
    };
    let offset = x - text_rect_min - alignment_offset;
    (offset / char_width).round().clamp(0.0, text_len as f32) as usize
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
