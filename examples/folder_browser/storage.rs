#[path = "storage/demo.rs"]
mod demo;
#[path = "storage/resource_graph.rs"]
mod resource_graph;
#[path = "storage/scan.rs"]
mod scan;
#[path = "storage/validation.rs"]
mod validation;

use super::{FolderEntry, ROOT_ENV_VAR};
use std::path::{Path, PathBuf};

pub(super) use demo::demo_root_folder;
pub(super) use resource_graph::{
    create_child_file_in_memory, create_child_folder_in_memory, delete_file_in_memory,
    move_folder_in_memory, rename_file_in_memory, rename_folder_in_memory,
};
pub(super) use scan::{
    file_extension, file_label, folder_label, load_folder, natural_name_cmp, path_id,
};
pub(super) use validation::{validate_entry_name, validate_file_rename, validate_folder_rename};

pub(super) fn resolve_browser_root() -> Option<PathBuf> {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(ROOT_ENV_VAR).map(PathBuf::from))
}

pub(super) fn load_root_folder(root: &Path) -> FolderEntry {
    load_folder(root, 0).unwrap_or_else(|| FolderEntry {
        id: path_id(root),
        name: root.display().to_string(),
        children: Vec::new(),
        files: Vec::new(),
    })
}
