use super::*;

#[test]
fn handle_pointer_press_action_arms_edit_selection_shift_without_emitting() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::BeginWaveformEditSelectionShift {
            pointer_micros: milli(420),
            start_micros: milli(250),
            end_micros: milli(650),
        },
        false,
    );

    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());
    assert_eq!(
        runner.waveform_drag_mode,
        Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: milli(420),
            start_micros: milli(250),
            end_micros: milli(650),
        })
    );
}

#[test]
fn handle_pointer_press_action_starts_selection_drag_immediately() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::StartWaveformSelectionDrag {
            pointer_x: 320,
            pointer_y: 240,
        },
        false,
    );

    assert!(emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::StartWaveformSelectionDrag {
            pointer_x: 320,
            pointer_y: 240,
        }]
    );
    assert!(runner.selection_drag_active);
}

#[test]
fn handle_pointer_press_action_starts_circular_slide_immediately() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::BeginWaveformCircularSlide {
            anchor_micros: milli(320),
        },
        false,
    );

    assert!(emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::BeginWaveformCircularSlide {
            anchor_micros: milli(320),
        }]
    );
    assert_eq!(
        runner.waveform_drag_mode,
        Some(WaveformPointerDragMode::CircularSlide {
            anchor_micros: milli(320),
        })
    );
}

#[test]
fn finish_volume_drag_emits_finish_edit_fade_action_for_waveform_fade_handles() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::EditFadeOutMuteEnd);
    runner.last_emitted_waveform_drag_action = Some(UiAction::SetWaveformEditFadeOutMuteEnd {
        position_micros: milli(500),
    });

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformEditFadeDrag]
    );
}

#[test]
fn finish_volume_drag_emits_finish_selection_drag_for_active_selection_export() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.selection_drag_active = true;

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformSelectionDrag]
    );
    assert!(!runner.selection_drag_active);
}

#[test]
fn finish_volume_drag_emits_finish_circular_slide_for_active_waveform_slide() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::CircularSlide {
        anchor_micros: milli(320),
    });

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformCircularSlide]
    );
}

#[test]
fn finish_volume_drag_emits_finish_selection_smart_scale_drag_for_alt_resize() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::SelectionSmartScale {
        anchor_micros: milli(250),
        boundary_lock: None,
    });

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformSelectionSmartScaleDrag]
    );
}

#[test]
fn finish_volume_drag_emits_finish_selection_range_drag_for_plain_selection_gestures() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::SelectionShift {
        pointer_micros: milli(320),
        start_micros: milli(250),
        end_micros: milli(650),
    });
    runner.last_emitted_waveform_drag_action = Some(UiAction::SetWaveformSelectionRange {
        start_micros: milli(300),
        end_micros: milli(700),
        snap_override: false,
        preserve_view_edge: false,
    });

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformSelectionRangeDrag]
    );
}

#[test]
fn finish_volume_drag_emits_finish_edit_selection_drag_for_plain_edit_gestures() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::EditSelectionShift {
        pointer_micros: milli(420),
        start_micros: milli(250),
        end_micros: milli(650),
    });
    runner.last_emitted_waveform_drag_action = Some(UiAction::SetWaveformEditSelectionRange {
        start_micros: milli(300),
        end_micros: milli(700),
        preserve_view_edge: false,
    });

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FinishWaveformEditSelectionDrag]
    );
}

#[test]
fn circular_slide_release_does_not_trigger_click_playback() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(point);

    let emitted = runner.handle_pointer_press_action(
        UiAction::BeginWaveformCircularSlide {
            anchor_micros: milli(250),
        },
        false,
    );
    assert!(emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::BeginWaveformCircularSlide {
                anchor_micros: milli(250),
            },
            UiAction::FinishWaveformCircularSlide,
        ]
    );
}

#[test]
fn outside_selection_click_release_clears_playback_selection_then_plays_from_click() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);
    let model = Arc::make_mut(&mut runner.model);
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));

    let emitted = runner.handle_pointer_press_action(
        UiAction::BeginWaveformSelectionAt {
            anchor_micros: milli(100),
        },
        false,
    );
    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: waveform_position_nanos_from_point(&layout, &runner.model, point),
            },
        ]
    );
}

#[test]
fn click_just_outside_selection_edge_clears_playback_selection_then_plays_from_click() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let selection = crate::app::NormalizedRangeModel::new(200, 800);
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2) - 2.0,
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);
    let model = Arc::make_mut(&mut runner.model);
    model.waveform.selection_milli = Some(selection);

    let mut shell_state = NativeShellState::new();
    let action = action_from_pointer(
        &layout,
        &runner.model,
        &mut shell_state,
        point,
        ModifiersState::default(),
    )
    .expect("waveform click should resolve to an action");
    assert_eq!(action, UiAction::ClearWaveformSelection);

    let emitted = runner.handle_pointer_press_action(action, false);
    assert!(emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: waveform_position_nanos_from_point(&layout, &runner.model, point),
            },
        ]
    );
}

#[test]
fn clear_playback_selection_press_release_plays_from_click_point() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);
    let model = Arc::make_mut(&mut runner.model);
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));

    let emitted = runner.handle_pointer_press_action(UiAction::ClearWaveformSelection, false);
    assert!(emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: waveform_position_nanos_from_point(&layout, &runner.model, point),
            },
        ]
    );
}

#[test]
fn clear_playback_selection_press_release_while_stopped_sets_cursor_then_plays() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);
    let model = Arc::make_mut(&mut runner.model);
    model.transport_running = false;
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));

    let emitted = runner.handle_pointer_press_action(UiAction::ClearWaveformSelection, false);
    assert!(emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: waveform_position_nanos_from_point(&layout, &runner.model, point),
            },
        ]
    );
}

#[test]
fn clear_both_waveform_marks_press_release_plays_from_click_point() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);
    let model = Arc::make_mut(&mut runner.model);
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    model.waveform.edit_selection_milli = Some(crate::app::NormalizedRangeModel::new(250, 750));

    let emitted = runner.handle_pointer_press_action(UiAction::ClearWaveformSelections, false);
    assert!(emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelections,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: waveform_position_nanos_from_point(&layout, &runner.model, point),
            },
        ]
    );
}

#[test]
fn outside_selection_drag_clears_then_creates_new_selection() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let drag = Point::new(anchor.x + 24.0, anchor.y);
    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(anchor);
    Arc::make_mut(&mut runner.model).waveform.selection_milli =
        Some(crate::app::NormalizedRangeModel::new(200, 800));

    let emitted = runner.handle_pointer_press_action(UiAction::ClearWaveformSelection, false);
    assert!(emitted);

    assert!(runner.process_waveform_drag_immediately(drag));
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros: waveform_position_micros_from_point(&layout, &runner.model, drag),
                snap_override: false,
                preserve_view_edge: false,
            },
        ]
    );
}

#[test]
fn edit_selection_drag_preserves_exact_micro_anchor_through_first_drag_update() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = milli(200);
    model.waveform.view_end_micros = milli(400);
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1234),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1712),
        anchor.y,
    );
    let anchor_micros = waveform_position_micros_from_point(&layout, &model, anchor);
    let drag_micros = waveform_position_micros_from_point(&layout, &model, drag);
    runner.model = Arc::new(model.clone());
    runner.shell_layout = Some(Arc::new(layout.clone()));

    let press_action =
        waveform_edit_action_from_pointer(&layout, &model, anchor, ModifiersState::default());
    assert_eq!(
        press_action,
        UiAction::SetWaveformEditSelectionRange {
            start_micros: anchor_micros,
            end_micros: anchor_micros,
            preserve_view_edge: false,
        }
    );

    let emitted = runner.handle_pointer_press_action(press_action, false);
    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());

    assert!(runner.process_waveform_drag_immediately(drag));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformEditSelectionRange {
            start_micros: anchor_micros,
            end_micros: drag_micros,
            preserve_view_edge: false,
        }]
    );
}

#[test]
/// Drag waveform actions should clamp pointer positions and preserve anchors or widths.
fn waveform_drag_action_clamps_and_preserves_selection_anchor() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let model = AppModel::default();
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let left = Point::new(layout.waveform_plot.min.x - 200.0, y);
    let right = Point::new(layout.waveform_plot.max.x + 200.0, y);
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            left,
            WaveformPointerDragMode::CircularSlide {
                anchor_micros: milli(200),
            },
            ModifiersState::default(),
        ),
        UiAction::UpdateWaveformCircularSlide {
            position_micros: milli(0)
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            left,
            WaveformPointerDragMode::Seek,
            ModifiersState::default(),
        ),
        UiAction::SeekWaveformPrecise { position_nanos: 0 }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::Cursor,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformCursorPrecise {
            position_nanos: 1_000_000_000
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::Selection {
                anchor_micros: milli(200),
                boundary_lock: None,
            },
            ModifiersState::default(),
        ),
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(200),
            end_micros: milli(1000),
            snap_override: false,
            preserve_view_edge: true,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::Selection {
                anchor_micros: milli(200),
                boundary_lock: None,
            },
            ModifiersState::ALT,
        ),
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(200),
            end_micros: milli(1000),
            snap_override: true,
            preserve_view_edge: true,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::SelectionSmartScale {
                anchor_micros: milli(200),
                boundary_lock: None,
            },
            ModifiersState::default(),
        ),
        UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: milli(200),
            end_micros: milli(1000),
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::SelectionShift {
                pointer_micros: milli(300) * 1000,
                start_micros: milli(200) * 1000,
                end_micros: milli(400) * 1000,
            },
            ModifiersState::default(),
        ),
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(800),
            end_micros: milli(1000),
            snap_override: false,
            preserve_view_edge: false,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::SelectionShift {
                pointer_micros: milli(300) * 1000,
                start_micros: milli(200) * 1000,
                end_micros: milli(400) * 1000,
            },
            ModifiersState::ALT,
        ),
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(800),
            end_micros: milli(1000),
            snap_override: true,
            preserve_view_edge: false,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::EditSelection {
                anchor_micros: milli(300),
                boundary_lock: None,
            },
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(300),
            end_micros: milli(1000),
            preserve_view_edge: true,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            left,
            WaveformPointerDragMode::EditSelectionShift {
                pointer_micros: milli(550),
                start_micros: milli(400),
                end_micros: milli(700),
            },
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(0),
            end_micros: milli(300),
            preserve_view_edge: false,
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            left,
            WaveformPointerDragMode::EditFadeInEnd,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeInEnd {
            position_micros: milli(0)
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            left,
            WaveformPointerDragMode::EditFadeInMuteStart,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeInMuteStart {
            position_micros: milli(0)
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            Point::new(layout.waveform_plot.min.x, layout.waveform_plot.min.y),
            WaveformPointerDragMode::EditFadeInCurve,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeInCurve { curve_milli: 1000 }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::EditFadeOutStart,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeOutStart {
            position_micros: milli(1000)
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            right,
            WaveformPointerDragMode::EditFadeOutMuteEnd,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeOutMuteEnd {
            position_micros: milli(1000)
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            &model,
            Point::new(layout.waveform_plot.max.x, layout.waveform_plot.max.y),
            WaveformPointerDragMode::EditFadeOutCurve,
            ModifiersState::default(),
        ),
        UiAction::SetWaveformEditFadeOutCurve { curve_milli: 0 }
    );
}

#[test]
fn waveform_resize_drag_keeps_outside_plot_lock_across_zoom_changes() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let outside_right = Point::new(layout.waveform_plot.max.x + 32.0, y);
    let mut model = AppModel::default();
    model.waveform.view_start_micros = milli(200);
    model.waveform.view_end_micros = milli(400);

    let (first_action, locked_mode) = waveform_drag_action_and_mode_for_point(
        &layout,
        &model,
        outside_right,
        WaveformPointerDragMode::Selection {
            anchor_micros: milli(250),
            boundary_lock: None,
        },
        ModifiersState::default(),
    );
    assert_eq!(
        first_action,
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(250),
            end_micros: milli(400),
            snap_override: false,
            preserve_view_edge: true,
        }
    );

    model.waveform.view_start_micros = milli(50);
    model.waveform.view_end_micros = milli(850);
    let (second_action, relocked_mode) = waveform_drag_action_and_mode_for_point(
        &layout,
        &model,
        outside_right,
        locked_mode,
        ModifiersState::default(),
    );
    assert_eq!(
        second_action,
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(250),
            end_micros: milli(400),
            snap_override: false,
            preserve_view_edge: true,
        }
    );
    assert_eq!(locked_mode, relocked_mode);
}
