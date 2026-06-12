use super::*;

#[test]
fn create_child_file_uses_unique_new_file_name_in_resource_graph() {
    let mut state = test_state();
    state.create_file_in_selected_folder();

    assert_eq!(
        state.selection.selected_file.as_deref(),
        Some("demo-root/New File.txt")
    );
    assert_eq!(state.rename.file.as_deref(), Some("demo-root/New File.txt"));
    assert_eq!(state.rename.file_draft, "New File.txt");

    state.create_file_in_selected_folder();

    assert_eq!(
        state.selection.selected_file.as_deref(),
        Some("demo-root/New File 1.txt")
    );
    assert!(
        state
            .selected_folder()
            .files
            .iter()
            .any(|file| file.id == "demo-root/New File 1.txt")
    );
}

#[test]
fn file_rename_rejects_empty_and_invalid_names() {
    let state = test_state();

    assert_eq!(
        validate_file_rename(&state.folders, TEST_SAMPLE, "  ", TEST_ROOT).unwrap_err(),
        "File name cannot be empty"
    );
    assert_eq!(
        validate_file_rename(&state.folders, TEST_SAMPLE, "bad/name.txt", TEST_ROOT).unwrap_err(),
        "File name contains invalid characters"
    );
}

#[test]
fn file_rename_changes_resource_graph_without_touching_disk() {
    let mut state = test_state();
    state.open_file_context_menu_at(
        String::from(TEST_SAMPLE),
        radiant::layout::Point::new(320.0, 180.0),
    );
    state.begin_file_rename_from_context();
    state.rename.file_draft = String::from(" renamed.txt ");

    state.commit_file_rename();

    assert_eq!(
        state.selection.selected_file.as_deref(),
        Some("demo-root/renamed.txt")
    );
    assert!(
        state
            .selected_folder()
            .files
            .iter()
            .any(|file| file.id == "demo-root/renamed.txt" && file.name == "renamed.txt")
    );
    assert!(
        !state
            .selected_folder()
            .files
            .iter()
            .any(|file| file.id == TEST_SAMPLE)
    );
}

#[test]
fn delete_file_from_context_clears_selection_and_removes_resource_entry() {
    let mut state = test_state();
    state.open_file_context_menu_at(
        String::from(TEST_SAMPLE),
        radiant::layout::Point::new(320.0, 180.0),
    );

    state.delete_file_from_context();

    assert_eq!(state.selection.selected_file, None);
    assert_eq!(state.context.context_file, None);
    assert!(
        !state
            .selected_folder()
            .files
            .iter()
            .any(|file| file.id == TEST_SAMPLE)
    );
}
