use super::super::{NativeTextRenderer, TextCursorStop, TextLayout, font_size_is_renderable};
use super::state::SingleLineTextEditorState;
use std::ops::Range;

/// Precomputed visual layout for one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextFieldLayoutState {
    pub(in crate::gui_runtime::native_vello) caret_offset: f32,
    pub(in crate::gui_runtime::native_vello) selection_offsets: Option<(f32, f32)>,
    visible_start_byte: usize,
    visible_end_byte: usize,
    visible_stops: Vec<TextCursorStop>,
}

impl TextFieldLayoutState {
    pub(in crate::gui_runtime::native_vello) fn visible_text<'text>(
        &self,
        text: &'text str,
    ) -> &'text str {
        &text[self.visible_text_range()]
    }

    pub(in crate::gui_runtime::native_vello) fn local_x_for_byte(&self, byte_index: usize) -> f32 {
        let local_byte = byte_index.saturating_sub(self.visible_start_byte);
        self.visible_stops
            .iter()
            .find(|stop| stop.byte_index == local_byte)
            .map(|stop| stop.x)
            .unwrap_or_else(|| {
                if byte_index <= self.visible_start_byte {
                    0.0
                } else {
                    self.visible_stops.last().map(|stop| stop.x).unwrap_or(0.0)
                }
            })
    }

    fn visible_text_range(&self) -> Range<usize> {
        self.visible_start_byte..self.visible_end_byte
    }
}

/// Build one visible text/caret/selection layout for an active text field.
pub(in crate::gui_runtime::native_vello) fn build_text_field_layout(
    renderer: &mut NativeTextRenderer,
    editor: &mut SingleLineTextEditorState,
    text: &str,
    font_size: f32,
    available_width: f32,
) -> TextFieldLayoutState {
    editor.clamp_to_text(text);
    let padding_font_size = if font_size_is_renderable(font_size) {
        font_size
    } else {
        1.0
    };
    let width = text_field_width(available_width);
    let fallback_layout;
    let layout = if let Some(layout) = renderer.layout_text(text, font_size) {
        layout
    } else {
        fallback_layout = TextLayout::empty_for(text);
        &fallback_layout
    };
    let left_padding = (padding_font_size * 0.35).clamp(4.0, 12.0);
    let right_padding = left_padding;
    let caret_x = cursor_stop_x(&layout.cursor_stops, editor.cursor_byte);
    let mut scroll_x = cursor_stop_x(&layout.cursor_stops, editor.scroll_start_byte);
    if caret_x - scroll_x > width - right_padding {
        scroll_x = (caret_x - (width - right_padding)).max(0.0);
    } else if caret_x - scroll_x < left_padding {
        scroll_x = (caret_x - left_padding).max(0.0);
    }
    let scroll_start_byte = last_stop_at_or_before_x(&layout.cursor_stops, scroll_x);
    editor.scroll_start_byte = scroll_start_byte;
    let scroll_start_x = cursor_stop_x(&layout.cursor_stops, scroll_start_byte);
    let visible_start_index = stop_index_for_byte(&layout.cursor_stops, scroll_start_byte);
    let visible_end_index = visible_end_stop_index(
        &layout.cursor_stops,
        visible_start_index,
        scroll_start_x,
        width,
    );
    let visible_start_byte = layout.cursor_stops[visible_start_index].byte_index;
    let visible_end_byte = layout.cursor_stops[visible_end_index].byte_index;
    let visible_stops = build_visible_cursor_stops(
        &layout.cursor_stops,
        visible_start_index,
        visible_end_index,
        visible_start_byte,
        scroll_start_x,
        width,
    );
    let caret_offset = (caret_x - scroll_start_x).clamp(0.0, width);
    let (selection_start, selection_end) = editor.selection_range();
    let selection_offsets = if selection_start < selection_end {
        let start = (cursor_stop_x(&layout.cursor_stops, selection_start) - scroll_start_x)
            .clamp(0.0, width);
        let end =
            (cursor_stop_x(&layout.cursor_stops, selection_end) - scroll_start_x).clamp(0.0, width);
        Some((start.min(end), start.max(end)))
    } else {
        None
    };
    TextFieldLayoutState {
        caret_offset,
        selection_offsets,
        visible_start_byte,
        visible_end_byte,
        visible_stops,
    }
}

/// Resolve the nearest text byte index for a pointer x-offset inside the field.
#[cfg(test)]
pub(in crate::gui_runtime::native_vello::text_edit) fn byte_index_for_local_x(
    layout: &TextFieldLayoutState,
    local_x: f32,
) -> usize {
    let mut best = layout.visible_start_byte;
    let mut best_distance = f32::INFINITY;
    for stop in &layout.visible_stops {
        let distance = (stop.x - local_x).abs();
        if distance < best_distance {
            best = layout.visible_start_byte + stop.byte_index;
            best_distance = distance;
        }
    }
    best
}

fn cursor_stop_x(stops: &[TextCursorStop], byte_index: usize) -> f32 {
    if let Some(stop) = stops.iter().find(|stop| stop.byte_index == byte_index)
        && let Some(x) = finite_stop_x(stop)
    {
        return x;
    }
    stops
        .iter()
        .rev()
        .find(|stop| stop.byte_index <= byte_index && finite_stop_x(stop).is_some())
        .and_then(finite_stop_x)
        .unwrap_or(0.0)
}

fn stop_index_for_byte(stops: &[TextCursorStop], byte_index: usize) -> usize {
    stops
        .iter()
        .position(|stop| stop.byte_index == byte_index)
        .unwrap_or_else(|| stops.len().saturating_sub(1))
}

fn last_stop_at_or_before_x(stops: &[TextCursorStop], x: f32) -> usize {
    if !x.is_finite() {
        return 0;
    }
    stops
        .iter()
        .take_while(|stop| finite_stop_x(stop).is_some_and(|stop_x| stop_x <= x))
        .last()
        .map(|stop| stop.byte_index)
        .unwrap_or(0)
}

fn visible_end_stop_index(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    scroll_start_x: f32,
    width: f32,
) -> usize {
    let mut end = visible_start_index;
    while end + 1 < stops.len()
        && stop_local_x(&stops[end + 1], scroll_start_x).is_some_and(|x| x <= width)
    {
        end += 1;
    }
    if end == visible_start_index
        && end + 1 < stops.len()
        && stop_local_x(&stops[end + 1], scroll_start_x).is_some()
    {
        end += 1;
    }
    end
}

fn build_visible_cursor_stops(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    visible_end_index: usize,
    visible_start_byte: usize,
    scroll_start_x: f32,
    width: f32,
) -> Vec<TextCursorStop> {
    stops[visible_start_index..=visible_end_index]
        .iter()
        .map(|stop| TextCursorStop {
            byte_index: stop.byte_index.saturating_sub(visible_start_byte),
            x: stop_local_x(stop, scroll_start_x)
                .map(|x| x.clamp(0.0, width))
                .unwrap_or(0.0),
        })
        .collect()
}

fn text_field_width(available_width: f32) -> f32 {
    if available_width.is_finite() && available_width > 0.0 {
        available_width
    } else {
        1.0
    }
}

fn finite_stop_x(stop: &TextCursorStop) -> Option<f32> {
    stop.x.is_finite().then_some(stop.x.max(0.0))
}

fn stop_local_x(stop: &TextCursorStop, scroll_start_x: f32) -> Option<f32> {
    let x = finite_stop_x(stop)? - scroll_start_x;
    x.is_finite().then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_stop_lookup_falls_back_from_non_finite_positions() {
        let stops = [
            TextCursorStop {
                byte_index: 0,
                x: 0.0,
            },
            TextCursorStop {
                byte_index: 2,
                x: f32::NAN,
            },
            TextCursorStop {
                byte_index: 4,
                x: 12.0,
            },
        ];

        assert_eq!(cursor_stop_x(&stops, 2), 0.0);
        assert_eq!(cursor_stop_x(&stops, 4), 12.0);
    }

    #[test]
    fn visible_cursor_stops_replace_invalid_positions_with_origin() {
        let stops = [
            TextCursorStop {
                byte_index: 0,
                x: 0.0,
            },
            TextCursorStop {
                byte_index: 2,
                x: f32::INFINITY,
            },
        ];

        let visible = build_visible_cursor_stops(&stops, 0, 1, 0, 0.0, 24.0);

        assert_eq!(
            visible,
            vec![
                TextCursorStop {
                    byte_index: 0,
                    x: 0.0,
                },
                TextCursorStop {
                    byte_index: 2,
                    x: 0.0,
                }
            ]
        );
    }
}
