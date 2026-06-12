use crate::model::{FileEntry, FolderEntry};
use std::path::Path;

use super::{
    file_extension, natural_name_cmp, validate_entry_name, validate_file_rename,
    validate_folder_rename,
};

pub(crate) fn move_folder_in_memory(
    folders: &mut Vec<FolderEntry>,
    source_id: &str,
    target_id: &str,
) -> Result<String, String> {
    let Some(root_id) = folders.first().map(|folder| folder.id.clone()) else {
        return Err(String::from("No resource root is loaded"));
    };
    validate_folder_move(folders, source_id, target_id, &root_id)?;
    let mut moved = take_folder(folders, source_id)
        .ok_or_else(|| String::from("Source folder no longer exists"))?;
    let destination_id = child_id(target_id, &moved.name);
    if find_folder(folders, &destination_id).is_some() {
        return Err(format!("{destination_id} already exists"));
    }
    let old_id = moved.id.clone();
    rewrite_folder_ids(&mut moved, &old_id, &destination_id);
    let target = find_folder_mut(folders, target_id)
        .ok_or_else(|| String::from("Target folder no longer exists"))?;
    target.children.push(moved);
    target
        .children
        .sort_by(|a, b| natural_name_cmp(&a.name, &b.name));
    Ok(destination_id)
}

pub(crate) fn create_child_folder_in_memory(
    folders: &mut [FolderEntry],
    parent_id: &str,
    base_name: &str,
) -> Result<String, String> {
    let parent =
        find_folder_mut(folders, parent_id).ok_or_else(|| String::from("Target folder missing"))?;
    let name = unique_folder_name(parent, base_name)?;
    let id = child_id(parent_id, &name);
    parent.children.push(FolderEntry {
        id: id.clone(),
        name,
        children: Vec::new(),
        files: Vec::new(),
    });
    parent
        .children
        .sort_by(|a, b| natural_name_cmp(&a.name, &b.name));
    Ok(id)
}

pub(crate) fn create_child_file_in_memory(
    folders: &mut [FolderEntry],
    parent_id: &str,
    base_name: &str,
) -> Result<String, String> {
    let parent =
        find_folder_mut(folders, parent_id).ok_or_else(|| String::from("Target folder missing"))?;
    let name = unique_file_name(parent, base_name)?;
    validate_entry_name(&name, "File")?;
    let id = child_id(parent_id, &name);
    parent.files.push(FileEntry {
        id: id.clone(),
        name,
        kind: file_kind_label(Path::new(&id)),
        size: String::from("0 B"),
        size_bytes: 0,
        modified: String::from("Today"),
        modified_rank: 0,
    });
    parent
        .files
        .sort_by(|a, b| natural_name_cmp(&a.name, &b.name));
    Ok(id)
}

pub(crate) fn rename_folder_in_memory(
    folders: &mut [FolderEntry],
    folder_id: &str,
    new_name: &str,
) -> Result<String, String> {
    let Some(root_id) = folders.first().map(|folder| folder.id.clone()) else {
        return Err(String::from("No resource root is loaded"));
    };
    validate_folder_rename(folders, folder_id, new_name, &root_id)?;
    let trimmed = new_name.trim();
    let parent_id = parent_id(folder_id).unwrap_or_else(|| root_id.clone());
    let destination_id = child_id(&parent_id, trimmed);
    if destination_id != folder_id && find_folder(folders, &destination_id).is_some() {
        return Err(format!("{destination_id} already exists"));
    }
    let folder = find_folder_mut(folders, folder_id)
        .ok_or_else(|| String::from("Folder no longer exists"))?;
    let old_id = folder.id.clone();
    folder.name = trimmed.to_owned();
    rewrite_folder_ids(folder, &old_id, &destination_id);
    Ok(destination_id)
}

pub(crate) fn rename_file_in_memory(
    folders: &mut [FolderEntry],
    file_id: &str,
    new_name: &str,
) -> Result<String, String> {
    let Some(root_id) = folders.first().map(|folder| folder.id.clone()) else {
        return Err(String::from("No resource root is loaded"));
    };
    validate_file_rename(folders, file_id, new_name, &root_id)?;
    let trimmed = new_name.trim();
    let Some(parent_id) = parent_id(file_id) else {
        return Err(String::from("Cannot rename unnamed file"));
    };
    let destination_id = child_id(&parent_id, trimmed);
    let folder = find_folder_mut(folders, &parent_id)
        .ok_or_else(|| String::from("Folder no longer exists"))?;
    if destination_id != file_id && folder.files.iter().any(|file| file.id == destination_id) {
        return Err(format!("{destination_id} already exists"));
    }
    let file = folder
        .files
        .iter_mut()
        .find(|file| file.id == file_id)
        .ok_or_else(|| String::from("File no longer exists"))?;
    file.id = destination_id.clone();
    file.name = trimmed.to_owned();
    file.kind = file_kind_label(Path::new(&destination_id));
    Ok(destination_id)
}

pub(crate) fn delete_file_in_memory(
    folders: &mut [FolderEntry],
    file_id: &str,
) -> Result<(), String> {
    let Some(parent_id) = parent_id(file_id) else {
        return Err(String::from("Cannot delete unnamed file"));
    };
    let folder = find_folder_mut(folders, &parent_id)
        .ok_or_else(|| String::from("Folder no longer exists"))?;
    let Some(index) = folder.files.iter().position(|file| file.id == file_id) else {
        return Err(String::from("File no longer exists"));
    };
    folder.files.remove(index);
    Ok(())
}

fn unique_folder_name(parent: &FolderEntry, base_name: &str) -> Result<String, String> {
    for index in 0..100 {
        let name = if index == 0 {
            base_name.to_owned()
        } else {
            format!("{base_name} {index}")
        };
        validate_entry_name(&name, "Folder")?;
        if parent.children.iter().all(|child| child.name != name) {
            return Ok(name);
        }
    }
    Err(String::from("No available New Folder name"))
}

fn unique_file_name(parent: &FolderEntry, base_name: &str) -> Result<String, String> {
    for index in 0..100 {
        let name = numbered_file_name(base_name, index);
        if parent.files.iter().all(|file| file.name != name) {
            return Ok(name);
        }
    }
    Err(String::from("No available New File name"))
}

fn numbered_file_name(base_name: &str, index: usize) -> String {
    if index == 0 {
        return base_name.to_owned();
    }
    let path = Path::new(base_name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| base_name.to_owned());
    path.extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .filter(|extension| !extension.is_empty())
        .map_or_else(
            || format!("{stem} {index}"),
            |extension| format!("{stem} {index}.{extension}"),
        )
}

fn validate_folder_move(
    folders: &[FolderEntry],
    source_id: &str,
    target_id: &str,
    root_id: &str,
) -> Result<(), String> {
    if source_id == root_id {
        return Err(String::from("Cannot move the root folder"));
    }
    if source_id == target_id {
        return Err(String::from("Cannot move a folder into itself"));
    }
    if is_descendant_id(target_id, source_id) {
        return Err(String::from(
            "Cannot move a folder into one of its descendants",
        ));
    }
    if !is_descendant_id(source_id, root_id) || !is_descendant_id(target_id, root_id) {
        return Err(String::from("Move must stay inside the resource root"));
    }
    if find_folder(folders, source_id).is_none() {
        return Err(String::from("Source folder no longer exists"));
    }
    if find_folder(folders, target_id).is_none() {
        return Err(String::from("Target folder no longer exists"));
    }
    Ok(())
}

fn file_kind_label(path: &Path) -> String {
    match file_extension(path).to_ascii_lowercase().as_str() {
        "wav" | "aiff" | "flac" | "mp3" => String::from("Audio"),
        "rs" | "toml" | "md" | "txt" | "json" => String::from("Text"),
        "png" | "jpg" | "jpeg" | "svg" => String::from("Image"),
        _ => String::from("File"),
    }
}

fn find_folder<'a>(folders: &'a [FolderEntry], id: &str) -> Option<&'a FolderEntry> {
    folders.iter().find_map(|folder| folder.find(id))
}

fn find_folder_mut<'a>(folders: &'a mut [FolderEntry], id: &str) -> Option<&'a mut FolderEntry> {
    folders
        .iter_mut()
        .find_map(|folder| find_folder_mut_in(folder, id))
}

fn find_folder_mut_in<'a>(folder: &'a mut FolderEntry, id: &str) -> Option<&'a mut FolderEntry> {
    if folder.id == id {
        return Some(folder);
    }
    folder
        .children
        .iter_mut()
        .find_map(|child| find_folder_mut_in(child, id))
}

fn take_folder(folders: &mut Vec<FolderEntry>, id: &str) -> Option<FolderEntry> {
    let index = folders.iter().position(|folder| folder.id == id);
    if let Some(index) = index {
        return Some(folders.remove(index));
    }
    folders
        .iter_mut()
        .find_map(|folder| take_child_folder(&mut folder.children, id))
}

fn take_child_folder(children: &mut Vec<FolderEntry>, id: &str) -> Option<FolderEntry> {
    let index = children.iter().position(|folder| folder.id == id);
    if let Some(index) = index {
        return Some(children.remove(index));
    }
    children
        .iter_mut()
        .find_map(|child| take_child_folder(&mut child.children, id))
}

fn rewrite_folder_ids(folder: &mut FolderEntry, old_prefix: &str, new_prefix: &str) {
    folder.id = replace_id_prefix(&folder.id, old_prefix, new_prefix);
    for file in &mut folder.files {
        file.id = replace_id_prefix(&file.id, old_prefix, new_prefix);
    }
    for child in &mut folder.children {
        rewrite_folder_ids(child, old_prefix, new_prefix);
    }
}

fn replace_id_prefix(id: &str, old_prefix: &str, new_prefix: &str) -> String {
    id.strip_prefix(old_prefix)
        .map_or_else(|| id.to_owned(), |suffix| format!("{new_prefix}{suffix}"))
}

fn parent_id(id: &str) -> Option<String> {
    id.rsplit_once('/').map(|(parent, _)| parent.to_owned())
}

fn child_id(parent_id: &str, child_name: &str) -> String {
    format!("{}/{}", parent_id.trim_end_matches('/'), child_name)
}

fn is_descendant_id(id: &str, root_id: &str) -> bool {
    id == root_id || id.starts_with(&format!("{root_id}/"))
}
