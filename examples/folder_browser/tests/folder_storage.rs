use super::*;

#[test]
fn folder_move_rejects_root_self_and_descendant_targets() {
    let mut state = test_state();

    assert_eq!(
        move_folder_in_memory(&mut state.folders, TEST_ROOT, TEST_ALPHA).unwrap_err(),
        "Cannot move the root folder"
    );
    assert_eq!(
        move_folder_in_memory(&mut state.folders, TEST_ALPHA, TEST_ALPHA).unwrap_err(),
        "Cannot move a folder into itself"
    );
    assert_eq!(
        move_folder_in_memory(&mut state.folders, TEST_ALPHA, TEST_ALPHA_NESTED).unwrap_err(),
        "Cannot move a folder into one of its descendants"
    );
}

#[test]
fn folder_move_reparents_resource_node() {
    let mut state = test_state();

    let moved = move_folder_in_memory(&mut state.folders, TEST_ALPHA, TEST_BETA)
        .expect("move should succeed");

    assert_eq!(moved, "demo-root/beta/alpha");
    assert!(state.find_folder("demo-root/beta/alpha").is_some());
    assert!(state.find_folder(TEST_ALPHA).is_none());
    assert!(state.find_folder("demo-root/beta/alpha/nested").is_some());
}

#[test]
fn create_child_folder_uses_unique_new_folder_name_in_resource_graph() {
    let mut state = test_state();

    let first = create_child_folder_in_memory(&mut state.folders, TEST_ROOT, "New Folder")
        .expect("create should succeed");
    let second = create_child_folder_in_memory(&mut state.folders, TEST_ROOT, "New Folder")
        .expect("second create should succeed");

    assert_eq!(first, "demo-root/New Folder");
    assert_eq!(second, "demo-root/New Folder 1");
    assert!(state.find_folder("demo-root/New Folder 1").is_some());
}

#[test]
fn folder_rename_rejects_root_empty_and_invalid_names() {
    let state = test_state();

    assert_eq!(
        validate_folder_rename(&state.folders, TEST_ROOT, "renamed", TEST_ROOT).unwrap_err(),
        "Cannot rename the root folder"
    );
    assert_eq!(
        validate_folder_rename(&state.folders, TEST_ALPHA, "  ", TEST_ROOT).unwrap_err(),
        "Folder name cannot be empty"
    );
    assert_eq!(
        validate_folder_rename(&state.folders, TEST_ALPHA, "bad/name", TEST_ROOT).unwrap_err(),
        "Folder name contains invalid characters"
    );
}

#[test]
fn folder_rename_updates_descendant_resource_ids() {
    let mut state = test_state();

    let renamed = rename_folder_in_memory(&mut state.folders, TEST_ALPHA, " renamed ")
        .expect("rename should succeed");

    assert_eq!(renamed, "demo-root/renamed");
    assert!(state.find_folder("demo-root/renamed").is_some());
    assert!(state.find_folder("demo-root/renamed/nested").is_some());
    assert!(state.find_folder(TEST_ALPHA).is_none());
}
