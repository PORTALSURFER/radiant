//! State-driven overlay builders for the native shell.

use super::*;

pub(super) fn render_state_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    push_waveform_toolbar_hover_tooltip(primitives, text_runs, layout, style, model, shell_state);
    if let Some(visual) = shell_state.browser_search_editor_visual.clone()
        && let (Some(search_field_rect), Some(search_text_rect)) = (
            shell_state.browser_search_field_rect(layout, model),
            shell_state.browser_search_text_rect(layout, model),
        )
    {
        render_active_browser_search_editor(
            primitives,
            text_runs,
            style,
            sizing,
            search_field_rect,
            search_text_rect,
            &visual,
        );
    }
    if let Some(hovered_visible_row) = shell_state.hovered_browser_visible_row {
        let browser_rows = shell_state.cached_browser_rows(layout, style, model);
        if let Some(row) = browser_rows
            .iter()
            .find(|row| row.visible_row == hovered_visible_row)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: browser_row_hover_fill(style),
                }),
            );
        }
    }
    if let Some(hovered_folder_row_index) = shell_state.hovered_folder_row_index {
        let folder_row_rects = shell_state.cached_folder_row_rects(layout, style, model);
        if let Some(row_rect) = folder_row_rects.get(hovered_folder_row_index) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: *row_rect,
                    color: subtle_item_hover_fill(style),
                }),
            );
        }
    }
    if shell_state.has_focus_emphasis {
        render_source_focus_overlay(shell_state, layout, style, model, primitives);
        render_folder_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
        render_browser_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
    }
    render_browser_tab_overlay(primitives, text_runs, layout, style, model);
    render_source_context_menu(
        primitives,
        text_runs,
        layout,
        style,
        model,
        shell_state.source_context_menu,
    );
    render_options_panel(primitives, text_runs, layout, style, model);
    render_progress_overlay(primitives, text_runs, layout, style, model);
    render_confirm_prompt(primitives, text_runs, layout, style, model);
    render_drag_overlay(primitives, text_runs, layout, style, model);
}

pub(super) fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    if sides.top {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
                color,
            }),
        );
    }
    if sides.bottom {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
                color,
            }),
        );
    }
    if sides.left {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
                color,
            }),
        );
    }
    if sides.right {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
                color,
            }),
        );
    }
}

fn render_source_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    let source_row_rects = shell_state.cached_source_row_rects(layout, style, model);
    for (row_index, row_rect) in source_row_rects.iter().enumerate() {
        let Some(row) = model.sources.rows.get(row_index) else {
            continue;
        };
        let row_selected = row.selected
            || model
                .sources
                .selected_row
                .is_some_and(|selected| selected == row_index);
        if !row_selected {
            continue;
        }
        push_border(
            primitives,
            *row_rect,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
            style.sizing.border_width,
        );
    }
}

fn render_folder_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    let folder_row_rects = shell_state.cached_folder_row_rects(layout, style, model);
    let last_folder_row_max_y = folder_row_rects.last().map(|rect| rect.max.y);
    for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
        let Some(row) = model.sources.folder_rows.get(row_index) else {
            continue;
        };
        if !(row.selected || row.focused) {
            continue;
        }
        if row.focused {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: *row_rect,
                    color: translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_strong,
                        style.state_focus_pulse_blend,
                    ),
                }),
            );
        }
        if row.focused || row.selected {
            push_browser_row_border(
                primitives,
                *row_rect,
                if row.focused {
                    blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    )
                } else {
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        style.state_selected_blend,
                    )
                },
                if row.focused {
                    sizing.focus_stroke_width
                } else {
                    sizing.border_width
                },
                BorderSides {
                    top: true,
                    bottom: row.focused || Some(row_rect.max.y) == last_folder_row_max_y,
                    left: row.focused,
                    right: row.focused,
                },
            );
        }
        if row.focused {
            let glyph = if row.is_root {
                "•"
            } else if row.has_children {
                if row.expanded { "▼" } else { "▶" }
            } else {
                "·"
            };
            let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                .min((row_rect.width() * 0.45).max(0.0));
            let row_text_rect =
                compute_sidebar_folder_row_text_rect(*row_rect, sizing, depth_indent);
            let row_text_width = row_text_rect.width().max(24.0);
            let row_label = format!("{glyph} {}", row.label);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(&row_label, row_text_width, sizing.font_body),
                    position: row_text_rect.min,
                    font_size: sizing.font_body,
                    color: blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                    max_width: Some(row_text_width),
                    align: TextAlign::Left,
                },
            );
        }
    }
}

fn render_browser_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    let browser_rows = shell_state.cached_browser_rows(layout, style, model);
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows.iter() {
        if !(row.selected || row.focused) {
            continue;
        }
        let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
        let row_border_stroke = browser_row_border_stroke(layout);
        let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
        if row.focused {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_strong,
                        style.state_focus_pulse_blend,
                    ),
                }),
            );
        } else if row.selected {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: if row.locked {
                        locked_browser_row_fill(style, selected_browser_row_fill(style))
                    } else {
                        selected_browser_row_fill(style)
                    },
                }),
            );
        }
        push_browser_row_border(
            primitives,
            row_border_rect,
            if row.focused {
                blend_color(
                    style.accent_warning,
                    style.text_primary,
                    style.state_focus_pulse_blend,
                )
            } else {
                style.border
            },
            row_border_stroke,
            BorderSides {
                top: true,
                bottom: row.focused || Some(row.rect.max.y) == last_row_max_y,
                left: row.focused,
                right: row.focused,
            },
        );
        if row.focused {
            let mut label_position = row_text_layout.sample_label.min;
            let inline_tag_reserved_width =
                browser_inline_tag_reserved_width(&row.bucket_label, sizing);
            let mut label_max_width = (row_text_layout.sample_label.width()
                - browser_rating_indicator_reserved_width(row.rating_level, row.locked, sizing)
                - inline_tag_reserved_width)
                .max(20.0);
            if row.missing {
                let marker_advance =
                    browser_missing_marker_advance(sizing.font_body).min(label_max_width.max(0.0));
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from(BROWSER_MISSING_SAMPLE_MARKER),
                        position: label_position,
                        font_size: sizing.font_body,
                        color: BROWSER_MISSING_SAMPLE_MARKER_COLOR,
                        max_width: Some(marker_advance),
                        align: TextAlign::Left,
                    },
                );
                label_position.x =
                    (label_position.x + marker_advance).min(row_text_layout.sample_label.max.x);
                label_max_width = (row_text_layout.sample_label.max.x - label_position.x).max(4.0);
            }
            emit_text(
                text_runs,
                TextRun {
                    text: row.visible_row.to_string(),
                    position: row_text_layout.index_label.min,
                    font_size: sizing.font_meta,
                    color: blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                    max_width: Some(row_text_layout.index_label.width().max(12.0)),
                    align: TextAlign::Right,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: row.label.clone(),
                    position: label_position,
                    font_size: sizing.font_body,
                    color: blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                    max_width: Some(label_max_width),
                    align: TextAlign::Left,
                },
            );
        }
    }
}

fn render_browser_tab_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let sizing = style.sizing;
    let tabs = compute_browser_tabs_rects(layout.browser_tabs, sizing);
    let (samples_fill, map_fill, samples_text_color, map_text_color) = if !model.map.active {
        (
            blend_color(
                style.surface_overlay,
                style.bg_tertiary,
                style.state_selected_blend + 0.1,
            ),
            style.surface_base,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
            style.text_muted,
        )
    } else {
        (
            style.surface_base,
            blend_color(
                style.surface_overlay,
                style.bg_tertiary,
                style.state_selected_blend + 0.1,
            ),
            style.text_muted,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
        )
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.samples,
            color: samples_fill,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }),
    );
    push_border(primitives, tabs.samples, style.border, sizing.border_width);
    push_border(
        primitives,
        tabs.map,
        blend_color(style.accent_mint, style.text_primary, 0.42),
        sizing.border_width,
    );
    let tabs_text_layout = compute_browser_tabs_text_layout(tabs.samples, tabs.map, sizing);
    let samples_text = format!(
        "{} ({})",
        model.browser_chrome.samples_tab_label,
        model
            .columns
            .get(1)
            .map(|column| column.item_count)
            .unwrap_or(0)
    );
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &samples_text,
                tabs_text_layout.samples_label.width().max(40.0),
                sizing.font_header,
            ),
            position: tabs_text_layout.samples_label.min,
            font_size: sizing.font_header,
            color: samples_text_color,
            max_width: Some(tabs_text_layout.samples_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from(model.browser_chrome.map_tab_label.as_str()),
            position: tabs_text_layout.map_label.min,
            font_size: sizing.font_header,
            color: map_text_color,
            max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_source_context_menu(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    source_context_menu: Option<SourceContextMenuState>,
) {
    let Some((menu_panel, menu_buttons)) =
        source_context_menu_spec(layout, style, model, source_context_menu)
    else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: menu_panel,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        menu_panel,
        style.border_emphasis,
        sizing.border_width,
    );
    for button in menu_buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    blend_color(style.surface_base, style.bg_secondary, 0.36)
                } else {
                    style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled {
                style.border
            } else {
                style.grid_soft
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some(label_rect.width().max(16.0)),
                align: TextAlign::Center,
            },
        );
    }
}

fn push_waveform_toolbar_hover_tooltip(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    shell_state: &mut NativeShellState,
) {
    let Some(hint) = shell_state.hovered_waveform_toolbar_hint else {
        return;
    };
    let motion_model = NativeMotionModel::from_app_model(model);
    let buttons = shell_state.cached_waveform_toolbar_buttons(layout, style, &motion_model);
    let Some(button_rect) = buttons
        .iter()
        .find(|button| waveform_toolbar_hover_hint(button.label) == Some(hint))
        .map(|button| button.rect)
    else {
        return;
    };
    let text = waveform_toolbar_hover_hint_text(hint, model);
    let font_size = style.sizing.font_status.max(9.0);
    let text_padding_x = (font_size * 0.55).max(4.0);
    let text_padding_y = (font_size * 0.35).max(3.0);
    let tooltip_width = ((text.chars().count() as f32 * font_size * 0.54) + (text_padding_x * 2.0))
        .clamp(84.0, layout.waveform_card.width().max(84.0));
    let tooltip_height = (font_size + (text_padding_y * 2.0)).max(16.0);
    let gap = style.sizing.border_width.max(1.0) * 3.0;
    let mut min_x = ((button_rect.min.x + button_rect.max.x) * 0.5) - (tooltip_width * 0.5);
    min_x = min_x.clamp(
        layout.waveform_card.min.x + style.sizing.text_inset_x,
        layout.waveform_card.max.x - style.sizing.text_inset_x - tooltip_width,
    );
    let preferred_top = button_rect.min.y - gap - tooltip_height;
    let min_y = if preferred_top >= layout.waveform_card.min.y + style.sizing.border_width {
        preferred_top
    } else {
        (button_rect.max.y + gap).min(layout.waveform_card.max.y - tooltip_height)
    };
    let tooltip_rect = Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + tooltip_width, min_y + tooltip_height),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tooltip_rect,
            color: blend_color(style.surface_overlay, style.bg_tertiary, 0.76),
        }),
    );
    push_border(
        primitives,
        tooltip_rect,
        blend_color(style.border_emphasis, style.text_primary, 0.58),
        style.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text,
            position: Point::new(
                tooltip_rect.min.x + text_padding_x,
                tooltip_rect.min.y + text_padding_y,
            ),
            font_size,
            color: style.text_primary,
            max_width: Some((tooltip_rect.width() - (text_padding_x * 2.0)).max(16.0)),
            align: TextAlign::Left,
        },
    );
}

fn waveform_toolbar_hover_hint_text(hint: WaveformToolbarHoverHint, model: &AppModel) -> String {
    match hint {
        WaveformToolbarHoverHint::ChannelView => {
            if model.waveform_chrome.channel_view == crate::app::WaveformChannelViewModel::Stereo {
                String::from("Switch waveform view to mono")
            } else {
                String::from("Switch waveform view to split stereo")
            }
        }
        WaveformToolbarHoverHint::NormalizedAudition => {
            if model.waveform_chrome.normalized_audition_enabled {
                String::from("Disable normalized audition")
            } else {
                String::from("Enable normalized audition")
            }
        }
        WaveformToolbarHoverHint::BpmValue => model
            .waveform
            .tempo_label
            .as_deref()
            .map(|tempo| format!("Edit playback BPM ({tempo})"))
            .unwrap_or_else(|| String::from("Edit playback BPM")),
        WaveformToolbarHoverHint::BpmSnap => {
            if model.waveform_chrome.bpm_snap_enabled {
                String::from("Disable BPM snapping")
            } else {
                String::from("Enable BPM snapping")
            }
        }
        WaveformToolbarHoverHint::TransientSnap => {
            if model.waveform_chrome.transient_snap_enabled {
                String::from("Disable transient snapping")
            } else {
                String::from("Enable transient snapping")
            }
        }
        WaveformToolbarHoverHint::ShowTransients => {
            if model.waveform_chrome.transient_markers_enabled {
                String::from("Hide transient markers")
            } else {
                String::from("Show transient markers")
            }
        }
        WaveformToolbarHoverHint::SliceMode => {
            if model.waveform_chrome.slice_mode_enabled {
                String::from("Disable slice mode")
            } else {
                String::from("Enable slice mode")
            }
        }
        WaveformToolbarHoverHint::Loop => {
            if model.waveform.loop_enabled {
                String::from("Disable loop playback")
            } else {
                String::from("Enable loop playback")
            }
        }
        WaveformToolbarHoverHint::Stop => String::from("Stop playback"),
        WaveformToolbarHoverHint::Play => {
            if model.transport_running {
                String::from("Pause playback")
            } else {
                String::from("Start playback")
            }
        }
        WaveformToolbarHoverHint::Record => String::from("Recording is currently unavailable"),
    }
}
