use std::path::Path;

pub(crate) fn validate_folder_rename(
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

pub(crate) fn validate_file_rename(
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

pub(super) fn validate_entry_name(new_name: &str, kind: &str) -> Result<(), String> {
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

pub(crate) fn validate_folder_move(
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
