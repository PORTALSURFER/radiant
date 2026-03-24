//! Stable signature builders used to decide whether retained scenes can be reused.

use super::*;

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

fn fingerprint_mix_range(state: &mut u64, range: &crate::app::NormalizedRangeModel) {
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
    fingerprint_mix_option_usize(&mut state, model.sources.focused_folder_row);
    fingerprint_mix_bool(&mut state, model.confirm_prompt.visible);
    fingerprint_mix_u8(
        &mut state,
        match model.confirm_prompt.kind {
            None => 0,
            Some(crate::app::ConfirmPromptKind::DestructiveEdit) => 1,
            Some(crate::app::ConfirmPromptKind::BrowserRename) => 2,
            Some(crate::app::ConfirmPromptKind::FolderRename) => 3,
            Some(crate::app::ConfirmPromptKind::FolderCreate) => 4,
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
    fingerprint_mix_u8(
        &mut state,
        match model.update.status {
            crate::app::UpdateStatusModel::Idle => 0,
            crate::app::UpdateStatusModel::Checking => 1,
            crate::app::UpdateStatusModel::Available => 2,
            crate::app::UpdateStatusModel::Error => 3,
        },
    );
    fingerprint_mix_bool(&mut state, model.map.active);
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
            crate::app::WaveformChannelViewModel::Mono => 0,
            crate::app::WaveformChannelViewModel::Stereo => 1,
        },
    );
    fingerprint_mix_bool(&mut state, model.waveform_normalized_audition_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_bpm_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_markers_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_slice_mode_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_loop_enabled);
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
