use super::*;

#[test]
fn pending_volume_updates_flush_last_write_wins() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.queue_volume_milli(140);
    runner.queue_volume_milli(760);
    assert!(runner.flush_pending_volume_action());
    assert!(!runner.flush_pending_volume_action());
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetVolume { value_milli: 760 }]
    );
}

#[test]
fn immediate_volume_emit_updates_action_queue_without_pending_buffer() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.emit_volume_milli_immediately(505);

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetVolume { value_milli: 505 }]
    );
    assert_eq!(runner.pending_volume_milli, None);
}

#[test]
fn immediate_wheel_emit_updates_action_queue_without_pending_buffer() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner
        .shell_state
        .set_browser_row_hover_for_tests(Some(12));
    assert!(runner.process_wheel_rows_immediately(3));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart { visible_row: 3 }]
    );
    assert_eq!(
        runner.shell_state.state_overlay_fingerprint().hovered_browser_visible_row,
        None
    );
}

#[test]
fn browser_wheel_uses_rendered_viewport_start_when_model_start_is_stale() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let host_window_start = 100usize;
    let projected_rows = runner
        .shell_state
        .browser_viewport_len(&layout, &browser_model_with_rows(5_000, 0))
        .saturating_add(12);

    let build_model = |focused_visible_row: usize| {
        let mut model = AppModel::default();
        for offset in 0..projected_rows {
            let visible_row = host_window_start + offset;
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = 5_000;
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = host_window_start;
        model
    };

    let row_capacity = runner
        .shell_state
        .browser_viewport_len(&layout, &build_model(host_window_start));
    let bottom_focus = host_window_start + row_capacity.saturating_sub(1);
    let bottom_model = build_model(bottom_focus);
    let scrolled_start = runner
        .shell_state
        .browser_viewport_start_row(&layout, &bottom_model)
        .expect("bottom viewport should render at least one row");
    assert!(scrolled_start > host_window_start);

    let stale_model = build_model(scrolled_start + (row_capacity / 2));
    runner.model = Arc::new(stale_model);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(Point::new(
        layout.browser_rows.min.x + 24.0,
        layout.browser_rows.min.y + 24.0,
    ));

    runner.handle_mouse_wheel_for_tests(MouseScrollDelta::LineDelta(0.0, -1.0));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart {
            visible_row: scrolled_start + 1
        }]
    );
}

#[test]
fn browser_scrollbar_drag_emit_updates_action_queue() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = browser_model_with_rows(500, 120);
    let mut shell_state = NativeShellState::new();
    let thumb_point = ((layout.browser_rows.max.x as i32 - 16)..=layout.browser_rows.max.x as i32)
        .rev()
        .find_map(|x| {
            (layout.browser_rows.min.y as i32..=layout.browser_rows.max.y as i32).find_map(|y| {
                let point = Point::new(x as f32, y as f32);
                shell_state
                    .browser_scrollbar_thumb_offset_at_point(&layout, &model, point)
                    .map(|_| point)
            })
        })
        .expect("overflowing browser list should expose a hittable scrollbar thumb");
    let thumb_pointer_offset_y = shell_state
        .browser_scrollbar_thumb_offset_at_point(&layout, &model, thumb_point)
        .expect("thumb center should be hittable");
    let expected_visible_row = shell_state
        .browser_scrollbar_view_start_for_drag(
            &layout,
            &model,
            layout.browser_rows.max.y,
            thumb_pointer_offset_y,
        )
        .expect("dragging the thumb should resolve a view start");

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout));
    runner.shell_state = shell_state;
    runner.browser_scrollbar_drag = Some(BrowserScrollbarDragState {
        thumb_pointer_offset_y,
    });

    let drag_point = Point::new(
        thumb_point.x,
        runner.shell_layout.as_ref().unwrap().browser_rows.max.y,
    );
    assert!(runner.process_browser_scrollbar_drag_immediately(drag_point));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart {
            visible_row: expected_visible_row
        }]
    );
}

#[test]
fn browser_scrollbar_track_click_emit_updates_action_queue() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = browser_model_with_rows(500, 120);
    let mut shell_state = NativeShellState::new();
    let track_point = ((layout.browser_rows.max.x as i32 - 16)..=layout.browser_rows.max.x as i32)
        .rev()
        .find_map(|x| {
            let point = Point::new(x as f32, layout.browser_rows.max.y - 24.0);
            shell_state
                .browser_scrollbar_view_start_at_point(&layout, &model, point)
                .map(|_| point)
        })
        .expect("track click should find one hittable scrollbar point");
    let expected_visible_row = shell_state
        .browser_scrollbar_view_start_at_point(&layout, &model, track_point)
        .expect("track click should resolve a view start");

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout));
    runner.shell_state = shell_state;

    assert!(runner.process_browser_scrollbar_track_click_immediately(track_point));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart {
            visible_row: expected_visible_row
        }]
    );
}

#[test]
fn browser_row_pointer_action_clears_row_hover_before_emitting() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner
        .shell_state
        .set_browser_row_hover_for_tests(Some(18));

    assert!(runner.handle_pointer_press_action(
        UiAction::FocusBrowserRow { visible_row: 12 },
        false
    ));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FocusBrowserRow { visible_row: 12 }]
    );
    assert_eq!(
        runner.shell_state.state_overlay_fingerprint().hovered_browser_visible_row,
        None
    );
}

#[test]
fn waveform_scrollbar_drag_emit_updates_action_queue() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 500_000;
    let shell_state = NativeShellState::new();
    let thumb_point = (layout.waveform_plot.min.x as i32..=layout.waveform_plot.max.x as i32)
        .find_map(|x| {
            (layout.waveform_plot.min.y as i32..=layout.waveform_plot.max.y as i32)
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
    let expected_center = shell_state
        .waveform_scrollbar_view_center_for_drag(
            &layout,
            &model,
            layout.waveform_plot.max.x,
            thumb_pointer_offset_x,
        )
        .expect("dragging the waveform thumb should resolve a center");

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout));
    runner.shell_state = shell_state;
    runner.waveform_scrollbar_drag = Some(WaveformScrollbarDragState {
        thumb_pointer_offset_x,
    });

    let drag_point = Point::new(
        runner.shell_layout.as_ref().unwrap().waveform_plot.max.x,
        thumb_point.y,
    );
    assert!(runner.process_waveform_scrollbar_drag_immediately(drag_point));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformViewCenter {
            center_micros: expected_center
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
    });

    let drag_point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.75),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert!(runner.process_waveform_pan_drag_immediately(drag_point));
    let emitted = runner.bridge.actions.first().cloned();
    match emitted {
        Some(UiAction::SetWaveformViewCenter { center_micros }) => {
            assert!(center_micros < 375_000);
        }
        other => panic!("expected waveform pan to emit SetWaveformViewCenter, got {other:?}"),
    }
}
