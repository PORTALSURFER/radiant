//! Folder browser for `C:\temp` with an expandable tree, details list, and resizable panes.

use radiant::prelude as ui;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const MIN_TREE_WIDTH: f32 = 190.0;
const MAX_TREE_WIDTH: f32 = 430.0;
const SPLITTER_OFFSET: f32 = 24.0;
const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_ROW_TOP: f32 = 104.0;

#[derive(Clone, Debug)]
struct FolderEntry {
    id: String,
    name: String,
    children: Vec<FolderEntry>,
    files: Vec<FileEntry>,
}

#[derive(Clone, Debug)]
struct FileEntry {
    id: String,
    name: String,
    kind: String,
    size: String,
    size_bytes: u64,
    modified: String,
    modified_rank: u64,
}

#[derive(Clone, Debug)]
struct BrowserState {
    selected_folder: String,
    selected_file: Option<String>,
    expanded_folders: HashSet<String>,
    folder_drag: Option<FolderDrag>,
    context_folder: Option<String>,
    rename_folder: Option<String>,
    rename_draft: String,
    sort: ui::DetailsSort,
    tree_width: f32,
    folders: Vec<FolderEntry>,
    status: String,
}

#[derive(Clone, Debug)]
struct FolderDrag {
    source_id: String,
    target_id: Option<String>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::from_root(temp_root())
    }
}

impl BrowserState {
    fn from_root(root: PathBuf) -> Self {
        let root_folder = load_root_folder(root);
        let root_id = root_folder.id.clone();
        Self {
            selected_folder: root_id.clone(),
            selected_file: None,
            expanded_folders: [root_id].into_iter().collect(),
            folder_drag: None,
            context_folder: None,
            rename_folder: None,
            rename_draft: String::new(),
            sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            tree_width: 300.0,
            folders: vec![root_folder],
            status: String::from("Drag a folder handle onto another folder"),
        }
    }

    fn selected_folder(&self) -> &FolderEntry {
        self.find_folder(&self.selected_folder)
            .unwrap_or(&self.folders[0])
    }

    fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    fn select_folder(&mut self, id: impl Into<String>) {
        self.selected_folder = id.into();
        self.selected_file = None;
        self.cancel_rename();
    }

    fn activate_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.context_folder = None;
        self.cancel_rename();
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    fn toggle_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.folder_has_children(&id) && !self.expanded_folders.remove(&id) {
            self.expanded_folders.insert(id);
        }
    }

    fn select_file_id(&mut self, id: String) {
        self.selected_file = Some(id);
        self.context_folder = None;
        self.cancel_rename();
    }

    fn open_context_menu(&mut self, id: String) {
        self.context_folder = Some(id);
        self.cancel_rename();
    }

    fn close_context_menu(&mut self) {
        self.context_folder = None;
    }

    fn create_folder_from_context(&mut self) {
        let Some(parent_id) = self.context_folder.take() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            return;
        };
        match create_child_folder(&parent_id, "New Folder") {
            Ok(created) => {
                self.status = format!("Created {}", folder_label(Path::new(&created)));
                self.selected_folder = created;
                self.selected_file = None;
                self.expanded_folders.insert(root_id.clone());
                self.expanded_folders.insert(parent_id);
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn begin_rename_from_context(&mut self) {
        let Some(folder_id) = self.context_folder.take() else {
            return;
        };
        if let Some(folder) = self.find_folder(&folder_id) {
            self.rename_draft = folder.name.clone();
            self.rename_folder = Some(folder_id);
            self.selected_file = None;
        }
    }

    fn cancel_rename(&mut self) {
        self.rename_folder = None;
        self.rename_draft.clear();
    }

    fn commit_rename(&mut self) {
        let Some(folder_id) = self.rename_folder.clone() else {
            return;
        };
        let Some(root_id) = self.folders.first().map(|folder| folder.id.clone()) else {
            self.cancel_rename();
            return;
        };
        match rename_folder_on_disk(&folder_id, &self.rename_draft, &root_id) {
            Ok(renamed) => {
                let parent_id = Path::new(&renamed)
                    .parent()
                    .map(path_id)
                    .unwrap_or_else(|| root_id.clone());
                self.status = format!("Renamed to {}", folder_label(Path::new(&renamed)));
                self.selected_folder = renamed.clone();
                self.selected_file = None;
                self.expanded_folders.insert(root_id.clone());
                self.expanded_folders.insert(parent_id);
                self.cancel_rename();
                self.folders = vec![load_root_folder(PathBuf::from(root_id))];
            }
            Err(message) => {
                self.status = message;
            }
        }
    }

    fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    fn selected_file_label(&self) -> String {
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

    fn sort_by(&mut self, column_id: String) {
        if self.sort.column_id == column_id {
            self.sort.direction = self.sort.direction.toggled();
        } else {
            self.sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    fn sorted_files(&self) -> Vec<&FileEntry> {
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
                _ => natural_name_cmp(&a.name, &b.name),
            };
            match self.sort.direction {
                ui::SortDirection::Ascending => ordering,
                ui::SortDirection::Descending => ordering.reverse(),
            }
        });
        files
    }

    fn resize_tree(&mut self, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position }
            | ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                self.tree_width =
                    (position.x - SPLITTER_OFFSET).clamp(MIN_TREE_WIDTH, MAX_TREE_WIDTH);
            }
        }
    }

    fn handle_folder_drag(&mut self, source_id: String, message: ui::DragHandleMessage) {
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

    fn visible_folders(&self) -> Vec<VisibleFolder> {
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
    fn visible_folder_ids(&self) -> Vec<String> {
        self.visible_folders()
            .into_iter()
            .map(|folder| folder.id)
            .collect()
    }
}

#[derive(Clone, Debug)]
struct VisibleFolder {
    id: String,
    name: String,
    depth: usize,
    has_children: bool,
    expanded: bool,
    selected: bool,
    drop_target: bool,
    draggable: bool,
}

impl FolderEntry {
    fn find(&self, id: &str) -> Option<&FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

fn main() -> radiant::Result {
    radiant::app(BrowserState::default())
        .title("Radiant Folder Browser")
        .size(900, 540)
        .min_size(640, 360)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut BrowserState) -> ui::StateView<BrowserState> {
    ui::column([
        header(state),
        ui::row([folder_tree(state), splitter(), file_view(state)])
            .style(ui::WidgetStyle::default())
            .fill_width()
            .fill_height()
            .padding(8.0)
            .spacing(8.0),
    ])
    .fill_width()
    .fill_height()
    .padding(12.0)
    .spacing(8.0)
}

fn header(state: &BrowserState) -> ui::StateView<BrowserState> {
    ui::row([
        ui::text("C:\\temp browser").size(170.0, 24.0),
        ui::text(format!(
            "{} items in {}",
            state.selected_folder().files.len(),
            state.selected_folder().name
        ))
        .size(220.0, 24.0),
        ui::text(state.selected_file_label()).size(180.0, 24.0),
        ui::text(state.status.clone()).fill_width().height(24.0),
    ])
    .fill_width()
    .spacing(12.0)
}

fn folder_tree(state: &BrowserState) -> ui::StateView<BrowserState> {
    let tree = ui::scroll(
        ui::column(
            state
                .visible_folders()
                .into_iter()
                .map(|folder| folder_row(state, folder))
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(1.0),
    )
    .fill_height();
    let content = if let Some(folder_id) = state.context_folder.as_ref() {
        ui::column([context_menu(state, folder_id), tree])
            .fill_width()
            .fill_height()
            .spacing(6.0)
    } else {
        tree
    };
    panel("Folder Tree", content)
        .width(state.tree_width)
        .fill_height()
}

fn context_menu(state: &BrowserState, folder_id: &str) -> ui::StateView<BrowserState> {
    let folder_name = state
        .find_folder(folder_id)
        .map(|folder| folder.name.clone())
        .unwrap_or_else(|| String::from("folder"));
    ui::column([
        ui::text(format!("Actions for {folder_name}"))
            .fill_width()
            .height(22.0),
        ui::button("Rename")
            .primary()
            .on_click(BrowserState::begin_rename_from_context)
            .fill_width()
            .height(28.0),
        ui::button("New Folder")
            .subtle()
            .on_click(BrowserState::create_folder_from_context)
            .fill_width()
            .height(28.0),
        ui::button("Cancel")
            .subtle()
            .on_click(BrowserState::close_context_menu)
            .fill_width()
            .height(28.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .fill_width()
    .height(126.0)
    .padding(8.0)
    .spacing(6.0)
}

fn folder_row(state: &BrowserState, folder: VisibleFolder) -> ui::StateView<BrowserState> {
    let id = folder.id.clone();
    let key = folder.id.clone();
    let toggle_id = folder.id.clone();
    let drag_id = folder.id.clone();
    let editing = state.rename_folder.as_deref() == Some(folder.id.as_str());
    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let label = if editing {
        ui::row([
            ui::text_input(state.rename_draft.clone())
                .placeholder("Folder name")
                .bind_submit(
                    |state: &mut BrowserState| &mut state.rename_draft,
                    BrowserState::commit_rename,
                )
                .key(format!("folder-rename-input-{key}"))
                .fill_width()
                .height(22.0),
            ui::button("OK")
                .primary()
                .on_click(BrowserState::commit_rename)
                .key(format!("folder-rename-ok-{key}"))
                .size(36.0, 22.0),
            ui::button("X")
                .subtle()
                .on_click(BrowserState::cancel_rename)
                .key(format!("folder-rename-cancel-{key}"))
                .size(28.0, 22.0),
        ])
        .fill_width()
        .height(22.0)
        .spacing(3.0)
    } else {
        let select_id = id.clone();
        let context_id = id.clone();
        let mut label = ui::button(folder.name)
            .on_click_or_secondary(
                move |state: &mut BrowserState| state.activate_folder(select_id.clone()),
                move |state: &mut BrowserState| state.open_context_menu(context_id.clone()),
            )
            .key(format!("folder-label-{key}"))
            .fill_width()
            .height(22.0);
        if folder.selected || folder.drop_target {
            label = label.primary();
        } else {
            label = label.subtle();
        }
        label
    };

    ui::row([
        ui::text("").size((folder.depth as f32) * 12.0, 22.0),
        if folder.draggable {
            ui::drag_handle()
                .on_drag(move |state: &mut BrowserState, message| {
                    state.handle_folder_drag(drag_id.clone(), message);
                })
                .key(format!("folder-drag-{id}"))
                .size(22.0, 22.0)
        } else {
            ui::text("")
                .key(format!("folder-drag-spacer-{id}"))
                .size(22.0, 22.0)
        },
        if folder.has_children {
            ui::button(expander)
                .on_click(move |state: &mut BrowserState| state.toggle_folder(toggle_id.clone()))
                .key(format!("folder-toggle-{id}"))
                .size(32.0, 22.0)
                .subtle()
        } else {
            ui::text("")
                .key(format!("folder-toggle-spacer-{id}"))
                .size(32.0, 22.0)
        },
        label,
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.drop_target {
        ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        ui::WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

fn splitter() -> ui::StateView<BrowserState> {
    ui::column([
        ui::text("").fill_width().fill_height(),
        ui::drag_handle()
            .on_drag(|state: &mut BrowserState, message| state.resize_tree(message))
            .key("splitter-handle")
            .size(18.0, 18.0),
        ui::text("").fill_width().fill_height(),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .width(24.0)
    .fill_height()
    .padding(3.0)
    .spacing(4.0)
}

fn file_view(state: &BrowserState) -> ui::StateView<BrowserState> {
    let folder = state.selected_folder();
    panel(
        folder.name.clone(),
        ui::selectable_sortable_details_list(
            [
                ui::DetailsColumn::flexible("name", "Name"),
                ui::DetailsColumn::fixed("size", "Size", 74),
                ui::DetailsColumn::fixed("kind", "Type", 132),
                ui::DetailsColumn::fixed("modified", "Modified", 112),
            ],
            state
                .sorted_files()
                .into_iter()
                .map(|file| file_row(state, file)),
            Some(state.sort.clone()),
            BrowserState::sort_by,
            Some(BrowserState::select_file_id),
        ),
    )
    .fill_width()
    .fill_height()
}

fn file_row(state: &BrowserState, file: &FileEntry) -> ui::DetailsRow {
    ui::DetailsRow::new(
        file.id.clone(),
        [
            file.name.clone(),
            file.size.clone(),
            file.kind.clone(),
            file.modified.clone(),
        ],
    )
    .selected(state.selected_file.as_deref() == Some(file.id.as_str()))
}

fn panel(
    title: impl Into<String>,
    content: ui::StateView<BrowserState>,
) -> ui::StateView<BrowserState> {
    ui::column([ui::text(title).fill_width().height(22.0), content])
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(8.0)
        .spacing(6.0)
}

fn temp_root() -> PathBuf {
    PathBuf::from(r"C:\temp")
}

fn load_root_folder(root: PathBuf) -> FolderEntry {
    let _ = fs::create_dir_all(&root);
    load_folder(&root, 0).unwrap_or_else(|| FolderEntry {
        id: path_id(&root),
        name: root.display().to_string(),
        children: Vec::new(),
        files: Vec::new(),
    })
}

fn move_folder_on_disk(source_id: &str, target_id: &str, root_id: &str) -> Result<String, String> {
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

fn create_child_folder(parent_id: &str, base_name: &str) -> Result<String, String> {
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

fn rename_folder_on_disk(folder_id: &str, new_name: &str, root_id: &str) -> Result<String, String> {
    let source = PathBuf::from(folder_id);
    let root = PathBuf::from(root_id);
    validate_folder_rename(&source, new_name, &root)?;
    let parent = source
        .parent()
        .ok_or_else(|| String::from("Cannot rename unnamed folder"))?;
    let destination = parent.join(new_name.trim());
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Rename failed: {error}"))
        .map(|_| path_id(&destination))
}

fn validate_folder_rename(source: &Path, new_name: &str, root: &Path) -> Result<(), String> {
    if source == root {
        return Err(String::from("Cannot rename the root folder"));
    }
    if !source.starts_with(root) {
        return Err(String::from("Rename must stay inside C:\\temp"));
    }
    if !source.is_dir() {
        return Err(String::from("Folder no longer exists"));
    }
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(String::from("Folder name cannot be empty"));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(String::from("Folder name is reserved"));
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err(String::from("Folder name cannot end with a dot or space"));
    }
    if trimmed
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err(String::from("Folder name contains invalid characters"));
    }
    Ok(())
}

fn validate_folder_move(source: &Path, target: &Path, root: &Path) -> Result<(), String> {
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
        return Err(String::from("Move must stay inside C:\\temp"));
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
    let mut entries = read_sorted_entries(path);
    let files = entries.iter().map(file_entry).collect::<Vec<_>>();
    let children = if depth >= MAX_SCAN_DEPTH {
        Vec::new()
    } else {
        entries
            .drain(..)
            .filter(|path| path.is_dir())
            .take(MAX_CHILD_FOLDERS)
            .filter_map(|path| load_folder(&path, depth + 1))
            .collect()
    };

    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn read_sorted_entries(path: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| natural_name_cmp(&file_label(a), &file_label(b)));
    entries
}

fn file_entry(path: &PathBuf) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let is_dir = metadata.as_ref().is_some_and(|metadata| metadata.is_dir());
    let size_bytes = metadata
        .as_ref()
        .filter(|_| !is_dir)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok());
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        kind: if is_dir {
            String::from("Folder")
        } else {
            file_kind(path)
        },
        size: if is_dir {
            String::from("-")
        } else {
            format_size(size_bytes)
        },
        size_bytes,
        modified: modified_label(modified),
        modified_rank: modified_rank(modified),
    }
}

fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn folder_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_kind(path: &Path) -> String {
    match path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("rs") => String::from("Rust Source File"),
        Some("toml") => String::from("TOML"),
        Some("md") => String::from("Markdown"),
        Some("json") => String::from("JSON"),
        Some("txt") => String::from("Text"),
        Some(extension) if !extension.is_empty() => format!("{extension} File"),
        _ => String::from("File"),
    }
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

fn natural_name_cmp(a: &str, b: &str) -> Ordering {
    a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> BrowserState {
        let root = folder_entry_for_test(
            r"C:\temp",
            "temp",
            vec![
                folder_entry_for_test(
                    r"C:\temp\alpha",
                    "alpha",
                    vec![folder_entry_for_test(
                        r"C:\temp\alpha\nested",
                        "nested",
                        Vec::new(),
                    )],
                ),
                folder_entry_for_test(r"C:\temp\beta", "beta", Vec::new()),
            ],
        );
        let selected_folder = root.id.clone();
        BrowserState {
            selected_folder: selected_folder.clone(),
            selected_file: None,
            expanded_folders: [selected_folder].into_iter().collect(),
            folder_drag: None,
            context_folder: None,
            rename_folder: None,
            rename_draft: String::new(),
            sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            tree_width: 300.0,
            folders: vec![root],
            status: String::from("Drag a folder handle onto another folder"),
        }
    }

    fn folder_entry_for_test(id: &str, name: &str, children: Vec<FolderEntry>) -> FolderEntry {
        FolderEntry {
            id: id.to_owned(),
            name: name.to_owned(),
            children,
            files: vec![FileEntry {
                id: format!("{id}\\sample.txt"),
                name: String::from("sample.txt"),
                kind: String::from("Text"),
                size: String::from("1 KB"),
                size_bytes: 1024,
                modified: String::from("Today"),
                modified_rank: 0,
            }],
        }
    }

    fn temp_test_root(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "radiant-folder-browser-{suffix}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn splitter_clamps_folder_tree_width() {
        let mut state = test_state();

        state.resize_tree(ui::DragHandleMessage::Moved {
            position: radiant::layout::Point::new(20.0, 0.0),
        });
        assert_eq!(state.tree_width, MIN_TREE_WIDTH);

        state.resize_tree(ui::DragHandleMessage::Moved {
            position: radiant::layout::Point::new(600.0, 0.0),
        });
        assert_eq!(state.tree_width, MAX_TREE_WIDTH);
    }

    #[test]
    fn folder_expansion_controls_visible_tree_rows() {
        let mut state = test_state();
        let alpha = String::from(r"C:\temp\alpha");
        let nested = String::from(r"C:\temp\alpha\nested");

        assert!(!state.visible_folder_ids().contains(&nested));
        state.toggle_folder(alpha);

        assert!(state.visible_folder_ids().contains(&nested));
    }

    #[test]
    fn expander_toggle_collapses_without_selecting_folder_first() {
        let mut state = test_state();
        let alpha = String::from(r"C:\temp\alpha");
        let beta = String::from(r"C:\temp\beta");

        state.activate_folder(alpha.clone());
        state.activate_folder(beta.clone());
        assert_eq!(state.selected_folder, beta);
        assert!(state.is_expanded(&alpha));

        state.toggle_folder(alpha.clone());

        assert_eq!(state.selected_folder, beta);
        assert!(!state.is_expanded(&alpha));
    }

    #[test]
    fn folder_click_expands_collapsed_branches_and_only_collapses_selected_expanded_branch() {
        let mut state = test_state();
        let alpha = String::from(r"C:\temp\alpha");
        let beta = String::from(r"C:\temp\beta");

        state.activate_folder(alpha.clone());
        assert!(state.is_expanded(&alpha));
        assert_eq!(state.selected_folder, alpha);

        state.activate_folder(beta.clone());
        assert_eq!(state.selected_folder, beta);
        state.activate_folder(alpha.clone());
        assert_eq!(state.selected_folder, alpha);
        assert!(state.is_expanded(&alpha));
        state.activate_folder(alpha.clone());
        assert!(!state.is_expanded(&alpha));
    }

    #[test]
    fn selecting_file_records_selected_file_id() {
        let mut state = test_state();

        state.select_file_id(String::from(r"C:\temp\sample.txt"));

        assert_eq!(state.selected_file.as_deref(), Some(r"C:\temp\sample.txt"));
    }

    #[test]
    fn opening_context_menu_records_target_folder() {
        let mut state = test_state();

        state.open_context_menu(String::from(r"C:\temp\alpha"));

        assert_eq!(state.context_folder.as_deref(), Some(r"C:\temp\alpha"));
    }

    #[test]
    fn rename_from_context_opens_inline_editor_with_folder_name() {
        let mut state = test_state();

        state.open_context_menu(String::from(r"C:\temp\alpha"));
        state.begin_rename_from_context();

        assert_eq!(state.context_folder, None);
        assert_eq!(state.rename_folder.as_deref(), Some(r"C:\temp\alpha"));
        assert_eq!(state.rename_draft, "alpha");
    }

    #[test]
    fn leaf_folder_click_selects_without_recording_expansion() {
        let mut state = test_state();
        let beta = String::from(r"C:\temp\beta");

        state.activate_folder(beta.clone());

        assert_eq!(state.selected_folder, beta);
        assert!(!state.is_expanded(r"C:\temp\beta"));
    }

    #[test]
    fn temp_root_is_the_browser_default_root() {
        let state = BrowserState::default();

        assert_eq!(state.folders[0].id, path_id(&temp_root()));
        assert_eq!(state.selected_folder, path_id(&temp_root()));
    }

    #[test]
    fn folder_move_rejects_root_self_and_descendant_targets() {
        let root = PathBuf::from(r"C:\temp");
        let source = PathBuf::from(r"C:\temp\alpha");
        let child = PathBuf::from(r"C:\temp\alpha\nested");

        assert_eq!(
            validate_folder_move(&root, &source, &root).unwrap_err(),
            "Cannot move the root folder"
        );
        assert_eq!(
            validate_folder_move(&source, &source, &root).unwrap_err(),
            "Cannot move a folder into itself"
        );
        assert_eq!(
            validate_folder_move(&source, &child, &root).unwrap_err(),
            "Cannot move a folder into one of its descendants"
        );
    }

    #[test]
    fn folder_move_renames_source_into_target_folder() {
        let root = temp_test_root("move-test");
        let source = root.join("source");
        let target = root.join("target");
        let destination = target.join("source");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&source).expect("source folder should be created");
        fs::create_dir_all(&target).expect("target folder should be created");

        let moved = move_folder_on_disk(&path_id(&source), &path_id(&target), &path_id(&root))
            .expect("move should succeed");

        assert_eq!(moved, path_id(&destination));
        assert!(destination.is_dir());
        assert!(!source.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_child_folder_uses_unique_new_folder_name() {
        let root = temp_test_root("create-test");
        let existing = root.join("New Folder");
        let expected = root.join("New Folder 1");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&existing).expect("existing folder should be created");

        let created =
            create_child_folder(&path_id(&root), "New Folder").expect("create should succeed");

        assert_eq!(created, path_id(&expected));
        assert!(expected.is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn folder_rename_rejects_root_empty_and_invalid_names() {
        let root = temp_test_root("rename-reject-test");
        let source = root.join("source");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&source).expect("source folder should be created");

        assert_eq!(
            validate_folder_rename(&root, "renamed", &root).unwrap_err(),
            "Cannot rename the root folder"
        );
        assert_eq!(
            validate_folder_rename(&source, "  ", &root).unwrap_err(),
            "Folder name cannot be empty"
        );
        assert_eq!(
            validate_folder_rename(&source, "bad/name", &root).unwrap_err(),
            "Folder name contains invalid characters"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn folder_rename_changes_folder_name_in_place() {
        let root = temp_test_root("rename-test");
        let source = root.join("source");
        let destination = root.join("renamed");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&source).expect("source folder should be created");

        let renamed = rename_folder_on_disk(&path_id(&source), " renamed ", &path_id(&root))
            .expect("rename should succeed");

        assert_eq!(renamed, path_id(&destination));
        assert!(destination.is_dir());
        assert!(!source.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
