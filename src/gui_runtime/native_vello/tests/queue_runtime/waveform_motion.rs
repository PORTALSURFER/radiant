use super::*;

#[test]
fn waveform_scrollbar_drag_emit_updates_action_queue() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 500_000;
    let shell_state = NativeShellState::new();
    let thumb_point = (layout.waveform_scrollbar_lane.min.x as i32
        ..=layout.waveform_scrollbar_lane.max.x as i32)
        .find_map(|x| {
            (layout.waveform_scrollbar_lane.min.y as i32
                ..=layout.waveform_scrollbar_lane.max.y as i32)
                .rev()
                .find_map(|y| {
                    let point = Point::new(x as f32, y as f32);
                    shell_state
                        .waveform_scrollbar_thumb_offset_at_point(&layout, &model, point)
                        .map(|_| point)
                })
        })
        .expect("waveform view should expose a hittable scrollbar thumb");
    let thumb_pointer_offset_x = shell_state
        .waveform_scrollbar_thumb_offset_at_point(&layout, &model, thumb_point)
        .expect("waveform thumb center should be hittable");
    let thumb_pointer_ratio_x = shell_state
        .waveform_scrollbar_thumb_ratio_at_point(&layout, &model, thumb_point)
        .expect("waveform thumb should expose a drag ratio");
    let expected_center = shell_state
        .waveform_scrollbar_view_center_for_drag(
            &layout,
            &model,
            layout.waveform_scrollbar_lane.max.x,
            thumb_pointer_offset_x,
        )
        .expect("dragging the waveform thumb should resolve a center");

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout));
    runner.shell_state = shell_state;
    runner.waveform_scrollbar_drag = Some(WaveformScrollbarDragState {
        thumb_pointer_offset_x,
        thumb_pointer_ratio_x,
    });

    let drag_point = Point::new(
        runner
            .shell_layout
            .as_ref()
            .unwrap()
            .waveform_scrollbar_lane
            .max
            .x,
        thumb_point.y,
    );
    assert!(runner.process_waveform_scrollbar_drag_immediately(drag_point));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformViewCenter {
            center_micros: expected_center,
            center_nanos: None,
        }]
    );
}

#[test]
fn waveform_middle_pan_emit_updates_action_queue() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 500_000;

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.waveform_pan_drag = Some(WaveformPanDragState {
        origin_x: layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        view_start_micros: 250_000,
        view_end_micros: 500_000,
        view_start_nanos: 250_000_000,
        view_end_nanos: 500_000_000,
    });

    let drag_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert!(runner.process_waveform_pan_drag_immediately(drag_point));
    let emitted = runner.bridge.actions.first().cloned();
    match emitted {
        Some(UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        }) => {
            assert!(center_micros < 375_000);
            assert_eq!(center_nanos, Some(center_micros * 1000));
        }
        other => panic!("expected waveform pan to emit SetWaveformViewCenter, got {other:?}"),
    }
}

#[test]
fn deep_zoom_pan_preserves_precise_click_play_mapping_after_refresh() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let plot_y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);
    let mut bridge = DeepZoomPanRefreshBridge::default();
    bridge.model.transport_running = false;
    bridge.model.waveform.view_start_micros = 500_000;
    bridge.model.waveform.view_end_micros = 500_000;
    bridge.model.waveform.view_start_nanos = 500_000_000;
    bridge.model.waveform.view_end_nanos = 500_000_200;
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.shell_layout = Some(Arc::new(layout.clone()));

    let origin_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5);
    let drag_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        plot_y,
    );
    runner.begin_waveform_pan_drag(origin_x);
    assert!(runner.process_waveform_pan_drag_immediately(drag_point));

    let expected_center_nanos = 500_000_050;
    assert_eq!(
        runner.bridge.actions.first(),
        Some(&UiAction::SetWaveformViewCenter {
            center_micros: 500_000,
            center_nanos: Some(expected_center_nanos),
        })
    );

    let click_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        plot_y,
    );
    let expected_position_nanos = 500_000_100;
    runner.last_cursor = Some(click_point);
    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        click_point,
        false,
        &mut action_emitted,
    ));
    assert!(!action_emitted);

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(runner.bridge.project_calls, 4);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetWaveformViewCenter {
                center_micros: 500_000,
                center_nanos: Some(expected_center_nanos),
            },
            UiAction::ClearWaveformSelection,
            UiAction::PlayWaveformAtPrecise {
                position_nanos: expected_position_nanos,
            },
        ]
    );
}
