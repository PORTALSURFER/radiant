use super::*;

#[test]
fn waveform_left_click_outside_selection_clears_it() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ClearWaveformSelection)
    );
}

#[test]
fn waveform_left_click_outside_edit_selection_clears_it() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ClearWaveformEditSelection)
    );
}

#[test]
fn waveform_left_click_outside_both_selection_types_clears_them_together() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(250, 750));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ClearWaveformSelections)
    );
}

#[test]
fn waveform_left_click_inside_edit_selection_only_clears_playback_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(100, 300));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.15),
        y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ClearWaveformSelection)
    );
}

#[test]
fn waveform_right_click_outside_edit_selection_clears_it() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        y,
    );

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::ClearWaveformEditSelection
    );
}
