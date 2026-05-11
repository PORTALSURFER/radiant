use super::super::NativeTextRenderer;
use super::layout::byte_index_for_local_x;
use super::sanitize::sanitize_single_line_insert;
use super::*;

#[test]
fn replace_selection_replaces_selected_range_and_collapses_caret() {
    let mut editor = SingleLineTextEditorState::collapsed_at_end("item alpha");
    editor.set_cursor("item alpha", 4, false);
    editor.set_cursor("item alpha", 10, true);

    let value = editor.replace_selection("item alpha", "beta");

    assert_eq!(value, "itembeta");
    assert_eq!(editor.selection_range(), (8, 8));
}

#[test]
fn backspace_deletes_selection_before_single_character() {
    let mut editor = SingleLineTextEditorState::collapsed_at_end("item");
    editor.set_cursor("item", 1, false);
    editor.set_cursor("item", 3, true);
    assert_eq!(editor.backspace("item").as_deref(), Some("im"));
    assert_eq!(editor.selection_range(), (1, 1));

    assert_eq!(editor.backspace("im").as_deref(), Some("m"));
    assert_eq!(editor.selection_range(), (0, 0));
}

#[test]
fn selected_text_clamps_stale_byte_offsets_to_text_boundaries() {
    let editor = SingleLineTextEditorState {
        anchor_byte: 1,
        cursor_byte: usize::MAX,
        scroll_start_byte: 0,
    };

    assert_eq!(editor.selected_text("aé日").as_deref(), Some("é日"));

    let editor = SingleLineTextEditorState {
        anchor_byte: 2,
        cursor_byte: 4,
        scroll_start_byte: 0,
    };

    assert_eq!(editor.selected_text("aé日").as_deref(), Some("é"));
}

#[test]
fn sanitize_single_line_insert_strips_line_breaks_and_controls() {
    assert_eq!(
        sanitize_single_line_insert("it\ne\tm\u{7}"),
        String::from("ite m")
    );
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
