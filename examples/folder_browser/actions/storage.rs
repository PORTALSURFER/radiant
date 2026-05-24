use std::path::{Path, PathBuf};

use super::super::*;

impl BrowserState {
    pub(crate) fn create_folder_from_context(&mut self) {
        let Some(parent_id) = self.context.context_folder.take() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match create_child_folder(&parent_id, "New Folder") {
            Ok(created) => {
                self.status = format!("Created {}", folder_label(Path::new(&created)));
                self.selection.selected_folder = created;
                self.selection.selected_file = None;
                self.tree.expanded_folders.insert(root_id.clone());
                self.tree.expanded_folders.insert(parent_id);
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    pub(crate) fn create_file_in_selected_folder(&mut self) {
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match create_child_file(&self.selection.selected_folder, "New File.txt", &root_id) {
            Ok(created) => {
                self.status = format!("Created {}", file_label(Path::new(&created)));
                self.selection.selected_file = Some(created.clone());
                self.context.context_file = None;
                self.context.context_folder = None;
                self.cancel_renames();
                self.rename.file = Some(created);
                self.rename.file_draft = String::from("New File.txt");
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    pub(crate) fn begin_rename_from_context(&mut self) {
        let Some(folder_id) = self.context.context_folder.take() else {
            return;
        };
        if let Some(folder) = self.find_folder(&folder_id) {
            self.rename.folder_draft = folder.name.clone();
            self.rename.folder = Some(folder_id);
            self.selection.selected_file = None;
            self.context.context_file = None;
            self.cancel_file_rename();
        }
    }

    pub(crate) fn commit_rename(&mut self) {
        let Some(folder_id) = self.rename.folder.clone() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            self.cancel_folder_rename();
            return;
        };
        match rename_folder_on_disk(&folder_id, &self.rename.folder_draft, &root_id) {
            Ok(renamed) => {
                let parent_id = Path::new(&renamed)
                    .parent()
                    .map(path_id)
                    .unwrap_or_else(|| root_id.clone());
                self.status = format!("Renamed to {}", folder_label(Path::new(&renamed)));
                self.selection.selected_folder = renamed.clone();
                self.selection.selected_file = None;
                self.tree.expanded_folders.insert(root_id.clone());
                self.tree.expanded_folders.insert(parent_id);
                self.cancel_folder_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    pub(crate) fn begin_file_rename_from_context(&mut self) {
        let Some(file_id) = self.context.context_file.take() else {
            return;
        };
        if let Some(file) = self
            .selected_folder()
            .files
            .iter()
            .find(|file| file.id == file_id)
        {
            self.rename.file_draft = file.name.clone();
            self.rename.file = Some(file_id);
            self.context.context_folder = None;
            self.cancel_folder_rename();
        }
    }

    pub(crate) fn commit_file_rename(&mut self) {
        let Some(file_id) = self.rename.file.clone() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            self.cancel_file_rename();
            return;
        };
        match rename_file_on_disk(&file_id, &self.rename.file_draft, &root_id) {
            Ok(renamed) => {
                self.status = format!("Renamed to {}", file_label(Path::new(&renamed)));
                self.selection.selected_file = Some(renamed);
                self.context.context_file = None;
                self.cancel_file_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    pub(crate) fn delete_file_from_context(&mut self) {
        let Some(file_id) = self.context.context_file.take() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match delete_file_on_disk(&file_id, &root_id) {
            Ok(()) => {
                self.status = format!("Deleted {}", file_label(Path::new(&file_id)));
                self.selection.selected_file = None;
                self.cancel_file_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }
}
