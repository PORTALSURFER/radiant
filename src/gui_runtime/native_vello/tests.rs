use super::*;
use crate::app::{
    BrowserPanelModel, ColumnModel, MapPanelModel, MapPointModel, SourcesPanelModel,
    UpdatePanelModel, UpdateStatusModel, WaveformPanelModel,
};
use crate::gui::types::Vector2;
use winit::event::MouseScrollDelta;

#[derive(Default)]
struct RecordingBridge {
    actions: Vec<UiAction>,
}

impl NativeAppBridge for RecordingBridge {
    fn pull_model(&mut self) -> AppModel {
        AppModel::default()
    }

    fn on_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

#[test]
fn action_scope_classification_routes_waveform_actions_to_motion_overlay() {
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SeekWaveform {
            position_milli: 420,
        }),
        RuntimeInvalidationScope::OverlayMotionOnly
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        }),
        RuntimeInvalidationScope::OverlayMotionOnly
    );
}

#[test]
fn action_scope_classification_defaults_to_static_and_overlays_for_non_waveform_actions() {
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetBrowserSearch {
            query: String::from("kick"),
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetPromptInput {
            value: String::from("rename-me"),
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::MoveBrowserFocus {
            delta: 1
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetVolume {
            value_milli: 250
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::CommitVolumeSetting),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::StartNewFolder),
        RuntimeInvalidationScope::StaticAndOverlays
    );
}

#[test]
fn model_overlay_dirty_does_not_force_static_scene_rebuild() {
    let mut state = NativeVelloFrameState::default();
    state.mark_model_overlay_dirty();
    assert!(state.take_model());
    assert!(!state.take_scene());
    assert!(state.take_state_overlay());
    assert!(state.take_motion_overlay());
}

#[test]
fn resolve_static_rebuild_skips_static_for_model_overlay_when_bridge_clean() {
    let dirty = DirtySegments::empty();
    assert!(!resolve_static_rebuild(true, false, dirty));
}

#[test]
fn resolve_static_rebuild_keeps_explicit_static_invalidation() {
    let dirty = DirtySegments::empty();
    assert!(resolve_static_rebuild(true, true, dirty));
}

#[test]
fn resolve_static_rebuild_honors_bridge_static_dirty_segments() {
    let dirty = DirtySegments::from_bits(DirtySegments::STATUS_BAR);
    assert!(resolve_static_rebuild(true, false, dirty));
}

#[test]
fn static_rebuild_from_dirty_mask_requires_model_refresh_without_explicit_request() {
    let dirty = DirtySegments::from_bits(DirtySegments::STATUS_BAR);
    assert!(static_rebuild_from_dirty_mask(true, false, dirty));
    assert!(!static_rebuild_from_dirty_mask(false, false, dirty));
    assert!(!static_rebuild_from_dirty_mask(true, true, dirty));
}

fn test_segment_fingerprints(
    revision_seed: u64,
) -> [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT] {
    std::array::from_fn(|idx| {
        let segment = NativeVelloRunner::<PreviewBridge>::static_segment_from_cache_index(idx);
        StaticSegmentCacheFingerprint {
            segment,
            layout_width_bits: 1920.0f32.to_bits(),
            layout_height_bits: 1080.0f32.to_bits(),
            layout_scale_bits: 1.0f32.to_bits(),
            style_signature: 7,
            segment_revision: revision_seed + idx as u64,
        }
    })
}

#[test]
fn static_segment_graph_diff_rebuilds_all_on_cold_start() {
    let graph = StaticSegmentStateGraph::default();
    let plan = graph.diff(DirtySegments::empty(), false, test_segment_fingerprints(10));
    for segment in StaticFrameSegment::ALL {
        assert!(plan.should_rebuild(segment));
    }
}

#[test]
fn static_segment_graph_diff_skips_when_fingerprints_match_and_clean() {
    let mut graph = StaticSegmentStateGraph::default();
    let fingerprints = test_segment_fingerprints(20);
    let first_plan = graph.diff(DirtySegments::empty(), false, fingerprints.clone());
    for segment in StaticFrameSegment::ALL {
        assert!(first_plan.should_rebuild(segment));
        graph.commit_segment(segment, first_plan.fingerprint(segment));
    }

    let second_plan = graph.diff(DirtySegments::empty(), false, fingerprints);
    for segment in StaticFrameSegment::ALL {
        assert!(!second_plan.should_rebuild(segment));
    }
}

#[test]
fn static_segment_graph_diff_targets_dirty_and_changed_segments() {
    let mut graph = StaticSegmentStateGraph::default();
    let fingerprints = test_segment_fingerprints(30);
    let first_plan = graph.diff(DirtySegments::empty(), false, fingerprints.clone());
    for segment in StaticFrameSegment::ALL {
        graph.commit_segment(segment, first_plan.fingerprint(segment));
    }

    let dirty_plan = graph.diff(
        DirtySegments::from_bits(DirtySegments::STATUS_BAR),
        false,
        fingerprints.clone(),
    );
    assert!(dirty_plan.should_rebuild(StaticFrameSegment::StatusBar));
    for segment in StaticFrameSegment::ALL {
        if segment != StaticFrameSegment::StatusBar {
            assert!(!dirty_plan.should_rebuild(segment));
        }
    }

    let mut changed_fingerprints = fingerprints;
    changed_fingerprints[StaticFrameSegment::MapPanel.index()].segment_revision += 1;
    let changed_plan = graph.diff(DirtySegments::empty(), false, changed_fingerprints);
    assert!(changed_plan.should_rebuild(StaticFrameSegment::MapPanel));
    for segment in StaticFrameSegment::ALL {
        if segment != StaticFrameSegment::MapPanel {
            assert!(!changed_plan.should_rebuild(segment));
        }
    }
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
    assert!(runner.process_wheel_rows_immediately(3));

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::MoveBrowserFocus { delta: 3 }]
    );
}

#[test]
fn startup_fast_path_defers_model_and_overlay_pulls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.frame_state.mark_layout_dirty();
    runner.frame_state.mark_model_dirty();

    runner.prepare_startup_first_frame_scene();

    assert!(!runner.frame_state.take_model());
    assert!(runner.frame_state.take_scene());
    assert!(!runner.frame_state.take_state_overlay());
    assert!(!runner.frame_state.take_motion_overlay());
}

#[test]
fn startup_placeholder_scene_uses_theme_clear_color_and_branding() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);

    runner.build_startup_placeholder_scene(&layout, &style);

    assert_eq!(runner.clear_color, style.clear_color);
    assert_eq!(runner.frame_cache.clear_color, style.clear_color);
    assert!(
        runner
            .frame_cache
            .text_runs
            .iter()
            .any(|run| run.text == "Sempal")
    );
}

#[test]
fn startup_fast_path_rebuild_uses_placeholder_scene_before_first_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);
    runner.shell_layout = Some(layout);
    runner.style_cache = Some(style);
    runner.frame_state.scene_dirty = true;
    runner.frame_state.model_dirty = false;
    runner.frame_state.state_overlay_dirty = false;
    runner.frame_state.motion_overlay_dirty = false;
    runner.startup_model_pull_pending = true;
    runner.first_frame_presented = false;

    runner.rebuild_scene_if_needed();

    assert!(
        runner
            .frame_cache
            .text_runs
            .iter()
            .any(|run| run.text.contains("Starting audio engine"))
    );
}

#[test]
fn complete_first_present_schedules_deferred_model_pull() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    assert!(runner.startup_model_pull_pending);
    assert!(!runner.startup_deferred_model_refresh_pending);
    assert!(!runner.first_frame_presented);
    assert!(!runner.startup_window_visible);

    runner.complete_first_present();

    assert!(runner.first_frame_presented);
    assert!(!runner.startup_model_pull_pending);
    assert!(runner.startup_deferred_model_refresh_pending);
    assert!(!runner.startup_window_visible);
    assert!(runner.frame_state.take_model());
    assert!(!runner.frame_state.take_scene());
    assert!(runner.frame_state.take_state_overlay());
    assert!(runner.frame_state.take_motion_overlay());
}

#[test]
fn startup_window_reveals_after_deferred_model_refresh_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.complete_first_present();
    assert!(!runner.startup_window_visible);

    runner.startup_deferred_model_refresh_pending = false;
    runner.complete_first_present();

    assert!(runner.startup_window_visible);
}

#[test]
fn startup_window_reveals_on_first_present_without_deferred_pull() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;

    runner.complete_first_present();

    assert!(runner.startup_window_visible);
    assert!(runner.first_frame_presented);
    assert!(!runner.startup_deferred_model_refresh_pending);
    assert_eq!(runner.startup_reveal_deadline, None);
}

#[test]
fn startup_window_force_reveal_fallback_unblocks_hidden_stalls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.complete_first_present();

    assert!(runner.startup_deferred_model_refresh_pending);
    assert!(!runner.startup_window_visible);
    runner.startup_reveal_deadline = Some(Instant::now() - Duration::from_millis(1));

    runner.maybe_force_reveal_startup_window_on_stall(Instant::now());

    assert!(runner.startup_window_visible);
    assert!(runner.startup_deferred_model_refresh_pending);
    assert_eq!(runner.startup_reveal_deadline, None);
}

#[test]
fn startup_window_force_reveal_fallback_unblocks_pre_first_present_stalls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_window_visible = false;
    runner.startup_reveal_deadline = Some(Instant::now() - Duration::from_millis(1));

    runner.maybe_force_reveal_startup_window_on_stall(Instant::now());

    assert!(runner.startup_window_visible);
    assert_eq!(runner.startup_reveal_deadline, None);
}

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
fn finish_volume_drag_flushes_pending_value_before_commit() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.queue_volume_milli(915);
    runner.volume_drag_active = true;
    runner.waveform_drag_mode = Some(WaveformPointerDragMode::Seek);
    runner.last_emitted_waveform_drag_action = Some(UiAction::SeekWaveform {
        position_milli: 915,
    });
    runner.map_focus_drag_active = true;
    runner.last_emitted_map_drag_sample_id = Some(String::from("source::kick.wav"));

    runner.finish_volume_drag();

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
fn key_bindings_emit_waveform_zoom_actions() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::OpenBracket, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
        })
    );
    assert_eq!(
        action_from_key(KeyCode::CloseBracket, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        })
    );
    assert_eq!(
        action_from_key(KeyCode::M, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveformToSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &model),
        Some(UiAction::ClearWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::Slash, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveformFull)
    );
}

#[test]
fn key_bindings_emit_browser_actions() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &model),
        Some(UiAction::DeleteBrowserSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::I, ModifiersState::default(), &model),
        Some(UiAction::StartBrowserRename)
    );
    assert_eq!(
        action_from_key(KeyCode::N, ModifiersState::default(), &model),
        Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Neutral
        })
    );
    assert_eq!(
        action_from_key(KeyCode::X, ModifiersState::default(), &model),
        Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash
        })
    );
}

#[test]
fn key_bindings_emit_folder_actions() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::B, ModifiersState::default(), &model),
        Some(UiAction::StartNewFolder)
    );
    assert_eq!(
        action_from_key(KeyCode::G, ModifiersState::default(), &model),
        Some(UiAction::DeleteFocusedFolder)
    );
    assert_eq!(
        action_from_key(KeyCode::Quote, ModifiersState::default(), &model),
        Some(UiAction::FocusFolderSearch)
    );
    assert_eq!(
        action_from_key(KeyCode::Z, ModifiersState::default(), &model),
        Some(UiAction::StartFolderRename)
    );
}

#[test]
fn prompt_visible_routes_enter_and_cancel_keys() {
    let mut model = AppModel::default();
    model.confirm_prompt.visible = true;
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        Some(UiAction::ConfirmPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &model),
        Some(UiAction::CancelPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::W, ModifiersState::default(), &model),
        None
    );

    model.confirm_prompt.input_error = Some(String::from("Folder already exists"));
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        None
    );
}

#[test]
fn key_bindings_handle_selection_modifiers() {
    let model = AppModel::default();

    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &model),
        Some(UiAction::MoveBrowserFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::SHIFT, &model),
        Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(
            KeyCode::ArrowUp,
            ModifiersState::SHIFT | ModifiersState::CONTROL,
            &model
        ),
        Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(
            KeyCode::ArrowDown,
            ModifiersState::SHIFT | ModifiersState::SUPER,
            &model
        ),
        Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
    );
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        Some(UiAction::CommitFocusedBrowserRow)
    );
}

#[test]
fn browser_row_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        browser: crate::app::BrowserPanelModel {
            rows: vec![crate::app::BrowserRowModel::new(
                17, "kick-row", 0, false, false,
            )],
            visible_count: 1,
            ..crate::app::BrowserPanelModel::default()
        },
        ..AppModel::default()
    };
    let row_center_y = layout.browser_rows.min.y
        + (StyleTokens::for_viewport_width(layout.root.rect.width())
            .sizing
            .browser_row_height
            * 0.5);
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        row_center_y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT,
        ),
        Some(UiAction::ExtendBrowserSelectionToRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SUPER,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT | ModifiersState::SUPER,
        ),
        Some(UiAction::AddRangeBrowserSelection { visible_row: 17 })
    );
}

#[test]
fn confirm_prompt_keys_ignore_other_shortcuts_when_visible() {
    let mut model = AppModel::default();
    model.confirm_prompt.visible = true;

    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::SHIFT, &model),
        Some(UiAction::ConfirmPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::SUPER, &model),
        Some(UiAction::CancelPrompt)
    );
}

#[test]
fn key_bindings_respect_progress_cancelability() {
    let mut model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::P, ModifiersState::default(), &model),
        None
    );

    model.progress_overlay.cancelable = true;
    assert_eq!(
        action_from_key(KeyCode::P, ModifiersState::default(), &model),
        Some(UiAction::CancelProgress)
    );
}

#[test]
fn waveform_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
        layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
    );
    let model = AppModel {
        columns: [
            ColumnModel::new("Trash", 0),
            ColumnModel::new("Neutral", 0),
            ColumnModel::new("Keep", 0),
        ],
        sources: SourcesPanelModel::default(),
        browser: BrowserPanelModel::default(),
        waveform: WaveformPanelModel {
            selection_milli: Some(crate::app::NormalizedRangeModel::new(120, 360)),
            cursor_milli: Some(220),
            playhead_milli: Some(260),
            ..WaveformPanelModel::default()
        },
        ..AppModel::default()
    };

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::SeekWaveform {
            position_milli: 500
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::SetWaveformCursor {
            position_milli: 500
        })
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT,
        ),
        Some(UiAction::SetWaveformSelectionRange {
            start_milli: 120,
            end_milli: 500,
        })
    );
}

#[test]
fn waveform_anchor_prefers_selection_then_cursor_then_playhead() {
    let mut model = AppModel::default();
    assert_eq!(waveform_anchor_milli(&model), 0);

    model.waveform.playhead_milli = Some(333);
    assert_eq!(waveform_anchor_milli(&model), 333);

    model.waveform.cursor_milli = Some(222);
    assert_eq!(waveform_anchor_milli(&model), 222);

    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(111, 444));
    assert_eq!(waveform_anchor_milli(&model), 111);
}

#[test]
/// Waveform drag-mode mapping should preserve the initial action intent.
fn waveform_drag_mode_maps_from_waveform_actions() {
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SeekWaveform {
            position_milli: 250
        }),
        Some(WaveformPointerDragMode::Seek)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformCursor {
            position_milli: 250
        }),
        Some(WaveformPointerDragMode::Cursor)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformSelectionRange {
            start_milli: 125,
            end_milli: 250,
        }),
        Some(WaveformPointerDragMode::Selection { anchor_milli: 125 })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::ToggleTransport),
        None
    );
}

#[test]
/// Drag waveform actions should clamp pointer positions and preserve anchors.
fn waveform_drag_action_clamps_and_preserves_selection_anchor() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let y = (layout.waveform_plot.min.y + layout.waveform_plot.max.y) * 0.5;
    let left = Point::new(layout.waveform_plot.min.x - 200.0, y);
    let right = Point::new(layout.waveform_plot.max.x + 200.0, y);
    assert_eq!(
        waveform_drag_action_for_mode(&layout, left, WaveformPointerDragMode::Seek),
        UiAction::SeekWaveform { position_milli: 0 }
    );
    assert_eq!(
        waveform_drag_action_for_mode(&layout, right, WaveformPointerDragMode::Cursor),
        UiAction::SetWaveformCursor {
            position_milli: 1000
        }
    );
    assert_eq!(
        waveform_drag_action_for_mode(
            &layout,
            right,
            WaveformPointerDragMode::Selection { anchor_milli: 200 }
        ),
        UiAction::SetWaveformSelectionRange {
            start_milli: 200,
            end_milli: 1000,
        }
    );
}

#[test]
fn browser_tab_clicks_route_to_tab_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let map_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.75),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            map_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetBrowserTab { map: true })
    );

    let list_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.25),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            list_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetBrowserTab { map: false })
    );
}

#[test]
fn map_point_click_routes_to_focus_map_sample() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let point = Point::new(
        layout.browser_rows.min.x + (layout.browser_rows.width() * 0.5),
        layout.browser_rows.min.y + (layout.browser_rows.height() * 0.5),
    );
    let model = AppModel {
        map: MapPanelModel {
            active: true,
            summary: String::from("1 point"),
            legend_label: String::from("Render: points"),
            selection_label: String::from("Selection: source::kick.wav"),
            hover_label: String::from("Hover: source::kick.wav"),
            cluster_label: String::from("Clusters: 1"),
            viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
            error: None,
            render_mode: crate::app::MapRenderModeModel::Points,
            points: vec![MapPointModel {
                sample_id: String::from("source::kick.wav"),
                x_milli: 500,
                y_milli: 500,
                cluster_id: Some(1),
                selected: true,
                focused: true,
            }],
        },
        ..AppModel::default()
    };
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusMapSample {
            sample_id: String::from("source::kick.wav")
        })
    );
}

#[test]
fn update_button_click_routes_update_check_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        update: UpdatePanelModel {
            status: UpdateStatusModel::Idle,
            status_label: String::from("Updates: idle"),
            action_hint_label: String::from("Action: check"),
            release_notes_label: String::new(),
            available_tag: None,
            available_url: None,
            last_error: None,
        },
        ..AppModel::default()
    };
    let button_point = Point::new(
        layout.top_bar_action_cluster.max.x - 18.0,
        layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            button_point,
            ModifiersState::default(),
        ),
        Some(UiAction::CheckForUpdates)
    );
}

#[test]
fn top_bar_volume_meter_click_routes_set_volume_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let mut first_hit_x = None;
    let mut last_hit_x = None;
    let y = layout.top_bar_controls_row.min.y + (layout.top_bar_controls_row.height() * 0.5);
    let mut x = layout.top_bar.min.x;
    while x <= layout.top_bar.max.x {
        let point = Point::new(x, y);
        if shell_state
            .top_bar_volume_action_at_point(&layout, point)
            .is_some()
        {
            if first_hit_x.is_none() {
                first_hit_x = Some(x);
            }
            last_hit_x = Some(x);
        }
        x += 2.0;
    }
    let meter_min_x = first_hit_x.expect("volume meter point should be discoverable");
    let meter_max_x = last_hit_x.expect("volume meter span should be discoverable");
    let meter_point = Point::new((meter_min_x + meter_max_x) * 0.5, y);
    match action_from_pointer(
        &layout,
        &model,
        &mut shell_state,
        meter_point,
        ModifiersState::default(),
    ) {
        Some(UiAction::SetVolume { value_milli }) => {
            assert!(value_milli >= 350);
            assert!(value_milli <= 650);
        }
        other => panic!("expected SetVolume action, got {other:?}"),
    }
}

#[test]
fn browser_wheel_delta_is_bounded_and_directional() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut model = AppModel::default();
    model.map.active = false;
    let point = Point::new(
        layout.browser_rows.min.x + 10.0,
        layout.browser_rows.min.y + 10.0,
    );

    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 3.0),
        ),
        Some(-3)
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 0.0)
        ),
        None
    );
    let header_point = Point::new(
        layout.browser_table_header.min.x + 5.0,
        layout.browser_table_header.min.y + 5.0,
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            header_point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 2.0),
        ),
        Some(-2)
    );
}
