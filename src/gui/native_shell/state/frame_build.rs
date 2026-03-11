//! Core static-frame and state-overlay builders extracted from native shell state.

use super::*;

mod browser;
mod chrome;
mod map;
mod status_bar;
mod waveform;

use self::{browser::*, chrome::*, map::*, status_bar::*, waveform::*};

struct StaticFrameCtx<'a> {
    layout: &'a ShellLayout,
    style: &'a StyleTokens,
    model: &'a AppModel,
    sizing: SizingTokens,
    motion_wave: f32,
    focus_fill_emphasis: f32,
    focus_text_emphasis: f32,
}

struct BrowserFrameData {
    buttons: Vec<ActionButton>,
    column_chips: Vec<BrowserColumnChip>,
    rows: Vec<CachedBrowserRow>,
}

struct SidebarFrameData {
    source_row_rects: Vec<Rect>,
    folder_row_rects: Vec<Rect>,
}

impl NativeShellState {
    pub(super) fn build_frame_with_style_into_with_motion_sinks(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        pulse_phase: f32,
        include_overlays: bool,
        motion_model: Option<&NativeMotionModel>,
        static_segment_filter: Option<StaticFrameSegment>,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(pulse_phase);
        let focus_fill_emphasis = focus_fill_blend(style, motion_wave);
        let focus_text_emphasis = focus_text_blend(style, motion_wave);
        let ctx = StaticFrameCtx {
            layout,
            style,
            model,
            sizing,
            motion_wave,
            focus_fill_emphasis,
            focus_text_emphasis,
        };
        let build_global_static =
            static_segment_matches(static_segment_filter, StaticFrameSegment::GlobalStatic);
        let build_waveform_overlay =
            static_segment_matches(static_segment_filter, StaticFrameSegment::WaveformOverlay);
        let build_browser_rows_window =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserRowsWindow);
        let build_map_panel =
            static_segment_matches(static_segment_filter, StaticFrameSegment::MapPanel);
        let build_browser_frame =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserFrame);
        let build_status_bar =
            static_segment_matches(static_segment_filter, StaticFrameSegment::StatusBar);
        let build_browser_rows_or_map = build_browser_rows_window || build_map_panel;

        render_static_shell_surfaces(&ctx, primitives);

        if build_waveform_overlay {
            render_waveform_static(self, &ctx, primitives, text_runs, motion_model);
        }

        let browser_buttons = browser_action_buttons(layout, style, model);
        let browser_frame_data = BrowserFrameData {
            column_chips: browser_column_chips(layout, style, model, &browser_buttons),
            buttons: browser_buttons,
            rows: if build_browser_rows_or_map {
                rendered_browser_rows(layout, model, style)
            } else {
                Vec::new()
            },
        };
        let sidebar_data = SidebarFrameData {
            source_row_rects: if build_global_static {
                rendered_source_row_rects(layout, style, model)
            } else {
                Vec::new()
            },
            folder_row_rects: if build_global_static {
                rendered_folder_row_rects(layout, style, model)
            } else {
                Vec::new()
            },
        };
        if build_browser_rows_or_map {
            if model.map.active && build_map_panel {
                render_map_panel(&ctx, primitives);
            } else if !model.map.active && build_browser_rows_window {
                render_browser_rows_window(&ctx, primitives, text_runs, &browser_frame_data.rows);
            }
        }

        render_shell_borders(&ctx, primitives);

        if build_global_static {
            render_top_bar_controls(self, &ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_frame(self, &ctx, primitives, text_runs, &browser_frame_data);
        }
        if build_global_static {
            render_sidebar(self, &ctx, primitives, text_runs, &sidebar_data);
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        if model.map.active && build_map_panel {
            render_map_header(&ctx, text_runs);
        } else if build_browser_frame {
            render_browser_table_header(&ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_footer(&ctx, text_runs);
        }

        if build_status_bar {
            let status_text = if model.status_text.is_empty() {
                if self.transport_running {
                    format!(
                        "Transport: running | Selected column: {}",
                        self.selected_column + 1
                    )
                } else {
                    format!(
                        "Transport: stopped | Selected column: {}",
                        self.selected_column + 1
                    )
                }
            } else {
                model.status_text.clone()
            };
            let browser_summary = format!(
                "rows: {} | selected: {} | anchor: {} | search: {}{}",
                model.browser.visible_count,
                model.browser.selected_path_count,
                model
                    .browser
                    .anchor_visible_row
                    .map(|row| row.to_string())
                    .unwrap_or_else(|| String::from("—")),
                if model.browser.search_query.is_empty() {
                    "—"
                } else {
                    model.browser.search_query.as_str()
                },
                if model.browser.busy {
                    " | filtering…"
                } else {
                    ""
                }
            );
            let status_left = if model.status.left.is_empty() {
                status_text
            } else {
                model.status.left.clone()
            };
            let status_center = if model.status.center.is_empty() {
                browser_summary
            } else {
                model.status.center.clone()
            };
            let status_right = if model.status.right.is_empty() {
                format!("col: {}/3", self.selected_column + 1)
            } else {
                model.status.right.clone()
            };
            let inline_progress_active =
                model.progress_overlay.visible && !model.progress_overlay.modal;
            let status_left_text_rect = compute_status_text_line_rect(
                layout.status_left_segment,
                sizing,
                sizing.font_status,
            );
            let status_center_text_rect = compute_status_text_line_rect(
                layout.status_center_segment,
                sizing,
                sizing.font_status,
            );
            let status_right_text_rect =
                status_right_text_rect(layout.status_right_segment, sizing, None);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &status_left,
                        status_left_text_rect.width().max(36.0),
                        sizing.font_status,
                    ),
                    position: status_left_text_rect.min,
                    font_size: sizing.font_status,
                    color: style.text_muted,
                    max_width: Some(status_left_text_rect.width().max(36.0)),
                    align: TextAlign::Left,
                },
            );
            if inline_progress_active {
                let (progress_label_rect, progress_counter_rect) =
                    status_progress_text_rects(layout.status_center_segment, sizing);
                let progress_track_rect =
                    status_progress_track_rect(layout.status_center_segment, sizing);
                let progress_label = status_progress_label(model);
                let progress_counter = status_progress_counter(model);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &progress_label,
                            progress_label_rect.width().max(36.0),
                            sizing.font_status,
                        ),
                        position: progress_label_rect.min,
                        font_size: sizing.font_status,
                        color: style.text_primary,
                        max_width: Some(progress_label_rect.width().max(36.0)),
                        align: TextAlign::Left,
                    },
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &progress_counter,
                            progress_counter_rect.width().max(24.0),
                            sizing.font_status,
                        ),
                        position: progress_counter_rect.min,
                        font_size: sizing.font_status,
                        color: style.text_muted,
                        max_width: Some(progress_counter_rect.width().max(24.0)),
                        align: TextAlign::Right,
                    },
                );
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: progress_track_rect,
                        color: style.grid_soft,
                    }),
                );
                if let Some(fill_rect) = status_progress_fill_rect(progress_track_rect, model) {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: fill_rect,
                            color: style.accent_mint,
                        }),
                    );
                }
            } else {
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &status_center,
                            status_center_text_rect.width().max(36.0),
                            sizing.font_status,
                        ),
                        position: status_center_text_rect.min,
                        font_size: sizing.font_status,
                        color: style.text_primary,
                        max_width: Some(status_center_text_rect.width().max(36.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &status_right,
                        status_right_text_rect.width().max(36.0),
                        sizing.font_status,
                    ),
                    position: status_right_text_rect.min,
                    font_size: sizing.font_status,
                    color: style.text_muted,
                    max_width: Some(status_right_text_rect.width().max(36.0)),
                    align: TextAlign::Right,
                },
            );
        }

        if include_overlays {
            render_modal_overlays(primitives, text_runs, layout, style, model);
        }
    }

    /// Build only state-driven overlays into reusable buffers.
    pub(crate) fn build_state_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        push_waveform_toolbar_hover_tooltip(primitives, text_runs, layout, style, model, self);
        if let Some(visual) = self.browser_search_editor_visual.clone()
            && let (Some(search_field_rect), Some(search_text_rect)) = (
                self.browser_search_field_rect(layout, model),
                self.browser_search_text_rect(layout, model),
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
        if let Some(hovered_visible_row) = self.hovered_browser_visible_row {
            let browser_rows = self.cached_browser_rows(layout, style, model);
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
        if let Some(hovered_folder_row_index) = self.hovered_folder_row_index {
            let folder_row_rects = self.cached_folder_row_rects(layout, style, model);
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

        if self.has_focus_emphasis {
            {
                let source_row_rects = self.cached_source_row_rects(layout, style, model);
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
                        sizing.border_width,
                    );
                }
            }

            {
                let folder_row_rects = self.cached_folder_row_rects(layout, style, model);
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
                                bottom: row.focused
                                    || Some(row_rect.max.y) == last_folder_row_max_y,
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
                                text: truncate_to_width(
                                    &row_label,
                                    row_text_width,
                                    sizing.font_body,
                                ),
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

            {
                let browser_rows = self.cached_browser_rows(layout, style, model);
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
                            - browser_rating_indicator_reserved_width(
                                row.rating_level,
                                row.locked,
                                sizing,
                            )
                            - inline_tag_reserved_width)
                            .max(20.0);
                        if row.missing {
                            let marker_advance = browser_missing_marker_advance(sizing.font_body)
                                .min(label_max_width.max(0.0));
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
                            label_position.x = (label_position.x + marker_advance)
                                .min(row_text_layout.sample_label.max.x);
                            label_max_width =
                                (row_text_layout.sample_label.max.x - label_position.x).max(4.0);
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
        }

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

        render_source_context_menu(
            primitives,
            text_runs,
            layout,
            style,
            model,
            self.source_context_menu,
        );
        render_options_panel(primitives, text_runs, layout, style, model);
        render_progress_overlay(primitives, text_runs, layout, style, model);
        render_confirm_prompt(primitives, text_runs, layout, style, model);
        render_drag_overlay(primitives, text_runs, layout, style, model);

        frame.clear_color = style.clear_color;
    }
}

/// Render BPM-aligned waveform grid lines when beat snapping is enabled.
fn emit_waveform_bpm_grid(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    model: &AppModel,
    style: &StyleTokens,
) {
    if !model.waveform_chrome.bpm_snap_enabled {
        return;
    }
    let Some(step_micros) = model.waveform.beat_step_micros.filter(|step| *step > 0) else {
        return;
    };
    let view_start = u64::from(model.waveform.view_start_milli) * 1000;
    let view_end = u64::from(model.waveform.view_end_milli) * 1000;
    if view_end <= view_start || waveform_plot.width() <= 0.0 {
        return;
    }
    let step_micros = u64::from(step_micros);
    let first_beat_index = view_start.div_ceil(step_micros);
    let view_width = (view_end - view_start) as f32;
    let mut beat_index = first_beat_index;
    let mut beat_micros = beat_index.saturating_mul(step_micros);
    while beat_micros <= view_end {
        let ratio = (beat_micros.saturating_sub(view_start)) as f32 / view_width;
        let x = (waveform_plot.min.x + (waveform_plot.width() * ratio))
            .round()
            .clamp(waveform_plot.min.x, waveform_plot.max.x);
        let (line_color, line_width) = waveform_bpm_grid_line_style(style, beat_index);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(x, waveform_plot.min.y),
                    Point::new(
                        (x + line_width).min(waveform_plot.max.x),
                        waveform_plot.max.y,
                    ),
                ),
                color: line_color,
            }),
        );
        beat_index = beat_index.saturating_add(1);
        beat_micros = beat_micros.saturating_add(step_micros);
    }
}

/// Resolve the per-beat grid line styling for BPM-aligned waveform overlays.
fn waveform_bpm_grid_line_style(style: &StyleTokens, beat_index: u64) -> (Rgba8, f32) {
    if beat_index.is_multiple_of(4) {
        (style.grid_strong, style.sizing.border_width.max(2.0))
    } else {
        (
            blend_color(style.grid_soft, style.text_muted, 0.32),
            style.sizing.border_width.max(1.0),
        )
    }
}

fn push_browser_row_border(
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
