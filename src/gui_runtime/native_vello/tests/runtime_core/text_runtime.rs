use super::super::*;
use crate::gui::types::Rect;

#[test]
fn active_text_field_visual_cache_tracks_text_and_editor_state() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let text_rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 16.0));

    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("item-a"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("item-a"));

    let first = runner
        .build_active_text_field_visual_state(&layout, text_rect)
        .expect("content search visual");
    let first_cache = runner
        .active_text_field_visual_cache
        .clone()
        .expect("cache entry after first build");

    let second = runner
        .build_active_text_field_visual_state(&layout, text_rect)
        .expect("content search visual on cache hit");
    assert_eq!(second, first);
    assert_eq!(
        runner.active_text_field_visual_cache,
        Some(first_cache.clone())
    );

    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("item-a"));
    runner
        .text_editor_state
        .as_mut()
        .expect("editor state")
        .move_left("item-a", false);
    let moved = runner
        .build_active_text_field_visual_state(&layout, text_rect)
        .expect("content search visual after cursor move");
    let moved_cache = runner
        .active_text_field_visual_cache
        .clone()
        .expect("cache entry after cursor move");
    assert_ne!(moved_cache.editor, first_cache.editor);
    assert!(moved.caret_offset < first.caret_offset);

    runner.text_input_buffer = Some(String::from("item-b"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("item-b"));
    let renamed = runner
        .build_active_text_field_visual_state(&layout, text_rect)
        .expect("content search visual after text change");
    let renamed_cache = runner
        .active_text_field_visual_cache
        .as_ref()
        .expect("cache entry after text change");
    assert_eq!(renamed_cache.text, "item-b");
    assert_eq!(renamed.text, "item-b");
}
