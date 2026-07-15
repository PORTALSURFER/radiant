use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::*;
use radiant::application::{DetailsSort, SortDirection};

#[derive(Clone, Debug)]
pub(super) struct BrowserState {
    pub(super) selection: BrowserSelection,
    pub(super) tree: BrowserTreeState,
    pub(super) context: BrowserContextState,
    pub(super) rename: BrowserRenameState,
    pub(super) columns: BrowserColumnState,
    pub(super) folders: Vec<FolderEntry>,
    pub(super) status: String,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserSelection {
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserTreeState {
    pub(super) expanded_folders: HashSet<String>,
    pub(super) folder_drag: Option<FolderDrag>,
    pub(super) tree_width: f32,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserContextState {
    pub(super) context_folder: Option<String>,
    pub(super) context_file: Option<String>,
    pub(super) context_position: Option<radiant::layout::Point>,
    pub(super) context_column: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserRenameState {
    pub(super) folder: Option<String>,
    pub(super) folder_draft: String,
    pub(super) file: Option<String>,
    pub(super) file_draft: String,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserColumnState {
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) sort: DetailsSort,
    pub(super) resize: Option<ColumnResize>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::from_demo()
    }
}

impl BrowserState {
    pub(super) fn from_demo() -> Self {
        Self::from_folder(
            demo_root_folder(),
            String::from("In-memory resource sandbox"),
        )
    }

    pub(super) fn from_root(root: &Path) -> Self {
        let root_folder = placeholder_root_folder(root);
        Self::from_folder(
            root_folder,
            format!("Loading resource view: {}", root.display()),
        )
    }

    fn from_folder(root_folder: FolderEntry, status: String) -> Self {
        let root_id = root_folder.id.clone();
        Self {
            selection: BrowserSelection {
                selected_folder: root_id.clone(),
                selected_file: None,
            },
            tree: BrowserTreeState {
                expanded_folders: [root_id].into_iter().collect(),
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
                sort: DetailsSort::new("name", SortDirection::Ascending),
                resize: None,
            },
            folders: vec![root_folder],
            status,
        }
    }

    pub(super) fn apply_root_folder(&mut self, root: PathBuf, root_folder: FolderEntry) {
        let root_id = root_folder.id.clone();
        self.selection.selected_folder = root_id.clone();
        self.selection.selected_file = None;
        self.tree.expanded_folders = [root_id].into_iter().collect();
        self.context.context_folder = None;
        self.context.context_file = None;
        self.context.context_column = None;
        self.context.context_position = None;
        self.rename.folder = None;
        self.rename.folder_draft.clear();
        self.rename.file = None;
        self.rename.file_draft.clear();
        self.folders = vec![root_folder];
        self.status = format!("Read-only resource view: {}", root.display());
    }

    pub(super) fn selected_folder(&self) -> &FolderEntry {
        self.find_folder(&self.selection.selected_folder)
            .unwrap_or(&self.folders[0])
    }

    pub(super) fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    pub(super) fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    pub(super) fn is_expanded(&self, id: &str) -> bool {
        self.tree.expanded_folders.contains(id)
    }

    pub(super) fn selected_file_label(&self) -> String {
        let Some(id) = self.selection.selected_file.as_deref() else {
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
        self.columns
            .file_columns
            .iter()
            .filter(|column| column.visible)
            .collect()
    }

    pub(super) fn sorted_files(&self) -> Vec<&FileEntry> {
        let mut files = self.selected_folder().files.iter().collect::<Vec<_>>();
        files.sort_by(|a, b| {
            let ordering = match self.columns.sort.column_id.as_str() {
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
            self.columns.sort.direction.apply_ordering(ordering)
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
            selected: self.selection.selected_folder == folder.id,
            drop_target: self
                .tree
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

fn placeholder_root_folder(root: &Path) -> FolderEntry {
    FolderEntry {
        id: path_id(root),
        name: folder_label(root),
        children: Vec::new(),
        files: Vec::new(),
    }
}
