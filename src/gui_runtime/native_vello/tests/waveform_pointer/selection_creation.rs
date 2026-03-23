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
            end_micros: milli(360),
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
        Some(UiAction::SeekWaveform {
            position_milli: 500
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
