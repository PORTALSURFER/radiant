use super::*;

#[test]
fn waveform_left_click_on_selection_edge_maps_to_resize_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x + 2.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(800),
            end_micros: position_micros,
            snap_override: false,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn waveform_left_click_just_outside_selection_edge_clears_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x - 2.0, y);
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
fn waveform_alt_click_on_selection_edge_maps_to_free_resize_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x + 2.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::ALT
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: milli(800),
            end_micros: position_micros,
            snap_override: false,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn waveform_shift_click_on_selection_edge_maps_to_smart_scale_resize_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x + 2.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT
        ),
        Some(UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: milli(800),
            end_micros: position_micros,
        })
    );
}

#[test]
fn waveform_right_click_on_edit_selection_edge_maps_to_resize_action() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x + 2.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(800),
            end_micros: position_micros,
            preserve_view_edge: false,
        }
    );
}

#[test]
fn waveform_resize_handle_hover_detects_edit_and_playback_handles() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(300, 700));
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let edit_left_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.3) + 2.0;
    let playback_left_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2) + 2.0;
    let outside_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5);

    assert!(waveform_resize_handle_hovered(
        &layout,
        &model,
        Point::new(edit_left_x, y),
    ));
    assert!(waveform_resize_handle_hovered(
        &layout,
        &model,
        Point::new(playback_left_x, y),
    ));
    assert!(!waveform_resize_handle_hovered(
        &layout,
        &model,
        Point::new(outside_x, y),
    ));
    let top_y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.1);
    let mut playback_only_model = AppModel::default();
    playback_only_model.waveform.selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    assert!(!waveform_resize_handle_hovered(
        &layout,
        &playback_only_model,
        Point::new(playback_left_x, top_y),
    ));
}

#[test]
fn waveform_left_click_on_selection_drag_handle_starts_drag() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8) - 4.0,
        layout.waveform_plot.max.y - 4.0,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::StartWaveformSelectionDrag {
            pointer_x: point.x.round() as u16,
            pointer_y: point.y.round() as u16,
        })
    );
}

#[test]
fn waveform_left_click_on_selection_shift_handle_starts_shift_gesture() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.max.y - 3.0,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::BeginWaveformSelectionShift {
            pointer_micros: milli(waveform_position_milli_from_point(&layout, &model, point)),
            start_micros: milli(200),
            end_micros: milli(800),
        })
    );
}

#[test]
fn waveform_left_click_on_edit_selection_shift_handle_starts_shift_gesture() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(300, 700));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.max.y - 3.0,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::BeginWaveformEditSelectionShift {
            pointer_micros: milli(waveform_position_milli_from_point(&layout, &model, point)),
            start_micros: milli(300),
            end_micros: milli(700),
        })
    );
}

#[test]
fn narrow_playback_selection_shift_handle_hit_rect_stays_stable() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    let selection = crate::sempal_app::NormalizedRangeModel::from_micros(500_000, 500_001);
    model.waveform.selection_milli = Some(selection);

    let rect = waveform_selection_shift_handle_hit_rect(&layout, &model, selection)
        .expect("narrow playback selection should still resolve a shift handle");

    assert!(rect.min.x.is_finite());
    assert!(rect.max.x.is_finite());
    assert!(rect.max.x >= rect.min.x);
    assert!(rect.min.x >= layout.waveform_plot.min.x);
    assert!(rect.max.x <= layout.waveform_plot.max.x);
}

#[test]
fn narrow_edit_selection_shift_handle_starts_gesture_without_panicking() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.edit_selection_milli = Some(
        crate::sempal_app::NormalizedRangeModel::from_micros(500_000, 500_001),
    );
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.max.y - 2.0,
    );

    assert_eq!(
        waveform_edit_action_from_pointer(&layout, &model, point, ModifiersState::default()),
        UiAction::BeginWaveformEditSelectionShift {
            pointer_micros: milli(waveform_position_milli_from_point(&layout, &model, point)),
            start_micros: 500_000,
            end_micros: 500_001,
        }
    );
}

#[test]
fn waveform_left_click_prefers_edit_resize_when_both_selection_types_exist() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_selection_milli =
        Some(crate::sempal_app::NormalizedRangeModel::new(200, 800));
    let mut shell_state = NativeShellState::new();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let start_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2);
    let point = Point::new(start_x + 2.0, y);
    let position_micros = waveform_position_micros_from_point(&layout, &model, point);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(800),
            end_micros: position_micros,
            preserve_view_edge: false,
        })
    );
}
