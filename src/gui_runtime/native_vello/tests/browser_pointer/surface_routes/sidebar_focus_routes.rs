use super::*;
use crate::sempal_app::FolderPaneIdModel;

#[test]
fn source_row_click_routes_focus_source_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let row = shell_state
        .rendered_source_row_rects_for_pane(&layout, &model, FolderPaneIdModel::Upper)
        .into_iter()
        .nth(1)
        .expect("second source row should be rendered");
    let point = Point::new((row.min.x + row.max.x) * 0.5, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusSourceRow {
            pane: Some(FolderPaneIdModel::Upper),
            index: 1,
        })
    );
}

#[test]
fn lower_source_row_click_routes_focus_source_row_for_lower_pane() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let row = shell_state
        .rendered_source_row_rects_for_pane(&layout, &model, FolderPaneIdModel::Lower)
        .into_iter()
        .nth(1)
        .expect("second lower source row should be rendered");
    let point = Point::new((row.min.x + row.max.x) * 0.5, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusSourceRow {
            pane: Some(FolderPaneIdModel::Lower),
            index: 1,
        })
    );
}

#[test]
fn empty_source_list_click_routes_focus_sources_panel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        sources: SourcesPanelModel {
            header: String::from("0 sources"),
            rows: Vec::new().into(),
            folder_rows: vec![
                crate::sempal_app::FolderRowModel::new(
                    "Root", "", 0, true, false, true, true, true,
                ),
                crate::sempal_app::FolderRowModel::new(
                    "Drums", "", 1, false, true, false, true, true,
                ),
            ]
            .into(),
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    };
    let point = sidebar_focus_background_point(
        &layout,
        &model,
        &mut shell_state,
        UiAction::FocusSourcesPanel,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusSourcesPanel)
    );
}

#[test]
fn empty_folder_section_click_routes_focus_folder_panel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let point = sidebar_focus_background_point(
        &layout,
        &model,
        &mut shell_state,
        UiAction::FocusFolderPanel {
            pane: Some(FolderPaneIdModel::Upper),
        },
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderPanel {
            pane: Some(FolderPaneIdModel::Upper),
        })
    );
}

#[test]
fn folder_disclosure_click_routes_toggle_folder_row_expanded_for_expandable_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let disclosure = shell_state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("folder disclosure should be rendered");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ToggleFolderRowExpanded {
            pane: Some(FolderPaneIdModel::Upper),
            index: 1,
        })
    );
}

#[test]
fn lower_folder_disclosure_click_routes_toggle_folder_row_expanded_for_expandable_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.sources.active_folder_pane = FolderPaneIdModel::Lower;
    let disclosure = shell_state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("lower folder disclosure should be rendered");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ToggleFolderRowExpanded {
            pane: Some(FolderPaneIdModel::Lower),
            index: 1,
        })
    );
}

#[test]
fn folder_row_body_click_keeps_focus_row_behavior_for_expandable_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let row = shell_state
        .rendered_folder_row_rects(&layout, &model)
        .into_iter()
        .nth(1)
        .expect("second folder row should be rendered");
    let point = Point::new(row.max.x - 8.0, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderRow {
            pane: Some(FolderPaneIdModel::Upper),
            index: 1,
        })
    );
}

#[test]
fn leaf_folder_row_click_keeps_focus_row_behavior() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let row = shell_state
        .rendered_folder_row_rects(&layout, &model)
        .into_iter()
        .nth(2)
        .expect("leaf folder row should be rendered");
    let point = Point::new(row.max.x - 8.0, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderRow {
            pane: Some(FolderPaneIdModel::Upper),
            index: 2,
        })
    );
}

#[test]
fn searchable_folder_row_click_keeps_focus_row_behavior() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model_with_search("drum");
    let row = shell_state
        .rendered_folder_row_rects(&layout, &model)
        .into_iter()
        .nth(1)
        .expect("expandable search result row should be rendered");
    let point = Point::new(row.max.x - 8.0, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderRow {
            pane: Some(FolderPaneIdModel::Upper),
            index: 1,
        })
    );
}

#[test]
fn inline_folder_create_row_click_focuses_folder_create_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.sources.folder_rows.make_mut().insert(
        2,
        crate::sempal_app::FolderRowModel::create_draft(
            2,
            String::from("new folder"),
            String::from("New folder name"),
            None,
            true,
        ),
    );
    let row = shell_state
        .rendered_folder_row_rects(&layout, &model)
        .into_iter()
        .nth(2)
        .expect("draft folder row should be rendered");
    let point = Point::new(row.max.x - 8.0, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderCreateInput)
    );
}

#[test]
fn inline_folder_rename_row_click_focuses_folder_create_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.sources.folder_rows.make_mut().insert(
        2,
        crate::sempal_app::FolderRowModel::rename_draft(
            2,
            String::from("drums"),
            String::from("Folder name"),
            None,
            true,
        ),
    );
    let row = shell_state
        .rendered_folder_row_rects(&layout, &model)
        .into_iter()
        .nth(2)
        .expect("rename draft folder row should be rendered");
    let point = Point::new(row.max.x - 8.0, (row.min.y + row.max.y) * 0.5);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderCreateInput)
    );
}
