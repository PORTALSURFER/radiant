use super::*;

fn expected_command_edge_adjust_range(
    selection_start: u32,
    selection_end: u32,
    position_micros: u32,
    shift: bool,
) -> (u32, u32) {
    if shift {
        if position_micros < selection_start {
            shift_waveform_range_micros(
                selection_end,
                position_micros,
                selection_start,
                selection_end,
            )
        } else {
            (selection_start, position_micros)
        }
    } else if position_micros > selection_end {
        shift_waveform_range_micros(
            selection_start,
            position_micros,
            selection_start,
            selection_end,
        )
    } else {
        (position_micros, selection_end)
    }
}

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
fn command_click_adjusts_start_until_overflow_then_slides_playback_selection() {
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
        let position_micros = waveform_position_micros_from_point(&layout, &model, point);
        let (expected_start, expected_end) =
            expected_command_edge_adjust_range(milli(200), milli(400), position_micros, false);
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
                end_micros: expected_end,
                preserve_view_edge: false,
            })
        );
    }
}

#[test]
fn command_shift_click_adjusts_end_until_overflow_then_slides_playback_selection() {
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
        let position_micros = waveform_position_micros_from_point(&layout, &model, point);
        let (expected_start, expected_end) =
            expected_command_edge_adjust_range(milli(200), milli(400), position_micros, true);
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
            start_micros: milli(500),
            end_micros: milli(820),
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
            start_micros: milli(180),
            end_micros: milli(500),
            preserve_view_edge: false,
        }
    );
}

#[test]
fn command_click_slides_existing_edit_selection_only_as_overflow_recovery() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let model = AppModel {
        waveform: WaveformPanelModel {
            edit_selection_milli: Some(crate::app::NormalizedRangeModel::new(200, 500)),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    let y = layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5;
    let before_start = Point::new(layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.1, y);
    let after_end = Point::new(layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.8, y);

    let before_position = waveform_position_micros_from_point(&layout, &model, before_start);
    let after_position = waveform_position_micros_from_point(&layout, &model, after_end);
    let (ctrl_shift_start, ctrl_shift_end) =
        expected_command_edge_adjust_range(milli(200), milli(500), before_position, true);
    let (ctrl_start, ctrl_end) =
        expected_command_edge_adjust_range(milli(200), milli(500), after_position, false);

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, after_end, ModifiersState::CONTROL),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: ctrl_start,
            end_micros: ctrl_end,
            preserve_view_edge: false,
        }
    );

    assert_eq!(
        waveform_edit_action_from_pointer(
            &layout,
            &model,
            before_start,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: ctrl_shift_start,
            end_micros: ctrl_shift_end,
            preserve_view_edge: false,
        }
    );
}

#[test]
fn command_waveform_edge_adjust_without_selection_preserves_existing_fallbacks() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + layout.waveform_plot.width() * 0.25,
        layout.waveform_plot.min.y + layout.waveform_plot.height() * 0.5,
    );
    let expected_position_nanos =
        waveform_position_nanos_from_point(&layout, &AppModel::default(), point);
    let expected_position_micros =
        waveform_position_micros_from_point(&layout, &AppModel::default(), point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &AppModel::default(),
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformCursorPrecise {
            position_nanos: expected_position_nanos,
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
        Some(UiAction::SetWaveformCursorPrecise {
            position_nanos: expected_position_nanos,
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
            start_micros: expected_position_micros,
            end_micros: expected_position_micros,
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
            start_micros: expected_position_micros,
            end_micros: expected_position_micros,
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
