use super::super::*;
use crate::gui::native_shell::{
    BROWSER_BANDS_ROOT_ID, ShellLayoutTreeKind, dirty_segments_for_layout_subtree,
};
use crate::gui::native_shell::{FocusOverlayFingerprint, HoverOverlayFingerprint};

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
fn motion_overlay_signature_changes_for_waveform_toolbar_options() {
    let baseline = NativeMotionModel::from_app_model(&AppModel::default());
    let chrome_baseline_signature = chrome_motion_overlay_model_signature(&baseline);
    let waveform_baseline_signature = waveform_motion_overlay_model_signature(&baseline);

    let mut changed_channel = baseline.clone();
    changed_channel.waveform_channel_view = match baseline.waveform_channel_view {
        crate::compat_app_contract::WaveformChannelViewModel::Mono => {
            crate::compat_app_contract::WaveformChannelViewModel::Stereo
        }
        crate::compat_app_contract::WaveformChannelViewModel::Stereo => {
            crate::compat_app_contract::WaveformChannelViewModel::Mono
        }
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
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(120, 240),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: false,
            duplicate_cleanup_exempted: false,
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
fn modal_overlay_signature_changes_for_drag_chip_pointer_motion() {
    let baseline = AppModel::default();
    let baseline_signature = modal_overlay_model_signature(&baseline);

    let mut changed = baseline.clone();
    changed.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("kick.wav"),
        target_label: String::from("Folder: drums"),
        valid_target: true,
        pointer_x: Some(320),
        pointer_y: Some(240),
    };
    let anchored_signature = modal_overlay_model_signature(&changed);
    assert_ne!(baseline_signature, anchored_signature);

    changed.drag_overlay.pointer_x = Some(321);
    assert_ne!(
        anchored_signature,
        modal_overlay_model_signature(&changed),
        "pointer motion should invalidate the modal overlay signature"
    );
}

#[test]
fn hover_overlay_signature_ignores_drag_chip_pointer_motion() {
    let mut baseline = AppModel::default();
    baseline.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("kick.wav"),
        target_label: String::from("Folder: drums"),
        valid_target: true,
        pointer_x: Some(320),
        pointer_y: Some(240),
    };
    let shell = HoverOverlayFingerprint {
        hovered: Some(ShellNodeKind::Sidebar),
        hovered_browser_visible_row: None,
        hovered_folder_pane: Some(crate::compat_app_contract::FolderPaneIdModel::Upper),
        hovered_folder_row_index: Some(0),
        hovered_waveform_toolbar_hint: None,
        browser_search_editor_signature: 0,
        folder_create_editor_signature: 0,
    };
    let baseline_signature = hover_overlay_model_signature(&baseline, &shell);

    let mut changed = baseline;
    changed.drag_overlay.pointer_x = Some(321);

    assert_eq!(
        baseline_signature,
        hover_overlay_model_signature(&changed, &shell),
        "drag-chip pointer motion should not invalidate hover overlays"
    );
}

#[test]
fn focus_overlay_signature_ignores_drag_chip_pointer_motion() {
    let mut baseline = AppModel::default();
    baseline.drag_overlay = crate::compat_app_contract::DragOverlayModel {
        active: true,
        label: String::from("kick.wav"),
        target_label: String::from("Folder: drums"),
        valid_target: true,
        pointer_x: Some(320),
        pointer_y: Some(240),
    };
    let shell = FocusOverlayFingerprint {
        has_focus_emphasis: true,
    };
    let baseline_signature = focus_overlay_model_signature(&baseline, &shell);

    let mut changed = baseline;
    changed.drag_overlay.pointer_y = Some(241);

    assert_eq!(
        baseline_signature,
        focus_overlay_model_signature(&changed, &shell),
        "drag-chip pointer motion should not invalidate focus overlays"
    );
}

#[test]
fn hover_overlay_signature_ignores_waveform_text_without_hover_tooltip() {
    let baseline = AppModel::default();
    let shell = HoverOverlayFingerprint {
        hovered: None,
        hovered_browser_visible_row: None,
        hovered_folder_pane: None,
        hovered_folder_row_index: None,
        hovered_waveform_toolbar_hint: None,
        browser_search_editor_signature: 0,
        folder_create_editor_signature: 0,
    };
    let baseline_signature = hover_overlay_model_signature(&baseline, &shell);

    let mut changed = baseline.clone();
    changed.transport_running = !baseline.transport_running;
    changed.waveform.tempo_label = Some(String::from("128 BPM"));
    changed.waveform_chrome.compare_anchor_label = Some(String::from("A1"));
    changed.waveform.loop_enabled = !baseline.waveform.loop_enabled;

    assert_eq!(
        baseline_signature,
        hover_overlay_model_signature(&changed, &shell),
        "unhovered waveform chrome should not invalidate hover overlays"
    );
}

#[test]
fn focus_overlay_signature_ignores_selected_only_browser_text_changes() {
    let mut baseline = AppModel::default();
    baseline
        .browser
        .rows
        .push(crate::compat_app_contract::BrowserRowModel::new(
            0,
            String::from("kick"),
            1,
            true,
            false,
        ));
    baseline.browser.rows.make_mut()[0].bucket_label = Some(String::from("drums").into());
    baseline.browser.rows.make_mut()[0].rating_level = 3;
    baseline.browser.rows.make_mut()[0].missing = true;
    let shell = FocusOverlayFingerprint {
        has_focus_emphasis: true,
    };
    let baseline_signature = focus_overlay_model_signature(&baseline, &shell);

    let mut changed = baseline.clone();
    changed.browser.rows.make_mut()[0].label = String::from("snare").into();
    changed.browser.rows.make_mut()[0].bucket_label = Some(String::from("perc").into());
    changed.browser.rows.make_mut()[0].rating_level = -2;
    changed.browser.rows.make_mut()[0].missing = false;

    assert_eq!(
        baseline_signature,
        focus_overlay_model_signature(&changed, &shell),
        "selected-only browser text metadata should not invalidate focus overlays"
    );
}

#[test]
fn focus_overlay_signature_changes_for_selected_only_browser_marker_state() {
    let mut baseline = AppModel::default();
    baseline
        .browser
        .rows
        .push(crate::compat_app_contract::BrowserRowModel::new(
            0,
            String::from("kick"),
            1,
            true,
            false,
        ));
    let shell = FocusOverlayFingerprint {
        has_focus_emphasis: true,
    };
    let baseline_signature = focus_overlay_model_signature(&baseline, &shell);

    let mut changed = baseline.clone();
    changed.browser.rows.make_mut()[0].locked = true;

    assert_ne!(
        baseline_signature,
        focus_overlay_model_signature(&changed, &shell),
        "selected-only browser marker state should still invalidate focus overlays"
    );
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

#[test]
fn static_segment_graph_diff_targets_browser_band_layout_dirty_segments() {
    let mut graph = StaticSegmentStateGraph::default();
    let fingerprints = test_segment_fingerprints(40);
    let first_plan = graph.diff(DirtySegments::empty(), false, fingerprints.clone());
    for segment in StaticFrameSegment::ALL {
        graph.commit_segment(segment, first_plan.fingerprint(segment));
    }

    let dirty =
        dirty_segments_for_layout_subtree(ShellLayoutTreeKind::BrowserBands, BROWSER_BANDS_ROOT_ID);
    let dirty_plan = graph.diff(dirty, false, fingerprints);

    assert!(dirty_plan.should_rebuild(StaticFrameSegment::BrowserFrame));
    assert!(dirty_plan.should_rebuild(StaticFrameSegment::BrowserRowsWindow));
    assert!(dirty_plan.should_rebuild(StaticFrameSegment::MapPanel));
    assert!(!dirty_plan.should_rebuild(StaticFrameSegment::GlobalStatic));
    assert!(!dirty_plan.should_rebuild(StaticFrameSegment::WaveformOverlay));
    assert!(!dirty_plan.should_rebuild(StaticFrameSegment::StatusBar));
}
