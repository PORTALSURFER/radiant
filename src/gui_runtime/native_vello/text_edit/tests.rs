use super::super::NativeTextRenderer;
use super::layout::byte_index_for_local_x;
use super::*;
use std::sync::Arc;

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
    assert_eq!((counters.layout.hits, counters.layout.misses), (0, 0));
    assert_eq!((counters.shape.hits, counters.shape.misses), (0, 1));
    assert_eq!((counters.width.hits, counters.width.misses), (0, 1));
    assert_eq!((counters.view.hits, counters.view.misses), (0, 1));
    assert_eq!(counters.atom.misses, 1);
    assert!(!layout.visible_text("item alpha beta").is_empty());
    assert!(byte_index_for_local_x(&layout, 0.0) <= "item alpha beta".len());
}

#[test]
fn aligned_text_field_layout_projects_physical_alignment_into_caret_geometry() {
    let mut renderer = NativeTextRenderer::new();
    let text = "value";
    let mut left_editor = SingleLineTextEditorState::collapsed_at_end(text);
    left_editor.set_cursor(text, 0, false);
    let left = super::layout::build_text_field_layout_aligned(
        &mut renderer,
        &mut left_editor,
        text,
        14.0,
        256.0,
        crate::gui::paint::TextAlign::Left,
    );
    let mut right_editor = SingleLineTextEditorState::collapsed_at_end(text);
    right_editor.set_cursor(text, 0, false);
    let right = super::layout::build_text_field_layout_aligned(
        &mut renderer,
        &mut right_editor,
        text,
        14.0,
        256.0,
        crate::gui::paint::TextAlign::Right,
    );

    assert!(right.snapshot.alignment_offset > left.snapshot.alignment_offset);
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
    assert_eq!((counters.layout.hits, counters.layout.misses), (0, 0));
    assert_eq!((counters.shape.hits, counters.shape.misses), (0, 1));
    assert_eq!((counters.width.hits, counters.width.misses), (0, 1));
    assert_eq!((counters.view.hits, counters.view.misses), (0, 1));
}

#[test]
fn snapshot_text_field_layout_reuses_supplied_arc_without_renderer_layout() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let snapshot = renderer
        .layout_text_view(
            text,
            14.0,
            Some(96.0),
            crate::gui::paint::TextAlign::Left,
            crate::widgets::TextWrap::None,
        )
        .expect("text input snapshot should be available")
        .snapshot();
    renderer.take_layout_profile_counters();

    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);
    editor.set_cursor(text, 4, false);
    editor.set_cursor(text, 10, true);
    let layout = super::layout::build_text_field_layout_from_snapshot(
        snapshot.clone(),
        &mut editor,
        text,
        14.0,
        96.0,
    );
    let counters = renderer.take_layout_profile_counters();

    assert!(Arc::ptr_eq(&layout.snapshot, &snapshot));
    assert!(layout.selection_offsets.is_some());
    assert_eq!(editor.scroll_start_byte, 0);
    assert_eq!((counters.layout.hits, counters.layout.misses), (0, 0));
    assert_eq!((counters.shape.hits, counters.shape.misses), (0, 0));
    assert_eq!((counters.width.hits, counters.width.misses), (0, 0));
    assert_eq!((counters.view.hits, counters.view.misses), (0, 0));
}

#[test]
fn snapshot_text_field_layout_rejects_mismatched_contract_inputs() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let snapshot = renderer
        .layout_text_view(
            text,
            14.0,
            Some(96.0),
            crate::gui::paint::TextAlign::Left,
            crate::widgets::TextWrap::None,
        )
        .expect("text input snapshot should be available")
        .snapshot();

    for (candidate_text, candidate_font_size, candidate_width) in [
        ("different", 14.0, 96.0),
        (text, 12.0, 96.0),
        (text, 14.0, 48.0),
        (text, 14.0, f32::INFINITY),
    ] {
        let mut editor = SingleLineTextEditorState::collapsed_at_end(candidate_text);
        let layout = super::layout::build_text_field_layout_from_snapshot(
            snapshot.clone(),
            &mut editor,
            candidate_text,
            candidate_font_size,
            candidate_width,
        );

        assert!(!Arc::ptr_eq(&layout.snapshot, &snapshot));
        assert!(layout.visible_text(candidate_text).is_empty());
        assert_eq!(layout.caret_offset, 0.0);
    }
}

#[test]
fn text_field_layout_resolves_local_x_for_exact_bytes_and_fallbacks() {
    let mut renderer = NativeTextRenderer::new();
    let text = "abcdef";
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);

    let layout = build_text_field_layout(&mut renderer, &mut editor, text, 14.0, 256.0);
    let start = layout.local_x_for_byte(0);
    let middle = layout.local_x_for_byte(3);
    let end = layout.local_x_for_byte(text.len());

    assert_eq!(start, 0.0);
    assert!(middle > start);
    assert!(end >= middle);
    assert_eq!(layout.local_x_for_byte(usize::MAX), end);
}

#[test]
fn text_field_layout_rejects_invalid_font_size_before_cache_work() {
    let mut renderer = NativeTextRenderer::new();
    let text = "item alpha beta";
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);

    let layout = build_text_field_layout(&mut renderer, &mut editor, text, f32::NAN, 96.0);
    let counters = renderer.take_layout_profile_counters();

    assert_eq!((counters.layout.hits, counters.layout.misses), (0, 0));
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
