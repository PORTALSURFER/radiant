use super::*;

#[test]
fn splitter_clamps_folder_tree_width() {
    let mut state = test_state();

    state.resize_tree(ui::DragHandleMessage::Moved {
        position: radiant::layout::Point::new(20.0, 0.0),
    });
    assert_eq!(state.tree.tree_width, MIN_TREE_WIDTH);

    state.resize_tree(ui::DragHandleMessage::Moved {
        position: radiant::layout::Point::new(600.0, 0.0),
    });
    assert_eq!(state.tree.tree_width, MAX_TREE_WIDTH);
}

#[test]
fn folder_expansion_controls_visible_tree_rows() {
    let mut state = test_state();
    let alpha = String::from(TEST_ALPHA);
    let nested = String::from(TEST_ALPHA_NESTED);

    assert!(!state.visible_folder_ids().contains(&nested));
    state.toggle_folder(alpha);

    assert!(state.visible_folder_ids().contains(&nested));
}

#[test]
fn expander_toggle_collapses_without_selecting_folder_first() {
    let mut state = test_state();
    let alpha = String::from(TEST_ALPHA);
    let beta = String::from(TEST_BETA);

    state.activate_folder(alpha.clone());
    state.activate_folder(beta.clone());
    assert_eq!(state.selection.selected_folder, beta);
    assert!(state.is_expanded(&alpha));

    state.toggle_folder(alpha.clone());

    assert_eq!(state.selection.selected_folder, beta);
    assert!(!state.is_expanded(&alpha));
}

#[test]
fn folder_click_expands_collapsed_branches_and_only_collapses_selected_expanded_branch() {
    let mut state = test_state();
    let alpha = String::from(TEST_ALPHA);
    let beta = String::from(TEST_BETA);

    state.activate_folder(alpha.clone());
    assert!(state.is_expanded(&alpha));
    assert_eq!(state.selection.selected_folder, alpha);

    state.activate_folder(beta.clone());
    assert_eq!(state.selection.selected_folder, beta);
    state.activate_folder(alpha.clone());
    assert_eq!(state.selection.selected_folder, alpha);
    assert!(state.is_expanded(&alpha));
    state.activate_folder(alpha.clone());
    assert!(!state.is_expanded(&alpha));
}

#[test]
fn leaf_folder_click_selects_without_recording_expansion() {
    let mut state = test_state();
    let beta = String::from(TEST_BETA);

    state.activate_folder(beta.clone());

    assert_eq!(state.selection.selected_folder, beta);
    assert!(!state.is_expanded(TEST_BETA));
}

#[test]
fn selecting_file_records_selected_file_id() {
    let mut state = test_state();

    state.select_file_id(String::from(TEST_SAMPLE));

    assert_eq!(state.selection.selected_file.as_deref(), Some(TEST_SAMPLE));
}

#[test]
fn in_memory_resource_graph_is_the_browser_default_root() {
    let state = BrowserState::default();

    assert_eq!(state.folders[0].id, "demo-root");
    assert_eq!(state.selection.selected_folder, "demo-root");
    assert_eq!(state.status, "In-memory resource sandbox");
}
