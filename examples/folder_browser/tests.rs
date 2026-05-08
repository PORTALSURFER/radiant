use super::*;
use std::fs;

const TEST_ROOT: &str = "demo-root";
const TEST_ALPHA: &str = "demo-root/alpha";
const TEST_ALPHA_NESTED: &str = "demo-root/alpha/nested";
const TEST_BETA: &str = "demo-root/beta";
const TEST_SAMPLE: &str = "demo-root/sample.txt";

fn test_state() -> BrowserState {
    let root = folder_entry_for_test(
        TEST_ROOT,
        "demo-root",
        vec![
            folder_entry_for_test(
                TEST_ALPHA,
                "alpha",
                vec![folder_entry_for_test(
                    TEST_ALPHA_NESTED,
                    "nested",
                    Vec::new(),
                )],
            ),
            folder_entry_for_test(TEST_BETA, "beta", Vec::new()),
        ],
    );
    let selected_folder = root.id.clone();
    BrowserState {
        selected_folder: selected_folder.clone(),
        selected_file: None,
        expanded_folders: [selected_folder].into_iter().collect(),
        folder_drag: None,
        context_folder: None,
        context_file: None,
        context_position: None,
        rename_folder: None,
        rename_draft: String::new(),
        rename_file: None,
        file_rename_draft: String::new(),
        context_column: None,
        column_resize: None,
        file_columns: default_file_columns(),
        sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
        tree_width: 300.0,
        folders: vec![root],
        status: String::from("Drag a folder handle onto another folder"),
    }
}

fn folder_entry_for_test(id: &str, name: &str, children: Vec<FolderEntry>) -> FolderEntry {
    FolderEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        children,
        files: vec![FileEntry {
            id: format!("{id}/sample.txt"),
            name: String::from("sample.txt"),
            kind: String::from("Text"),
            size: String::from("1 KB"),
            size_bytes: 1024,
            modified: String::from("Today"),
            modified_rank: 0,
        }],
    }
}

fn temp_test_root(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "radiant-folder-browser-{suffix}-{}",
        std::process::id()
    ))
}

#[test]
fn splitter_clamps_folder_tree_width() {
    let mut state = test_state();

    state.resize_tree(ui::DragHandleMessage::Moved {
        position: radiant::layout::Point::new(20.0, 0.0),
    });
    assert_eq!(state.tree_width, MIN_TREE_WIDTH);

    state.resize_tree(ui::DragHandleMessage::Moved {
        position: radiant::layout::Point::new(600.0, 0.0),
    });
    assert_eq!(state.tree_width, MAX_TREE_WIDTH);
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
    assert_eq!(state.selected_folder, beta);
    assert!(state.is_expanded(&alpha));

    state.toggle_folder(alpha.clone());

    assert_eq!(state.selected_folder, beta);
    assert!(!state.is_expanded(&alpha));
}

#[test]
fn folder_click_expands_collapsed_branches_and_only_collapses_selected_expanded_branch() {
    let mut state = test_state();
    let alpha = String::from(TEST_ALPHA);
    let beta = String::from(TEST_BETA);

    state.activate_folder(alpha.clone());
    assert!(state.is_expanded(&alpha));
    assert_eq!(state.selected_folder, alpha);

    state.activate_folder(beta.clone());
    assert_eq!(state.selected_folder, beta);
    state.activate_folder(alpha.clone());
    assert_eq!(state.selected_folder, alpha);
    assert!(state.is_expanded(&alpha));
    state.activate_folder(alpha.clone());
    assert!(!state.is_expanded(&alpha));
}

#[test]
fn selecting_file_records_selected_file_id() {
    let mut state = test_state();

    state.select_file_id(String::from(TEST_SAMPLE));

    assert_eq!(state.selected_file.as_deref(), Some(TEST_SAMPLE));
}

#[test]
fn file_columns_start_with_default_visible_set() {
    let state = test_state();

    let visible = state
        .visible_file_columns()
        .into_iter()
        .map(|column| column.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(visible, ["name", "size", "kind", "modified"]);
}

#[test]
fn toggling_file_column_updates_visible_columns_and_keeps_name_locked() {
    let mut state = test_state();

    state.toggle_file_column(String::from("extension"));
    state.toggle_file_column(String::from("name"));

    assert!(
        state
            .visible_file_columns()
            .iter()
            .any(|column| column.id == "extension")
    );
    assert!(
        state
            .visible_file_columns()
            .iter()
            .any(|column| column.id == "name")
    );
    assert_eq!(state.status, "Name column stays visible");
}

#[test]
fn file_column_resize_clamps_width() {
    let mut state = test_state();

    state.resize_file_column(
        String::from("kind"),
        ui::DragHandleMessage::Started {
            position: radiant::layout::Point::new(100.0, 0.0),
        },
    );
    state.resize_file_column(
        String::from("kind"),
        ui::DragHandleMessage::Moved {
            position: radiant::layout::Point::new(-200.0, 0.0),
        },
    );

    let width = state
        .file_columns
        .iter()
        .find(|column| column.id == "kind")
        .map(|column| column.width)
        .unwrap();
    assert_eq!(width, MIN_FILE_COLUMN_WIDTH);
}

#[test]
fn opening_file_context_selects_file_and_records_target() {
    let mut state = test_state();
    let position = radiant::layout::Point::new(320.0, 180.0);

    state.open_file_context_menu_at(String::from(TEST_SAMPLE), position);

    assert_eq!(state.selected_file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.context_file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.context_position, Some(position));
}

#[test]
fn opening_context_menu_records_target_folder() {
    let mut state = test_state();
    let position = radiant::layout::Point::new(120.0, 140.0);

    state.open_context_menu_at(String::from(TEST_ALPHA), position);

    assert_eq!(state.context_folder.as_deref(), Some(TEST_ALPHA));
    assert_eq!(state.context_position, Some(position));
}

#[test]
fn context_menu_position_anchors_to_cursor_and_flips_near_bottom() {
    let cursor = radiant::layout::Point::new(300.0, 200.0);

    assert_eq!(
        anchored_context_menu_position(Some(cursor), FOLDER_MENU_WIDTH, FOLDER_MENU_HEIGHT),
        (300.0, 200.0)
    );
    assert_eq!(
        anchored_context_menu_position(
            Some(radiant::layout::Point::new(300.0, 520.0)),
            FOLDER_MENU_WIDTH,
            FOLDER_MENU_HEIGHT
        ),
        (300.0, 394.0)
    );
    assert_eq!(
        anchored_context_menu_position(
            Some(radiant::layout::Point::new(880.0, 200.0)),
            FOLDER_MENU_WIDTH,
            FOLDER_MENU_HEIGHT
        ),
        (710.0, 200.0)
    );
}

#[test]
fn rename_from_context_opens_inline_editor_with_folder_name() {
    let mut state = test_state();

    state.open_context_menu_at(
        String::from(TEST_ALPHA),
        radiant::layout::Point::new(120.0, 140.0),
    );
    state.begin_rename_from_context();

    assert_eq!(state.context_folder, None);
    assert_eq!(state.rename_folder.as_deref(), Some(TEST_ALPHA));
    assert_eq!(state.rename_draft, "alpha");
}

#[test]
fn file_rename_from_context_opens_inline_editor_with_file_name() {
    let mut state = test_state();

    state.open_file_context_menu_at(
        String::from(TEST_SAMPLE),
        radiant::layout::Point::new(320.0, 180.0),
    );
    state.begin_file_rename_from_context();

    assert_eq!(state.context_file, None);
    assert_eq!(state.rename_file.as_deref(), Some(TEST_SAMPLE));
    assert_eq!(state.file_rename_draft, "sample.txt");
}

#[test]
fn leaf_folder_click_selects_without_recording_expansion() {
    let mut state = test_state();
    let beta = String::from(TEST_BETA);

    state.activate_folder(beta.clone());

    assert_eq!(state.selected_folder, beta);
    assert!(!state.is_expanded(TEST_BETA));
}

#[test]
fn temp_root_is_the_browser_default_root() {
    let state = BrowserState::default();

    assert_eq!(state.folders[0].id, path_id(&temp_root()));
    assert_eq!(state.selected_folder, path_id(&temp_root()));
}

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
        state.selected_file.as_deref(),
        Some(path_id(&expected).as_str())
    );
    assert_eq!(
        state.rename_file.as_deref(),
        Some(path_id(&expected).as_str())
    );
    assert_eq!(state.file_rename_draft, "New File.txt");
    assert!(expected.is_file());
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

    assert_eq!(state.selected_file, None);
    assert_eq!(state.context_file, None);
    assert!(!file.exists());
    assert!(state.selected_folder().files.is_empty());
    let _ = fs::remove_dir_all(&root);
}
