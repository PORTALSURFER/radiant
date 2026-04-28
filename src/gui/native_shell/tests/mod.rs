use super::*;
use crate::gui::{
    input::KeyCode,
    types::{Point, Vector2},
};

mod contracts;
mod layout;
mod prompts;
mod sidebar;
mod toolbar;

fn canonical_shell_model() -> crate::app::AppModel {
    let mut model = crate::app::AppModel::default();
    model.title = String::from("Radiant Native");
    model.backend_label = String::from("radiant/native_vello");
    model.transport_running = true;
    model.status.left = String::from("Indexing complete");
    model.status.center = String::from("rows: 48 | selected: 3");
    model.status.right = String::from("col: 2/3");
    model.sources.search_query = String::from("drum");
    model.sources.folder_search_query = String::from("kicks");
    model.sources.folder_recovery.in_progress = false;
    model.sources.folder_recovery.entry_count = 12;
    model.sources.folder_actions.can_create_folder = true;
    model.sources.folder_actions.can_create_folder_at_root = true;
    model.sources.folder_actions.can_rename_folder = true;
    model.sources.folder_actions.can_delete_folder = true;
    model.sources.folder_actions.can_clear_recovery_log = true;
    model.sources.upper_folder_pane.folder_search_query = model.sources.folder_search_query.clone();
    model.sources.upper_folder_pane.folder_actions = model.sources.folder_actions.clone();
    model.sources.upper_folder_pane.active = true;
    model.sources.upper_folder_pane.has_source = true;
    model.sources.upper_folder_pane.source_label = String::from("source_02");
    for index in 0..10 {
        model.sources.rows.push(crate::app::SourceRowModel::new(
            format!("source_{index:02}"),
            format!("/samples/source_{index:02}"),
            index == 2,
            index == 5,
        ));
    }
    for index in 0..14 {
        model
            .sources
            .folder_rows
            .push(crate::app::FolderRowModel::new(
                format!("folder_{index:02}"),
                String::new(),
                index % 3,
                index == 1,
                index == 3,
                index == 0,
                true,
                true,
            ));
        model.sources.upper_folder_pane.folder_rows.push(
            model
                .sources
                .folder_rows
                .last()
                .expect("folder row was just inserted")
                .clone(),
        );
    }
    for index in 0..36 {
        model.browser.rows.push(crate::app::BrowserRowModel::new(
            index,
            format!("row_{index:02}.wav"),
            index % 3,
            index % 8 == 0,
            index == 5,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_path_count = 3;
    model.browser.search_query = String::from("kick");
    model.browser_chrome.search_prefix_label = String::from("Find");
    model.browser_chrome.sort_prefix_label = String::from("Order");
    model.browser_chrome.sort_order_label = String::from("List order");
    model.browser_chrome.item_count_label = String::from("36 items");
    model.waveform_chrome.transport_hint = String::from("Loop engaged");
    model.waveform.loaded_label = Some(String::from("Kick-Loop-01.wav"));
    model.waveform.cursor_milli = Some(345);
    model.waveform.playhead_milli = Some(512);
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 680));
    model.waveform.loop_enabled = true;
    model.waveform.view_start_milli = 100;
    model.waveform.view_end_milli = 900;
    model
}
