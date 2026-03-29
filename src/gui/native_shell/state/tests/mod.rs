use super::*;
use crate::app::{
    BrowserRowModel, FolderActionsModel, FolderRowModel, NativeMotionModel, NormalizedRangeModel,
    SourceRowModel,
};
use crate::gui::types::{ImageRgba, Point, Rgba8, Vector2};

fn populated_sidebar_model() -> AppModel {
    let mut model = AppModel::default();
    for index in 0..20 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("detail_{index:02}"),
            index == 2,
            false,
        ));
    }
    for index in 0..24 {
        model.sources.folder_rows.push(FolderRowModel::new(
            format!("folder_{index:02}"),
            String::new(),
            index % 4,
            false,
            index == 3,
            index == 0,
            true,
            true,
        ));
    }
    model.sources.folder_actions = FolderActionsModel {
        can_create_folder: true,
        can_create_folder_at_root: true,
        can_rename_folder: true,
        can_delete_folder: true,
        can_restore_retained_deletes: true,
        can_purge_retained_deletes: true,
        can_clear_recovery_log: true,
    };
    model.sources.can_toggle_show_all_folders = true;
    model.sources.can_toggle_flattened_view = true;
    model
}

fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
    let mut model = AppModel::default();
    for visible_row in 0..total {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

/// Build cached browser rows from rects for hit-test unit coverage.
fn cached_browser_rows_from_rects(rects: &[Rect]) -> Vec<CachedBrowserRow> {
    rects
        .iter()
        .copied()
        .enumerate()
        .map(|(index, rect)| CachedBrowserRow {
            visible_row: index,
            label: format!("row_{index}"),
            bucket_label: String::new(),
            column: 1,
            rating_level: 0,
            selected: false,
            focused: false,
            missing: false,
            locked: false,
            rect,
        })
        .collect()
}

fn assert_rect_inside(outer: Rect, inner: Rect) {
    assert!(inner.min.x >= outer.min.x);
    assert!(inner.min.y >= outer.min.y);
    assert!(inner.max.x <= outer.max.x);
    assert!(inner.max.y <= outer.max.y);
}

fn assert_text_run_inside_band(run: &TextRun, band: Rect) {
    assert!(run.position.x >= band.min.x);
    assert!(run.position.x <= band.max.x);
    assert!(run.position.y >= band.min.y);
    assert!(run.position.y + run.font_size <= band.max.y + 0.5);
}

mod browser_rows;
mod browser_scrollbars;
mod browser_toolbar;
mod chrome_layout;
mod folder_visibility_toggle;
mod frame_build;
mod overlay_controls;
mod overlays;
mod playhead_trail_render;
mod playhead_trail_state;
mod selection_states;
mod sidebar;
mod status_bar_progress;
mod waveform_edit_fades;
mod waveform_edit_handles;
mod waveform_selection;
mod waveform_slices;
