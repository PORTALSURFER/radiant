use super::*;

#[test]
fn process_cursor_move_immediately_defers_when_layout_is_unavailable() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    assert_eq!(
        runner.process_cursor_move_immediately(Point::new(10.0, 20.0)),
        (false, false)
    );
}

#[test]
fn process_cursor_move_waveform_hover_only_marks_motion_overlay_dirty() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let first = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let second = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.7),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));

    let _ = runner.process_cursor_move_immediately(first);
    let _ = runner.frame_state.take_state_overlay();
    let _ = runner.frame_state.take_motion_overlay();
    let _ = runner.frame_state.take_model();
    let _ = runner.frame_state.take_scene();

    assert_eq!(runner.process_cursor_move_immediately(second), (true, true));
    assert!(!runner.frame_state.take_state_overlay());
    assert!(runner.frame_state.take_motion_overlay());
}

#[test]
fn cursor_activity_redraw_deadline_tracks_recent_pointer_activity() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let now = Instant::now();
    runner.last_redraw = now - runner.cursor_activity_redraw_interval;
    runner.note_cursor_activity(now);

    let deadline = runner.next_cursor_activity_redraw_deadline(now);

    assert_eq!(deadline, Some(now));
    assert!(runner.cursor_activity_redraw_until.is_some());
}

#[test]
fn cursor_activity_redraw_deadline_expires_after_activity_window() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let now = Instant::now();
    runner.note_cursor_activity(now);

    let deadline = runner.next_cursor_activity_redraw_deadline(
        now + CURSOR_ACTIVITY_REDRAW_WINDOW + Duration::from_millis(1),
    );

    assert_eq!(deadline, None);
    assert_eq!(runner.cursor_activity_redraw_until, None);
}

#[test]
fn rebuild_scene_processes_waveform_hover_motion_overlay_without_model_motion_change() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.motion_model = Some(NativeMotionModel::from_app_model(&runner.model));
    runner.frame_state.model_dirty = false;
    runner.frame_state.scene_dirty = false;
    runner.frame_state.state_overlay_dirty = false;
    runner.frame_state.motion_overlay_dirty = true;

    let effect = runner
        .shell_state
        .handle_cursor_move_effect(&layout, &runner.model, point);
    assert_ne!(effect, CursorMoveEffect::None);
    runner.rebuild_scene_if_needed();

    assert!(
        runner.waveform_motion_overlay_fingerprint.is_some(),
        "waveform-hover updates should rebuild motion overlay without requiring transport motion"
    );
}

#[test]
fn finish_volume_drag_flushes_pending_value_before_commit() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.queue_volume_milli(915);
    runner.volume_drag_active = true;
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Seek);
    runner.last_emitted_waveform_drag_action = Some(UiAction::SeekWaveformPrecise {
        position_nanos: 915_000_000,
    });
    runner.map_focus_drag_active = true;
    runner.last_emitted_map_drag_sample_id = Some(String::from("source::kick.wav"));

    runner.finish_volume_drag(None);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetVolume { value_milli: 915 },
            UiAction::CommitVolumeSetting,
        ]
    );
    assert!(!runner.volume_drag_active);
    assert_eq!(runner.last_emitted_volume_milli, None);
    assert_eq!(runner.pending_volume_milli, None);
    assert_eq!(runner.waveform_drag_mode, None);
    assert_eq!(runner.last_emitted_waveform_drag_action, None);
    assert!(!runner.map_focus_drag_active);
    assert_eq!(runner.last_emitted_map_drag_sample_id, None);
}

#[test]
fn finish_volume_drag_left_click_on_waveform_emits_seek() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let point = Point::new(
        layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
        layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
    );
    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, point);
    let position_nanos = waveform_position_nanos_from_point(&layout, &runner.model, point);
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(point);
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Selection {
        anchor_micros,
        boundary_lock: None,
    });
    runner.waveform_click_seek_press = Some(WaveformClickSeekPress {
        press_x: point.x,
        position_micros: anchor_micros,
        position_nanos,
        clear_selection_on_release: false,
    });
    runner.last_emitted_waveform_drag_action = None;

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SeekWaveformPrecise { position_nanos }]
    );
    assert_eq!(runner.waveform_drag_mode, None);
    assert_eq!(runner.last_emitted_waveform_drag_action, None);
}

#[test]
fn process_waveform_drag_immediately_ignores_tiny_selection_wobble() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let anchor = Point::new(
        layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
        layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
    );
    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    runner.shell_layout = Some(Arc::new(layout));
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Selection {
        anchor_micros,
        boundary_lock: None,
    });

    let handled = runner.process_waveform_drag_immediately(Point::new(anchor.x + 2.0, anchor.y));

    assert!(!handled);
    assert!(runner.bridge.actions.is_empty());
    assert_eq!(runner.last_emitted_waveform_drag_action, None);
}

#[test]
fn finish_volume_drag_small_waveform_wobble_still_emits_seek() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let anchor = Point::new(
        layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
        layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
    );
    let release = Point::new(anchor.x + 2.0, anchor.y);
    let anchor_micros = waveform_position_micros_from_point(&layout, &runner.model, anchor);
    let anchor_nanos = waveform_position_nanos_from_point(&layout, &runner.model, anchor);
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(release);
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Selection {
        anchor_micros,
        boundary_lock: None,
    });
    runner.waveform_click_seek_press = Some(WaveformClickSeekPress {
        press_x: anchor.x,
        position_micros: anchor_micros,
        position_nanos: anchor_nanos,
        clear_selection_on_release: false,
    });
    runner.last_emitted_waveform_drag_action = None;

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SeekWaveformPrecise {
            position_nanos: anchor_nanos
        }]
    );
    assert_eq!(runner.waveform_drag_mode, None);
    assert_eq!(runner.last_emitted_waveform_drag_action, None);
}

#[test]
fn flush_pending_input_drains_volume_and_cursor_updates() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.4),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    runner.shell_layout = Some(Arc::new(layout));
    runner.queue_volume_milli(777);
    runner.queue_cursor(point);

    assert!(runner.flush_pending_input());

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetVolume { value_milli: 777 }]
    );
    assert_eq!(runner.pending_volume_milli, None);
    assert_eq!(runner.pending_cursor, None);
    assert!(runner.frame_state.take_motion_overlay());
}
