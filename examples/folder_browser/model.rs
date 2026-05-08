//! Data model types for the folder browser example.

#[derive(Clone, Debug)]
pub(super) struct FolderEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) children: Vec<FolderEntry>,
    pub(super) files: Vec<FileEntry>,
}

impl FolderEntry {
    pub(super) fn find(&self, id: &str) -> Option<&FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    pub(super) fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Clone, Debug)]
pub(super) struct FileEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) size: String,
    pub(super) size_bytes: u64,
    pub(super) modified: String,
    pub(super) modified_rank: u64,
}

#[derive(Clone, Debug)]
pub(super) struct FolderDrag {
    pub(super) source_id: String,
    pub(super) target_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ColumnResize {
    pub(super) column_id: String,
    pub(super) start_x: f32,
    pub(super) start_width: f32,
}

#[derive(Clone, Debug)]
pub(super) struct VisibleFolder {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) depth: usize,
    pub(super) has_children: bool,
    pub(super) expanded: bool,
    pub(super) selected: bool,
    pub(super) drop_target: bool,
    pub(super) draggable: bool,
}
