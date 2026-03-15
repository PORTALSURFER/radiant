use super::*;

#[test]
/// Source context menu hit testing should emit reload for the targeted row.
fn source_context_menu_hit_test_emits_reload_action_for_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    let row_rect = *state
        .rendered_source_row_rects(&layout, &model)
        .first()
        .expect("source row should be rendered");
    let anchor = Point::new(
        (row_rect.min.x + row_rect.max.x) * 0.5,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    );
    state.open_source_context_menu_for_row(0, anchor);

    let reload_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::ReloadSourceRow { index: 0 })
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::ReloadSourceRow { index: 0 })
    );
}

#[test]
/// Source context menu geometry should disappear after explicit close.
fn source_context_menu_contains_point_tracks_open_close_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    state.open_source_context_menu_for_row(
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );
    let reload_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::ReloadSourceRow { index: 0 })
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert!(state.source_context_menu_contains_point(&layout, &model, point));
    assert!(state.close_source_context_menu());
    assert!(!state.source_context_menu_contains_point(&layout, &model, point));
}

#[test]
/// Source context menu should expose source removal and render in the overlay pass.
fn source_context_menu_exposes_remove_action_in_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    state.open_source_context_menu_for_row(
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );

    let remove_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::RemoveSourceRow { index: 0 })
        .expect("remove source action button should be present");
    let point = Point::new(
        (remove_rect.min.x + remove_rect.max.x) * 0.5,
        (remove_rect.min.y + remove_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::RemoveSourceRow { index: 0 })
    );

    let frame = state.build_frame(&layout, &model);
    assert!(
        !frame
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );
}

#[test]
fn tick_with_style_uses_tier_motion_speed_tokens() {
    let mut model = AppModel::default();
    model.transport_running = true;
    let compact_style = StyleTokens::for_viewport_width(820.0);
    let wide_style = StyleTokens::for_viewport_width(2300.0);

    let mut compact_state = NativeShellState::new();
    compact_state.sync_from_model(&model);
    compact_state.tick_with_style(1.0, &compact_style);

    let mut wide_state = NativeShellState::new();
    wide_state.sync_from_model(&model);
    wide_state.tick_with_style(1.0, &wide_style);

    assert!(compact_state.pulse_phase > 0.0);
    assert!(wide_state.pulse_phase > compact_state.pulse_phase);
}

#[test]
fn top_bar_volume_click_maps_to_set_volume_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
    assert!(controls.active);
    let point = Point::new(
        controls.volume_meter.min.x + (controls.volume_meter.width() * 0.75),
        controls.volume_meter.min.y + (controls.volume_meter.height() * 0.5),
    );
    let action = state
        .top_bar_volume_action_at_point(&layout, point)
        .expect("volume click should produce action");
    assert_eq!(action, UiAction::SetVolume { value_milli: 750 });
}

#[test]
fn status_options_click_maps_to_open_options_menu_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let button = state
        .status_options_button_rect(&layout)
        .expect("status options button should render");
    let point = Point::new(
        button.min.x + (button.width() * 0.5),
        button.min.y + (button.height() * 0.5),
    );
    let action = state
        .status_options_action_at_point(&layout, &model, point)
        .expect("options click should produce action");
    assert_eq!(action, UiAction::OpenOptionsMenu);
}

#[test]
fn options_panel_contains_points_inside_panel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::app::OptionsPanelModel {
            visible: true,
            ..crate::app::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };
    let point = Point::new(layout.top_bar.max.x - 40.0, layout.top_bar.max.y + 40.0);
    assert!(state.options_panel_contains_point(&layout, &model, point));
}

#[test]
fn options_panel_trash_folder_buttons_emit_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::app::OptionsPanelModel {
            visible: true,
            trash_folder_label: Some(String::from("trash_bin")),
            ..crate::app::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };
    let panel = options_panel_layout(&layout, &style, &model)
        .expect("visible options panel should resolve layout");
    let set_button = panel
        .buttons
        .iter()
        .find(|button| button.action == UiAction::PickTrashFolder)
        .expect("set trash folder button should be present");
    let set_point = Point::new(
        (set_button.rect.min.x + set_button.rect.max.x) * 0.5,
        (set_button.rect.min.y + set_button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, set_point),
        Some(UiAction::PickTrashFolder)
    );

    let open_button = panel
        .buttons
        .iter()
        .find(|button| button.action == UiAction::OpenTrashFolder)
        .expect("open trash folder button should be present");
    let open_point = Point::new(
        (open_button.rect.min.x + open_button.rect.max.x) * 0.5,
        (open_button.rect.min.y + open_button.rect.max.y) * 0.5,
    );
    assert_eq!(
        state.options_panel_action_at_point(&layout, &model, open_point),
        Some(UiAction::OpenTrashFolder)
    );
}

#[test]
fn non_modal_progress_renders_status_bar_indicator_without_overlay_dialog() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Normalizing sample"),
            detail: Some(String::from("kick.wav")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);
    let overlay_rect = compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        false,
        0.4,
    )
    .sections
    .dialog;

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("job active")),
        "status bar should announce an active job"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "2/5"),
        "status bar should show progress counts"
    );
    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x >= layout.status_center_segment.min.x
                    && rect.rect.max.x <= layout.status_center_segment.max.x
                    && rect.color == style.accent_mint
        )),
        "status bar should render an inline progress fill"
    );
    assert!(
        !frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect == overlay_rect && rect.color == style.surface_overlay
        )),
        "non-modal jobs should not render the floating overlay dialog"
    );
}

#[test]
fn non_modal_progress_does_not_expose_cancel_hit_target() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Normalizing"),
            completed: 0,
            total: 1,
            cancelable: true,
            cancel_requested: false,
            ..crate::app::ProgressOverlayModel::default()
        },
        ..AppModel::default()
    };
    let cancel_button = progress_cancel_button(&layout, &style, false);
    let point = Point::new(
        (cancel_button.min.x + cancel_button.max.x) * 0.5,
        (cancel_button.min.y + cancel_button.max.y) * 0.5,
    );

    assert_eq!(state.progress_action_at_point(&layout, &model, point), None);
}

#[test]
fn modal_progress_overlay_renders_cancelling_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: true,
            title: String::from("Normalizing"),
            detail: Some(String::from("kick.wav")),
            completed: 1,
            total: 4,
            cancelable: true,
            cancel_requested: true,
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.text_runs.iter().any(|run| run.text == "Cancelling"));
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == progress_cancel_button(&layout, &style, true)
                && rect.color == style.grid_soft
    )));
}

#[test]
fn confirm_prompt_validation_error_renders_disabled_confirm_button_and_error_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        confirm_prompt: crate::app::ConfirmPromptModel {
            visible: true,
            title: String::from("Rename"),
            message: String::from("Choose a new name"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Dismiss"),
            input_value: Some(String::from("bad/name")),
            input_error: Some(String::from("Slash not allowed")),
            ..crate::app::ConfirmPromptModel::default()
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Slash not allowed" && run.color == style.accent_warning)
    );
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if rect.color == style.control_disabled_fill
    )));
}

#[test]
fn drag_overlay_renders_target_arrow_and_warning_text_for_invalid_drop() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel {
        drag_overlay: crate::app::DragOverlayModel {
            active: true,
            label: String::from("kick.wav"),
            target_label: String::from("Trash"),
            valid_target: false,
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.text_runs.iter().any(|run| {
        run.text == "kick.wav -> Trash" && run.color == style.accent_warning
    }));
    assert!(overlay.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == drag_overlay_rect(&layout, &style)
                && rect.color == style.surface_overlay
    )));
}

#[test]
fn state_overlay_renders_options_panel_when_visible() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        options_panel: crate::app::OptionsPanelModel {
            visible: true,
            ..crate::app::OptionsPanelModel::default()
        },
        ..AppModel::default()
    };

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.text_runs.iter().any(|run| run.text == "Options"));
}
