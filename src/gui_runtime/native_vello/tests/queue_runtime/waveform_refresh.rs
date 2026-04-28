use super::*;

#[test]
fn begin_waveform_selection_press_does_not_project_zero_width_selection() {
    let mut bridge = ImmediateWaveformSelectionBridge::default();

    bridge.reduce_action(UiAction::BeginWaveformSelectionAt {
        anchor_micros: 125_000,
    });

    assert_eq!(
        bridge.model.focus_context,
        crate::sempal_app::FocusContextModel::Waveform
    );
    assert!(bridge.model.waveform.selection_milli.is_none());
    assert_eq!(
        bridge.actions,
        vec![UiAction::BeginWaveformSelectionAt {
            anchor_micros: 125_000,
        }]
    );
}

#[test]
fn waveform_wheel_zoom_refreshes_local_view_before_next_drag_sample() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        y,
    );
    let drag_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.9),
        y,
    );
    let mut bridge = WaveformZoomRefreshBridge::default();
    bridge.model.waveform.view_start_micros = 200_000;
    bridge.model.waveform.view_end_micros = 400_000;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(wheel_point);
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Selection {
        anchor_micros: 250_000,
        boundary_lock: None,
    });

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -3.0));

    assert_eq!(runner.bridge.project_calls, 1);
    assert!(runner.waveform_view_refresh_pending);
    assert_eq!(runner.model.waveform.view_start_micros, 200_000);
    assert_eq!(runner.model.waveform.view_end_micros, 400_000);

    assert!(runner.process_waveform_drag_immediately(drag_point));
    assert_eq!(runner.bridge.project_calls, 2);
    assert!(!runner.waveform_view_refresh_pending);
    assert_eq!(runner.model.waveform.view_start_micros, 100_000);
    assert_eq!(runner.model.waveform.view_end_micros, 900_000);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 3,
                anchor_ratio_micros: Some(750_000),
            },
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos: 250_000,
                end_nanos: 819_999_981,
                snap_override: false,
                preserve_view_edge: false,
            },
        ]
    );
}

#[test]
fn waveform_wheel_zoom_noop_refresh_does_not_emit_view_center_pan() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut bridge = WaveformNoopZoomRefreshBridge::default();
    bridge.model.waveform.view_start_micros = 500_000;
    bridge.model.waveform.view_end_micros = 500_000;
    bridge.model.waveform.view_start_nanos = 500_000_000;
    bridge.model.waveform.view_end_nanos = 500_000_200;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(wheel_point);

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, 1.0));

    assert!(runner.waveform_view_refresh_pending);
    runner.refresh_waveform_view_if_needed();
    assert!(!runner.waveform_view_refresh_pending);
    assert_eq!(runner.bridge.project_calls, 2);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: Some(250_000),
        }]
    );
}

#[test]
fn waveform_click_play_refreshes_pending_zoom_view_before_mapping_press_position() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        y,
    );
    let click_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        y,
    );
    let mut bridge = DeepZoomClickRefreshBridge::default();
    bridge.model.transport_running = false;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(wheel_point);

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -3.0));

    assert!(runner.waveform_view_refresh_pending);
    let expected_position_nanos =
        waveform_position_nanos_from_point(&layout, &runner.bridge.model, click_point);

    runner.last_cursor = Some(click_point);
    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        click_point,
        false,
        &mut action_emitted,
    ));
    assert!(!action_emitted);
    assert!(!runner.waveform_view_refresh_pending);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 3,
                anchor_ratio_micros: Some(750_000),
            },
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: expected_position_nanos,
            },
        ]
    );
}

#[test]
fn waveform_middle_pan_refreshes_stale_view_before_capturing_drag_baseline() {
    let mut bridge = WaveformZoomRefreshBridge::default();
    bridge.model.waveform.view_start_micros = 200_000;
    bridge.model.waveform.view_end_micros = 400_000;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(wheel_point);

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -3.0));

    assert!(runner.waveform_view_refresh_pending);
    assert_eq!(runner.bridge.project_calls, 1);

    let origin_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5);
    runner.begin_waveform_pan_drag(origin_x);

    assert_eq!(runner.bridge.project_calls, 2);
    assert!(!runner.waveform_view_refresh_pending);
    assert_eq!(
        runner.waveform_pan_drag,
        Some(WaveformPanDragState {
            origin_x,
            view_start_micros: 100_000,
            view_end_micros: 900_000,
            view_start_nanos: 100_000_000,
            view_end_nanos: 900_000_000,
        })
    );
}

#[test]
fn waveform_scrollbar_thumb_press_refreshes_pending_zoom_before_hit_testing() {
    let mut bridge = WaveformScrollbarPressBridge::default();
    bridge.model.waveform.view_start_micros = 250_000;
    bridge.model.waveform.view_end_micros = 500_000;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(wheel_point);

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -3.0));

    assert!(runner.waveform_view_refresh_pending);
    assert_eq!(runner.bridge.project_calls, 1);

    let stale_model = runner.model.as_ref().clone();
    let refreshed_model = runner.bridge.model.clone();
    let thumb_point = (layout.waveform_scrollbar_lane.min.x as i32
        ..=layout.waveform_scrollbar_lane.max.x as i32)
        .find_map(|x| {
            (layout.waveform_scrollbar_lane.min.y as i32
                ..=layout.waveform_scrollbar_lane.max.y as i32)
                .rev()
                .find_map(|y| {
                    let point = Point::new(x as f32, y as f32);
                    runner
                        .shell_state
                        .waveform_scrollbar_thumb_offset_at_point(&layout, &stale_model, point)
                        .filter(|_| {
                            runner
                                .shell_state
                                .waveform_scrollbar_thumb_offset_at_point(
                                    &layout,
                                    &refreshed_model,
                                    point,
                                )
                                .is_none()
                        })
                        .map(|_| point)
                })
        })
        .expect("zoomed waveform should move the thumb away from at least one stale pixel");
    let _stale_thumb_ratio_x = runner
        .shell_state
        .waveform_scrollbar_thumb_ratio_at_point(&layout, &stale_model, thumb_point)
        .expect("stale thumb should expose a drag ratio");
    let expected_center = runner
        .shell_state
        .waveform_scrollbar_view_center_at_point(&layout, &refreshed_model, thumb_point)
        .expect("refreshed track click should resolve a waveform center");

    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        thumb_point,
        false,
        &mut action_emitted,
    ));

    assert_eq!(runner.bridge.project_calls, 2);
    assert!(action_emitted);
    assert_eq!(runner.waveform_scrollbar_drag, None);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 3,
                anchor_ratio_micros: Some(750_000),
            },
            UiAction::SetWaveformViewCenter {
                center_micros: expected_center,
                center_nanos: None,
            },
        ]
    );
}

#[test]
fn waveform_scrollbar_stale_thumb_press_retargets_after_refresh() {
    let mut bridge = WaveformScrollbarPressBridge::default();
    bridge.model.waveform.view_start_micros = 250_000;
    bridge.model.waveform.view_end_micros = 500_000;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let wheel_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(wheel_point);

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -3.0));

    let stale_model = runner.model.as_ref().clone();
    let refreshed_model = runner.bridge.model.clone();
    let thumb_point = (layout.waveform_scrollbar_lane.min.x as i32
        ..=layout.waveform_scrollbar_lane.max.x as i32)
        .find_map(|x| {
            (layout.waveform_scrollbar_lane.min.y as i32
                ..=layout.waveform_scrollbar_lane.max.y as i32)
                .rev()
                .find_map(|y| {
                    let point = Point::new(x as f32, y as f32);
                    runner
                        .shell_state
                        .waveform_scrollbar_thumb_offset_at_point(&layout, &stale_model, point)
                        .filter(|_| {
                            runner
                                .shell_state
                                .waveform_scrollbar_thumb_offset_at_point(
                                    &layout,
                                    &refreshed_model,
                                    point,
                                )
                                .is_none()
                        })
                        .map(|_| point)
                })
        })
        .expect("zoomed waveform should move the thumb away from at least one stale pixel");
    let _stale_thumb_ratio_x = runner
        .shell_state
        .waveform_scrollbar_thumb_ratio_at_point(&layout, &stale_model, thumb_point)
        .expect("stale thumb should expose a drag ratio");
    let expected_center = runner
        .shell_state
        .waveform_scrollbar_view_center_at_point(&layout, &refreshed_model, thumb_point)
        .expect("refreshed track click should resolve a waveform center");
    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        thumb_point,
        false,
        &mut action_emitted,
    ));

    assert!(action_emitted);
    assert_eq!(runner.bridge.project_calls, 2);
    assert_eq!(runner.waveform_scrollbar_drag, None);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 3,
                anchor_ratio_micros: Some(750_000),
            },
            UiAction::SetWaveformViewCenter {
                center_micros: expected_center,
                center_nanos: None,
            },
        ]
    );
}
