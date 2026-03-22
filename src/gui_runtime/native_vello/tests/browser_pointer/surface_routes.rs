use super::*;

fn sidebar_focus_background_point(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    expected: UiAction,
) -> Point {
    let mut y = layout.sidebar_rows.min.y + 1.0;
    while y < layout.sidebar_rows.max.y {
        let mut x = layout.sidebar_rows.min.x + 1.0;
        while x < layout.sidebar_rows.max.x {
            let point = Point::new(x, y);
            if shell_state
                .source_row_at_point(layout, model, point)
                .is_none()
                && shell_state
                    .folder_row_at_point(layout, model, point)
                    .is_none()
                && shell_state.sidebar_focus_action_at_point(layout, model, point)
                    == Some(expected.clone())
            {
                return point;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("failed to find sidebar background point for {expected:?}");
}

fn populated_sidebar_model() -> AppModel {
    AppModel {
        sources: SourcesPanelModel {
            header: String::from("2 sources"),
            selected_row: Some(0),
            focused_folder_row: Some(1),
            rows: vec![
                crate::app::SourceRowModel::new("Source A", "ready", true, false),
                crate::app::SourceRowModel::new("Source B", "ready", false, false),
            ],
            folder_rows: vec![
                crate::app::FolderRowModel::new("Root", "", 0, true, false, true, true, true),
                crate::app::FolderRowModel::new("Drums", "", 1, false, true, false, true, true),
            ],
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    }
}

#[test]
fn map_point_click_routes_to_focus_map_sample() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.browser_rows.min.x + (layout.browser_rows.width() * 0.5),
        layout.browser_rows.min.y + (layout.browser_rows.height() * 0.5),
    );
    let model = AppModel {
        map: MapPanelModel {
            active: true,
            summary: String::from("1 point"),
            legend_label: String::from("Render: points"),
            selection_label: String::from("Selection: source::kick.wav"),
            hover_label: String::from("Hover: source::kick.wav"),
            cluster_label: String::from("Clusters: 1"),
            viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
            error: None,
            render_mode: crate::app::MapRenderModeModel::Points,
            selected_sample_id: Some(String::from("source::kick.wav")),
            focused_sample_id: Some(String::from("source::kick.wav")),
            points: Arc::from(vec![MapPointModel {
                sample_id: Arc::<str>::from("source::kick.wav"),
                x_milli: 500,
                y_milli: 500,
                cluster_id: Some(1),
            }]),
        },
        ..AppModel::default()
    };
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusMapSample {
            sample_id: String::from("source::kick.wav")
        })
    );
}

#[test]
fn top_bar_volume_meter_click_routes_set_volume_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let mut first_hit_x = None;
    let mut last_hit_x = None;
    let y = layout.top_bar_controls_row.min.y + (layout.top_bar_controls_row.height() * 0.5);
    let mut x = layout.top_bar.min.x;
    while x <= layout.top_bar.max.x {
        let point = Point::new(x, y);
        if shell_state
            .top_bar_volume_action_at_point(&layout, point)
            .is_some()
        {
            if first_hit_x.is_none() {
                first_hit_x = Some(x);
            }
            last_hit_x = Some(x);
        }
        x += 2.0;
    }
    let meter_min_x = first_hit_x.expect("volume meter point should be discoverable");
    let meter_max_x = last_hit_x.expect("volume meter span should be discoverable");
    let meter_point = Point::new((meter_min_x + meter_max_x) * 0.5, y);
    match action_from_pointer(
        &layout,
        &model,
        &mut shell_state,
        meter_point,
        ModifiersState::default(),
    ) {
        Some(UiAction::SetVolume { value_milli }) => {
            assert!(value_milli >= 350);
            assert!(value_milli <= 650);
        }
        other => panic!("expected SetVolume action, got {other:?}"),
    }
}

#[test]
fn status_options_click_routes_open_options_menu_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let button = shell_state
        .status_options_button_rect(&layout)
        .expect("status options button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::OpenOptionsMenu)
    );
}

#[test]
fn source_row_click_routes_focus_source_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = populated_sidebar_model();
    let row = shell_state
        .rendered_source_row_rects(&layout, &model)
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
        Some(UiAction::FocusSourceRow { index: 1 })
    );
}

#[test]
fn empty_source_list_click_routes_focus_sources_panel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        sources: SourcesPanelModel {
            header: String::from("0 sources"),
            rows: Vec::new(),
            folder_rows: vec![
                crate::app::FolderRowModel::new("Root", "", 0, true, false, true, true, true),
                crate::app::FolderRowModel::new("Drums", "", 1, false, true, false, true, true),
            ],
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
        UiAction::FocusFolderPanel,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusFolderPanel)
    );
}
