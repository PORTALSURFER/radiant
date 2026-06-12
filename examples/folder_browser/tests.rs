use super::*;

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
        selection: BrowserSelection {
            selected_folder: selected_folder.clone(),
            selected_file: None,
        },
        tree: BrowserTreeState {
            expanded_folders: [selected_folder].into_iter().collect(),
            folder_drag: None,
            tree_width: 300.0,
        },
        context: BrowserContextState {
            context_folder: None,
            context_file: None,
            context_position: None,
            context_column: None,
        },
        rename: BrowserRenameState {
            folder: None,
            folder_draft: String::new(),
            file: None,
            file_draft: String::new(),
        },
        columns: BrowserColumnState {
            file_columns: default_file_columns(),
            sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            resize: None,
        },
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

#[path = "tests/context_menus.rs"]
mod context_menus;
#[path = "tests/file_columns.rs"]
mod file_columns;
#[path = "tests/file_storage.rs"]
mod file_storage;
#[path = "tests/folder_storage.rs"]
mod folder_storage;
#[path = "tests/layout_tree.rs"]
mod layout_tree;
