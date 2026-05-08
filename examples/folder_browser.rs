//! Folder browser with an expandable tree, details list, and resizable panes.

use radiant::prelude as ui;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[path = "folder_browser/columns.rs"]
mod columns;
#[path = "folder_browser/menu_geometry.rs"]
mod menu_geometry;
#[path = "folder_browser/model.rs"]
mod model;
#[path = "folder_browser/storage.rs"]
mod storage;
#[path = "folder_browser/view.rs"]
mod view;
use columns::*;
use menu_geometry::*;
use model::*;
use storage::*;

const MIN_TREE_WIDTH: f32 = 190.0;
const MAX_TREE_WIDTH: f32 = 430.0;
const SPLITTER_OFFSET: f32 = 24.0;
const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_ROW_TOP: f32 = 104.0;
const MIN_FILE_COLUMN_WIDTH: f32 = 56.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 360.0;
const ROOT_ENV_VAR: &str = "RADIANT_FOLDER_BROWSER_ROOT";

#[derive(Clone, Debug)]
struct BrowserState {
    selected_folder: String,
    selected_file: Option<String>,
    expanded_folders: HashSet<String>,
    folder_drag: Option<FolderDrag>,
    context_folder: Option<String>,
    context_file: Option<String>,
    context_position: Option<radiant::layout::Point>,
    rename_folder: Option<String>,
    rename_draft: String,
    rename_file: Option<String>,
    file_rename_draft: String,
    context_column: Option<String>,
    column_resize: Option<ColumnResize>,
    file_columns: Vec<FileColumn>,
    sort: ui::DetailsSort,
    tree_width: f32,
    folders: Vec<FolderEntry>,
    status: String,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::from_root(temp_root())
    }
}

impl BrowserState {
    fn from_root(root: PathBuf) -> Self {
        let root_folder = load_root_folder(root);
        let root_id = root_folder.id.clone();
        Self {
            selected_folder: root_id.clone(),
            selected_file: None,
            expanded_folders: [root_id].into_iter().collect(),
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
            folders: vec![root_folder],
            status: String::from("Drag a folder handle onto another folder"),
        }
    }

    fn selected_folder(&self) -> &FolderEntry {
        self.find_folder(&self.selected_folder)
            .unwrap_or(&self.folders[0])
    }

    fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    fn select_folder(&mut self, id: impl Into<String>) {
        self.selected_folder = id.into();
        self.selected_file = None;
        self.context_file = None;
        self.context_position = None;
        self.cancel_renames();
    }

    fn activate_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.context_folder = None;
        self.context_file = None;
        self.context_position = None;
        self.cancel_renames();
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    fn toggle_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.folder_has_children(&id) && !self.expanded_folders.remove(&id) {
            self.expanded_folders.insert(id);
        }
    }

    fn select_file_id(&mut self, id: String) {
        self.selected_file = Some(id);
        self.context_folder = None;
        self.context_file = None;
        self.context_column = None;
        self.context_position = None;
        self.cancel_renames();
    }

    fn open_context_menu_at(&mut self, id: String, position: radiant::layout::Point) {
        self.context_folder = Some(id);
        self.context_file = None;
        self.context_column = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    fn close_context_menu(&mut self) {
        self.context_folder = None;
        self.context_position = None;
    }

    fn open_file_context_menu_at(&mut self, id: String, position: radiant::layout::Point) {
        self.selected_file = Some(id.clone());
        self.context_file = Some(id);
        self.context_folder = None;
        self.context_column = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    fn close_file_context_menu(&mut self) {
        self.context_file = None;
        self.context_position = None;
    }

    fn open_column_context_menu_at(&mut self, id: String, position: radiant::layout::Point) {
        self.context_column = Some(id);
        self.context_file = None;
        self.context_folder = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    fn close_column_context_menu(&mut self) {
        self.context_column = None;
        self.context_position = None;
    }

    fn create_folder_from_context(&mut self) {
        let Some(parent_id) = self.context_folder.take() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match create_child_folder(&parent_id, "New Folder") {
            Ok(created) => {
                self.status = format!("Created {}", folder_label(Path::new(&created)));
                self.selected_folder = created;
                self.selected_file = None;
                self.expanded_folders.insert(root_id.clone());
                self.expanded_folders.insert(parent_id);
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn create_file_in_selected_folder(&mut self) {
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match create_child_file(&self.selected_folder, "New File.txt", &root_id) {
            Ok(created) => {
                self.status = format!("Created {}", file_label(Path::new(&created)));
                self.selected_file = Some(created.clone());
                self.context_file = None;
                self.context_folder = None;
                self.cancel_renames();
                self.rename_file = Some(created);
                self.file_rename_draft = String::from("New File.txt");
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn begin_rename_from_context(&mut self) {
        let Some(folder_id) = self.context_folder.take() else {
            return;
        };
        if let Some(folder) = self.find_folder(&folder_id) {
            self.rename_draft = folder.name.clone();
            self.rename_folder = Some(folder_id);
            self.selected_file = None;
            self.context_file = None;
            self.cancel_file_rename();
        }
    }

    fn cancel_renames(&mut self) {
        self.cancel_folder_rename();
        self.cancel_file_rename();
    }

    fn cancel_folder_rename(&mut self) {
        self.rename_folder = None;
        self.rename_draft.clear();
    }

    fn cancel_file_rename(&mut self) {
        self.rename_file = None;
        self.file_rename_draft.clear();
    }

    fn commit_rename(&mut self) {
        let Some(folder_id) = self.rename_folder.clone() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            self.cancel_folder_rename();
            return;
        };
        match rename_folder_on_disk(&folder_id, &self.rename_draft, &root_id) {
            Ok(renamed) => {
                let parent_id = Path::new(&renamed)
                    .parent()
                    .map(path_id)
                    .unwrap_or_else(|| root_id.clone());
                self.status = format!("Renamed to {}", folder_label(Path::new(&renamed)));
                self.selected_folder = renamed.clone();
                self.selected_file = None;
                self.expanded_folders.insert(root_id.clone());
                self.expanded_folders.insert(parent_id);
                self.cancel_folder_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn begin_file_rename_from_context(&mut self) {
        let Some(file_id) = self.context_file.take() else {
            return;
        };
        if let Some(file) = self
            .selected_folder()
            .files
            .iter()
            .find(|file| file.id == file_id)
        {
            self.file_rename_draft = file.name.clone();
            self.rename_file = Some(file_id);
            self.context_folder = None;
            self.cancel_folder_rename();
        }
    }

    fn commit_file_rename(&mut self) {
        let Some(file_id) = self.rename_file.clone() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            self.cancel_file_rename();
            return;
        };
        match rename_file_on_disk(&file_id, &self.file_rename_draft, &root_id) {
            Ok(renamed) => {
                self.status = format!("Renamed to {}", file_label(Path::new(&renamed)));
                self.selected_file = Some(renamed);
                self.context_file = None;
                self.cancel_file_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn delete_file_from_context(&mut self) {
        let Some(file_id) = self.context_file.take() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match delete_file_on_disk(&file_id, &root_id) {
            Ok(()) => {
                self.status = format!("Deleted {}", file_label(Path::new(&file_id)));
                self.selected_file = None;
                self.cancel_file_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    fn selected_file_label(&self) -> String {
        let Some(id) = self.selected_file.as_deref() else {
            return String::from("No file selected");
        };
        self.selected_folder()
            .files
            .iter()
            .find(|file| file.id == id)
            .map(|file| file.name.clone())
            .unwrap_or_else(|| String::from("No file selected"))
    }

    fn sort_by(&mut self, column_id: String) {
        if self.sort.column_id == column_id {
            self.sort.direction = self.sort.direction.toggled();
        } else {
            self.sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns
            .iter()
            .filter(|column| column.visible)
            .collect()
    }

    fn toggle_file_column(&mut self, column_id: String) {
        let visible_count = self
            .file_columns
            .iter()
            .filter(|column| column.visible)
            .count();
        let Some(column) = self
            .file_columns
            .iter_mut()
            .find(|column| column.id == column_id)
        else {
            return;
        };
        if column.id == "name" {
            self.status = String::from("Name column stays visible");
            return;
        }
        if column.visible && visible_count <= 1 {
            self.status = String::from("Keep at least one column visible");
            return;
        }
        column.visible = !column.visible;
        if !column.visible && self.sort.column_id == column.id {
            self.sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        }
        self.context_column = Some(column.id.clone());
    }

    fn reset_file_columns(&mut self) {
        self.file_columns = default_file_columns();
        self.sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        self.context_column = None;
        self.status = String::from("Reset file columns");
    }

    fn resize_file_column(&mut self, column_id: String, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position } => {
                if let Some(column) = self
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.column_resize = Some(ColumnResize {
                        column_id,
                        start_x: position.x,
                        start_width: column.width,
                    });
                }
            }
            ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                let Some(resize) = self.column_resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, ui::DragHandleMessage::Ended { .. }) {
                    self.column_resize = None;
                }
            }
        }
    }

    fn sorted_files(&self) -> Vec<&FileEntry> {
        let mut files = self.selected_folder().files.iter().collect::<Vec<_>>();
        files.sort_by(|a, b| {
            let ordering = match self.sort.column_id.as_str() {
                "size" => a
                    .size_bytes
                    .cmp(&b.size_bytes)
                    .then_with(|| a.name.cmp(&b.name)),
                "kind" => a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)),
                "modified" => a
                    .modified_rank
                    .cmp(&b.modified_rank)
                    .then_with(|| a.name.cmp(&b.name)),
                "extension" => file_extension(Path::new(&a.id))
                    .cmp(&file_extension(Path::new(&b.id)))
                    .then_with(|| a.name.cmp(&b.name)),
                "path" => a.id.cmp(&b.id),
                _ => natural_name_cmp(&a.name, &b.name),
            };
            match self.sort.direction {
                ui::SortDirection::Ascending => ordering,
                ui::SortDirection::Descending => ordering.reverse(),
            }
        });
        files
    }

    fn resize_tree(&mut self, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position }
            | ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                self.tree_width =
                    (position.x - SPLITTER_OFFSET).clamp(MIN_TREE_WIDTH, MAX_TREE_WIDTH);
            }
        }
    }

    fn handle_folder_drag(&mut self, source_id: String, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position } => {
                self.folder_drag = Some(FolderDrag {
                    source_id,
                    target_id: self.folder_drop_target_for_y(position.y),
                });
            }
            ui::DragHandleMessage::Moved { position } => {
                let target_id = self.folder_drop_target_for_y(position.y);
                if let Some(drag) = self.folder_drag.as_mut() {
                    drag.target_id = target_id;
                }
            }
            ui::DragHandleMessage::Ended { position } => {
                let source_id = self
                    .folder_drag
                    .as_ref()
                    .map(|drag| drag.source_id.clone())
                    .unwrap_or(source_id);
                let target_id = self.folder_drop_target_for_y(position.y).or_else(|| {
                    self.folder_drag
                        .as_ref()
                        .and_then(|drag| drag.target_id.clone())
                });
                self.folder_drag = None;
                if let Some(target_id) = target_id {
                    self.move_folder(source_id, target_id);
                }
            }
        }
    }

    fn folder_drop_target_for_y(&self, y: f32) -> Option<String> {
        let index = ((y - TREE_ROW_TOP) / TREE_ROW_HEIGHT).floor() as isize;
        if index < 0 {
            return None;
        }
        self.visible_folders()
            .get(index as usize)
            .map(|folder| folder.id.clone())
    }

    fn move_folder(&mut self, source_id: String, target_id: String) {
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match move_folder_on_disk(&source_id, &target_id, &root_id) {
            Ok(destination) => {
                self.status = format!(
                    "Moved {} into {}",
                    folder_label(Path::new(&source_id)),
                    folder_label(Path::new(&target_id))
                );
                self.selected_folder = destination.clone();
                self.selected_file = None;
                self.expanded_folders.insert(root_id.clone());
                self.expanded_folders.insert(target_id);
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn visible_folders(&self) -> Vec<VisibleFolder> {
        let mut folders = Vec::new();
        for folder in &self.folders {
            self.push_visible_folder(folder, 0, &mut folders);
        }
        folders
    }

    fn push_visible_folder(
        &self,
        folder: &FolderEntry,
        depth: usize,
        folders: &mut Vec<VisibleFolder>,
    ) {
        folders.push(VisibleFolder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            depth,
            has_children: folder.has_children(),
            expanded: self.is_expanded(&folder.id),
            selected: self.selected_folder == folder.id,
            drop_target: self
                .folder_drag
                .as_ref()
                .and_then(|drag| drag.target_id.as_ref())
                == Some(&folder.id),
            draggable: self.folders.first().is_none_or(|root| root.id != folder.id),
        });
        if self.is_expanded(&folder.id) {
            for child in &folder.children {
                self.push_visible_folder(child, depth + 1, folders);
            }
        }
    }

    #[cfg(test)]
    fn visible_folder_ids(&self) -> Vec<String> {
        self.visible_folders()
            .into_iter()
            .map(|folder| folder.id)
            .collect()
    }
}

fn main() -> radiant::Result {
    radiant::app(BrowserState::from_root(resolve_browser_root()))
        .title("Radiant Folder Browser")
        .size(900, 540)
        .min_size(640, 360)
        .view(view::project_surface)
        .run()
}

#[cfg(test)]
mod tests {
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
}
