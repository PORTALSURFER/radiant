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
        .run()
}

#[cfg(test)]
#[path = "folder_browser/tests.rs"]
mod tests;
