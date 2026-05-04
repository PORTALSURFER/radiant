use super::*;

#[test]
fn immediate_e_after_selection_creation_uses_refreshed_waveform_focus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8),
        anchor.y,
    );
    let mut runner = NativeVelloRunner::new(
        NativeRunOptions::default(),
        ImmediateWaveformSelectionBridge::default(),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(anchor);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    let anchor_nanos = anchor_micros.saturating_mul(1000);
    let end_nanos = waveform_position_nanos_from_point(&layout, &runner.model, drag);
    assert!(
        !runner.handle_pointer_press_action(
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            false,
        )
    );
    assert!(runner.process_waveform_drag_immediately(drag));
    runner.finish_volume_drag(Some(MouseButton::Left));

    runner.handle_hotkey_press_for_tests(KeyCode::E);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos: anchor_nanos,
                end_nanos,
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::FinishWaveformSelectionRangeDrag,
            UiAction::SaveWaveformSelectionAsContent,
        ]
    );
}

#[test]
fn immediate_shift_e_after_selection_creation_uses_refreshed_waveform_focus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8),
        anchor.y,
    );
    let mut runner = NativeVelloRunner::new(
        NativeRunOptions::default(),
        ImmediateWaveformSelectionBridge::default(),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(anchor);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    let anchor_nanos = anchor_micros.saturating_mul(1000);
    let end_nanos = waveform_position_nanos_from_point(&layout, &runner.model, drag);
    assert!(
        !runner.handle_pointer_press_action(
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            false,
        )
    );
    assert!(runner.process_waveform_drag_immediately(drag));
    runner.finish_volume_drag(Some(MouseButton::Left));

    runner.modifiers = ModifiersState::SHIFT;
    runner.handle_hotkey_press_for_tests(KeyCode::E);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos: anchor_nanos,
                end_nanos,
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::FinishWaveformSelectionRangeDrag,
            UiAction::SaveWaveformSelectionAsAlternateContent,
        ]
    );
}

#[test]
fn immediate_selection_handle_press_after_creation_uses_refreshed_model() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8),
        anchor.y,
    );
    let mut runner = NativeVelloRunner::new(
        NativeRunOptions::default(),
        ImmediateWaveformSelectionBridge::default(),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(anchor);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    let anchor_nanos = anchor_micros.saturating_mul(1000);
    let end_nanos = waveform_position_nanos_from_point(&layout, &runner.model, drag);
    assert!(
        !runner.handle_pointer_press_action(
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            false,
        )
    );
    assert!(runner.process_waveform_drag_immediately(drag));
    runner.finish_volume_drag(Some(MouseButton::Left));

    let handle_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.8) - 4.0,
        layout.waveform_plot.max.y - 4.0,
    );
    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        handle_point,
        false,
        &mut action_emitted,
    ));
    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos: anchor_nanos,
                end_nanos,
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::FinishWaveformSelectionRangeDrag,
            UiAction::StartWaveformSelectionDrag {
                pointer_x: handle_point.x.round() as u16,
                pointer_y: handle_point.y.round() as u16,
            },
        ]
    );
}

#[test]
fn alt_drag_selection_updates_bypass_bpm_snap_until_alt_is_released() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let anchor = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let free_drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.73),
        anchor.y,
    );
    let snapped_drag = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.76),
        anchor.y,
    );
    let mut runner = NativeVelloRunner::new(
        NativeRunOptions::default(),
        ImmediateWaveformSelectionBridge::default(),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(anchor);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    let anchor_nanos = anchor_micros.saturating_mul(1000);
    assert!(
        !runner.handle_pointer_press_action(
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            false,
        )
    );

    runner.modifiers = ModifiersState::ALT;
    assert!(runner.process_waveform_drag_immediately(free_drag));
    assert_eq!(
        runner.bridge.actions.last(),
        Some(&UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: anchor_nanos,
            end_nanos: waveform_position_nanos_from_point(&layout, &runner.model, free_drag),
            snap_override: true,
            preserve_view_edge: false,
        })
    );

    runner.modifiers = ModifiersState::default();
    assert!(runner.process_waveform_drag_immediately(snapped_drag));
    assert_eq!(
        runner.bridge.actions.last(),
        Some(&UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: anchor_nanos,
            end_nanos: waveform_position_nanos_from_point(&layout, &runner.model, snapped_drag),
            snap_override: false,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn command_waveform_edge_adjust_press_emits_immediately_without_arming_drag() {
    let mut bridge = ImmediateWaveformSelectionBridge::default();
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.6),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::compat_app_contract::NormalizedRangeModel::new(
        200, 800,
    ));
    bridge.model = model;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.modifiers = ModifiersState::CONTROL;
    let position_nanos = waveform_position_nanos_from_point(&layout, &runner.model, point);

    let mut action_emitted = false;
    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );

    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: position_nanos,
            end_nanos: 800_000_000,
            snap_override: false,
            preserve_view_edge: false,
        }]
    );
    assert_eq!(runner.waveform_drag_mode, None);
}

#[test]
fn shift_click_playback_selection_slide_emits_immediately_without_arming_drag() {
    let mut bridge = ImmediateWaveformSelectionBridge::default();
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1234),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::compat_app_contract::NormalizedRangeModel::new(
        200, 800,
    ));
    bridge.model = model;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.modifiers = ModifiersState::SHIFT;
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);

    let position_nanos = waveform_position_nanos_from_point(&layout, &runner.model, point);
    let (expected_start, expected_end) =
        shift_waveform_range_nanos(200_000_000, position_nanos, 200_000_000, 800_000_000);

    let mut action_emitted = false;
    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );

    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: expected_start,
            end_nanos: expected_end,
            snap_override: false,
            preserve_view_edge: false,
        }]
    );
    assert_eq!(
        runner.last_emitted_waveform_drag_action,
        Some(UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: expected_start,
            end_nanos: expected_end,
            snap_override: false,
            preserve_view_edge: false,
        })
    );

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: expected_start,
            end_nanos: expected_end,
            snap_override: false,
            preserve_view_edge: false,
        }]
    );
}

#[test]
fn click_seek_release_pulls_queued_waveform_bridge_state_immediately() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut bridge = QueuedWaveformClickBridge::default();
    bridge.model.transport_running = false;
    bridge.model.waveform.selection_milli = Some(
        crate::compat_app_contract::NormalizedRangeModel::new(200, 800),
    );
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, point);
    let position_nanos = waveform_position_nanos_from_point(&layout, &runner.model, point);
    let emitted = runner
        .handle_pointer_press_action(UiAction::BeginWaveformSelectionAt { anchor_micros }, false);
    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(runner.bridge.project_calls, 3);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise { position_nanos },
        ]
    );
    assert!(runner.model.waveform.selection_milli.is_none());
    assert_eq!(
        runner.model.waveform.cursor_milli,
        Some((position_nanos / 1_000_000) as u16)
    );
    assert!(runner.model.transport_running);
}

#[test]
fn click_seek_release_arms_from_live_layout_borrow() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut bridge = QueuedWaveformClickBridge::default();
    bridge.model.transport_running = false;
    bridge.model.waveform.selection_milli = Some(
        crate::compat_app_contract::NormalizedRangeModel::new(200, 800),
    );
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);

    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, point);
    let position_nanos = waveform_position_nanos_from_point(&layout, &runner.model, point);
    let emitted = runner
        .with_shell_layout(|runner, layout| {
            runner.handle_pointer_press_action_at_point(
                UiAction::BeginWaveformSelectionAt { anchor_micros },
                false,
                layout,
                point,
            )
        })
        .expect("retained layout should be available");
    assert!(!emitted);
    assert!(runner.waveform_click_seek_press.is_some());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise { position_nanos },
        ]
    );
}
