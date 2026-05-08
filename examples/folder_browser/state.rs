use std::{collections::HashSet, path::PathBuf};

use super::*;

#[derive(Clone, Debug)]
pub(super) struct BrowserState {
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
    pub(super) expanded_folders: HashSet<String>,
    pub(super) folder_drag: Option<FolderDrag>,
    pub(super) context_folder: Option<String>,
    pub(super) context_file: Option<String>,
    pub(super) context_position: Option<radiant::layout::Point>,
    pub(super) rename_folder: Option<String>,
    pub(super) rename_draft: String,
    pub(super) rename_file: Option<String>,
    pub(super) file_rename_draft: String,
    pub(super) context_column: Option<String>,
    pub(super) column_resize: Option<ColumnResize>,
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) sort: ui::DetailsSort,
    pub(super) tree_width: f32,
    pub(super) folders: Vec<FolderEntry>,
    pub(super) status: String,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::from_root(temp_root())
    }
}

impl BrowserState {
    pub(super) fn from_root(root: PathBuf) -> Self {
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

    pub(super) fn selected_folder(&self) -> &FolderEntry {
        self.find_folder(&self.selected_folder)
            .unwrap_or(&self.folders[0])
    }

    pub(super) fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    pub(super) fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    pub(super) fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    pub(super) fn selected_file_label(&self) -> String {
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

    pub(super) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns
            .iter()
            .filter(|column| column.visible)
            .collect()
    }

    pub(super) fn sorted_files(&self) -> Vec<&FileEntry> {
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

    pub(super) fn visible_folders(&self) -> Vec<VisibleFolder> {
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
    pub(super) fn visible_folder_ids(&self) -> Vec<String> {
        self.visible_folders()
            .into_iter()
            .map(|folder| folder.id)
            .collect()
    }
}
