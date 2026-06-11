//! Folder browser with an expandable tree, details list, and resizable panes.

use radiant::prelude as ui;

#[path = "folder_browser/actions.rs"]
mod actions;
#[path = "folder_browser/columns.rs"]
mod columns;
#[path = "folder_browser/file_view.rs"]
mod file_view;
#[path = "folder_browser/model.rs"]
mod model;
#[path = "folder_browser/state.rs"]
mod state;
#[path = "folder_browser/storage.rs"]
mod storage;
#[path = "folder_browser/tree.rs"]
mod tree;
#[path = "folder_browser/view.rs"]
mod view;
use columns::*;
use model::*;
use radiant::widgets::{ButtonMessage, TextInputMessage};
use state::*;
use storage::*;

const MIN_TREE_WIDTH: f32 = 190.0;
const MAX_TREE_WIDTH: f32 = 430.0;
const SPLITTER_OFFSET: f32 = 24.0;
const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_ROW_TOP: f32 = 104.0;
const MIN_FILE_COLUMN_WIDTH: f32 = 56.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 360.0;
const ROOT_ENV_VAR: &str = "RADIANT_FOLDER_BROWSER_ROOT";

fn main() -> radiant::Result {
    radiant::app(BrowserState::from_root(resolve_browser_root()))
        .title("Radiant Folder Browser")
        .size(900, 540)
        .min_size(640, 360)
        .view(view::project_surface)
        .update(update)
        .run()
}

#[derive(Clone, Debug, PartialEq)]
enum BrowserMessage {
    CreateFileInSelectedFolder,
    FolderRenameInput(TextInputMessage),
    CommitFolderRename,
    CancelFolderRename,
    FolderLabel {
        folder_id: String,
        event: ButtonMessage,
    },
    ToggleFolder(String),
    ResizeTree(ui::DragHandleMessage),
    FileRenameInput(TextInputMessage),
    CommitFileRename,
    CancelFileRename,
    FileButton {
        file_id: String,
        event: ButtonMessage,
    },
    ColumnHeader {
        column_id: String,
        event: ButtonMessage,
    },
    ResizeFileColumn {
        column_id: String,
        event: ui::DragHandleMessage,
    },
    BeginFolderRenameFromContext,
    CreateFolderFromContext,
    CloseFolderContextMenu,
    BeginFileRenameFromContext,
    DeleteFileFromContext,
    CloseFileContextMenu,
    ToggleFileColumn(String),
    ResetFileColumns,
    CloseColumnContextMenu,
}

fn update(state: &mut BrowserState, message: BrowserMessage) {
    match message {
        BrowserMessage::CreateFileInSelectedFolder => state.create_file_in_selected_folder(),
        BrowserMessage::FolderRenameInput(input) => {
            let submitted = input.is_submitted();
            state.rename.folder_draft = input.into_value();
            if submitted {
                state.commit_rename();
            }
        }
        BrowserMessage::CommitFolderRename => state.commit_rename(),
        BrowserMessage::CancelFolderRename => state.cancel_folder_rename(),
        BrowserMessage::FolderLabel { folder_id, event } => match event {
            ButtonMessage::Activate => state.activate_folder(folder_id),
            ButtonMessage::SecondaryActivate { position } => {
                state.open_context_menu_at(folder_id, position);
            }
            ButtonMessage::Drag(message) => state.handle_folder_drag(folder_id, message),
        },
        BrowserMessage::ToggleFolder(folder_id) => state.toggle_folder(folder_id),
        BrowserMessage::ResizeTree(message) => state.resize_tree(message),
        BrowserMessage::FileRenameInput(input) => {
            let submitted = input.is_submitted();
            state.rename.file_draft = input.into_value();
            if submitted {
                state.commit_file_rename();
            }
        }
        BrowserMessage::CommitFileRename => state.commit_file_rename(),
        BrowserMessage::CancelFileRename => state.cancel_file_rename(),
        BrowserMessage::FileButton { file_id, event } => match event {
            ButtonMessage::Activate => state.select_file_id(file_id),
            ButtonMessage::SecondaryActivate { position } => {
                state.open_file_context_menu_at(file_id, position);
            }
            ButtonMessage::Drag(_) => {}
        },
        BrowserMessage::ColumnHeader { column_id, event } => match event {
            ButtonMessage::Activate => state.sort_by(column_id),
            ButtonMessage::SecondaryActivate { position } => {
                state.open_column_context_menu_at(column_id, position);
            }
            ButtonMessage::Drag(_) => {}
        },
        BrowserMessage::ResizeFileColumn { column_id, event } => {
            state.resize_file_column(column_id, event);
        }
        BrowserMessage::BeginFolderRenameFromContext => state.begin_rename_from_context(),
        BrowserMessage::CreateFolderFromContext => state.create_folder_from_context(),
        BrowserMessage::CloseFolderContextMenu => state.close_context_menu(),
        BrowserMessage::BeginFileRenameFromContext => state.begin_file_rename_from_context(),
        BrowserMessage::DeleteFileFromContext => state.delete_file_from_context(),
        BrowserMessage::CloseFileContextMenu => state.close_file_context_menu(),
        BrowserMessage::ToggleFileColumn(column_id) => state.toggle_file_column(column_id),
        BrowserMessage::ResetFileColumns => state.reset_file_columns(),
        BrowserMessage::CloseColumnContextMenu => state.close_column_context_menu(),
    }
}

#[cfg(test)]
#[path = "folder_browser/tests.rs"]
mod tests;
