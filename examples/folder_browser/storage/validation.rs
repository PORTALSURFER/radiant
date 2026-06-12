use crate::model::FolderEntry;

pub(crate) fn validate_folder_rename(
    folders: &[FolderEntry],
    source_id: &str,
    new_name: &str,
    root_id: &str,
) -> Result<(), String> {
    if source_id == root_id {
        return Err(String::from("Cannot rename the root folder"));
    }
    if !is_descendant_id(source_id, root_id) {
        return Err(String::from("Rename must stay inside the resource root"));
    }
    if find_folder(folders, source_id).is_none() {
        return Err(String::from("Folder no longer exists"));
    }
    validate_entry_name(new_name, "Folder")
}

pub(crate) fn validate_file_rename(
    folders: &[FolderEntry],
    source_id: &str,
    new_name: &str,
    root_id: &str,
) -> Result<(), String> {
    if source_id == root_id {
        return Err(String::from("Cannot rename the root folder"));
    }
    if !is_descendant_id(source_id, root_id) {
        return Err(String::from("Rename must stay inside the resource root"));
    }
    if find_file(folders, source_id).is_none() {
        return Err(String::from("File no longer exists"));
    }
    validate_entry_name(new_name, "File")
}

pub(crate) fn validate_entry_name(new_name: &str, kind: &str) -> Result<(), String> {
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

fn find_folder<'a>(folders: &'a [FolderEntry], id: &str) -> Option<&'a FolderEntry> {
    folders.iter().find_map(|folder| folder.find(id))
}

fn find_file<'a>(folders: &'a [FolderEntry], id: &str) -> Option<&'a crate::model::FileEntry> {
    folders.iter().find_map(|folder| find_file_in(folder, id))
}

fn find_file_in<'a>(folder: &'a FolderEntry, id: &str) -> Option<&'a crate::model::FileEntry> {
    folder.files.iter().find(|file| file.id == id).or_else(|| {
        folder
            .children
            .iter()
            .find_map(|child| find_file_in(child, id))
    })
}

fn is_descendant_id(id: &str, root_id: &str) -> bool {
    id == root_id || id.starts_with(&format!("{root_id}/"))
}
