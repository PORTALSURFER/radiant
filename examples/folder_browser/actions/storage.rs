use std::path::Path;

use super::super::*;

impl BrowserState {
    pub(crate) fn create_folder_from_context(&mut self) {
        let Some(parent_id) = self.context.context_folder.take() else {
            return;
        };
        match create_child_folder_in_memory(&mut self.folders, &parent_id, "New Folder") {
            Ok(created) => {
                self.status = format!("Created {}", folder_label(Path::new(&created)));
                self.selection.selected_folder = created;
                self.selection.selected_file = None;
                self.tree.expanded_folders.insert(parent_id);
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    pub(crate) fn create_file_in_selected_folder(&mut self) {
        match create_child_file_in_memory(
            &mut self.folders,
            &self.selection.selected_folder,
            "New File.txt",
        ) {
            Ok(created) => {
                self.status = format!("Created {}", file_label(Path::new(&created)));
                self.selection.selected_file = Some(created.clone());
                self.context.context_file = None;
                self.context.context_folder = None;
                self.cancel_renames();
                self.rename.file = Some(created);
                self.rename.file_draft = String::from("New File.txt");
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
        match rename_folder_in_memory(&mut self.folders, &folder_id, &self.rename.folder_draft) {
            Ok(renamed) => {
                let parent_id = Path::new(&renamed)
                    .parent()
                    .map(path_id)
                    .unwrap_or_else(|| renamed.clone());
                self.status = format!("Renamed to {}", folder_label(Path::new(&renamed)));
                self.selection.selected_folder = renamed.clone();
                self.selection.selected_file = None;
                self.tree.expanded_folders.insert(parent_id);
                self.cancel_folder_rename();
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
        match rename_file_in_memory(&mut self.folders, &file_id, &self.rename.file_draft) {
            Ok(renamed) => {
                self.status = format!("Renamed to {}", file_label(Path::new(&renamed)));
                self.selection.selected_file = Some(renamed);
                self.context.context_file = None;
                self.cancel_file_rename();
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
        match delete_file_in_memory(&mut self.folders, &file_id) {
            Ok(()) => {
                self.status = format!("Deleted {}", file_label(Path::new(&file_id)));
                self.selection.selected_file = None;
                self.cancel_file_rename();
            }
            Err(message) => {
                self.status = message;
            }
        }
    }
}
