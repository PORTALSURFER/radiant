use super::*;

#[test]
fn waveform_click_over_edit_fade_handle_routes_fade_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_fade_in_end_milli = Some(300);
    model.waveform.edit_fade_in_end_micros = Some(milli(300));
    model.waveform.edit_fade_out_start_milli = Some(700);
    model.waveform.edit_fade_out_start_micros = Some(milli(700));
    let y = layout.waveform_plot.min.y + 3.5;
    let fade_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.3);
    let point = Point::new(fade_in_x + 1.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetWaveformEditFadeInEnd { position_micros })
    );
}

#[test]
fn waveform_right_click_over_edit_fade_handle_routes_edit_fade_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_fade_in_end_milli = Some(300);
    model.waveform.edit_fade_in_end_micros = Some(milli(300));
    model.waveform.edit_fade_out_start_milli = Some(700);
    model.waveform.edit_fade_out_start_micros = Some(milli(700));
    let y = layout.waveform_plot.min.y + 3.5;
    let fade_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.3);
    let point = Point::new(fade_in_x + 1.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::SetWaveformEditFadeInEnd { position_micros }
    );
}

#[test]
fn waveform_bottom_click_over_edit_fade_handle_routes_mute_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_end_micros = Some(milli(320));
    model.waveform.edit_fade_in_mute_start_milli = Some(150);
    model.waveform.edit_fade_in_mute_start_micros = Some(milli(150));
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_start_micros = Some(milli(690));
    model.waveform.edit_fade_out_mute_end_milli = Some(860);
    model.waveform.edit_fade_out_mute_end_micros = Some(milli(860));
    let bottom_y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.85);
    let fade_in_mute_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.15);
    let point = Point::new(fade_in_mute_x + 1.0, bottom_y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::SetWaveformEditFadeInMuteStart { position_micros }
    );
}

#[test]
fn waveform_alt_click_over_edit_fade_region_routes_curve_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_end_micros = Some(milli(320));
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_start_micros = Some(milli(690));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.2),
    );

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::ALT),
        UiAction::SetWaveformEditFadeInCurve { curve_milli: 800 }
    );
}

#[test]

fn waveform_bottom_click_without_edit_fade_does_not_hit_top_handle() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    let bottom_y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.85);
    let fade_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(fade_in_x + 1.0, bottom_y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: position_micros,
            end_micros: position_micros,
            preserve_view_edge: false,
        }
    );
}
