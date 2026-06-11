use super::file_view::file_view;
use super::tree::{folder_tree, splitter};
use super::*;

const FOLDER_MENU_SIZE: ui::Vector2 = ui::Vector2 { x: 190.0, y: 126.0 };
const FILE_MENU_SIZE: ui::Vector2 = ui::Vector2 { x: 190.0, y: 126.0 };
const COLUMN_MENU_SIZE: ui::Vector2 = ui::Vector2 { x: 210.0, y: 250.0 };

pub(super) fn project_surface(state: &mut BrowserState) -> ui::View<BrowserMessage> {
    let page = ui::column([
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
    .spacing(8.0);
    if has_context_menu(state) {
        ui::stack([page, context_menu_layer(state)])
            .fill_width()
            .fill_height()
    } else {
        page
    }
}

fn has_context_menu(state: &BrowserState) -> bool {
    state.context.context_folder.is_some()
        || state.context.context_file.is_some()
        || state.context.context_column.is_some()
}

fn header(state: &BrowserState) -> ui::View<BrowserMessage> {
    ui::row([
        ui::text("Folder browser").size(170.0, 24.0),
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

fn context_menu_layer(state: &BrowserState) -> ui::View<BrowserMessage> {
    let anchor = state.context.context_position.unwrap_or_default();
    if let Some(folder_id) = state.context.context_folder.as_ref() {
        return ui::message_context_menu_overlay(
            anchor,
            FOLDER_MENU_SIZE,
            folder_context_menu_title(state, folder_id),
            [
                ui::MenuCommand::new("Rename", BrowserMessage::BeginFolderRenameFromContext)
                    .primary(),
                ui::MenuCommand::new("New Folder", BrowserMessage::CreateFolderFromContext)
                    .subtle(),
                ui::MenuCommand::new("Cancel", BrowserMessage::CloseFolderContextMenu).subtle(),
            ],
        )
        .key("context-menu-overlay");
    }
    if let Some(column_id) = state.context.context_column.as_ref() {
        return ui::message_context_menu_overlay(
            anchor,
            COLUMN_MENU_SIZE,
            column_context_menu_title(state, column_id),
            column_context_menu_items(state),
        )
        .key("context-menu-overlay");
    }
    if let Some(file_id) = state.context.context_file.as_ref() {
        return ui::message_context_menu_overlay(
            anchor,
            FILE_MENU_SIZE,
            file_context_menu_title(state, file_id),
            [
                ui::MenuCommand::new("Rename", BrowserMessage::BeginFileRenameFromContext)
                    .primary(),
                ui::MenuCommand::new("Delete", BrowserMessage::DeleteFileFromContext).subtle(),
                ui::MenuCommand::new("Cancel", BrowserMessage::CloseFileContextMenu).subtle(),
            ],
        )
        .key("context-menu-overlay");
    }
    ui::text("").key("context-menu-overlay")
}

fn folder_context_menu_title(state: &BrowserState, folder_id: &str) -> String {
    let folder_name = state
        .find_folder(folder_id)
        .map(|folder| folder.name.clone())
        .unwrap_or_else(|| String::from("folder"));
    format!("Actions for {folder_name}")
}

fn file_context_menu_title(state: &BrowserState, file_id: &str) -> String {
    let file_name = state
        .selected_folder()
        .files
        .iter()
        .find(|file| file.id == file_id)
        .map(|file| file.name.clone())
        .unwrap_or_else(|| String::from("item"));
    format!("Actions for {file_name}")
}

fn column_context_menu_title(state: &BrowserState, column_id: &str) -> String {
    let column_name = state
        .columns
        .file_columns
        .iter()
        .find(|column| column.id == column_id)
        .map(|column| column.label.clone())
        .unwrap_or_else(|| String::from("columns"));
    format!("Columns from {column_name}")
}

fn column_context_menu_items(state: &BrowserState) -> Vec<ui::MenuCommand<BrowserMessage>> {
    let mut items = state
        .columns
        .file_columns
        .iter()
        .map(|column| {
            let id = column.id.clone();
            let marker = if column.visible { "[x]" } else { "[ ]" };
            let label = if column.id == "name" {
                format!("{marker} {} locked", column.label)
            } else {
                format!("{marker} {}", column.label)
            };
            ui::MenuCommand::new(label, BrowserMessage::ToggleFileColumn(id)).subtle()
        })
        .collect::<Vec<_>>();
    items.push(ui::MenuCommand::new("Reset Defaults", BrowserMessage::ResetFileColumns).primary());
    items.push(ui::MenuCommand::new("Close", BrowserMessage::CloseColumnContextMenu).subtle());
    items
}

pub(super) fn panel(
    title: impl Into<String>,
    content: ui::View<BrowserMessage>,
) -> ui::View<BrowserMessage> {
    ui::column([ui::text(title).fill_width().height(22.0), content])
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(8.0)
        .spacing(6.0)
}
