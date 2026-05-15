use super::super::NativeTextRenderer;
use super::layout::byte_index_for_local_x;
use super::*;

#[test]
fn editor_state_clamps_stale_offsets_to_text_boundaries() {
    let mut editor = SingleLineTextEditorState {
        anchor_byte: 1,
        cursor_byte: usize::MAX,
        scroll_start_byte: 0,
    };

    editor.clamp_to_text("aé日");
    assert_eq!(editor.selection_range(), (1, "aé日".len()));

    let mut editor = SingleLineTextEditorState {
        anchor_byte: 2,
        cursor_byte: 4,
        scroll_start_byte: 0,
    };

    editor.clamp_to_text("aé日");
    assert_eq!(editor.selection_range(), (1, 3));
}

#[test]
fn build_text_field_layout_uses_one_full_layout_pass() {
    let mut renderer = NativeTextRenderer::new();
    let mut editor = SingleLineTextEditorState::collapsed_at_end("item alpha beta");

    let layout = build_text_field_layout(&mut renderer, &mut editor, "item alpha beta", 14.0, 48.0);

    let counters = renderer.take_layout_profile_counters();
    assert_eq!((counters.layout_hits, counters.layout_misses), (0, 1));
    assert_eq!(counters.atom_misses, 1);
    assert!(!layout.visible_text("item alpha beta").is_empty());
    assert!(byte_index_for_local_x(&layout, 0.0) <= "item alpha beta".len());
}

#[test]
fn text_field_layout_resolves_caret_offsets_without_second_layout_pass() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);
    editor.set_cursor(text, 4, false);
    editor.set_cursor(text, 10, true);

    let layout = build_text_field_layout(&mut renderer, &mut editor, text, 14.0, 96.0);
    let selection_start = layout.local_x_for_byte(4);
    let selection_end = layout.local_x_for_byte(10);
    let counters = renderer.take_layout_profile_counters();

    assert!(selection_end > selection_start);
    assert_eq!((counters.layout_hits, counters.layout_misses), (0, 1));
}

#[test]
fn text_field_layout_rejects_invalid_font_size_before_cache_work() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);

    let layout = build_text_field_layout(&mut renderer, &mut editor, text, f32::NAN, 96.0);
    let counters = renderer.take_layout_profile_counters();

    assert_eq!((counters.layout_hits, counters.layout_misses), (0, 0));
    assert!(layout.visible_text(text).is_empty());
    assert_eq!(layout.caret_offset, 0.0);
}

#[test]
fn text_field_layout_sanitizes_invalid_available_width() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);

    let layout = build_text_field_layout(&mut renderer, &mut editor, text, 14.0, f32::INFINITY);

    assert!(layout.caret_offset.is_finite());
    assert!(layout.caret_offset <= 1.0);
    assert!(
        layout
            .selection_offsets
            .is_none_or(|(start, end)| start.is_finite() && end.is_finite())
    );
    assert!(layout.local_x_for_byte(0).is_finite());
    assert!(layout.local_x_for_byte(text.len()).is_finite());
}
