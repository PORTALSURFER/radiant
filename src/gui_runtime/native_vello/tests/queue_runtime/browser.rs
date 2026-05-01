use super::*;

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
                hovered_folder_pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                hovered_folder_row: Some(7),
                over_folder_panel: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
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
                hovered_folder_pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                hovered_folder_row: Some(7),
                over_folder_panel: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                shift_down: false,
                alt_down: false,
            },
            UiAction::FinishBrowserSampleDrag,
        ]
    );
}

#[cfg(target_os = "windows")]
#[test]
fn cursor_left_polls_external_drag_for_active_browser_drag_session() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut runner = NativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingBridge {
            external_drag_consume_next: true,
            ..RecordingBridge::default()
        },
    );
    runner.model = Arc::new(browser_drag_model());
    runner.shell_layout = Some(Arc::new(layout.clone()));
    let press_point = browser_row_point(&layout);
    let drag_point = folder_row_point(&mut runner.shell_state, &layout, &runner.model, 1);
    runner.last_cursor = Some(press_point);

    assert!(
        runner.handle_pointer_press_action(UiAction::FocusBrowserRow { visible_row: 0 }, false)
    );
    runner.handle_cursor_moved_for_tests(drag_point);
    assert!(runner.browser_sample_drag.is_some());

    runner.handle_cursor_left();

    assert_eq!(runner.bridge.external_drag_requests, vec![(false, true)]);
    assert!(runner.browser_sample_drag.is_none());
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
            over_folder_panel: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
            shift_down: false,
            alt_down: false,
        }
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
fn folder_create_click_outside_cancels_then_processes_target_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut bridge = FolderCreateCancelBridge::default();
    bridge
        .model
        .sources
        .rows
        .push(crate::compat_app_contract::SourceRowModel::new(
            "source_a",
            String::from("/tmp/source_a"),
            false,
            false,
        ));
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = Arc::new(AppModel {
        sources: SourcesPanelModel {
            rows: vec![crate::compat_app_contract::SourceRowModel::new(
                "source_a",
                String::from("/tmp/source_a"),
                false,
                false,
            )]
            .into(),
            folder_rows: vec![
                crate::compat_app_contract::FolderRowModel::new(
                    "Root", "", 0, false, false, true, true, true,
                ),
                crate::compat_app_contract::FolderRowModel::create_draft(
                    1,
                    String::from("new folder"),
                    String::from("New folder name"),
                    None,
                    true,
                ),
            ]
            .into(),
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
            UiAction::FocusSourceRow {
                pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
                index: 0,
            },
        ]
    );
}

#[test]
fn tag_sidebar_pill_click_with_active_input_blurs_and_toggles_once() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.browser.tag_sidebar.open = true;
    model.browser.tag_sidebar.input_value = String::from("rfx");
    model.browser.tag_sidebar.normal_tag_pills.push(
        crate::compat_app_contract::BrowserTagPillModel {
            id: String::from("rare_fx"),
            label: String::from("Rare FX"),
            state: crate::compat_app_contract::BrowserTagState::Off,
        },
    );
    runner.model = Arc::new(model);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.frame_state.model_dirty = false;
    runner.text_input_target = TextInputTarget::BrowserTagSidebar;
    runner.text_input_buffer = Some(String::from("rfx"));
    let point = (layout.browser_rows.min.x as i32..=layout.browser_rows.max.x as i32)
        .find_map(|x| {
            (layout.browser_rows.min.y as i32..=layout.browser_rows.max.y as i32).find_map(|y| {
                let point = Point::new(x as f32, y as f32);
                matches!(
                    runner.shell_state.browser_action_at_point(
                        &layout,
                        &runner.model,
                        point,
                        false,
                    ),
                    Some(UiAction::ToggleBrowserSidebarNormalTag { ref label })
                        if label == "Rare FX"
                )
                .then_some(point)
            })
        })
        .expect("normal tag pill should be hittable");
    let mut action_emitted = false;

    assert!(
        runner.handle_left_pointer_press_for_tests(&layout, point, false, &mut action_emitted,)
    );

    assert!(action_emitted);
    assert_eq!(runner.text_input_target, TextInputTarget::None);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::ToggleBrowserSidebarNormalTag {
            label: String::from("Rare FX")
        }]
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
