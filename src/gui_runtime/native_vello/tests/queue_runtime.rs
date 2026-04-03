use super::*;

fn browser_drag_model() -> AppModel {
    let mut model = browser_model_with_rows(4, 0);
    model.sources = SourcesPanelModel {
        folder_rows: vec![
            crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
                .with_source_index(0),
            crate::app::FolderRowModel::new("Drums", "drums", 1, false, false, false, true, true)
                .with_source_index(7),
        ],
        ..SourcesPanelModel::default()
    };
    model
}

fn browser_row_point(layout: &ShellLayout) -> Point {
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    Point::new(
        layout.browser_rows.min.x + 24.0,
        layout.browser_rows.min.y + (style.sizing.browser_row_height * 0.5),
    )
}

fn folder_row_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    row_index: usize,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if shell_state
                .folder_row_at_point(layout, model, point)
                .is_some_and(|(_, found_row)| found_row == row_index)
            {
                return point;
            }
        }
    }
    panic!("expected hittable folder row at index {row_index}");
}

fn folder_panel_background_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if shell_state.folder_panel_contains_point(layout, model, point)
                && shell_state
                    .folder_row_at_point(layout, model, point)
                    .is_none()
                && shell_state
                    .source_row_at_point(layout, model, point)
                    .is_none()
            {
                return point;
            }
        }
    }
    panic!("expected hittable folder-panel background point");
}

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
    runner.shell_state.set_browser_row_hover_for_tests(Some(12));
    assert!(runner.process_wheel_rows_immediately(3));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart { visible_row: 3 }]
    );
    assert_eq!(
        runner
            .shell_state
            .state_overlay_fingerprint()
            .hovered_browser_visible_row,
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
    runner.shell_state.set_browser_row_hover_for_tests(Some(18));
    runner.last_cursor = Some(Point::new(24.0, 24.0));

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 12 }, false)
    );
    assert!(runner.pending_browser_row_press.is_some());
    assert!(runner.bridge.actions.is_empty());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::FocusBrowserRow { visible_row: 12 }]
    );
    assert_eq!(
        runner
            .shell_state
            .state_overlay_fingerprint()
            .hovered_browser_visible_row,
        None
    );
}

#[test]
fn browser_row_pointer_action_syncs_viewport_before_bottom_edge_autoscroll() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    runner.model = Arc::new(browser_model_with_rows(40, 0));
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(browser_row_point(runner.shell_layout.as_ref().unwrap()));

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 18 }, false)
    );
    assert!(runner.pending_browser_row_press.is_some());
    assert!(runner.bridge.actions.is_empty());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetBrowserViewStart { visible_row: 1 },
            UiAction::FocusBrowserRow { visible_row: 18 }
        ]
    );
}

#[test]
fn browser_row_drag_starts_updates_and_finishes_without_click_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(browser_drag_model());
    runner.shell_layout = Some(Arc::new(layout.clone()));
    let press_point = browser_row_point(&layout);
    let drag_point = folder_row_point(&mut runner.shell_state, &layout, &runner.model, 1);
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    assert!(runner.bridge.actions.is_empty());

    runner.handle_cursor_moved_for_tests(drag_point);

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartBrowserSampleDrag {
                visible_row: 0,
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
                hovered_folder_pane: Some(crate::app::FolderPaneIdModel::Upper),
                hovered_folder_row: Some(7),
                over_folder_panel: Some(crate::app::FolderPaneIdModel::Upper),
                shift_down: false,
                alt_down: false,
            },
        ]
    );

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartBrowserSampleDrag {
                visible_row: 0,
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x: drag_point.x.round() as u16,
                pointer_y: drag_point.y.round() as u16,
                hovered_folder_pane: Some(crate::app::FolderPaneIdModel::Upper),
                hovered_folder_row: Some(7),
                over_folder_panel: Some(crate::app::FolderPaneIdModel::Upper),
                shift_down: false,
                alt_down: false,
            },
            UiAction::FinishBrowserSampleDrag,
        ]
    );
}

#[test]
fn browser_row_drag_reports_folder_panel_background_without_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(browser_drag_model());
    runner.shell_layout = Some(Arc::new(layout.clone()));
    let press_point = browser_row_point(&layout);
    let drag_point = folder_panel_background_point(&mut runner.shell_state, &layout, &runner.model);
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    runner.handle_cursor_moved_for_tests(drag_point);

    let update = runner
        .bridge
        .actions
        .last()
        .cloned()
        .expect("dragging should emit a browser drag update");
    assert_eq!(
        update,
        UiAction::UpdateBrowserSampleDrag {
            pointer_x: drag_point.x.round() as u16,
            pointer_y: drag_point.y.round() as u16,
            hovered_folder_pane: None,
            hovered_folder_row: None,
            over_folder_panel: Some(crate::app::FolderPaneIdModel::Upper),
            shift_down: false,
            alt_down: false,
        }
    );
}

#[derive(Default)]
struct WaveformZoomRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for WaveformZoomRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 100_000;
            self.model.waveform.view_end_micros = 900_000;
            self.model.waveform.view_start_nanos = 100_000_000;
            self.model.waveform.view_end_nanos = 900_000_000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct DeepZoomClickRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for DeepZoomClickRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 500_000;
            self.model.waveform.view_end_micros = 500_000;
            self.model.waveform.view_start_nanos = 500_000_000;
            self.model.waveform.view_end_nanos = 500_000_200;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct DeepZoomPanRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for DeepZoomPanRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if let UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        } = &action
        {
            let span_nanos = self
                .model
                .waveform
                .view_end_nanos
                .saturating_sub(self.model.waveform.view_start_nanos);
            let resolved_center_nanos = center_nanos.unwrap_or(center_micros.saturating_mul(1000));
            let half_span = span_nanos / 2;
            let max_start_nanos = 1_000_000_000u32.saturating_sub(span_nanos);
            let next_start_nanos = resolved_center_nanos
                .saturating_sub(half_span)
                .min(max_start_nanos);
            let next_end_nanos = next_start_nanos
                .saturating_add(span_nanos)
                .min(1_000_000_000);
            self.model.waveform.view_start_nanos = next_start_nanos;
            self.model.waveform.view_end_nanos = next_end_nanos;
            self.model.waveform.view_start_micros = next_start_nanos / 1000;
            self.model.waveform.view_end_micros = next_end_nanos / 1000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct WaveformScrollbarPressBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for WaveformScrollbarPressBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 0;
            self.model.waveform.view_end_micros = 250_000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct ImmediateWaveformSelectionBridge {
    actions: Vec<UiAction>,
    model: AppModel,
}

impl NativeAppBridge for ImmediateWaveformSelectionBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::BeginWaveformSelectionAt { .. } => {
                self.model.focus_context = crate::app::FocusContextModel::Waveform;
            }
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                ..
            } => {
                self.model.focus_context = crate::app::FocusContextModel::Waveform;
                self.model.waveform.selection_milli = Some(
                    crate::app::NormalizedRangeModel::from_micros(*start_micros, *end_micros),
                );
            }
            UiAction::FinishWaveformSelectionDrag => {
                self.model.focus_context = crate::app::FocusContextModel::Waveform;
            }
            _ => {}
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct QueuedWaveformClickBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    queued_actions: Vec<UiAction>,
    project_calls: usize,
}

impl NativeAppBridge for QueuedWaveformClickBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        for action in self.queued_actions.drain(..) {
            match &action {
                UiAction::ClearWaveformSelection => {
                    self.model.waveform.selection_milli = None;
                }
                UiAction::SeekWaveformPrecise { position_nanos }
                | UiAction::SetWaveformCursorPrecise { position_nanos } => {
                    self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                }
                UiAction::PlayWaveformAtPrecise { position_nanos } => {
                    self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                    self.model.transport_running = true;
                }
                UiAction::PlayFromCurrentPlayhead | UiAction::PlayFromWaveformCursor => {
                    self.model.transport_running = true;
                }
                _ => {}
            }
            self.actions.push(action);
        }
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::BeginWaveformSelectionAt { .. } => {
                self.model.focus_context = crate::app::FocusContextModel::Waveform;
                self.actions.push(action);
            }
            UiAction::ClearWaveformSelection | UiAction::SeekWaveformPrecise { .. } => {
                self.queued_actions.push(action);
            }
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                self.actions.push(action);
            }
            UiAction::PlayWaveformAtPrecise { position_nanos } => {
                self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                self.model.transport_running = true;
                self.actions.push(action);
            }
            UiAction::PlayFromCurrentPlayhead | UiAction::PlayFromWaveformCursor => {
                self.model.transport_running = true;
                self.actions.push(action);
            }
            _ => {
                self.actions.push(action);
            }
        }
    }
}

#[derive(Default)]
struct FolderCreateCancelBridge {
    actions: Vec<UiAction>,
    model: AppModel,
}

impl NativeAppBridge for FolderCreateCancelBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

#[test]
fn begin_waveform_selection_press_does_not_project_zero_width_selection() {
    let mut bridge = ImmediateWaveformSelectionBridge::default();

    bridge.reduce_action(UiAction::BeginWaveformSelectionAt {
        anchor_micros: 125_000,
    });

    assert_eq!(
        bridge.model.focus_context,
        crate::app::FocusContextModel::Waveform
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
            UiAction::SetWaveformSelectionRange {
                start_micros: 250_000,
                end_micros: 820_000,
                snap_override: false,
                preserve_view_edge: false,
            },
        ]
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
fn waveform_scrollbar_thumb_press_uses_rendered_thumb_while_refresh_is_pending() {
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
    let stale_thumb_offset_x = runner
        .shell_state
        .waveform_scrollbar_thumb_offset_at_point(&layout, &stale_model, thumb_point)
        .expect("stale thumb point should remain hittable");
    let stale_thumb_ratio_x = runner
        .shell_state
        .waveform_scrollbar_thumb_ratio_at_point(&layout, &stale_model, thumb_point)
        .expect("stale thumb should expose a drag ratio");

    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        thumb_point,
        false,
        &mut action_emitted,
    ));

    assert_eq!(runner.bridge.project_calls, 1);
    assert!(!action_emitted);
    assert_eq!(
        runner.waveform_scrollbar_drag,
        Some(WaveformScrollbarDragState {
            thumb_pointer_offset_x: stale_thumb_offset_x,
            thumb_pointer_ratio_x: stale_thumb_ratio_x,
        })
    );
}

#[test]
fn waveform_scrollbar_drag_rebases_stale_thumb_ratio_before_first_sample() {
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
    let stale_thumb_ratio_x = runner
        .shell_state
        .waveform_scrollbar_thumb_ratio_at_point(&layout, &stale_model, thumb_point)
        .expect("stale thumb should expose a drag ratio");
    let mut action_emitted = false;
    assert!(runner.handle_left_pointer_press_for_tests(
        &layout,
        thumb_point,
        false,
        &mut action_emitted,
    ));
    assert!(!action_emitted);

    let drag_point = Point::new(
        (thumb_point.x + 80.0).min(layout.waveform_scrollbar_lane.max.x),
        thumb_point.y,
    );
    let remapped_offset_x = runner
        .shell_state
        .waveform_scrollbar_thumb_width(&layout, &refreshed_model)
        .map(|thumb_width| thumb_width * stale_thumb_ratio_x)
        .expect("refreshed thumb should expose a layout");
    let expected_center = runner
        .shell_state
        .waveform_scrollbar_view_center_for_drag(
            &layout,
            &refreshed_model,
            drag_point.x,
            remapped_offset_x,
        )
        .expect("dragging refreshed thumb should resolve a waveform center");

    assert!(runner.process_waveform_scrollbar_drag_immediately(drag_point));
    assert_eq!(runner.bridge.project_calls, 2);
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
fn browser_row_pointer_action_preserves_shell_viewport_for_interior_refocus() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = browser_model_with_rows(40, 20);
    let resolved_view_start = runner
        .shell_state
        .browser_viewport_start_row(&layout, &model)
        .expect("focused browser viewport should resolve a visible start");
    assert_eq!(resolved_view_start, 3);

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout));
    runner.last_cursor = Some(browser_row_point(runner.shell_layout.as_ref().unwrap()));
    runner.bridge.actions.clear();

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 15 }, false)
    );
    assert!(runner.pending_browser_row_press.is_some());
    assert!(runner.bridge.actions.is_empty());

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetBrowserViewStart { visible_row: 3 },
            UiAction::FocusBrowserRow { visible_row: 15 }
        ]
    );
}

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
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros: waveform_position_micros_from_point(&layout, &runner.model, drag),
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::SaveWaveformSelectionToBrowser,
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
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros: waveform_position_micros_from_point(&layout, &runner.model, drag),
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::SaveWaveformSelectionToBrowserWithKeep2,
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
    let end_micros = waveform_position_micros_from_point(&layout, &runner.model, drag);
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
            UiAction::BeginWaveformSelectionAt { anchor_micros },
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros,
                snap_override: false,
                preserve_view_edge: false,
            },
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
        Some(&UiAction::SetWaveformSelectionRange {
            start_micros: anchor_micros,
            end_micros: waveform_position_micros_from_point(&layout, &runner.model, free_drag),
            snap_override: true,
            preserve_view_edge: false,
        })
    );

    runner.modifiers = ModifiersState::default();
    assert!(runner.process_waveform_drag_immediately(snapped_drag));
    assert_eq!(
        runner.bridge.actions.last(),
        Some(&UiAction::SetWaveformSelectionRange {
            start_micros: anchor_micros,
            end_micros: waveform_position_micros_from_point(&layout, &runner.model, snapped_drag),
            snap_override: false,
            preserve_view_edge: false,
        })
    );
}

#[test]
fn command_waveform_edge_adjust_press_emits_immediately_without_arming_drag() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.6),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    runner.model = Arc::new(model);
    runner.modifiers = ModifiersState::CONTROL;

    let mut action_emitted = false;
    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );

    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformSelectionRange {
            start_micros: 600_000,
            end_micros: 800_000,
            snap_override: false,
            preserve_view_edge: false,
        }]
    );
    assert_eq!(runner.waveform_drag_mode, None);
}

#[test]
fn shift_click_playback_selection_slide_emits_immediately_and_finishes_on_release() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.1234),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let mut model = AppModel::default();
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
    runner.model = Arc::new(model);
    runner.modifiers = ModifiersState::SHIFT;
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.last_cursor = Some(point);

    let position_micros = waveform_position_micros_from_point(&layout, &runner.model, point);
    let (expected_start, expected_end) =
        shift_waveform_range_micros(milli(200), position_micros, milli(200), milli(800));

    let mut action_emitted = false;
    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );

    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformSelectionRange {
            start_micros: expected_start,
            end_micros: expected_end,
            snap_override: false,
            preserve_view_edge: false,
        }]
    );
    assert_eq!(
        runner.last_emitted_waveform_drag_action,
        Some(UiAction::SetWaveformSelectionRange {
            start_micros: expected_start,
            end_micros: expected_end,
            snap_override: false,
            preserve_view_edge: false,
        })
    );

    runner.finish_volume_drag(Some(MouseButton::Left));

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetWaveformSelectionRange {
                start_micros: expected_start,
                end_micros: expected_end,
                snap_override: false,
                preserve_view_edge: false,
            },
            UiAction::FinishWaveformSelectionRangeDrag,
        ]
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
    bridge.model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
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
    bridge.model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 800));
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

#[test]
fn folder_create_click_outside_cancels_then_processes_target_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut bridge = FolderCreateCancelBridge::default();
    bridge
        .model
        .sources
        .rows
        .push(crate::app::SourceRowModel::new(
            "source_a",
            String::from("/tmp/source_a"),
            false,
            false,
        ));
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = Arc::new(AppModel {
        sources: SourcesPanelModel {
            rows: vec![crate::app::SourceRowModel::new(
                "source_a",
                String::from("/tmp/source_a"),
                false,
                false,
            )],
            folder_rows: vec![
                crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true),
                crate::app::FolderRowModel::create_draft(
                    1,
                    String::from("new folder"),
                    String::from("New folder name"),
                    None,
                    true,
                ),
            ],
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::FolderCreate;
    runner.text_input_buffer = Some(String::from("new folder"));
    runner.frame_state.model_dirty = false;
    let source_row = runner
        .shell_state
        .rendered_source_row_rects(&layout, &runner.model)
        .into_iter()
        .next()
        .expect("source row should render");
    let point = Point::new(
        (source_row.min.x + source_row.max.x) * 0.5,
        (source_row.min.y + source_row.max.y) * 0.5,
    );
    let mut action_emitted = false;

    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );
    assert!(action_emitted);
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::CancelFolderCreate,
            UiAction::FocusSourceRow { index: 0 },
        ]
    );
}

#[test]
fn render_sync_emits_browser_view_start_when_shell_viewport_outruns_model() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = browser_model_with_rows(40, 20);
    let resolved_view_start = runner
        .shell_state
        .browser_viewport_start_row(&layout, &model)
        .expect("focused browser viewport should resolve a visible start");
    assert_eq!(resolved_view_start, 3);

    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.bridge.actions.clear();

    runner.sync_browser_viewport_from_shell(&layout);

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserViewStart { visible_row: 3 }]
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
