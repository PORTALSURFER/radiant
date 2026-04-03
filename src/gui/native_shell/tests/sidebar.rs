use crate::app::FolderPaneIdModel;
use super::*;

#[test]
fn source_action_hit_test_emits_folder_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model.sources.folder_actions.can_delete_folder = true;
    let button = state
        .source_action_button_rect(&layout, &model, crate::app::UiAction::DeleteFocusedFolder)
        .expect("delete action button should be present");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );
    assert_eq!(
        state.source_action_at_point(&layout, &model, point),
        Some(crate::app::UiAction::DeleteFocusedFolder)
    );
}

#[test]
fn source_action_hit_test_ignores_disabled_button() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model.sources.folder_actions.can_delete_folder = false;
    let button = state
        .source_action_button_rect(&layout, &model, crate::app::UiAction::DeleteFocusedFolder)
        .expect("delete action button should be present");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );
    assert_eq!(state.source_action_at_point(&layout, &model, point), None);
}

#[test]
fn folder_row_hit_test_resolves_rendered_folder_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model
        .sources
        .folder_rows
        .push(crate::app::FolderRowModel::new(
            "Drums", "Drums", 0, false, true, false, true, true,
        ));
    let folder_rects = state.rendered_folder_row_rects(&layout, &model);
    assert_eq!(folder_rects.len(), 1);
    let folder_rect = folder_rects[0];
    let point = Point::new(
        (folder_rect.min.x + folder_rect.max.x) * 0.5,
        (folder_rect.min.y + folder_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.folder_row_at_point(&layout, &model, point),
        Some((FolderPaneIdModel::Upper, 0))
    );
}

#[test]
fn folder_row_hit_test_survives_source_row_cache_priming() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model.sources.rows.push(crate::app::SourceRowModel::new(
        "Pack", "pack", false, false,
    ));
    model
        .sources
        .folder_rows
        .push(crate::app::FolderRowModel::new(
            "Drums", "Drums", 0, false, true, false, true, true,
        ));

    let source_rects = state.rendered_source_row_rects(&layout, &model);
    assert_eq!(source_rects.len(), 1);
    let source_point = Point::new(
        (source_rects[0].min.x + source_rects[0].max.x) * 0.5,
        (source_rects[0].min.y + source_rects[0].max.y) * 0.5,
    );
    assert_eq!(
        state.source_row_at_point(&layout, &model, source_point),
        Some(0)
    );

    let folder_rects = state.rendered_folder_row_rects(&layout, &model);
    assert_eq!(folder_rects.len(), 1);
    let folder_point = Point::new(
        (folder_rects[0].min.x + folder_rects[0].max.x) * 0.5,
        (folder_rects[0].min.y + folder_rects[0].max.y) * 0.5,
    );
    assert_eq!(
        state.folder_row_at_point(&layout, &model, folder_point),
        Some((FolderPaneIdModel::Upper, 0))
    );
}

#[test]
fn folder_rows_fill_sidebar_width_and_touch_without_gap() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    for index in 0..3 {
        model
            .sources
            .folder_rows
            .push(crate::app::FolderRowModel::new(
                format!("Folder {index}"),
                String::new(),
                0,
                false,
                index == 1,
                false,
                true,
                true,
            ));
    }

    let folder_rects = state.rendered_folder_row_rects(&layout, &model);
    assert_eq!(folder_rects.len(), 3);
    assert_eq!(folder_rects[0].min.x, layout.sidebar_rows.min.x);
    assert_eq!(folder_rects[0].max.x, layout.sidebar_rows.max.x);
    assert_eq!(folder_rects[0].max.y, folder_rects[1].min.y);
    assert_eq!(folder_rects[1].max.y, folder_rects[2].min.y);
}
