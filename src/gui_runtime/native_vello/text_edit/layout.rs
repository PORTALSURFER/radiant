#[cfg(test)]
mod cursor_stops;

#[cfg(test)]
use super::super::TextCursorStop;
use super::super::{
    CaretAffinity, NativeTextRenderer, ParagraphSnapshot, TextLayout, font_size_is_renderable,
};
use super::state::SingleLineTextEditorState;
#[cfg(test)]
use cursor_stops::{
    build_visible_cursor_stops, cursor_stop_x, last_stop_at_or_before_x, stop_index_for_byte,
    text_field_width as legacy_text_field_width, visible_end_stop_index,
};
#[cfg(test)]
use std::ops::Range;
use std::sync::Arc;

/// The width-independent paragraph snapshot is the only source for field
/// caret, hit-test, selection, and scroll geometry.
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct TextFieldLayoutState {
    pub(in crate::gui_runtime::native_vello) snapshot: Arc<ParagraphSnapshot>,
    pub(in crate::gui_runtime::native_vello) caret_offset: f32,
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) selection_offsets: Option<(f32, f32)>,
    pub(in crate::gui_runtime::native_vello) selection_rects: Vec<(f32, f32)>,
    pub(in crate::gui_runtime::native_vello) scroll_x: f32,
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) visible_start_byte: usize,
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) visible_end_byte: usize,
    #[cfg(test)]
    visible_stops: Vec<TextCursorStop>,
    width: f32,
}

impl TextFieldLayoutState {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn visible_text<'text>(
        &self,
        text: &'text str,
    ) -> &'text str {
        let range = self.visible_text_range();
        text.get(range).unwrap_or("")
    }

    pub(in crate::gui_runtime::native_vello) fn local_x_for_byte(&self, byte_index: usize) -> f32 {
        (self.snapshot.caret_x(byte_index, CaretAffinity::Downstream) - self.scroll_x)
            .clamp(0.0, self.width)
    }

    pub(in crate::gui_runtime::native_vello) fn selection_rects(&self) -> &[(f32, f32)] {
        &self.selection_rects
    }

    #[cfg(test)]
    fn visible_text_range(&self) -> Range<usize> {
        self.visible_start_byte..self.visible_end_byte
    }
}

/// Build one visible text/caret/selection layout from one retained snapshot.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "legacy left-aligned text-field helper remains covered by native text tests"
    )
)]
pub(in crate::gui_runtime::native_vello) fn build_text_field_layout(
    renderer: &mut NativeTextRenderer,
    editor: &mut SingleLineTextEditorState,
    text: &str,
    font_size: f32,
    available_width: f32,
) -> TextFieldLayoutState {
    build_text_field_layout_aligned(
        renderer,
        editor,
        text,
        font_size,
        available_width,
        crate::gui::paint::TextAlign::Left,
    )
}

/// Build one visible text/caret/selection layout using the input's physical
/// alignment. The legacy wrapper above keeps existing internal callers
/// source-compatible while runtime consumers pass the resolved alignment.
pub(in crate::gui_runtime::native_vello) fn build_text_field_layout_aligned(
    renderer: &mut NativeTextRenderer,
    editor: &mut SingleLineTextEditorState,
    text: &str,
    font_size: f32,
    available_width: f32,
    align: crate::gui::paint::TextAlign,
) -> TextFieldLayoutState {
    editor.clamp_to_text(text);
    let width = text_field_width(available_width);
    let Some(snapshot) = renderer
        .layout_text_view(
            text,
            font_size,
            Some(width),
            align,
            crate::widgets::TextWrap::None,
        )
        .map(TextLayout::snapshot)
    else {
        return empty_field_layout(text, width);
    };
    build_text_field_layout_from_snapshot(snapshot, editor, text, font_size, available_width)
}

/// Build field geometry from one validated retained paragraph snapshot.
pub(in crate::gui_runtime::native_vello) fn build_text_field_layout_from_snapshot(
    snapshot: Arc<ParagraphSnapshot>,
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
    let snapshot_matches_field = font_size_is_renderable(font_size)
        && snapshot.matches_source(text, snapshot.revision)
        && snapshot.is_usable_for(font_size)
        && snapshot.available_width.map(f32::to_bits) == Some(width.to_bits());
    if !snapshot_matches_field {
        return empty_field_layout(text, width);
    }

    let caret_byte = snapshot.canonical_byte(editor.cursor_byte, CaretAffinity::Downstream);
    let scroll_byte = snapshot.canonical_byte(editor.scroll_start_byte, CaretAffinity::Downstream);
    let caret_x = finite_x(snapshot.caret_x(caret_byte, CaretAffinity::Downstream));
    let mut scroll_x = finite_x(snapshot.caret_x(scroll_byte, CaretAffinity::Downstream));
    let left_padding = (padding_font_size * 0.35).clamp(4.0, 12.0);
    let right_padding = left_padding;
    if caret_x - scroll_x > width - right_padding {
        scroll_x = (caret_x - (width - right_padding)).max(0.0);
    } else if caret_x - scroll_x < left_padding {
        scroll_x = (caret_x - left_padding).max(0.0);
    }

    let (scroll_scalar, scroll_affinity) = snapshot.hit_test(scroll_x);
    let scroll_byte = snapshot
        .scalar_boundaries
        .get(scroll_scalar)
        .map(|byte| snapshot.canonical_byte(byte.0, scroll_affinity))
        .unwrap_or(0);
    scroll_x = finite_x(snapshot.caret_x(scroll_byte, scroll_affinity));
    editor.scroll_start_byte = scroll_byte;

    let (visible_start_byte, visible_end_byte) = snapshot.visible_byte_range(scroll_x, width);
    if visible_start_byte > visible_end_byte || visible_end_byte > text.len() {
        return empty_field_layout(text, width);
    }
    #[cfg(test)]
    let visible_start_byte = snapshot.canonical_byte(visible_start_byte, CaretAffinity::Upstream);
    #[cfg(test)]
    let visible_end_byte = snapshot.canonical_byte(visible_end_byte, CaretAffinity::Downstream);
    #[cfg(test)]
    let visible_stops = {
        let legacy_stops = snapshot
            .caret_geometry
            .iter()
            .map(|stop| TextCursorStop {
                byte_index: stop.byte.0,
                x: stop.downstream_x.or(stop.upstream_x).unwrap_or(scroll_x),
            })
            .collect::<Vec<_>>();
        if legacy_stops.is_empty() {
            Vec::new()
        } else {
            let legacy_anchor_byte = last_stop_at_or_before_x(&legacy_stops, scroll_x);
            let visible_start_index =
                stop_index_for_byte(&legacy_stops, visible_start_byte.min(legacy_anchor_byte))
                    .min(legacy_stops.len() - 1);
            let legacy_scroll_x = cursor_stop_x(&legacy_stops, visible_start_byte)
                .min(scroll_x)
                .max(0.0);
            let legacy_width = legacy_text_field_width(width);
            let visible_end_index = visible_end_stop_index(
                &legacy_stops,
                visible_start_index,
                legacy_scroll_x,
                legacy_width,
            )
            .min(legacy_stops.len() - 1)
            .max(visible_start_index);
            build_visible_cursor_stops(
                &legacy_stops,
                visible_start_index,
                visible_end_index,
                visible_start_byte,
                legacy_scroll_x,
                legacy_width,
            )
        }
    };

    let (selection_start, selection_end) = editor.selection_range();
    let selection_rects = snapshot
        .selection_rects(selection_start, selection_end)
        .into_iter()
        .map(|(start, end)| {
            (
                (start - scroll_x).clamp(0.0, width),
                (end - scroll_x).clamp(0.0, width),
            )
        })
        .filter(|(start, end)| end > start)
        .collect::<Vec<_>>();
    #[cfg(test)]
    let selection_offsets =
        selection_rects
            .iter()
            .copied()
            .fold(None::<(f32, f32)>, |range, rect| {
                Some(match range {
                    Some((start, end)) => (start.min(rect.0), end.max(rect.1)),
                    None => rect,
                })
            });

    TextFieldLayoutState {
        snapshot,
        caret_offset: (caret_x - scroll_x).clamp(0.0, width),
        #[cfg(test)]
        selection_offsets,
        selection_rects,
        scroll_x,
        #[cfg(test)]
        visible_start_byte,
        #[cfg(test)]
        visible_end_byte,
        #[cfg(test)]
        visible_stops,
        width,
    }
}

fn empty_field_layout(text: &str, width: f32) -> TextFieldLayoutState {
    let snapshot = ParagraphSnapshot::empty(text);
    TextFieldLayoutState {
        snapshot,
        caret_offset: 0.0,
        #[cfg(test)]
        selection_offsets: None,
        selection_rects: Vec::new(),
        scroll_x: 0.0,
        #[cfg(test)]
        visible_start_byte: 0,
        #[cfg(test)]
        visible_end_byte: 0,
        #[cfg(test)]
        visible_stops: vec![TextCursorStop {
            byte_index: text.len(),
            x: 0.0,
        }],
        width,
    }
}

/// Resolve a pointer x-offset through the same snapshot used for painting.
#[cfg(test)]
pub(in crate::gui_runtime::native_vello::text_edit) fn byte_index_for_local_x(
    layout: &TextFieldLayoutState,
    local_x: f32,
) -> usize {
    let scalar = layout.snapshot.hit_test(layout.scroll_x + local_x).0;
    layout
        .snapshot
        .scalar_boundaries
        .get(scalar)
        .map(|byte| {
            layout
                .snapshot
                .canonical_byte(byte.0, CaretAffinity::Downstream)
        })
        .or_else(|| layout.visible_stops.last().map(|stop| stop.byte_index))
        .unwrap_or(0)
}

fn finite_x(x: f32) -> f32 {
    if x.is_finite() { x.max(0.0) } else { 0.0 }
}

fn text_field_width(available_width: f32) -> f32 {
    if available_width.is_finite() && available_width > 0.0 {
        available_width
    } else {
        1.0
    }
}
