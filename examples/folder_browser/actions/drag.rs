use std::path::{Path, PathBuf};

use super::super::*;

impl BrowserState {
    pub(crate) fn handle_folder_drag(&mut self, source_id: String, message: ui::DragHandleMessage) {
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
