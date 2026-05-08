//! Folder browser with an expandable tree, details list, and resizable panes.

use radiant::prelude as ui;
use std::path::{Path, PathBuf};

#[path = "folder_browser/columns.rs"]
mod columns;
#[path = "folder_browser/menu_geometry.rs"]
mod menu_geometry;
#[path = "folder_browser/model.rs"]
mod model;
#[path = "folder_browser/state.rs"]
mod state;
#[path = "folder_browser/storage.rs"]
mod storage;
#[path = "folder_browser/view.rs"]
mod view;
use columns::*;
use menu_geometry::*;
use model::*;
use state::BrowserState;
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

impl BrowserState {
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

    fn sort_by(&mut self, column_id: String) {
        if self.sort.column_id == column_id {
            self.sort.direction = self.sort.direction.toggled();
        } else {
            self.sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
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
#[path = "folder_browser/tests.rs"]
mod tests;
