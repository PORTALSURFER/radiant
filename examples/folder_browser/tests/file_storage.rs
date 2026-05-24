use super::*;

#[test]
fn create_child_file_uses_unique_new_file_name() {
    let root = temp_test_root("create-file-test");
    let existing = root.join("New File.txt");
    let expected = root.join("New File 1.txt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root folder should be created");
    fs::write(&existing, "sample").expect("existing file should be created");

    let created =
        create_child_file(&path_id(&root), "New File.txt", &path_id(&root)).expect("create");

    assert_eq!(created, path_id(&expected));
    assert!(expected.is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn create_file_in_selected_folder_selects_created_file_for_rename() {
    let root = temp_test_root("state-create-file-test");
    let expected = root.join("New File.txt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root folder should be created");
    let mut state = BrowserState::from_root(root.clone());

    state.create_file_in_selected_folder();

    assert_eq!(
        state.selection.selected_file.as_deref(),
        Some(path_id(&expected).as_str())
    );
    assert_eq!(
        state.rename.file.as_deref(),
        Some(path_id(&expected).as_str())
    );
    assert_eq!(state.rename.file_draft, "New File.txt");
    assert!(expected.is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn file_rename_rejects_empty_and_invalid_names() {
    let root = temp_test_root("file-rename-reject-test");
    let source = root.join("sample.txt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root folder should be created");
    fs::write(&source, "sample").expect("sample file should be created");

    assert_eq!(
        validate_file_rename(&source, "  ", &root).unwrap_err(),
        "File name cannot be empty"
    );
    assert_eq!(
        validate_file_rename(&source, "bad/name.txt", &root).unwrap_err(),
        "File name contains invalid characters"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn file_rename_changes_file_name_in_place() {
    let root = temp_test_root("file-rename-test");
    let source = root.join("sample.txt");
    let destination = root.join("renamed.txt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root folder should be created");
    fs::write(&source, "sample").expect("sample file should be created");

    let renamed = rename_file_on_disk(&path_id(&source), " renamed.txt ", &path_id(&root))
        .expect("rename should succeed");

    assert_eq!(renamed, path_id(&destination));
    assert!(destination.is_file());
    assert!(!source.exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn delete_file_removes_file_but_rejects_folders() {
    let root = temp_test_root("delete-file-test");
    let file = root.join("sample.txt");
    let folder = root.join("folder");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&folder).expect("folder should be created");
    fs::write(&file, "sample").expect("file should be created");

    assert_eq!(
        delete_file_on_disk(&path_id(&folder), &path_id(&root)).unwrap_err(),
        "Only files can be deleted here"
    );
    delete_file_on_disk(&path_id(&file), &path_id(&root)).expect("delete should succeed");

    assert!(!file.exists());
    assert!(folder.is_dir());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn delete_file_from_context_clears_selection_and_reloads() {
    let root = temp_test_root("state-delete-file-test");
    let file = root.join("sample.txt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root folder should be created");
    fs::write(&file, "sample").expect("file should be created");
    let mut state = BrowserState::from_root(root.clone());
    state.open_file_context_menu_at(path_id(&file), radiant::layout::Point::new(320.0, 180.0));

    state.delete_file_from_context();

    assert_eq!(state.selection.selected_file, None);
    assert_eq!(state.context.context_file, None);
    assert!(!file.exists());
    assert!(state.selected_folder().files.is_empty());
    let _ = fs::remove_dir_all(&root);
}
