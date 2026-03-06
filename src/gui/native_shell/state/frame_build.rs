//! Core static-frame and state-overlay builders extracted from native shell state.

use super::*;

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

        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.top_bar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.sidebar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.content,
                color: style.surface_base,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.waveform_card,
                color: style.surface_raised,
            }),
        );
        let waveform_deck_backplate = waveform_deck_backplate_rect(layout.waveform_card, sizing);
        if waveform_deck_backplate.width() > 0.0 && waveform_deck_backplate.height() > 0.0 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: waveform_deck_backplate,
                    color: blend_color(style.surface_overlay, style.bg_secondary, 0.38),
                }),
            );
            push_border(
                primitives,
                waveform_deck_backplate,
                blend_color(style.border_emphasis, style.bg_tertiary, 0.32),
                sizing.border_width,
            );
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.status_bar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_panel,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_tabs,
                color: style.surface_overlay,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_toolbar,
                color: style.surface_base,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_table_header,
                color: style.surface_overlay,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_footer,
                color: style.surface_overlay,
            }),
        );

        let waveform_inner = layout.waveform_plot;
        if build_waveform_overlay {
            let scan_step = sizing.waveform_scan_step;
            let mut x = waveform_inner.min.x;
            while x < waveform_inner.max.x {
                let strong = ((x - waveform_inner.min.x) / scan_step).floor() as i32 % 4 == 0;
                let line_color = if strong {
                    style.grid_strong
                } else {
                    style.grid_soft
                };
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(x, waveform_inner.min.y),
                            Point::new(
                                (x + sizing.border_width).min(waveform_inner.max.x),
                                waveform_inner.max.y,
                            ),
                        ),
                        color: line_color,
                    }),
                );
                x += scan_step;
            }
            push_waveform_image(
                primitives,
                waveform_inner,
                model.waveform.waveform_image.as_deref(),
            );
            let owned_motion_model;
            let motion_model = if let Some(motion_model) = motion_model {
                motion_model
            } else {
                owned_motion_model = NativeMotionModel::from_app_model(model);
                &owned_motion_model
            };
            let waveform_toolbar_buttons = waveform_toolbar_buttons(
                layout,
                style,
                &motion_model,
                self.waveform_bpm_input_active,
                self.waveform_bpm_input_display.as_deref(),
            );
            let waveform_toolbar_left = waveform_toolbar_left_edge(
                &waveform_toolbar_buttons,
                layout.waveform_header.max.x - sizing.text_inset_x,
            );
            push_waveform_header_overlay(
                primitives,
                text_runs,
                layout,
                style,
                &motion_model,
                Some(waveform_toolbar_left - sizing.action_button_gap),
            );
            render_waveform_toolbar_buttons(
                primitives,
                text_runs,
                style,
                sizing,
                &waveform_toolbar_buttons,
                self.hovered_waveform_toolbar_hint,
                self.waveform_toolbar_flash.map(|flash| flash.hint),
                motion_wave,
            );
        }

        let browser_buttons = browser_action_buttons(layout, style, model);
        let browser_column_chips = browser_column_chips(layout, style, model, &browser_buttons);
        let source_row_rects = if build_global_static {
            rendered_source_row_rects(layout, style, model)
        } else {
            Vec::new()
        };
        let folder_row_rects = if build_global_static {
            rendered_folder_row_rects(layout, style, model)
        } else {
            Vec::new()
        };
        let browser_rows = if build_browser_rows_or_map {
            rendered_browser_rows(layout, model, style)
        } else {
            Vec::new()
        };
        if build_browser_rows_or_map {
            if model.map.active && build_map_panel {
                let canvas = compute_browser_map_canvas_rect(layout.browser_rows, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: canvas,
                        color: blend_color(style.surface_base, style.bg_secondary, 0.24),
                    }),
                );
                push_border(
                    primitives,
                    canvas,
                    style.border_emphasis,
                    sizing.border_width,
                );
                for point in &model.map.points {
                    let center =
                        compute_browser_map_point_center(canvas, point.x_milli, point.y_milli);
                    let color = map_point_color(style, point);
                    let radius = if point.focused {
                        4.5
                    } else if point.selected {
                        3.8
                    } else {
                        2.6
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Circle(FillCircle {
                            center,
                            radius,
                            color,
                        }),
                    );
                }
            } else if !model.map.active && build_browser_rows_window {
                let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
                for row in browser_rows.iter() {
                    let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
                    let row_border_stroke = browser_row_border_stroke(layout);
                    let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
                    let row_columns = row_text_layout.columns;
                    let row_fill = if row.focused {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_strong,
                            focus_fill_emphasis,
                        )
                    } else if row.selected {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_soft,
                            style.state_selected_blend,
                        )
                    } else if row.visible_row % 2 == 0 {
                        blend_color(style.surface_base, style.bg_secondary, 0.20)
                    } else {
                        blend_color(style.surface_base, style.bg_secondary, 0.10)
                    };
                    let row_border = if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            motion_wave * style.state_focus_pulse_blend,
                        )
                    } else if row.selected {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            motion_wave * style.state_selected_blend,
                        )
                    } else {
                        style.border
                    };
                    let row_text_color = if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            motion_wave * focus_text_emphasis,
                        )
                    } else if row.selected {
                        style.accent_mint
                    } else {
                        style.text_primary
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: row.rect,
                            color: row_fill,
                        }),
                    );
                    for separator_x in [row_columns.index.max.x, row_columns.sample.max.x] {
                        emit_primitive(
                            primitives,
                            Primitive::Rect(FillRect {
                                rect: Rect::from_min_max(
                                    Point::new(separator_x, row.rect.min.y),
                                    Point::new(
                                        (separator_x + sizing.border_width).min(row.rect.max.x),
                                        row.rect.max.y,
                                    ),
                                ),
                                color: blend_color(style.border, style.grid_soft, 0.36),
                            }),
                        );
                    }
                    push_browser_row_border(
                        primitives,
                        row_border_rect,
                        row_border,
                        row_border_stroke,
                        Some(row.rect.max.y) == last_row_max_y,
                    );
                    let chip_rect = row_text_layout.bucket_chip;
                    let chip_color = match row.column {
                        0 => blend_color(style.accent_warning, style.bg_secondary, 0.54),
                        2 => blend_color(style.accent_mint, style.bg_secondary, 0.54),
                        _ => blend_color(style.text_muted, style.bg_secondary, 0.54),
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: chip_rect,
                            color: chip_color,
                        }),
                    );
                    push_border(primitives, chip_rect, style.border, sizing.border_width);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: row.visible_row.to_string(),
                            position: row_text_layout.index_label.min,
                            font_size: sizing.font_meta,
                            color: style.text_muted,
                            max_width: Some(row_text_layout.index_label.width().max(12.0)),
                            align: TextAlign::Right,
                        },
                    );
                    let mut label_position = row_text_layout.sample_label.min;
                    let mut label_max_width = row_text_layout.sample_label.width().max(20.0);
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
                    let rating_indicator_layout = browser_rating_indicator_layout(
                        row_text_layout.sample_label,
                        row.rating_level,
                        sizing,
                    );
                    let inline_tag_reserved_width =
                        browser_inline_tag_reserved_width(&row.bucket_label, sizing);
                    if let Some(indicators) = rating_indicator_layout {
                        label_max_width = (label_max_width
                            - browser_rating_indicator_reserved_width(row.rating_level, sizing)
                            - inline_tag_reserved_width)
                            .max(4.0);
                        for rect in indicators.rects.into_iter().take(indicators.count) {
                            emit_primitive(
                                primitives,
                                Primitive::Rect(FillRect {
                                    rect,
                                    color: browser_rating_indicator_color(style, row.rating_level),
                                }),
                            );
                            push_border(
                                primitives,
                                rect,
                                blend_color(style.border, style.text_primary, 0.28),
                                sizing.border_width,
                            );
                        }
                    } else {
                        label_max_width = (label_max_width - inline_tag_reserved_width).max(4.0);
                    }
                    emit_text(
                        text_runs,
                        TextRun {
                            text: row.label.clone(),
                            position: label_position,
                            font_size: sizing.font_body,
                            color: row_text_color,
                            max_width: Some(label_max_width),
                            align: TextAlign::Left,
                        },
                    );
                    if !row.bucket_label.is_empty() {
                        let rating_reserved_width =
                            browser_rating_indicator_reserved_width(row.rating_level, sizing);
                        let chip_rects = browser_inline_tag_chip_rects(
                            row_text_layout.sample_label,
                            &row.bucket_label,
                            rating_reserved_width,
                            sizing,
                        );
                        for (chip_rect, chip_label) in chip_rects
                            .into_iter()
                            .zip(browser_inline_tag_labels(&row.bucket_label))
                        {
                            let text_origin = browser_inline_tag_text_origin(chip_rect, sizing);
                            emit_primitive(
                                primitives,
                                Primitive::Rect(FillRect {
                                    rect: chip_rect,
                                    color: blend_color(
                                        style.surface_overlay,
                                        style.bg_tertiary,
                                        0.54,
                                    ),
                                }),
                            );
                            push_border(
                                primitives,
                                chip_rect,
                                blend_color(style.border_emphasis, style.text_muted, 0.18),
                                sizing.border_width,
                            );
                            emit_text(
                                text_runs,
                                TextRun {
                                    text: chip_label.to_owned(),
                                    position: text_origin,
                                    font_size: sizing.font_meta,
                                    color: style.text_primary,
                                    max_width: Some((chip_rect.max.x - text_origin.x).max(4.0)),
                                    align: TextAlign::Left,
                                },
                            );
                        }
                    }
                }
            }
        }

        push_border(
            primitives,
            layout.top_bar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.sidebar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.waveform_card,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_panel,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_table_header,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.status_bar,
            style.border,
            sizing.border_width,
        );

        if build_global_static {
            let lamp_radius = sizing.lamp_radius_base
                + (((self.pulse_phase.sin() + 1.0) * 0.5) * sizing.lamp_radius_amp);
            let lamp_color = if self.transport_running {
                style.accent_mint
            } else {
                style.accent_copper
            };
            emit_primitive(
                primitives,
                Primitive::Circle(FillCircle {
                    center: Point::new(
                        layout.top_bar.max.x - (sizing.text_inset_x + 14.0),
                        layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
                    ),
                    radius: lamp_radius,
                    color: lamp_color,
                }),
            );

            let _top_title_rect = compute_top_bar_title_text_rect(
                layout.top_bar_title_cluster,
                layout.top_bar_title_row,
                sizing,
            );
            let top_controls = top_bar_controls_layout(layout, sizing);
            if top_controls.active {
                let top_controls_text = compute_top_bar_controls_text_layout(
                    top_controls.options_label,
                    top_controls.volume_value,
                    top_controls.volume_label,
                    sizing,
                );
                let divider_y = layout.top_bar_controls_row.min.y;
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(layout.top_bar.min.x, divider_y),
                            Point::new(
                                layout.top_bar.max.x,
                                (divider_y + sizing.border_width).min(layout.top_bar.max.y),
                            ),
                        ),
                        color: style.border,
                    }),
                );
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: top_controls.volume_meter,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    top_controls.volume_meter,
                    style.border_emphasis,
                    sizing.border_width,
                );
                let volume_level = model.volume.clamp(0.0, 1.0);
                let fill_width = (top_controls.volume_meter.width() * volume_level)
                    .clamp(1.0, top_controls.volume_meter.width());
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            top_controls.volume_meter.min,
                            Point::new(
                                top_controls.volume_meter.min.x + fill_width,
                                top_controls.volume_meter.max.y,
                            ),
                        ),
                        color: blend_color(style.accent_mint, style.text_primary, 0.28),
                    }),
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from("Options"),
                        position: top_controls_text.options_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_primary,
                        max_width: Some(top_controls_text.options_label.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("{volume_level:.2}"),
                        position: top_controls_text.volume_value.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(top_controls_text.volume_value.width().max(20.0)),
                        align: TextAlign::Right,
                    },
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from("Vol"),
                        position: top_controls_text.volume_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(top_controls_text.volume_label.width().max(18.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            let update_buttons = update_action_buttons(layout, style, model);
            let update_status_text = update_status_text(model);
            let update_hint_text = update_hint_text(model);
            let update_notes_text = update_notes_text(model);
            let update_controls_text = if update_notes_text.is_empty() {
                update_hint_text
            } else if update_hint_text.is_empty() {
                update_notes_text
            } else {
                format!("{update_hint_text} | {update_notes_text}")
            };
            let update_button_rects: Vec<Rect> =
                update_buttons.iter().map(|button| button.rect).collect();
            let update_text_layout = compute_top_bar_update_text_layout(
                layout.top_bar_action_cluster,
                layout.top_bar_title_row,
                layout.top_bar_controls_row,
                sizing,
                &update_button_rects,
            );
            let update_status_width = update_text_layout.status_line.width().max(20.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &update_status_text,
                        update_status_width,
                        sizing.font_meta,
                    ),
                    position: update_text_layout.status_line.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(update_status_width),
                    align: TextAlign::Left,
                },
            );
            if !update_controls_text.is_empty() && update_text_layout.controls_line.width() > 0.0 {
                let controls_width = update_text_layout.controls_line.width().max(20.0);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &update_controls_text,
                            controls_width,
                            sizing.font_meta,
                        ),
                        position: update_text_layout.controls_line.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(controls_width),
                        align: TextAlign::Left,
                    },
                );
            }
            for button in &update_buttons {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
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
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if build_browser_frame {
            for button in &browser_buttons {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
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
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if build_global_static {
            let sources_header = if model.sources.header.is_empty() {
                model.sources_label.as_str()
            } else {
                model.sources.header.as_str()
            };
            let sidebar_sections = sidebar_sections(layout, style, model);
            let sidebar_header_text =
                compute_sidebar_header_text_layout(layout.sidebar_header, sizing);
            let source_add_button = source_add_button_rect(layout.sidebar_header, sizing);
            let sidebar_header_title_width = source_add_button
                .map(|button| {
                    (button.min.x - sidebar_header_text.title_row.min.x - sizing.text_inset_x)
                        .max(24.0)
                })
                .unwrap_or_else(|| sidebar_header_text.title_row.width().max(72.0));
            let sidebar_header_query_width = source_add_button
                .map(|button| {
                    (button.min.x - sidebar_header_text.query_row.min.x - sizing.text_inset_x)
                        .max(24.0)
                })
                .unwrap_or_else(|| sidebar_header_text.query_row.width().max(72.0));
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        sources_header,
                        sidebar_header_title_width,
                        sizing.font_header,
                    ),
                    position: sidebar_header_text.title_row.min,
                    font_size: sizing.font_header,
                    color: style.text_primary,
                    max_width: Some(sidebar_header_title_width),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: format!(
                        "search: {}",
                        if model.sources.search_query.is_empty() {
                            "—"
                        } else {
                            model.sources.search_query.as_str()
                        }
                    ),
                    position: sidebar_header_text.query_row.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(sidebar_header_query_width),
                    align: TextAlign::Left,
                },
            );
            if let Some(button_rect) = source_add_button {
                let label_rect = compute_action_button_text_rect(button_rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button_rect,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    button_rect,
                    blend_color(
                        style.border_emphasis,
                        style.text_primary,
                        style.state_hover_soft,
                    ),
                    sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from("+"),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: style.accent_mint,
                        max_width: Some(label_rect.width().max(8.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            let rendered_sources = source_row_rects.len();
            for (row_index, row_rect) in source_row_rects.iter().enumerate() {
                let row_rect = *row_rect;
                let row = &model.sources.rows[row_index];
                let row_selected = row.selected
                    || model
                        .sources
                        .selected_row
                        .is_some_and(|selected| selected == row_index);
                let row_fill = if row_selected {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_soft,
                        style.state_selected_blend,
                    )
                } else {
                    style.surface_base
                };
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: row_rect,
                        color: row_fill,
                    }),
                );
                push_border(
                    primitives,
                    row_rect,
                    if row_selected {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            motion_wave * style.state_selected_blend,
                        )
                    } else if row.missing {
                        style.accent_warning
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                let source_label_rect = compute_sidebar_source_row_text_rect(row_rect, sizing);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &row.label,
                            source_label_rect.width().max(24.0),
                            sizing.font_body,
                        ),
                        position: source_label_rect.min,
                        font_size: sizing.font_body,
                        color: if row_selected {
                            style.accent_mint
                        } else {
                            style.text_primary
                        },
                        max_width: Some(source_label_rect.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            let rendered_folders = folder_row_rects.len();
            if rendered_folders > 0 {
                if let Some(divider_rect) = compute_source_section_divider_rect(
                    sidebar_sections.source_rows,
                    sidebar_sections.folder_header,
                    sizing,
                ) {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: divider_rect,
                            color: style.source_section_divider,
                        }),
                    );
                }
                let folder_header_layout = compute_sidebar_folder_header_layout(
                    sidebar_sections.folder_header,
                    sizing,
                    model.sources.folder_recovery.in_progress,
                    model.sources.folder_recovery.entry_count,
                );
                if let Some(badge) = folder_header_layout.badge.as_ref() {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: badge.rect,
                            color: if badge.active {
                                style.source_recovery_badge_active
                            } else {
                                style.source_recovery_badge_idle
                            },
                        }),
                    );
                    push_border(
                        primitives,
                        badge.rect,
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        ),
                        sizing.border_width,
                    );
                    let badge_text_rect =
                        compute_sidebar_recovery_badge_text_rect(badge.rect, sizing);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: badge.label.clone(),
                            position: badge_text_rect.min,
                            font_size: sizing.font_meta,
                            color: style.text_primary,
                            max_width: Some(badge_text_rect.width().max(18.0)),
                            align: TextAlign::Center,
                        },
                    );
                }
                if folder_header_layout.title_row.width() > 8.0 {
                    emit_text(
                        text_runs,
                        TextRun {
                            text: format!("Folders ({})", model.sources.folder_rows.len()),
                            position: Point::new(
                                folder_header_layout.title_row.min.x,
                                folder_header_layout.title_row.min.y,
                            ),
                            font_size: sizing.font_header,
                            color: style.text_primary,
                            max_width: Some(folder_header_layout.title_row.width()),
                            align: TextAlign::Left,
                        },
                    );
                    if let Some(metadata_row) = folder_header_layout
                        .metadata_row
                        .filter(|row| row.width() > 24.0)
                    {
                        emit_text(
                            text_runs,
                            TextRun {
                                text: format!(
                                    "query: {}",
                                    if model.sources.folder_search_query.is_empty() {
                                        "—"
                                    } else {
                                        model.sources.folder_search_query.as_str()
                                    }
                                ),
                                position: metadata_row.min,
                                font_size: sizing.font_meta,
                                color: style.text_muted,
                                max_width: Some(metadata_row.width()),
                                align: TextAlign::Left,
                            },
                        );
                    }
                }
                for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
                    let row_rect = *row_rect;
                    let row = &model.sources.folder_rows[row_index];
                    let row_selected = row.selected || row.focused;
                    let row_fill = if row.focused {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_strong,
                            (style.state_hover_soft + (motion_wave * style.motion_focus_wave_amp))
                                .clamp(0.0, 1.0),
                        )
                    } else if row_selected {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_soft,
                            style.state_selected_blend,
                        )
                    } else {
                        style.surface_base
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: row_rect,
                            color: row_fill,
                        }),
                    );
                    push_border(
                        primitives,
                        row_rect,
                        if row.focused {
                            blend_color(
                                style.accent_warning,
                                style.text_primary,
                                motion_wave * style.state_focus_pulse_blend,
                            )
                        } else if row.selected {
                            blend_color(
                                style.accent_mint,
                                style.text_primary,
                                motion_wave * style.state_selected_blend,
                            )
                        } else {
                            style.border
                        },
                        if row.focused {
                            sizing.focus_stroke_width
                        } else {
                            sizing.border_width
                        },
                    );
                    let glyph = if row.is_root {
                        "•"
                    } else if row.has_children {
                        if row.expanded { "▼" } else { "▶" }
                    } else {
                        "·"
                    };
                    let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                        .min((row_rect.width() * 0.45).max(0.0));
                    let label_text = format!("{glyph} {}", row.label);
                    let folder_text_rect =
                        compute_sidebar_folder_row_text_rect(row_rect, sizing, depth_indent);
                    let folder_text_width = folder_text_rect.width().max(24.0);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: truncate_to_width(
                                &label_text,
                                folder_text_width,
                                sizing.font_body,
                            ),
                            position: folder_text_rect.min,
                            font_size: sizing.font_body,
                            color: if row.focused {
                                style.accent_warning
                            } else if row.selected {
                                style.accent_mint
                            } else {
                                style.text_primary
                            },
                            max_width: Some(folder_text_width),
                            align: TextAlign::Left,
                        },
                    );
                }
            }
            for button in source_action_buttons(layout, style, model) {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
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
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            let sidebar_footer_text =
                compute_sidebar_footer_text_layout(layout.sidebar_footer, sizing);
            let sidebar_footer_primary_width = sidebar_footer_text.primary_row.width().max(56.0);
            let sidebar_footer_secondary_width =
                sidebar_footer_text.secondary_row.width().max(56.0);
            if model.sources.rows.len() > rendered_sources {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("+{} more…", model.sources.rows.len() - rendered_sources),
                        position: sidebar_footer_text.primary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_primary_width),
                        align: TextAlign::Left,
                    },
                );
            }
            if model.sources.folder_rows.len() > rendered_folders {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!(
                            "folders: +{} more…",
                            model.sources.folder_rows.len() - rendered_folders
                        ),
                        position: sidebar_footer_text.secondary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_secondary_width),
                        align: TextAlign::Left,
                    },
                );
            } else if model.sources.folder_recovery.entry_count > 0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!(
                            "recovery entries: {}",
                            model.sources.folder_recovery.entry_count
                        ),
                        position: sidebar_footer_text.secondary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_secondary_width),
                        align: TextAlign::Left,
                    },
                );
            }
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        if build_browser_frame {
            let tabs = compute_browser_tabs_rects(layout.browser_tabs, sizing);
            let map_active = model.map.active;
            let list_active = !map_active;
            let (
                samples_fill,
                map_fill,
                samples_border,
                map_border,
                samples_text_color,
                map_text_color,
            ) = if list_active {
                (
                    blend_color(
                        style.surface_overlay,
                        style.bg_tertiary,
                        style.state_selected_blend + (motion_wave * 0.1),
                    ),
                    style.surface_base,
                    blend_color(style.accent_mint, style.text_primary, 0.42),
                    style.border,
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
                    ),
                    style.text_muted,
                )
            } else {
                (
                    style.surface_base,
                    blend_color(
                        style.surface_overlay,
                        style.bg_tertiary,
                        style.state_selected_blend + (motion_wave * 0.1),
                    ),
                    style.border,
                    blend_color(style.accent_mint, style.text_primary, 0.42),
                    style.text_muted,
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
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
            push_border(
                primitives,
                tabs.samples,
                samples_border,
                sizing.border_width,
            );
            push_border(primitives, tabs.map, map_border, sizing.border_width);
            let tabs_text_layout = compute_browser_tabs_text_layout(tabs.samples, tabs.map, sizing);
            let samples_text = format!(
                "{} ({})",
                model.browser_chrome.samples_tab_label,
                model.columns.get(1).map(|c| c.item_count).unwrap_or(0)
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
            let map_text = model.browser_chrome.map_tab_label.as_str();
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(map_text),
                    position: tabs_text_layout.map_label.min,
                    font_size: sizing.font_header,
                    color: map_text_color,
                    max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
                    align: TextAlign::Left,
                },
            );
            let toolbar = browser_toolbar_layout(layout, style, &browser_buttons);
            if toolbar.search_field.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.search_field,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    toolbar.search_field,
                    blend_color(style.border_emphasis, style.text_primary, 0.35),
                    sizing.border_width,
                );
            }
            if toolbar.activity_chip.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.activity_chip,
                        color: if model.browser.busy {
                            blend_color(style.accent_warning, style.bg_secondary, 0.45)
                        } else {
                            blend_color(style.accent_mint, style.bg_secondary, 0.40)
                        },
                    }),
                );
                push_border(
                    primitives,
                    toolbar.activity_chip,
                    style.border,
                    sizing.border_width,
                );
            }
            if toolbar.sort_chip.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.sort_chip,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    toolbar.sort_chip,
                    style.border,
                    sizing.border_width,
                );
            }
            for chip in &browser_column_chips {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: chip.rect,
                        color: if chip.selected {
                            match chip.column {
                                0 => blend_color(style.accent_warning, style.bg_secondary, 0.50),
                                2 => blend_color(style.accent_mint, style.bg_secondary, 0.50),
                                _ => blend_color(style.text_primary, style.bg_secondary, 0.42),
                            }
                        } else {
                            match chip.column {
                                0 => blend_color(style.accent_warning, style.bg_secondary, 0.34),
                                2 => blend_color(style.accent_mint, style.bg_secondary, 0.34),
                                _ => blend_color(style.text_muted, style.bg_secondary, 0.28),
                            }
                        },
                    }),
                );
                push_border(
                    primitives,
                    chip.rect,
                    if chip.selected {
                        blend_color(style.border_emphasis, style.text_primary, 0.55)
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                let label_rect = compute_action_button_text_rect(chip.rect, sizing);
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("{} ({})", chip.label, chip.item_count),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: if chip.selected {
                            style.text_primary
                        } else {
                            style.text_muted
                        },
                        max_width: Some(label_rect.width().max(16.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            let toolbar_text_layout = compute_browser_toolbar_text_layout(
                toolbar.search_field,
                toolbar.activity_chip,
                toolbar.sort_chip,
                sizing,
            );
            let search_text = if model.browser.search_query.is_empty() {
                model.browser_chrome.search_placeholder.clone()
            } else {
                model.browser.search_query.clone()
            };
            if toolbar.search_field.width() > 1.0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &search_text,
                            toolbar_text_layout.search_label.width().max(24.0),
                            sizing.font_meta,
                        ),
                        position: toolbar_text_layout.search_label.min,
                        font_size: sizing.font_meta,
                        color: if model.browser.search_query.is_empty() {
                            style.text_muted
                        } else {
                            style.text_primary
                        },
                        max_width: Some(toolbar_text_layout.search_label.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            if toolbar.activity_chip.width() > 1.0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: if model.browser.busy {
                            model.browser_chrome.activity_busy_label.clone()
                        } else {
                            model.browser_chrome.activity_ready_label.clone()
                        },
                        position: toolbar_text_layout.activity_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_primary,
                        max_width: Some(toolbar_text_layout.activity_label.width().max(20.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            if toolbar.sort_chip.width() > 1.0 {
                let sort_label = if model.browser_chrome.sort_order_label.is_empty() {
                    model.browser.sort_label.as_deref().unwrap_or("List order")
                } else {
                    model.browser_chrome.sort_order_label.as_str()
                };
                let sort_text = if model.browser_chrome.sort_prefix_label.is_empty() {
                    String::from(sort_label)
                } else {
                    format!("{}: {}", model.browser_chrome.sort_prefix_label, sort_label)
                };
                emit_text(
                    text_runs,
                    TextRun {
                        text: sort_text,
                        position: toolbar_text_layout.sort_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(toolbar_text_layout.sort_label.width().max(20.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if model.map.active && build_map_panel {
            let mode_label = match model.map.render_mode {
                crate::app::MapRenderModeModel::Heatmap => "heatmap",
                crate::app::MapRenderModeModel::Points => "points",
            };
            let legend_text = if model.map.legend_label.is_empty() {
                format!(
                    "{}: {mode_label}",
                    model.browser_chrome.similarity_toggle_label
                )
            } else {
                model.map.legend_label.clone()
            };
            let header_left_text =
                format!("{} | {}", model.browser_chrome.map_tab_label, legend_text);
            let selection_or_error = if let Some(error) = model.map.error.as_deref() {
                (error.to_string(), style.accent_warning)
            } else if !model.map.selection_label.is_empty() {
                (model.map.selection_label.clone(), style.text_muted)
            } else if !model.map.hover_label.is_empty() {
                (model.map.hover_label.clone(), style.text_muted)
            } else {
                (String::from("Selection: —"), style.text_muted)
            };
            let map_header_text_layout =
                compute_browser_map_header_text_layout(layout.browser_table_header, sizing);
            let left_max_width = map_header_text_layout.left_label.width().max(24.0);
            let right_max_width = map_header_text_layout.right_label.width().max(36.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(&header_left_text, left_max_width, sizing.font_meta),
                    position: map_header_text_layout.left_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(left_max_width),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &selection_or_error.0,
                        right_max_width,
                        sizing.font_meta,
                    ),
                    position: map_header_text_layout.right_label.min,
                    font_size: sizing.font_meta,
                    color: selection_or_error.1,
                    max_width: Some(right_max_width),
                    align: TextAlign::Right,
                },
            );
        } else if build_browser_frame {
            let header_text_layout =
                compute_browser_header_text_layout(layout.browser_table_header, sizing);
            let header = header_text_layout.columns;
            for separator_x in [header.index.max.x, header.sample.max.x] {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(separator_x, layout.browser_table_header.min.y),
                            Point::new(
                                (separator_x + sizing.border_width)
                                    .min(layout.browser_table_header.max.x),
                                layout.browser_table_header.max.y,
                            ),
                        ),
                        color: style.border,
                    }),
                );
            }
            emit_text(
                text_runs,
                TextRun {
                    text: String::from("#"),
                    position: header_text_layout.index_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(header_text_layout.index_label.width().max(12.0)),
                    align: TextAlign::Right,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: String::from("Sample"),
                    position: header_text_layout.sample_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(header_text_layout.sample_label.width().max(24.0)),
                    align: TextAlign::Left,
                },
            );
        }
        if build_browser_frame {
            let footer_text = if model.map.active {
                let mut parts = Vec::new();
                if !model.map.summary.is_empty() {
                    parts.push(model.map.summary.clone());
                }
                if !model.map.cluster_label.is_empty() {
                    parts.push(model.map.cluster_label.clone());
                }
                if !model.map.hover_label.is_empty() {
                    parts.push(model.map.hover_label.clone());
                }
                if !model.map.viewport_label.is_empty() {
                    parts.push(model.map.viewport_label.clone());
                }
                if parts.is_empty() {
                    model.map.summary.clone()
                } else {
                    parts.join(" | ")
                }
            } else {
                format!(
                    "{} | {} selected{}",
                    model.browser_chrome.item_count_label,
                    model.browser.selected_path_count,
                    if model.browser.busy {
                        " | filtering…"
                    } else {
                        ""
                    }
                )
            };
            let browser_footer_text =
                compute_browser_footer_text_rect(layout.browser_footer, sizing);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &footer_text,
                        browser_footer_text.width().max(36.0),
                        sizing.font_meta,
                    ),
                    position: browser_footer_text.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(browser_footer_text.width().max(36.0)),
                    align: TextAlign::Left,
                },
            );
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
            let status_right_text_rect = compute_status_text_line_rect(
                layout.status_right_segment,
                sizing,
                sizing.font_status,
            );
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
            render_progress_overlay(primitives, text_runs, layout, style, model);
            render_confirm_prompt(primitives, text_runs, layout, style, model);
            render_drag_overlay(primitives, text_runs, layout, style, model);
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

        if self.hovered == Some(ShellNodeKind::TopBar) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.top_bar,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }

        if self.hovered == Some(ShellNodeKind::Sidebar) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.sidebar,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }

        if self.hovered == Some(ShellNodeKind::WaveformCard) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.waveform_card,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }
        push_waveform_toolbar_hover_tooltip(primitives, text_runs, layout, style, model, self);
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
                        push_border(
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
                            blend_color(
                                style.accent_mint,
                                style.text_primary,
                                style.state_selected_blend,
                            )
                        },
                        row_border_stroke,
                        Some(row.rect.max.y) == last_row_max_y,
                    );
                    if row.focused {
                        let mut label_position = row_text_layout.sample_label.min;
                        let inline_tag_reserved_width =
                            browser_inline_tag_reserved_width(&row.bucket_label, sizing);
                        let mut label_max_width = (row_text_layout.sample_label.width()
                            - browser_rating_indicator_reserved_width(row.rating_level, sizing)
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
        render_progress_overlay(primitives, text_runs, layout, style, model);
        render_confirm_prompt(primitives, text_runs, layout, style, model);
        render_drag_overlay(primitives, text_runs, layout, style, model);

        frame.clear_color = style.clear_color;
    }
}

fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    include_bottom: bool,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
            color,
        }),
    );
    if include_bottom {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
                color,
            }),
        );
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
            color,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
            color,
        }),
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
        WaveformToolbarHoverHint::Mono => String::from("View waveform in mono"),
        WaveformToolbarHoverHint::Stereo => String::from("View waveform in split stereo"),
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
