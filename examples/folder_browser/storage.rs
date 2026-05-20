#[path = "storage/scan.rs"]
mod scan;
#[path = "storage/validation.rs"]
mod validation;

use super::{FolderEntry, ROOT_ENV_VAR};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) use scan::{
    file_extension, file_label, folder_label, load_folder, natural_name_cmp, path_id,
};
use validation::validate_entry_name;
pub(super) use validation::{validate_file_rename, validate_folder_move, validate_folder_rename};

pub(super) fn temp_root() -> PathBuf {
    std::env::temp_dir().join("radiant-folder-browser-demo")
}

pub(super) fn resolve_browser_root() -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(ROOT_ENV_VAR).map(PathBuf::from))
        .unwrap_or_else(temp_root)
}

pub(super) fn load_root_folder(root: PathBuf) -> FolderEntry {
    let _ = fs::create_dir_all(&root);
    load_folder(&root, 0).unwrap_or_else(|| FolderEntry {
        id: path_id(&root),
        name: root.display().to_string(),
        children: Vec::new(),
        files: Vec::new(),
    })
}

pub(super) fn move_folder_on_disk(
    source_id: &str,
    target_id: &str,
    root_id: &str,
) -> Result<String, String> {
    let source = PathBuf::from(source_id);
    let target = PathBuf::from(target_id);
    let root = PathBuf::from(root_id);
    validate_folder_move(&source, &target, &root)?;
    let name = source
        .file_name()
        .ok_or_else(|| String::from("Cannot move unnamed folder"))?;
    let destination = target.join(name);
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Move failed: {error}"))
        .map(|_| path_id(&destination))
}

pub(super) fn create_child_folder(parent_id: &str, base_name: &str) -> Result<String, String> {
    let parent = PathBuf::from(parent_id);
    if !parent.is_dir() {
        return Err(String::from("Target folder no longer exists"));
    }
    for index in 0..100 {
        let name = if index == 0 {
            base_name.to_owned()
        } else {
            format!("{base_name} {index}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            fs::create_dir(&candidate).map_err(|error| format!("Create folder failed: {error}"))?;
            return Ok(path_id(&candidate));
        }
    }
    Err(String::from("No available New Folder name"))
}

pub(super) fn create_child_file(
    parent_id: &str,
    base_name: &str,
    root_id: &str,
) -> Result<String, String> {
    let parent = PathBuf::from(parent_id);
    let root = PathBuf::from(root_id);
    if !parent.starts_with(&root) {
        return Err(String::from("Create must stay inside the browser root"));
    }
    if !parent.is_dir() {
        return Err(String::from("Target folder no longer exists"));
    }
    for index in 0..100 {
        let name = unique_file_name(base_name, index);
        validate_entry_name(&name, "File")?;
        let candidate = parent.join(name);
        if !candidate.exists() {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
                .map_err(|error| format!("Create file failed: {error}"))?;
            return Ok(path_id(&candidate));
        }
    }
    Err(String::from("No available New File name"))
}

fn unique_file_name(base_name: &str, index: usize) -> String {
    if index == 0 {
        return base_name.to_owned();
    }
    let path = Path::new(base_name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| base_name.to_owned());
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .filter(|extension| !extension.is_empty());
    match extension {
        Some(extension) => format!("{stem} {index}.{extension}"),
        None => format!("{stem} {index}"),
    }
}

pub(super) fn rename_folder_on_disk(
    folder_id: &str,
    new_name: &str,
    root_id: &str,
) -> Result<String, String> {
    let source = PathBuf::from(folder_id);
    let root = PathBuf::from(root_id);
    validate_folder_rename(&source, new_name, &root)?;
    let parent = source
        .parent()
        .ok_or_else(|| String::from("Cannot rename unnamed folder"))?;
    let destination = parent.join(new_name.trim());
    if destination == source {
        return Ok(path_id(&source));
    }
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Rename failed: {error}"))
        .map(|_| path_id(&destination))
}

pub(super) fn rename_file_on_disk(
    file_id: &str,
    new_name: &str,
    root_id: &str,
) -> Result<String, String> {
    let source = PathBuf::from(file_id);
    let root = PathBuf::from(root_id);
    validate_file_rename(&source, new_name, &root)?;
    let parent = source
        .parent()
        .ok_or_else(|| String::from("Cannot rename unnamed file"))?;
    let destination = parent.join(new_name.trim());
    if destination == source {
        return Ok(path_id(&source));
    }
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Rename failed: {error}"))
        .map(|_| path_id(&destination))
}

pub(super) fn delete_file_on_disk(file_id: &str, root_id: &str) -> Result<(), String> {
    let target = PathBuf::from(file_id);
    let root = PathBuf::from(root_id);
    if !target.starts_with(&root) {
        return Err(String::from("Delete must stay inside the browser root"));
    }
    if !target.exists() {
        return Err(String::from("File no longer exists"));
    }
    if !target.is_file() {
        return Err(String::from("Only files can be deleted here"));
    }
    fs::remove_file(&target).map_err(|error| format!("Delete failed: {error}"))
}
