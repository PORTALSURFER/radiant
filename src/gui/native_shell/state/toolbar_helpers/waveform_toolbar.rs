//! Waveform toolbar layout and button rendering helpers.

use super::super::*;
use super::{waveform_toolbar_icon_rect, waveform_toolbar_visual_color};

pub(in crate::gui::native_shell::state) fn waveform_toolbar_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    bpm_input_active: bool,
    bpm_input_display: Option<&str>,
) -> Vec<WaveformToolbarButton> {
    let bpm_value_label = waveform_toolbar_bpm_value_label(model, bpm_input_display);
    let (transport_label, transport_icon, transport_action, transport_color) =
        if model.transport_running {
            (
                "Stop",
                Some(WaveformToolbarIcon::Stop),
                Some(UiAction::HandleEscape),
                style.highlight_orange_soft,
            )
        } else {
            (
                "Play",
                Some(WaveformToolbarIcon::Play),
                Some(UiAction::ToggleTransport),
                style.accent_warning,
            )
        };
    let specs = vec![
        (
            "Channel",
            Some(
                if model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo {
                    WaveformToolbarIcon::Stereo
                } else {
                    WaveformToolbarIcon::Mono
                },
            ),
            None,
            true,
            false,
            Some(UiAction::SetWaveformChannelView {
                stereo: model.waveform_channel_view != crate::app::WaveformChannelViewModel::Stereo,
            }),
            style.text_primary,
        ),
        (
            "Norm",
            Some(WaveformToolbarIcon::Normalize),
            None,
            true,
            model.waveform_normalized_audition_enabled,
            Some(UiAction::SetNormalizedAuditionEnabled {
                enabled: !model.waveform_normalized_audition_enabled,
            }),
            if model.waveform_normalized_audition_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "BPM Value",
            None,
            Some(bpm_value_label),
            true,
            bpm_input_active,
            None,
            style.text_primary,
        ),
        (
            "BPM Snap",
            Some(WaveformToolbarIcon::BpmSnap),
            None,
            true,
            model.waveform_bpm_snap_enabled,
            Some(UiAction::SetBpmSnapEnabled {
                enabled: !model.waveform_bpm_snap_enabled,
            }),
            if model.waveform_bpm_snap_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Rel Grid",
            Some(WaveformToolbarIcon::RelativeBpmGrid),
            None,
            true,
            model.waveform_relative_bpm_grid_enabled,
            Some(UiAction::SetRelativeBpmGridEnabled {
                enabled: !model.waveform_relative_bpm_grid_enabled,
            }),
            if model.waveform_relative_bpm_grid_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Tr Snap",
            Some(WaveformToolbarIcon::TransientSnap),
            None,
            true,
            model.waveform_transient_snap_enabled,
            Some(UiAction::SetTransientSnapEnabled {
                enabled: !model.waveform_transient_snap_enabled,
            }),
            if model.waveform_transient_snap_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Show Tr",
            Some(WaveformToolbarIcon::ShowTransients),
            None,
            true,
            model.waveform_transient_markers_enabled,
            Some(UiAction::SetTransientMarkersEnabled {
                enabled: !model.waveform_transient_markers_enabled,
            }),
            if model.waveform_transient_markers_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Slice",
            Some(WaveformToolbarIcon::Slice),
            None,
            true,
            model.waveform_slice_mode_enabled,
            Some(UiAction::SetSliceModeEnabled {
                enabled: !model.waveform_slice_mode_enabled,
            }),
            if model.waveform_slice_mode_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Silence Split",
            Some(WaveformToolbarIcon::Slice),
            None,
            model.waveform_loaded_label.is_some() && !model.waveform_loading,
            false,
            Some(UiAction::DetectWaveformSilenceSlices),
            style.highlight_blue_soft,
        ),
        (
            "Loop",
            Some(WaveformToolbarIcon::Loop),
            None,
            true,
            model.waveform_loop_enabled,
            Some(UiAction::ToggleLoopPlayback),
            if model.waveform_loop_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            transport_label,
            transport_icon,
            None,
            true,
            model.transport_running,
            transport_action,
            transport_color,
        ),
        (
            "Rec",
            Some(WaveformToolbarIcon::Record),
            None,
            false,
            false,
            None,
            style.highlight_blue_soft,
        ),
    ];
    let label_strings: Vec<String> = specs
        .iter()
        .map(|(label, _, display_text, ..)| waveform_toolbar_layout_label(label, display_text))
        .collect();
    let labels: Vec<&str> = label_strings.iter().map(String::as_str).collect();
    let cluster = Rect::from_min_max(
        Point::new(
            layout.waveform_header.min.x + (layout.waveform_header.width() * 0.42),
            layout.waveform_header.min.y,
        ),
        layout.waveform_header.max,
    );
    let rects =
        compute_update_action_button_rects(layout.waveform_header, cluster, style.sizing, &labels);
    let start_index = specs.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(specs.into_iter().skip(start_index))
        .map(
            |(rect, (label, icon, display_text, enabled, active, action, text_color))| {
                WaveformToolbarButton {
                    rect,
                    label,
                    icon,
                    display_text,
                    enabled,
                    active,
                    action,
                    text_color,
                }
            },
        )
        .collect()
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_layout_label(
    label: &str,
    display_text: &Option<String>,
) -> String {
    if label == "BPM Value" {
        return display_text
            .clone()
            .unwrap_or_else(|| String::from("120.0"));
    }
    String::from("Mono")
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_bpm_value_label(
    model: &NativeMotionModel,
    bpm_input_display: Option<&str>,
) -> String {
    if let Some(display) = bpm_input_display {
        return display.to_string();
    }
    model
        .waveform_tempo_label
        .as_deref()
        .and_then(crate::app::parse_waveform_tempo_number_text)
        .unwrap_or_else(|| String::from("120.0"))
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_left_edge(
    buttons: &[WaveformToolbarButton],
    fallback: f32,
) -> f32 {
    buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(fallback)
}

pub(in crate::gui::native_shell::state) fn render_waveform_toolbar_buttons(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    buttons: &[WaveformToolbarButton],
    hovered_hint: Option<WaveformToolbarHoverHint>,
    flashed_hint: Option<WaveformToolbarHoverHint>,
    motion_wave: f32,
    hide_active_bpm_value_text: bool,
) {
    for button in buttons {
        if hide_active_bpm_value_text && button.label == "BPM Value" {
            continue;
        }
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        let button_hint = waveform_toolbar_hover_hint(button.label);
        let is_hovered = button_hint.is_some() && button_hint == hovered_hint;
        let is_flashed = button_hint.is_some() && button_hint == flashed_hint;
        let icon_color = waveform_toolbar_visual_color(
            style,
            button.text_color,
            button.enabled,
            button.active,
            is_hovered,
            is_flashed,
            motion_wave,
        );
        if let Some(icon) = toolbar_icon_for_button(button) {
            if emit_toolbar_svg_icon(
                primitives,
                icon,
                waveform_toolbar_icon_rect(
                    button.rect,
                    sizing,
                    button.active,
                    is_hovered,
                    is_flashed,
                ),
                icon_color,
            ) {
                continue;
            }
        }
        emit_text(
            text_runs,
            TextRun {
                text: button
                    .display_text
                    .clone()
                    .unwrap_or_else(|| button.label.to_string()),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: icon_color,
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}
