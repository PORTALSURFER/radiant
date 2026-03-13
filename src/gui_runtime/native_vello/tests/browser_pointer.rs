use super::*;

#[test]
fn browser_row_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        browser: crate::app::BrowserPanelModel {
            rows: vec![crate::app::BrowserRowModel::new(
                17, "kick-row", 0, false, false,
            )],
            visible_count: 1,
            ..crate::app::BrowserPanelModel::default()
        },
        ..AppModel::default()
    };
    let row_center_y = layout.browser_rows.min.y
        + (StyleTokens::for_viewport_width(layout.root.rect.width())
            .sizing
            .browser_row_height
            * 0.5);
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        row_center_y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT,
        ),
        Some(UiAction::ExtendBrowserSelectionToRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SUPER,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT | ModifiersState::SUPER,
        ),
        Some(UiAction::AddRangeBrowserSelection { visible_row: 17 })
    );
}

#[test]
fn browser_row_click_targets_interior_row_after_downward_autoscroll() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut shell_state = NativeShellState::new();
    let mut model = AppModel::default();
    for visible_row in 0..40 {
        model.browser.rows.push(crate::app::BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:02}"),
            1,
            false,
            visible_row == 18,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(18);
    model.browser.anchor_visible_row = Some(18);
    let row_stride = style.sizing.browser_row_height + style.sizing.browser_row_gap;
    let row_center_y =
        layout.browser_rows.min.y + (row_stride * 11.0) + (style.sizing.browser_row_height * 0.5);
    let point = Point::new(layout.browser_rows.min.x + 24.0, row_center_y);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserRow { visible_row: 12 })
    );
}

#[test]
fn browser_toolbar_alt_click_maps_to_inverted_rating_filter_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let chip = shell_state
        .browser_rating_filter_chip_rect(&layout, &model, 4)
        .expect("locked keep rating filter chip should exist");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::ALT,
        ),
        Some(UiAction::ToggleBrowserRatingFilter {
            level: 4,
            invert: true,
        })
    );
}

#[test]
fn browser_random_action_button_click_routes_toggle_random_navigation_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let button = shell_state
        .browser_action_button_rect(&layout, &model, "Random")
        .expect("random browser action button should exist");
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
        Some(UiAction::ToggleRandomNavigationMode)
    );
}

#[test]

fn browser_tab_clicks_route_to_tab_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let map_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.75),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            map_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetBrowserTab { map: true })
    );

    let list_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.25),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            list_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetBrowserTab { map: false })
    );
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
fn browser_wheel_delta_is_bounded_and_directional() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut model = AppModel::default();
    model.map.active = false;
    let point = Point::new(
        layout.browser_rows.min.x + 10.0,
        layout.browser_rows.min.y + 10.0,
    );

    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 3.0),
        ),
        Some(-3)
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 0.0)
        ),
        None
    );
    let header_point = Point::new(
        layout.browser_table_header.min.x + 5.0,
        layout.browser_table_header.min.y + 5.0,
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            header_point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 2.0),
        ),
        Some(-2)
    );
}

#[test]
fn browser_view_start_after_wheel_clamps_to_visible_bounds() {
    assert_eq!(browser_view_start_after_wheel(10, 40, 12, -3), Some(7));
    assert_eq!(browser_view_start_after_wheel(0, 40, 12, -3), Some(0));
    assert_eq!(browser_view_start_after_wheel(27, 40, 12, 5), Some(28));
    assert_eq!(browser_view_start_after_wheel(4, 0, 12, 2), None);
    assert_eq!(browser_view_start_after_wheel(4, 20, 0, 2), None);
    assert_eq!(browser_view_start_after_wheel(4, 20, 12, 0), None);
}

#[test]
fn waveform_wheel_zoom_requires_waveform_hover_and_maps_direction() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, 2.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: Some(500_000),
        })
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, -1.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
            anchor_ratio_micros: Some(500_000),
        })
    );

    let outside_point = Point::new(
        layout.browser_rows.min.x + 12.0,
        layout.browser_rows.min.y + 12.0,
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            outside_point,
            MouseScrollDelta::LineDelta(0.0, 2.0)
        ),
        None
    );
}

#[test]
fn waveform_pointer_position_tracks_active_view_window() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 750_000;
    let y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);

    let left = Point::new(layout.waveform_plot.min.x, y);
    let center = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        y,
    );
    let right = Point::new(layout.waveform_plot.max.x, y);

    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, left),
        250
    );
    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, center),
        500
    );
    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, right),
        750
    );
}

#[test]
fn waveform_wheel_zoom_action_uses_pointer_anchor_ratio() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, 1.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: Some(250_000),
        })
    );
}
