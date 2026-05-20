use super::*;

#[test]
fn folder_move_rejects_root_self_and_descendant_targets() {
    let root = PathBuf::from(TEST_ROOT);
    let source = PathBuf::from(TEST_ALPHA);
    let child = PathBuf::from(TEST_ALPHA_NESTED);

    assert_eq!(
        validate_folder_move(&root, &source, &root).unwrap_err(),
        "Cannot move the root folder"
    );
    assert_eq!(
        validate_folder_move(&source, &source, &root).unwrap_err(),
        "Cannot move a folder into itself"
    );
    assert_eq!(
        validate_folder_move(&source, &child, &root).unwrap_err(),
        "Cannot move a folder into one of its descendants"
    );
}

#[test]
fn folder_move_renames_source_into_target_folder() {
    let root = temp_test_root("move-test");
    let source = root.join("source");
    let target = root.join("target");
    let destination = target.join("source");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source).expect("source folder should be created");
    fs::create_dir_all(&target).expect("target folder should be created");

    let moved = move_folder_on_disk(&path_id(&source), &path_id(&target), &path_id(&root))
        .expect("move should succeed");

    assert_eq!(moved, path_id(&destination));
    assert!(destination.is_dir());
    assert!(!source.exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn create_child_folder_uses_unique_new_folder_name() {
    let root = temp_test_root("create-test");
    let existing = root.join("New Folder");
    let expected = root.join("New Folder 1");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&existing).expect("existing folder should be created");

    let created =
        create_child_folder(&path_id(&root), "New Folder").expect("create should succeed");

    assert_eq!(created, path_id(&expected));
    assert!(expected.is_dir());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn folder_rename_rejects_root_empty_and_invalid_names() {
    let root = temp_test_root("rename-reject-test");
    let source = root.join("source");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source).expect("source folder should be created");

    assert_eq!(
        validate_folder_rename(&root, "renamed", &root).unwrap_err(),
        "Cannot rename the root folder"
    );
    assert_eq!(
        validate_folder_rename(&source, "  ", &root).unwrap_err(),
        "Folder name cannot be empty"
    );
    assert_eq!(
        validate_folder_rename(&source, "bad/name", &root).unwrap_err(),
        "Folder name contains invalid characters"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn folder_rename_changes_folder_name_in_place() {
    let root = temp_test_root("rename-test");
    let source = root.join("source");
    let destination = root.join("renamed");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source).expect("source folder should be created");

    let renamed = rename_folder_on_disk(&path_id(&source), " renamed ", &path_id(&root))
        .expect("rename should succeed");

    assert_eq!(renamed, path_id(&destination));
    assert!(destination.is_dir());
    assert!(!source.exists());
    let _ = fs::remove_dir_all(&root);
}
