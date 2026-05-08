use super::{FileEntry, FolderEntry, MAX_CHILD_FOLDERS, MAX_SCAN_DEPTH, ROOT_ENV_VAR};
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

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

pub(super) fn validate_folder_rename(
    source: &Path,
    new_name: &str,
    root: &Path,
) -> Result<(), String> {
    if source == root {
        return Err(String::from("Cannot rename the root folder"));
    }
    if !source.starts_with(root) {
        return Err(String::from("Rename must stay inside the browser root"));
    }
    if !source.is_dir() {
        return Err(String::from("Folder no longer exists"));
    }
    validate_entry_name(new_name, "Folder")
}

pub(super) fn validate_file_rename(
    source: &Path,
    new_name: &str,
    root: &Path,
) -> Result<(), String> {
    if source == root {
        return Err(String::from("Cannot rename the root folder"));
    }
    if !source.starts_with(root) {
        return Err(String::from("Rename must stay inside the browser root"));
    }
    if !source.exists() {
        return Err(String::from("File no longer exists"));
    }
    validate_entry_name(new_name, "File")
}

fn validate_entry_name(new_name: &str, kind: &str) -> Result<(), String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{kind} name cannot be empty"));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(format!("{kind} name is reserved"));
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err(format!("{kind} name cannot end with a dot or space"));
    }
    if trimmed
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err(format!("{kind} name contains invalid characters"));
    }
    Ok(())
}

pub(super) fn validate_folder_move(
    source: &Path,
    target: &Path,
    root: &Path,
) -> Result<(), String> {
    if source == root {
        return Err(String::from("Cannot move the root folder"));
    }
    if source == target {
        return Err(String::from("Cannot move a folder into itself"));
    }
    if target.starts_with(source) {
        return Err(String::from(
            "Cannot move a folder into one of its descendants",
        ));
    }
    if !source.starts_with(root) || !target.starts_with(root) {
        return Err(String::from("Move must stay inside the browser root"));
    }
    if !source.is_dir() {
        return Err(String::from("Source folder no longer exists"));
    }
    if !target.is_dir() {
        return Err(String::from("Target folder no longer exists"));
    }
    Ok(())
}

fn load_folder(path: &Path, depth: usize) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| load_folder(entry, depth + 1))
        .collect::<Vec<_>>();
    let files = entries.iter().map(file_entry).collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn read_sorted_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| natural_name_cmp(&file_label(a), &file_label(b)));
    entries
}

fn file_entry(path: &PathBuf) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    let modified = metadata.and_then(|metadata| metadata.modified().ok());
    let kind = if path.is_dir() {
        String::from("Folder")
    } else {
        file_kind(path)
    };
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        kind,
        size: if path.is_dir() {
            String::from("-")
        } else {
            format_size(size_bytes)
        },
        size_bytes,
        modified: modified_label(modified),
        modified_rank: modified_rank(modified),
    }
}

pub(super) fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(super) fn folder_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_kind(path: &Path) -> String {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return String::from("File");
    };
    match extension.to_ascii_lowercase().as_str() {
        "wav" | "aiff" | "flac" | "mp3" => String::from("Audio"),
        "rs" | "toml" | "md" | "txt" | "json" => String::from("Text"),
        "png" | "jpg" | "jpeg" | "svg" => String::from("Image"),
        _ => String::from("File"),
    }
}

pub(super) fn file_extension(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| String::from("-"))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}

fn modified_label(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else {
        return String::from("-");
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    let days = age.as_secs() / 86_400;
    if days == 0 {
        String::from("Today")
    } else if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    }
}

fn modified_rank(modified: Option<SystemTime>) -> u64 {
    modified
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .unwrap_or(u64::MAX)
}

pub(super) fn natural_name_cmp(a: &str, b: &str) -> Ordering {
    a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
}
