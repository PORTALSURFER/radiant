//! Browser, waveform-toolbar, and sidebar helper geometry for native shell state.

use super::*;

pub(super) fn browser_toolbar_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    browser_buttons: &[ActionButton],
) -> BrowserToolbarLayout {
    let action_left = browser_buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .or(Some(
            layout.browser_toolbar.max.x - style.sizing.text_inset_x,
        ));
    let sections =
        compute_browser_toolbar_sections(layout.browser_toolbar, style.sizing, action_left);
    BrowserToolbarLayout {
        rating_filter_chips: sections.rating_filter_chips,
        search_field: sections.search_field,
        activity_chip: sections.activity_chip,
        sort_chip: sections.sort_chip,
        triage_chips: sections.triage_chips,
    }
}

pub(super) fn browser_rating_filter_chip_index(level: i8) -> Option<usize> {
    BROWSER_RATING_FILTER_LEVELS
        .iter()
        .position(|chip| *chip == level)
}

pub(super) fn browser_rating_filter_level_at_point(chips: [Rect; 8], point: Point) -> Option<i8> {
    chips
        .iter()
        .position(|rect| rect.width() > 1.0 && rect.contains(point))
        .map(|index| BROWSER_RATING_FILTER_LEVELS[index])
}

pub(super) fn browser_column_chips(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    browser_buttons: &[ActionButton],
) -> Vec<BrowserColumnChip> {
    let _ = (layout, style, model, browser_buttons);
    Vec::new()
}

pub(super) fn waveform_toolbar_buttons(
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

pub(super) fn waveform_toolbar_layout_label(label: &str, display_text: &Option<String>) -> String {
    if label == "BPM Value" {
        return display_text
            .clone()
            .unwrap_or_else(|| String::from("120.0"));
    }
    String::from("Mono")
}

pub(super) fn waveform_toolbar_bpm_value_label(
    model: &NativeMotionModel,
    bpm_input_display: Option<&str>,
) -> String {
    if let Some(display) = bpm_input_display {
        return display.to_string();
    }
    model
        .waveform_tempo_label
        .as_deref()
        .and_then(parse_waveform_tempo_number_text)
        .unwrap_or_else(|| String::from("120.0"))
}

pub(super) fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

pub(super) fn waveform_toolbar_left_edge(buttons: &[WaveformToolbarButton], fallback: f32) -> f32 {
    buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(fallback)
}

pub(super) fn render_waveform_toolbar_buttons(
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

pub(super) fn render_source_add_button_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let fill = source_add_button_fill(style, hovered, flashed, motion_wave);
    let border = source_add_button_border(style, hovered, flashed, motion_wave);
    let icon_color = source_add_button_icon_color(style, hovered, flashed, motion_wave);
    let label_rect = compute_action_button_text_rect(button_rect, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: fill,
        }),
    );
    push_border(primitives, button_rect, border, sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: String::from("+"),
            position: label_rect.min,
            font_size: sizing.font_meta,
            color: icon_color,
            max_width: Some(label_rect.width().max(8.0)),
            align: TextAlign::Center,
        },
    );
}

pub(super) fn source_add_button_fill(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.surface_overlay;
    let hover = blend_color(idle, style.accent_mint, 0.14 + (motion_wave * 0.04));
    let flash = blend_color(hover, style.text_primary, 0.16);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(super) fn render_browser_search_field_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    search_field_rect: Rect,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: search_field_rect,
            color: browser_search_field_hover_fill(style, motion_wave),
        }),
    );
    push_border(
        primitives,
        search_field_rect,
        browser_search_field_hover_border(style, motion_wave),
        sizing.border_width,
    );
}

pub(super) fn render_browser_rating_filter_chip_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    chip_rect: Rect,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: browser_rating_filter_chip_hover_fill(style, rating_level, active, motion_wave),
        }),
    );
    push_border(
        primitives,
        chip_rect,
        browser_rating_filter_chip_hover_border(style, rating_level, active, motion_wave),
        sizing.border_width,
    );
}

pub(super) fn render_waveform_bpm_input_focus_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    input_rect: Rect,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: input_rect,
            color: waveform_bpm_input_focus_fill(style, motion_wave),
        }),
    );
    push_border(
        primitives,
        input_rect,
        waveform_bpm_input_focus_border(style, motion_wave),
        sizing.border_width,
    );
}

pub(super) fn browser_search_field_hover_fill(style: &StyleTokens, motion_wave: f32) -> Rgba8 {
    translucent_overlay_color(
        style.surface_base,
        style.bg_tertiary,
        0.22 + (motion_wave * 0.04),
    )
}

pub(super) fn browser_rating_filter_chip_hover_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange_soft
    };
    let amount = if active { 0.34 } else { 0.2 } + (motion_wave * 0.04);
    translucent_overlay_color(
        browser_rating_filter_chip_fill(style, rating_level, active),
        tint,
        amount,
    )
}

pub(super) fn browser_search_field_hover_border(style: &StyleTokens, motion_wave: f32) -> Rgba8 {
    blend_color(
        style.border_emphasis,
        style.text_primary,
        0.48 + (motion_wave * 0.06),
    )
}

pub(super) fn browser_rating_filter_chip_hover_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange
    };
    blend_color(
        browser_rating_filter_chip_border(style, rating_level, active),
        tint,
        0.52 + (motion_wave * 0.08),
    )
}

pub(super) fn waveform_bpm_input_focus_fill(style: &StyleTokens, motion_wave: f32) -> Rgba8 {
    translucent_overlay_color(
        style.surface_base,
        style.highlight_orange_soft,
        0.24 + (motion_wave * 0.05),
    )
}

pub(super) fn waveform_bpm_input_focus_border(style: &StyleTokens, motion_wave: f32) -> Rgba8 {
    blend_color(
        style.border_emphasis,
        style.highlight_orange,
        0.58 + (motion_wave * 0.08),
    )
}

pub(super) fn source_add_button_border(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = blend_color(
        style.border_emphasis,
        style.text_primary,
        style.state_hover_soft,
    );
    let hover = blend_color(idle, style.accent_mint, 0.34 + (motion_wave * 0.08));
    if flashed {
        blend_color(hover, style.text_primary, 0.38)
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(super) fn source_add_button_icon_color(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.accent_mint;
    let hover = blend_color(idle, style.text_primary, 0.24 + (motion_wave * 0.06));
    if flashed {
        blend_color(hover, style.text_primary, 0.4)
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(super) fn waveform_toolbar_visual_color(
    style: &StyleTokens,
    base_color: Rgba8,
    enabled: bool,
    active: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    if !enabled {
        return blend_color(style.text_muted, style.bg_tertiary, 0.42);
    }
    let idle_color = blend_color(style.text_muted, style.bg_tertiary, 0.26);
    let active_color = if active {
        blend_color(base_color, style.text_primary, 0.08 + (motion_wave * 0.06))
    } else {
        idle_color
    };
    let hover_color = if hovered {
        let hover_emphasis = if active { 0.28 } else { 0.82 };
        blend_color(
            active_color,
            base_color,
            hover_emphasis + (motion_wave * 0.06),
        )
    } else {
        active_color
    };
    if flashed {
        blend_color(hover_color, style.text_primary, 0.42)
    } else {
        hover_color
    }
}

pub(super) fn waveform_toolbar_icon_rect(
    button_rect: Rect,
    sizing: SizingTokens,
    active: bool,
    hovered: bool,
    flashed: bool,
) -> Rect {
    let max_side =
        (button_rect.width().min(button_rect.height()) - (sizing.border_width * 4.0)).max(6.0);
    let emphasis = if flashed {
        2.0
    } else if hovered {
        1.0
    } else if active {
        0.6
    } else {
        0.0
    };
    let icon_side = (max_side + emphasis).clamp(8.0, 18.0);
    let offset_x = (button_rect.width() - icon_side).max(0.0) * 0.5;
    let offset_y = (button_rect.height() - icon_side).max(0.0) * 0.5;
    Rect::from_min_max(
        Point::new(button_rect.min.x + offset_x, button_rect.min.y + offset_y),
        Point::new(
            button_rect.min.x + offset_x + icon_side,
            button_rect.min.y + offset_y + icon_side,
        ),
    )
}

pub(super) fn top_bar_controls_layout(
    layout: &ShellLayout,
    sizing: SizingTokens,
) -> TopBarControlsLayout {
    let resolved = compute_top_bar_controls_sections(layout, sizing);
    TopBarControlsLayout {
        active: resolved.active,
        options_label: resolved.options_label,
        volume_meter: resolved.volume_meter,
        volume_value: resolved.volume_value,
        volume_label: resolved.volume_label,
    }
}

/// Resolve a stable browser-row border stroke in logical units.
///
/// At `ui_scale == 1.0` this resolves to `1.0` logical px so row borders stay
/// visually consistent at 100% scale.
pub(super) fn browser_row_border_stroke(layout: &ShellLayout) -> f32 {
    layout.ui_scale.max(1.0)
}

/// Return x-advance reserved for the missing-file marker before a sample label.
pub(super) fn browser_missing_marker_advance(font_size: f32) -> f32 {
    (font_size * 1.05).max(7.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserRatingIndicatorLayout {
    pub(super) rects: [Rect; 3],
    pub(super) count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserRatingIndicatorAnchor {
    pub(super) sample_label: Rect,
    pub(super) label_origin_x: f32,
    pub(super) label_rendered_width: f32,
    pub(super) right_limit_x: f32,
}

pub(super) fn browser_rating_indicator_reserved_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let count = browser_rating_indicator_count(rating_level, locked);
    if count == 0 {
        return 0.0;
    }
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing);
    let gap = browser_rating_indicator_gap(sizing);
    let text_gap = browser_rating_indicator_text_gap(sizing);
    (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap) + text_gap
}

pub(super) fn browser_rating_indicator_layout(
    anchor: BrowserRatingIndicatorAnchor,
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> Option<BrowserRatingIndicatorLayout> {
    let count = browser_rating_indicator_count(rating_level, locked);
    let sample_label = anchor.sample_label;
    if count == 0 || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return None;
    }
    let side = browser_rating_indicator_side(sizing).min(sample_label.height().max(1.0));
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing)
        .min(sample_label.width().max(1.0));
    let gap = browser_rating_indicator_gap(sizing);
    let total_width = (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap);
    let ideal_start_x = anchor.label_origin_x
        + anchor.label_rendered_width.max(0.0)
        + browser_rating_indicator_text_gap(sizing);
    let right_limit_x = anchor
        .right_limit_x
        .clamp(sample_label.min.x, sample_label.max.x);
    let max_start_x = (right_limit_x - total_width).max(sample_label.min.x);
    let start_x = ideal_start_x.clamp(sample_label.min.x, max_start_x);
    let min_y = sample_label.min.y + ((sample_label.height() - side) * 0.5).floor();
    let max_y = (min_y + side).min(sample_label.max.y);
    let mut rects = [Rect::from_min_max(sample_label.min, sample_label.min); 3];
    for (index, rect) in rects.iter_mut().take(count).enumerate() {
        let min_x = start_x + index as f32 * (width + gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + width).min(sample_label.max.x), max_y),
        );
    }
    Some(BrowserRatingIndicatorLayout { rects, count })
}

pub(super) fn browser_rating_indicator_color(style: &StyleTokens, rating_level: i8) -> Rgba8 {
    if rating_level < 0 {
        style.accent_trash
    } else {
        style.accent_mint
    }
}

pub(super) fn browser_rating_filter_chip_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level == 4 {
        blend_color(style.accent_mint, style.text_primary, 0.28)
    } else if rating_level > 0 {
        style.accent_mint
    } else if active {
        style.highlight_orange
    } else {
        style.text_primary
    };
    let amount = if active {
        0.9
    } else if rating_level == 0 {
        0.14
    } else {
        0.18
    };
    blend_color(
        if active {
            style.surface_overlay
        } else {
            style.surface_base
        },
        tint,
        amount,
    )
}

pub(super) fn browser_rating_filter_chip_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    if active {
        if rating_level < 0 {
            blend_color(style.accent_trash, style.text_primary, 0.24)
        } else if rating_level == 4 {
            blend_color(style.accent_mint, style.text_primary, 0.44)
        } else if rating_level > 0 {
            blend_color(style.accent_mint, style.text_primary, 0.24)
        } else {
            blend_color(style.highlight_orange, style.text_primary, 0.22)
        }
    } else {
        blend_color(style.border, style.surface_overlay, 0.25)
    }
}

pub(super) fn browser_rating_indicator_side(sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 0.68).round().clamp(5.0, 8.0)
}

pub(super) fn browser_rating_indicator_count(rating_level: i8, locked: bool) -> usize {
    if locked && rating_level > 0 {
        1
    } else {
        rating_level.unsigned_abs().min(3) as usize
    }
}

pub(super) fn browser_rating_indicator_unit_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let side = browser_rating_indicator_side(sizing);
    if locked && rating_level > 0 {
        (side * 2.4).round().max(side + 2.0)
    } else {
        side
    }
}

pub(super) fn browser_rating_indicator_gap(sizing: SizingTokens) -> f32 {
    sizing.border_width.max(1.0) + 1.0
}

pub(super) fn browser_rating_indicator_text_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(5.0).max(2.0)
}

/// Return width reserved for the inline browser metadata chip cluster plus its left gutter.
pub(super) fn browser_inline_tag_reserved_width(text: &str, sizing: SizingTokens) -> f32 {
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return 0.0;
    }
    let chips_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum();
    let chip_gap_count = labels.len().saturating_sub(1) as f32;
    chips_width
        + (chip_gap_count * browser_inline_tag_chip_gap(sizing))
        + browser_inline_tag_gap(sizing)
}

/// Approximate the rendered width of one inline browser metadata label.
pub(super) fn browser_inline_tag_text_width(text: &str, sizing: SizingTokens) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (sizing.font_meta * 0.56).max(1.0)).ceil()
}

/// Return the horizontal gap between a sample label and its inline metadata label.
pub(super) fn browser_inline_tag_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(3.0)
}

/// Split one inline browser metadata payload into stable per-chip labels.
pub(super) fn browser_inline_tag_labels(text: &str) -> impl Iterator<Item = &str> + '_ {
    text.split(" · ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Return the filled chip width needed for one inline browser metadata label.
pub(super) fn browser_inline_tag_chip_width(text: &str, sizing: SizingTokens) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    browser_inline_tag_text_width(text, sizing) + (browser_inline_tag_chip_padding_x(sizing) * 2.0)
}

/// Compute chip rects for one inline browser metadata cluster.
pub(super) fn browser_inline_tag_chip_rects(
    sample_label: Rect,
    text: &str,
    trailing_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    if text.is_empty() || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return Vec::new();
    }
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return Vec::new();
    }
    let chip_gap = browser_inline_tag_chip_gap(sizing);
    let total_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * chip_gap);
    let right_edge = (sample_label.max.x - trailing_reserved_width).max(sample_label.min.x);
    let start_x = (right_edge - total_width).max(sample_label.min.x);
    let chip_height = browser_inline_tag_chip_height(sample_label, sizing);
    let min_y = sample_label.min.y + ((sample_label.height() - chip_height) * 0.5).floor();
    let max_y = (min_y + chip_height).min(sample_label.max.y);
    let mut x = start_x;
    labels
        .into_iter()
        .map(|label| {
            let width = browser_inline_tag_chip_width(label, sizing);
            let rect = Rect::from_min_max(
                Point::new(x, min_y),
                Point::new((x + width).min(right_edge), max_y),
            );
            x = (rect.max.x + chip_gap).min(right_edge);
            rect
        })
        .collect()
}

/// Return the inset text origin for one inline browser metadata chip.
pub(super) fn browser_inline_tag_text_origin(chip_rect: Rect, sizing: SizingTokens) -> Point {
    Point::new(
        chip_rect.min.x + browser_inline_tag_chip_padding_x(sizing),
        chip_rect.min.y + ((chip_rect.height() - sizing.font_meta) * 0.5).floor(),
    )
}

pub(super) fn browser_inline_tag_chip_height(sample_label: Rect, sizing: SizingTokens) -> f32 {
    (sizing.font_meta + (browser_inline_tag_chip_padding_y(sizing) * 2.0))
        .round()
        .clamp(10.0, sample_label.height().max(1.0))
}

pub(super) fn browser_inline_tag_chip_padding_x(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}

pub(super) fn browser_inline_tag_chip_padding_y(sizing: SizingTokens) -> f32 {
    sizing.text_inset_y.min(3.0).max(1.0)
}

pub(super) fn browser_inline_tag_chip_gap(sizing: SizingTokens) -> f32 {
    sizing.border_width.max(1.0) + 2.0
}

pub(super) fn source_add_button_rect(header_rect: Rect, sizing: SizingTokens) -> Option<Rect> {
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return None;
    }
    let side = (sizing.font_header + (sizing.text_inset_y * 1.5))
        .round()
        .clamp(12.0, header_rect.height().max(12.0));
    if header_rect.width() < side + (sizing.text_inset_x * 2.0) {
        return None;
    }
    let max_x = header_rect.max.x - sizing.text_inset_x.max(0.0);
    let min_x = (max_x - side).max(header_rect.min.x);
    let min_y = header_rect.min.y + ((header_rect.height() - side) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, (min_y + side).min(header_rect.max.y)),
    ))
}

/// Snap browser-row border bounds to the border stroke grid to avoid uneven AA
/// widths between top/bottom edges.
pub(super) fn browser_row_border_rect(rect: Rect, stroke: f32) -> Rect {
    let stroke = stroke.max(1.0);
    let snap = |value: f32| (value / stroke).round() * stroke;
    let min_x = snap(rect.min.x);
    let min_y = snap(rect.min.y);
    let max_x = snap(rect.max.x);
    let max_y = snap(rect.max.y);
    let snapped = Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y));
    if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
        rect
    } else {
        snapped
    }
}

pub(super) fn sidebar_sections(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarSections {
    let resolved = compute_sidebar_row_sections(
        layout.sidebar_rows,
        style.sizing,
        SidebarRowCounts {
            source_rows: rendered_source_rows(style, model),
            folder_rows: rendered_folder_rows(style, model),
        },
    );
    SidebarSections {
        source_rows: resolved.source_rows,
        folder_header: resolved.folder_header,
        folder_rows: resolved.folder_rows,
    }
}

pub(super) fn browser_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let _ = (layout, style, model);
    Vec::new()
}

pub(super) fn source_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "New",
            model.sources.folder_actions.can_create_folder,
            UiAction::StartNewFolder,
            style.text_primary,
        ),
        (
            "Root",
            model.sources.folder_actions.can_create_folder_at_root,
            UiAction::StartNewFolderAtRoot,
            style.text_muted,
        ),
        (
            "Rename",
            model.sources.folder_actions.can_rename_folder,
            UiAction::StartFolderRename,
            style.accent_warning,
        ),
        (
            "Delete",
            model.sources.folder_actions.can_delete_folder,
            UiAction::DeleteFocusedFolder,
            style.accent_copper,
        ),
        (
            "Recovery",
            model.sources.folder_actions.can_clear_recovery_log,
            UiAction::ClearFolderDeleteRecoveryLog,
            style.accent_mint,
        ),
    ];
    let rects =
        compute_sidebar_action_button_rects(layout.sidebar_footer, style.sizing, definitions.len());
    let start_index = definitions.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(definitions.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, action, text_color))| ActionButton {
                rect,
                label,
                enabled,
                action,
                text_color,
            },
        )
        .collect()
}

/// Build source context-menu panel geometry and action buttons.
pub(super) fn source_context_menu_spec(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    menu: Option<SourceContextMenuState>,
) -> Option<(Rect, Vec<ActionButton>)> {
    let menu = menu?;
    if menu.row_index >= model.sources.rows.len() {
        return None;
    }
    let source_index = menu.row_index;
    let definitions = [
        (
            "Reload",
            true,
            UiAction::ReloadSourceRow {
                index: source_index,
            },
            style.text_primary,
        ),
        (
            "Hard sync",
            true,
            UiAction::HardSyncSourceRow {
                index: source_index,
            },
            style.accent_warning,
        ),
        (
            "Open folder",
            true,
            UiAction::OpenSourceFolderRow {
                index: source_index,
            },
            style.accent_mint,
        ),
        (
            "Remove source",
            true,
            UiAction::RemoveSourceRow {
                index: source_index,
            },
            style.accent_copper,
        ),
        (
            "Remove dead links",
            true,
            UiAction::RemoveDeadLinksForSourceRow {
                index: source_index,
            },
            style.accent_copper,
        ),
    ];
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = sizing.sidebar_action_button_width.max(168.0);
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let button_gap = sizing.sidebar_action_button_gap.max(2.0);
    let button_count = definitions.len();
    let panel_width = button_width + panel_padding * 2.0;
    let panel_height = (button_height * button_count as f32)
        + (button_gap * button_count.saturating_sub(1) as f32)
        + panel_padding * 2.0;
    let min_x = layout.sidebar.min.x + sizing.panel_inset;
    let max_x = (layout.sidebar.max.x - sizing.panel_inset - panel_width).max(min_x);
    let min_y = layout.sidebar.min.y + sizing.panel_inset;
    let max_y = (layout.sidebar.max.y - sizing.panel_inset - panel_height).max(min_y);
    let panel_min = Point::new(
        menu.anchor.x.clamp(min_x, max_x),
        menu.anchor.y.clamp(min_y, max_y),
    );
    let panel_rect = Rect::from_min_max(
        panel_min,
        Point::new(panel_min.x + panel_width, panel_min.y + panel_height),
    );
    let mut buttons = Vec::with_capacity(button_count);
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = panel_rect.min.y + panel_padding;
    for (label, enabled, action, text_color) in definitions {
        let rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        buttons.push(ActionButton {
            rect,
            label,
            enabled,
            action,
            text_color,
        });
        button_y += button_height + button_gap;
    }
    Some((panel_rect, buttons))
}
