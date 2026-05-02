use super::*;

#[test]
fn map_point_click_routes_to_focus_spatial_content() {
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
            render_mode: crate::compat_app_contract::MapRenderModeModel::Points,
            selected_item_id: Some(String::from("source::kick.wav")),
            focused_item_id: Some(String::from("source::kick.wav")),
            points: Arc::from(vec![MapPointModel {
                id: Arc::<str>::from("source::kick.wav"),
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
        Some(UiAction::FocusSpatialContentItem {
            content_id: String::from("source::kick.wav")
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
            .top_bar_volume_action_at_point(&layout, &model, point)
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
        .status_options_button_rect(&layout, &model)
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
