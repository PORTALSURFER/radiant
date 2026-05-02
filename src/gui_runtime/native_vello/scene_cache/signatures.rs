//! Stable signature builders used to decide whether retained scenes can be reused.

use super::*;
use crate::gui::native_shell::{
    FocusOverlayFingerprint, HoverOverlayFingerprint, WaveformToolbarHoverHint,
};

const FINGERPRINT_FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FINGERPRINT_FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fingerprint_mix_u8(state: &mut u64, value: u8) {
    *state ^= u64::from(value);
    *state = state.wrapping_mul(FINGERPRINT_FNV_PRIME);
}

fn fingerprint_mix_u16(state: &mut u64, value: u16) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

fn fingerprint_mix_u32(state: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

fn fingerprint_mix_u64(state: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

fn fingerprint_mix_usize(state: &mut u64, value: usize) {
    fingerprint_mix_u64(state, value as u64);
}

fn fingerprint_mix_bool(state: &mut u64, value: bool) {
    fingerprint_mix_u8(state, u8::from(value));
}

fn fingerprint_mix_f32(state: &mut u64, value: f32) {
    fingerprint_mix_u32(state, value.to_bits());
}

fn fingerprint_mix_string(state: &mut u64, value: &str) {
    fingerprint_mix_usize(state, value.len());
    for byte in value.as_bytes() {
        fingerprint_mix_u8(state, *byte);
    }
}

fn fingerprint_mix_option_string(state: &mut u64, value: Option<&str>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_string(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_usize(state: &mut u64, value: Option<usize>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_usize(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_u16(state: &mut u64, value: Option<u16>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_u16(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_u32(state: &mut u64, value: Option<u32>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_u32(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_i8(state: &mut u64, value: Option<i8>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_u8(state, value as u8);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_range(
    state: &mut u64,
    range: &crate::compat_app_contract::NormalizedRangeModel,
) {
    fingerprint_mix_u16(state, range.start_milli);
    fingerprint_mix_u16(state, range.end_milli);
    fingerprint_mix_u32(state, range.start_micros);
    fingerprint_mix_u32(state, range.end_micros);
}

pub(in super::super) fn state_overlay_model_signature(model: &AppModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_usize(&mut state, model.selected_column);
    fingerprint_mix_option_usize(&mut state, model.browser.selected_visible_row);
    fingerprint_mix_option_usize(&mut state, model.browser.anchor_visible_row);
    fingerprint_mix_option_usize(&mut state, model.sources.selected_row);
    fingerprint_mix_option_usize(&mut state, model.sources.focused_tree_row);
    fingerprint_mix_bool(&mut state, model.confirm_prompt.visible);
    fingerprint_mix_u8(
        &mut state,
        match model.confirm_prompt.kind {
            None => 0,
            Some(crate::compat_app_contract::ConfirmPromptKind::DestructiveOperation) => 1,
            Some(crate::compat_app_contract::ConfirmPromptKind::RenameContent) => 2,
            Some(crate::compat_app_contract::ConfirmPromptKind::RenameNavigationItem) => 3,
            Some(crate::compat_app_contract::ConfirmPromptKind::CreateNavigationItem) => 4,
            Some(crate::compat_app_contract::ConfirmPromptKind::RestoreRetainedItems) => 5,
            Some(crate::compat_app_contract::ConfirmPromptKind::PurgeRetainedItems) => 6,
            Some(crate::compat_app_contract::ConfirmPromptKind::EditConfiguration) => 7,
        },
    );
    fingerprint_mix_string(&mut state, &model.confirm_prompt.title);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.message);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.confirm_label);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.cancel_label);
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.target_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.input_value.as_deref());
    fingerprint_mix_option_string(
        &mut state,
        model.confirm_prompt.input_placeholder.as_deref(),
    );
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.input_error.as_deref());
    fingerprint_mix_bool(&mut state, model.progress_overlay.visible);
    fingerprint_mix_bool(&mut state, model.progress_overlay.modal);
    fingerprint_mix_string(&mut state, &model.progress_overlay.title);
    fingerprint_mix_option_string(&mut state, model.progress_overlay.detail.as_deref());
    fingerprint_mix_usize(&mut state, model.progress_overlay.completed);
    fingerprint_mix_usize(&mut state, model.progress_overlay.total);
    fingerprint_mix_bool(&mut state, model.progress_overlay.cancelable);
    fingerprint_mix_bool(&mut state, model.progress_overlay.cancel_requested);
    fingerprint_mix_bool(&mut state, model.drag_overlay.active);
    fingerprint_mix_string(&mut state, &model.drag_overlay.label);
    fingerprint_mix_string(&mut state, &model.drag_overlay.target_label);
    fingerprint_mix_bool(&mut state, model.drag_overlay.valid_target);
    fingerprint_mix_option_u16(&mut state, model.drag_overlay.pointer_x);
    fingerprint_mix_option_u16(&mut state, model.drag_overlay.pointer_y);
    fingerprint_mix_u8(
        &mut state,
        match model.update.status {
            crate::compat_app_contract::UpdateStatusModel::Idle => 0,
            crate::compat_app_contract::UpdateStatusModel::Checking => 1,
            crate::compat_app_contract::UpdateStatusModel::Available => 2,
            crate::compat_app_contract::UpdateStatusModel::Error => 3,
        },
    );
    fingerprint_mix_bool(&mut state, model.map.active);
    state
}

pub(in super::super) fn hover_overlay_model_signature(
    model: &AppModel,
    shell: &HoverOverlayFingerprint,
) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    if let Some(hint) = shell.hovered_waveform_toolbar_hint {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u8(&mut state, hint as u8);
        match hint {
            WaveformToolbarHoverHint::ChannelView => fingerprint_mix_u8(
                &mut state,
                match model.waveform_chrome.channel_view {
                    crate::compat_app_contract::WaveformChannelViewModel::Mono => 0,
                    crate::compat_app_contract::WaveformChannelViewModel::Stereo => 1,
                },
            ),
            WaveformToolbarHoverHint::NormalizedAudition => fingerprint_mix_bool(
                &mut state,
                model.waveform_chrome.normalized_audition_enabled,
            ),
            WaveformToolbarHoverHint::BpmValue => {
                fingerprint_mix_option_string(&mut state, model.waveform.tempo_label.as_deref())
            }
            WaveformToolbarHoverHint::BpmSnap => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.bpm_snap_enabled)
            }
            WaveformToolbarHoverHint::RelativeBpmGrid => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.relative_bpm_grid_enabled)
            }
            WaveformToolbarHoverHint::TransientSnap => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.transient_snap_enabled)
            }
            WaveformToolbarHoverHint::ShowTransients => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.transient_markers_enabled)
            }
            WaveformToolbarHoverHint::SliceMode => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.slice_mode_enabled)
            }
            WaveformToolbarHoverHint::Loop => {
                fingerprint_mix_bool(&mut state, model.waveform_chrome.loop_lock_enabled);
                fingerprint_mix_bool(&mut state, model.waveform.loop_enabled);
            }
            WaveformToolbarHoverHint::Compare => fingerprint_mix_option_string(
                &mut state,
                model.waveform_chrome.compare_anchor_label.as_deref(),
            ),
            WaveformToolbarHoverHint::Play => {
                fingerprint_mix_bool(&mut state, model.transport_running)
            }
            WaveformToolbarHoverHint::SilenceSplit
            | WaveformToolbarHoverHint::ExactDedupe
            | WaveformToolbarHoverHint::CleanDuplicates
            | WaveformToolbarHoverHint::Stop
            | WaveformToolbarHoverHint::Record => {}
        }
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    if let Some((pane, row_index)) = shell
        .hovered_folder_pane
        .zip(shell.hovered_folder_row_index)
    {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_bool(&mut state, model.drag_overlay.active);
        fingerprint_mix_bool(&mut state, model.drag_overlay.valid_target);
        if let Some(row) = model.sources.folder_pane(pane).tree_rows.get(row_index) {
            fingerprint_mix_u8(
                &mut state,
                match row.kind {
                    crate::compat_app_contract::FolderRowKind::CreateDraft => 0,
                    crate::compat_app_contract::FolderRowKind::RenameDraft => 1,
                    crate::compat_app_contract::FolderRowKind::Existing => 2,
                },
            );
        } else {
            fingerprint_mix_u8(&mut state, u8::MAX);
        }
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    if shell.folder_create_editor_signature != 0 {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u8(
            &mut state,
            match model.sources.active_folder_pane {
                crate::compat_app_contract::FolderPaneIdModel::Upper => 0,
                crate::compat_app_contract::FolderPaneIdModel::Lower => 1,
            },
        );
        let active_tree_rows = &model.sources.active_folder_pane_model().tree_rows;
        let draft_row = active_tree_rows
            .iter()
            .find(|row| row.kind == crate::compat_app_contract::FolderRowKind::RenameDraft)
            .or_else(|| {
                active_tree_rows
                    .iter()
                    .find(|row| row.kind == crate::compat_app_contract::FolderRowKind::CreateDraft)
            });
        if let Some(row) = draft_row {
            fingerprint_mix_bool(&mut state, true);
            fingerprint_mix_u8(
                &mut state,
                match row.kind {
                    crate::compat_app_contract::FolderRowKind::CreateDraft => 0,
                    crate::compat_app_contract::FolderRowKind::RenameDraft => 1,
                    crate::compat_app_contract::FolderRowKind::Existing => 2,
                },
            );
            fingerprint_mix_option_string(&mut state, row.input_error.as_deref());
        } else {
            fingerprint_mix_bool(&mut state, false);
        }
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    state
}

pub(in super::super) fn focus_overlay_model_signature(
    model: &AppModel,
    shell: &FocusOverlayFingerprint,
) -> u64 {
    if !shell.has_focus_emphasis {
        return 0;
    }
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.browser.similarity_filtered);
    fingerprint_mix_bool(&mut state, model.browser.duplicate_cleanup_active);
    fingerprint_mix_u8(
        &mut state,
        match model.focus_context {
            crate::compat_app_contract::FocusContextModel::None => 0,
            crate::compat_app_contract::FocusContextModel::NavigationList => 1,
            crate::compat_app_contract::FocusContextModel::NavigationTree => 2,
            crate::compat_app_contract::FocusContextModel::ContentList => 3,
            crate::compat_app_contract::FocusContextModel::Timeline => 4,
        },
    );
    for row in model
        .browser
        .rows
        .iter()
        .filter(|row| row.selected || row.focused)
    {
        fingerprint_mix_usize(&mut state, row.visible_row);
        fingerprint_mix_bool(&mut state, row.selected);
        fingerprint_mix_bool(&mut state, row.focused);
        fingerprint_mix_bool(&mut state, row.locked);
        fingerprint_mix_u8(&mut state, row.playback_age_bucket as u8);
        if row.focused {
            fingerprint_mix_bool(&mut state, row.missing);
            fingerprint_mix_option_i8(&mut state, Some(row.rating_level));
            fingerprint_mix_string(&mut state, &row.label);
            fingerprint_mix_option_string(&mut state, row.bucket_label.as_deref());
        }
    }
    for (index, row) in model.sources.rows.iter().enumerate() {
        if row.assigned_to_upper_pane || row.assigned_to_lower_pane {
            fingerprint_mix_usize(&mut state, index);
            fingerprint_mix_bool(&mut state, row.assigned_to_upper_pane);
            fingerprint_mix_bool(&mut state, row.assigned_to_lower_pane);
        }
    }
    for pane in [
        &model.sources.upper_folder_pane,
        &model.sources.lower_folder_pane,
    ] {
        for (index, row) in pane
            .tree_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.selected || row.focused)
        {
            fingerprint_mix_usize(&mut state, index);
            fingerprint_mix_bool(&mut state, row.selected);
            fingerprint_mix_bool(&mut state, row.focused);
            if row.focused {
                fingerprint_mix_string(&mut state, &row.label);
                fingerprint_mix_usize(&mut state, row.depth);
            }
        }
    }
    state
}

pub(in super::super) fn modal_overlay_model_signature(model: &AppModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.options_panel.visible);
    fingerprint_mix_string(&mut state, &model.options_panel.default_identifier);
    fingerprint_mix_bool(&mut state, model.options_panel.input_monitoring_enabled);
    fingerprint_mix_bool(&mut state, model.options_panel.advance_after_rating_enabled);
    fingerprint_mix_bool(
        &mut state,
        model.options_panel.destructive_yolo_mode_enabled,
    );
    fingerprint_mix_bool(
        &mut state,
        model.options_panel.invert_waveform_scroll_enabled,
    );
    fingerprint_mix_option_string(
        &mut state,
        model.options_panel.trash_folder_label.as_deref(),
    );
    fingerprint_mix_string(&mut state, &model.browser_chrome.items_tab_label);
    fingerprint_mix_string(&mut state, &model.browser_chrome.map_tab_label);
    fingerprint_mix_usize(
        &mut state,
        model
            .columns
            .get(1)
            .map(|column| column.item_count)
            .unwrap_or(0),
    );
    fingerprint_mix_u64(&mut state, state_overlay_model_signature(model));
    state
}

pub(in super::super) fn waveform_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.transport_running);
    if let Some(selection) = model.waveform_selection_milli {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u16(&mut state, selection.start_milli);
        fingerprint_mix_u16(&mut state, selection.end_milli);
        fingerprint_mix_u32(&mut state, selection.start_micros);
        fingerprint_mix_u32(&mut state, selection.end_micros);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    if let Some(edit_selection) = model.waveform_edit_selection_milli {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_range(&mut state, &edit_selection);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    fingerprint_mix_usize(&mut state, model.waveform_slices.len());
    for slice in &model.waveform_slices {
        fingerprint_mix_range(&mut state, &slice.range);
        fingerprint_mix_bool(&mut state, slice.selected);
    }
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_end_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_in_end_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_mute_start_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_in_mute_start_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_curve_milli);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_start_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_out_start_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_mute_end_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_out_mute_end_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_curve_milli);
    fingerprint_mix_bool(&mut state, model.waveform_loop_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_loop_lock_enabled);
    fingerprint_mix_option_u16(&mut state, model.waveform_cursor_milli);
    fingerprint_mix_option_u16(&mut state, model.waveform_playhead_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_playhead_micros);
    fingerprint_mix_u16(&mut state, model.waveform_view_start_milli);
    fingerprint_mix_u16(&mut state, model.waveform_view_end_milli);
    fingerprint_mix_u32(&mut state, model.waveform_view_start_micros);
    fingerprint_mix_u32(&mut state, model.waveform_view_end_micros);
    fingerprint_mix_bool(&mut state, model.waveform_loading);
    if let Some(signature) = model.waveform_image_signature {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u64(&mut state, signature);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    state
}

pub(in super::super) fn chrome_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.transport_running);
    fingerprint_mix_bool(&mut state, model.map_active);
    fingerprint_mix_option_string(&mut state, model.waveform_tempo_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.waveform_zoom_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.waveform_loaded_label.as_deref());
    fingerprint_mix_u8(
        &mut state,
        match model.waveform_channel_view {
            crate::compat_app_contract::WaveformChannelViewModel::Mono => 0,
            crate::compat_app_contract::WaveformChannelViewModel::Stereo => 1,
        },
    );
    fingerprint_mix_bool(&mut state, model.waveform_normalized_audition_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_bpm_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_relative_bpm_grid_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_markers_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_slice_mode_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_exact_duplicate_cleanup_available);
    fingerprint_mix_bool(&mut state, model.waveform_loop_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_loop_lock_enabled);
    fingerprint_mix_string(&mut state, &model.waveform_transport_hint);
    fingerprint_mix_string(&mut state, &model.status_right);
    state
}

fn fingerprint_mix_rgba8(state: &mut u64, color: Rgba8) {
    fingerprint_mix_u8(state, color.r);
    fingerprint_mix_u8(state, color.g);
    fingerprint_mix_u8(state, color.b);
    fingerprint_mix_u8(state, color.a);
}

pub(in super::super) fn static_segment_style_signature(style: &StyleTokens) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_rgba8(&mut state, style.clear_color);
    fingerprint_mix_rgba8(&mut state, style.surface_base);
    fingerprint_mix_rgba8(&mut state, style.surface_raised);
    fingerprint_mix_rgba8(&mut state, style.surface_overlay);
    fingerprint_mix_rgba8(&mut state, style.border);
    fingerprint_mix_rgba8(&mut state, style.border_emphasis);
    fingerprint_mix_f32(&mut state, style.sizing.border_width);
    fingerprint_mix_f32(&mut state, style.sizing.focus_stroke_width);
    fingerprint_mix_f32(&mut state, style.sizing.font_header);
    fingerprint_mix_f32(&mut state, style.sizing.font_body);
    fingerprint_mix_f32(&mut state, style.sizing.font_meta);
    fingerprint_mix_f32(&mut state, style.sizing.font_status);
    state
}
