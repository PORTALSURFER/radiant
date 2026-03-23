use super::*;

#[test]
fn waveform_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
        layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
    );
    let model = AppModel {
        columns: [
            ColumnModel::new("Trash", 0),
            ColumnModel::new("Neutral", 0),
            ColumnModel::new("Keep", 0),
        ],
        sources: SourcesPanelModel::default(),
        browser: BrowserPanelModel::default(),
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(120, 360)),
            cursor_milli: Some(220),
            playhead_milli: Some(260),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };
    let expected_position_nanos = waveform_position_nanos_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::BeginWaveformSelectionAt {
            anchor_micros: milli(500),
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(500),
            end_micros: milli(740),
            preserve_view_edge: false,
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(260),
            end_micros: milli(500),
            preserve_view_edge: false,
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(120),
            end_micros: milli(500),
            preserve_view_edge: false,
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::ALT
        ),
        Some(UiAction::SeekWaveformPrecise {
            position_nanos: expected_position_nanos
        })
    );
}

#[test]
fn waveform_plain_click_preserves_exact_micro_anchor() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.1234,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let expected_anchor = waveform_position_micros_from_point(&layout, &AppModel::default(), point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &AppModel::default(),
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::BeginWaveformSelectionAt {
            anchor_micros: expected_anchor,
        })
    );
}

#[test]
fn waveform_shift_click_preserves_exact_micro_endpoint() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.1234,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let expected_endpoint =
        waveform_position_micros_from_point(&layout, &AppModel::default(), point);
    let model = AppModel {
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(200, 800)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(200),
            end_micros: expected_endpoint,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn waveform_command_click_without_selection_sets_precise_cursor() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.1234,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let expected_position_nanos = waveform_position_nanos_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformCursorPrecise {
            position_nanos: expected_position_nanos,
        })
    );
}

#[test]
fn waveform_pointer_position_uses_nanounit_view_bounds_at_deep_zoom() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.75,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 500_000;
    model.waveform.view_end_micros = 500_000;
    model.waveform.view_start_nanos = 500_000_000;
    model.waveform.view_end_nanos = 500_000_200;

    let position_nanos = waveform_position_nanos_from_point(&layout, &model, point);

    assert_eq!(position_nanos, 500_000_150);
    assert_eq!(
        waveform_position_micros_from_point(&layout, &model, point),
        500_000
    );
}

#[test]
fn command_click_slides_existing_playback_selection_from_new_start_position() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(200, 400)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    for ratio in [0.1, 0.3, 0.7] {
        let point = Point::new(
            layout.waveform_plot.min.x + layout.waveform_plot.width() * ratio,
            layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
        );
        let expected_start = waveform_position_micros_from_point(&layout, &model, point);
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::CONTROL,
            ),
            Some(UiAction::SetWaveformSelectionRange {
                start_micros: expected_start,
                end_micros: expected_start + milli(200),
                preserve_view_edge: false,
            })
        );
    }
}

#[test]
fn command_shift_click_slides_existing_playback_selection_from_new_end_position() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(200, 400)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    for ratio in [0.1, 0.3, 0.7] {
        let point = Point::new(
            layout.waveform_plot.min.x + layout.waveform_plot.width() * ratio,
            layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
        );
        let expected_end = waveform_position_micros_from_point(&layout, &model, point);
        let (expected_start, expected_end) =
            shift_waveform_range_micros(milli(400), expected_end, milli(200), milli(400));
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            ),
            Some(UiAction::SetWaveformSelectionRange {
                start_micros: expected_start,
                end_micros: expected_end,
                preserve_view_edge: false,
            })
        );
    }
}

#[test]
fn command_click_clamps_existing_selection_translation_to_waveform_bounds() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(300, 900)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };
    let y = layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5;

    let left_point = Point::new(layout.waveform_plot.min.x, y);
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            left_point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: 0,
            end_micros: milli(600),
            preserve_view_edge: false,
        })
    );

    let right_point = Point::new(layout.waveform_plot.max.x, y);
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            right_point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(400),
            end_micros: 1_000_000,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn waveform_right_click_maps_to_edit_selection_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.5,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &AppModel::default(),
            point,
            ModifiersState::default()
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(500),
            end_micros: milli(500),
            preserve_view_edge: false,
        }
    );

    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(180, 820));

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::CONTROL),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(360),
            end_micros: 1_000_000,
            preserve_view_edge: false,
        }
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &model,
            point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: 0,
            end_micros: milli(640),
            preserve_view_edge: false,
        }
    );
}

#[test]
fn command_click_slides_existing_edit_selection_with_same_rules_as_playback_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.35,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let expected_position =
        waveform_position_micros_from_point(&layout, &AppModel::default(), point);
    let model = AppModel {
        waveform: WaveformPanelModel {
            edit_selection_milli: Some(crate::app::NormalizedRangeModel::new(200, 500)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::CONTROL),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: expected_position,
            end_micros: expected_position + milli(300),
            preserve_view_edge: false,
        }
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &model,
            point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: expected_position - milli(300),
            end_micros: expected_position,
            preserve_view_edge: false,
        }
    );
}

#[test]
fn command_waveform_edge_adjusts_default_to_full_span_when_no_selection_exists() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.25,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &AppModel::default(),
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(250),
            end_micros: 1_000_000,
            preserve_view_edge: false,
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &AppModel::default(),
            &mut shell_state,
            point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: 0,
            end_micros: milli(250),
            preserve_view_edge: false,
        })
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &AppModel::default(),
            point,
            ModifiersState::CONTROL,
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(250),
            end_micros: 1_000_000,
            preserve_view_edge: false,
        }
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &AppModel::default(),
            point,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: 0,
            end_micros: milli(250),
            preserve_view_edge: false,
        }
    );
}

#[test]
fn waveform_gutter_click_focuses_panel_instead_of_starting_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_card.min.x + 5.0,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &AppModel::default(),
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusWaveformPanel)
    );
}

#[test]
fn waveform_right_click_in_gutter_focuses_panel_instead_of_editing() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_card.min.x + 5.0,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &AppModel::default(),
            point,
            ModifiersState::default()
        ),
        UiAction::FocusWaveformPanel
    );
}
