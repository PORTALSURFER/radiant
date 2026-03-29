use super::*;

#[test]
fn action_scope_classification_routes_waveform_actions_by_cost() {
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
            anchor_ratio_micros: None,
        }),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::SetWaveformViewCenter {
                center_micros: 420_000
            }
        ),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::DetectWaveformSilenceSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::DetectWaveformExactDuplicateSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::CleanWaveformExactDuplicateSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::StartWaveformSelectionDrag {
                pointer_x: 320,
                pointer_y: 240,
            }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::PlayFromStart),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::PlayFromCurrentPlayhead
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
}

#[test]
fn high_refresh_present_mode_candidates_prefer_non_vsync_fallback_before_vsync() {
    assert_eq!(
        present_mode_candidates(120),
        &[
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::AutoVsync,
        ]
    );
    assert_eq!(present_mode_candidates(240), present_mode_candidates(120));
}

#[test]
fn standard_present_mode_candidates_use_vsync_only() {
    assert_eq!(present_mode_candidates(60), &[wgpu::PresentMode::AutoVsync]);
    assert_eq!(present_mode_candidates(119), present_mode_candidates(60));
}

#[test]
fn select_present_mode_prefers_mailbox_for_high_refresh_when_supported() {
    let supported_present_modes = [
        wgpu::PresentMode::Mailbox,
        wgpu::PresentMode::Immediate,
        wgpu::PresentMode::Fifo,
    ];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::Mailbox
    );
}

#[test]
fn select_present_mode_falls_back_to_immediate_when_mailbox_is_unavailable() {
    let supported_present_modes = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::Immediate
    );
}

#[test]
fn select_present_mode_uses_auto_vsync_when_only_fifo_is_available() {
    let supported_present_modes = [wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(120, &supported_present_modes),
        wgpu::PresentMode::AutoVsync
    );
}

#[test]
fn select_present_mode_keeps_standard_refresh_on_auto_vsync() {
    let supported_present_modes = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo];

    assert_eq!(
        select_present_mode(60, &supported_present_modes),
        wgpu::PresentMode::AutoVsync
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
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::FocusBrowserRow {
            visible_row: 12
        }),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetBrowserViewStart {
            visible_row: 4
        }),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ToggleRandomNavigationMode
        ),
        RuntimeInvalidationScope::StaticAndOverlays
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
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::FinishWaveformEditFadeDrag
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ToggleWaveformSliceSelection { index: 0 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::MoveWaveformSliceFocus { delta: 1 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::StartNewFolder),
        RuntimeInvalidationScope::StaticAndOverlays
    );
}

#[test]
fn repaint_event_pending_gate_coalesces_duplicate_requests() {
    let pending = AtomicBool::new(false);

    assert!(try_mark_repaint_event_pending(&pending));
    assert!(pending.load(Ordering::Acquire));
    assert!(!try_mark_repaint_event_pending(&pending));
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
fn motion_overlay_signature_changes_for_waveform_toolbar_options() {
    let baseline = NativeMotionModel::from_app_model(&AppModel::default());
    let chrome_baseline_signature = chrome_motion_overlay_model_signature(&baseline);
    let waveform_baseline_signature = waveform_motion_overlay_model_signature(&baseline);

    let mut changed_channel = baseline.clone();
    changed_channel.waveform_channel_view = match baseline.waveform_channel_view {
        crate::app::WaveformChannelViewModel::Mono => crate::app::WaveformChannelViewModel::Stereo,
        crate::app::WaveformChannelViewModel::Stereo => crate::app::WaveformChannelViewModel::Mono,
    };
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_channel)
    );

    let mut changed_normalized = baseline.clone();
    changed_normalized.waveform_normalized_audition_enabled =
        !baseline.waveform_normalized_audition_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_normalized)
    );

    let mut changed_bpm_snap = baseline.clone();
    changed_bpm_snap.waveform_bpm_snap_enabled = !baseline.waveform_bpm_snap_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_bpm_snap)
    );

    let mut changed_relative_grid = baseline.clone();
    changed_relative_grid.waveform_relative_bpm_grid_enabled =
        !baseline.waveform_relative_bpm_grid_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_relative_grid)
    );

    let mut changed_transient_snap = baseline.clone();
    changed_transient_snap.waveform_transient_snap_enabled =
        !baseline.waveform_transient_snap_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_transient_snap)
    );

    let mut changed_transient_markers = baseline.clone();
    changed_transient_markers.waveform_transient_markers_enabled =
        !baseline.waveform_transient_markers_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_transient_markers)
    );

    let mut changed_slice_mode = baseline.clone();
    changed_slice_mode.waveform_slice_mode_enabled = !baseline.waveform_slice_mode_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_slice_mode)
    );

    let mut changed_duplicate_cleanup = baseline.clone();
    changed_duplicate_cleanup.waveform_exact_duplicate_cleanup_available =
        !baseline.waveform_exact_duplicate_cleanup_available;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_duplicate_cleanup)
    );

    let mut changed_slices = baseline.clone();
    changed_slices
        .waveform_slices
        .push(crate::app::WaveformSlicePreviewModel {
            range: crate::app::NormalizedRangeModel::new(120, 240),
            selected: false,
            focused: false,
            marked_for_export: false,
        });
    assert_ne!(
        waveform_baseline_signature,
        waveform_motion_overlay_model_signature(&changed_slices)
    );

    let mut changed_loop = baseline.clone();
    changed_loop.waveform_loop_enabled = !baseline.waveform_loop_enabled;
    assert_ne!(
        chrome_baseline_signature,
        chrome_motion_overlay_model_signature(&changed_loop)
    );
}

#[test]
fn state_overlay_signature_changes_for_drag_chip_pointer_motion() {
    let baseline = AppModel::default();
    let baseline_signature = state_overlay_model_signature(&baseline);

    let mut changed = baseline.clone();
    changed.drag_overlay = crate::app::DragOverlayModel {
        active: true,
        label: String::from("kick.wav"),
        target_label: String::from("Folder: drums"),
        valid_target: true,
        pointer_x: Some(320),
        pointer_y: Some(240),
    };
    let anchored_signature = state_overlay_model_signature(&changed);
    assert_ne!(baseline_signature, anchored_signature);

    changed.drag_overlay.pointer_x = Some(321);
    assert_ne!(
        anchored_signature,
        state_overlay_model_signature(&changed),
        "pointer motion should invalidate the state overlay signature"
    );
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
fn cached_image_upload_blob_reuses_existing_entry_without_growing_cache() {
    let mut cache = HashMap::new();
    let mut cache_order = VecDeque::new();
    let pixels: Arc<[u8]> = Arc::from(vec![1u8; 16]);

    let _first = NativeVelloRunner::<PreviewBridge>::cached_image_upload_blob(
        &mut cache,
        &mut cache_order,
        &pixels,
        2,
        2,
    );
    let _second = NativeVelloRunner::<PreviewBridge>::cached_image_upload_blob(
        &mut cache,
        &mut cache_order,
        &pixels,
        2,
        2,
    );

    assert_eq!(cache.len(), 1);
    assert_eq!(cache_order.len(), 1);
}

#[test]
fn cached_image_upload_blob_evicts_oldest_entry_instead_of_clearing_all() {
    let mut cache = HashMap::new();
    let mut cache_order = VecDeque::new();
    let mut first_key = None;
    let mut newest_key = None;

    for index in 0..=IMAGE_UPLOAD_BLOB_CACHE_LIMIT {
        let pixels: Arc<[u8]> = Arc::from(vec![index as u8; 16]);
        let key = ImageUploadBlobCacheKey {
            pixels_ptr: pixels.as_ptr() as usize,
            width: 2,
            height: 2,
        };
        if index == 0 {
            first_key = Some(key);
        }
        newest_key = Some(key);
        let _ = NativeVelloRunner::<PreviewBridge>::cached_image_upload_blob(
            &mut cache,
            &mut cache_order,
            &pixels,
            2,
            2,
        );
    }

    assert_eq!(cache.len(), IMAGE_UPLOAD_BLOB_CACHE_LIMIT);
    assert!(!cache.contains_key(&first_key.expect("expected oldest cache key")));
    assert!(cache.contains_key(&newest_key.expect("expected newest cache key")));
    assert_eq!(cache_order.len(), IMAGE_UPLOAD_BLOB_CACHE_LIMIT);
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
