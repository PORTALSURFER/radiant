use super::*;

#[test]
fn opening_file_context_selects_file_and_records_target() {
    let mut state = test_state();
    let position = radiant::layout::Point::new(320.0, 180.0);

    state.open_file_context_menu_at(String::from(TEST_SAMPLE), position);

    assert_eq!(state.selection.selected_file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.context.context_file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.context.context_position, Some(position));
}

#[test]
fn opening_context_menu_records_target_folder() {
    let mut state = test_state();
    let position = radiant::layout::Point::new(120.0, 140.0);

    state.open_context_menu_at(String::from(TEST_ALPHA), position);

    assert_eq!(state.context.context_folder.as_deref(), Some(TEST_ALPHA));
    assert_eq!(state.context.context_position, Some(position));
}

#[test]
fn rename_from_context_opens_inline_editor_with_folder_name() {
    let mut state = test_state();

    state.open_context_menu_at(
        String::from(TEST_ALPHA),
        radiant::layout::Point::new(120.0, 140.0),
    );
    state.begin_rename_from_context();

    assert_eq!(state.context.context_folder, None);
    assert_eq!(state.rename.folder.as_deref(), Some(TEST_ALPHA));
    assert_eq!(state.rename.folder_draft, "alpha");
}

#[test]
fn file_rename_from_context_opens_inline_editor_with_file_name() {
    let mut state = test_state();

    state.open_file_context_menu_at(
        String::from(TEST_SAMPLE),
        radiant::layout::Point::new(320.0, 180.0),
    );
    state.begin_file_rename_from_context();

    assert_eq!(state.context.context_file, None);
    assert_eq!(state.rename.file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.rename.file_draft, "sample.txt");
}
