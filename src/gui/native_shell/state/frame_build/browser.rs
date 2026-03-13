use super::*;
use super::{BrowserFrameData, StaticFrameCtx};

pub(super) fn render_browser_rows_window(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    browser_rows: &[CachedBrowserRow],
) {
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows {
        let row_text_layout = compute_browser_row_text_layout(row.rect, ctx.sizing);
        let row_border_stroke = browser_row_border_stroke(ctx.layout);
        let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
        let row_columns = row_text_layout.columns;
        let base_fill = browser_row_stripe_fill(ctx.style, row.visible_row);
        let row_fill = if row.locked {
            locked_browser_row_fill(ctx.style, base_fill)
        } else {
            base_fill
        };
        let row_border = ctx.style.border;
        let row_text_color = ctx.style.text_primary;
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
                            (separator_x + ctx.sizing.border_width).min(row.rect.max.x),
                            row.rect.max.y,
                        ),
                    ),
                    color: blend_color(ctx.style.border, ctx.style.grid_soft, 0.36),
                }),
            );
        }
        push_browser_row_border(
            primitives,
            row_border_rect,
            row_border,
            row_border_stroke,
            BorderSides {
                top: true,
                bottom: Some(row.rect.max.y) == last_row_max_y,
                left: false,
                right: false,
            },
        );
        let chip_rect = row_text_layout.bucket_chip;
        let chip_color = match row.column {
            0 => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.54),
            2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.54),
            _ => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.54),
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip_rect,
                color: chip_color,
            }),
        );
        push_border(
            primitives,
            chip_rect,
            ctx.style.border,
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: row.visible_row.to_string(),
                position: row_text_layout.index_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(row_text_layout.index_label.width().max(12.0)),
                align: TextAlign::Right,
            },
        );
        let mut label_position = row_text_layout.sample_label.min;
        let mut label_max_width = row_text_layout.sample_label.width().max(20.0);
        if row.missing {
            let marker_advance =
                browser_missing_marker_advance(ctx.sizing.font_body).min(label_max_width.max(0.0));
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(BROWSER_MISSING_SAMPLE_MARKER),
                    position: label_position,
                    font_size: ctx.sizing.font_body,
                    color: BROWSER_MISSING_SAMPLE_MARKER_COLOR,
                    max_width: Some(marker_advance),
                    align: TextAlign::Left,
                },
            );
            label_position.x =
                (label_position.x + marker_advance).min(row_text_layout.sample_label.max.x);
            label_max_width = (row_text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        let inline_tag_reserved_width =
            browser_inline_tag_reserved_width(&row.bucket_label, ctx.sizing);
        let rating_reserved_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, ctx.sizing);
        let rating_indicator_layout = browser_rating_indicator_layout(
            BrowserRatingIndicatorAnchor {
                sample_label: row_text_layout.sample_label,
                label_origin_x: label_position.x,
                label_rendered_width: browser_approx_text_width(&row.label, ctx.sizing.font_body)
                    .min(label_max_width.max(0.0)),
                right_limit_x: row_text_layout.sample_label.max.x - inline_tag_reserved_width,
            },
            row.rating_level,
            row.locked,
            ctx.sizing,
        );
        if let Some(indicators) = rating_indicator_layout {
            label_max_width =
                (label_max_width - rating_reserved_width - inline_tag_reserved_width).max(4.0);
            for rect in indicators.rects.into_iter().take(indicators.count) {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect,
                        color: browser_rating_indicator_color(ctx.style, row.rating_level),
                    }),
                );
                push_border(
                    primitives,
                    rect,
                    blend_color(ctx.style.border, ctx.style.text_primary, 0.28),
                    ctx.sizing.border_width,
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
                font_size: ctx.sizing.font_body,
                color: row_text_color,
                max_width: Some(label_max_width),
                align: TextAlign::Left,
            },
        );
        if !row.bucket_label.is_empty() {
            let chip_rects = browser_inline_tag_chip_rects(
                row_text_layout.sample_label,
                &row.bucket_label,
                0.0,
                ctx.sizing,
            );
            for (chip_rect, chip_label) in chip_rects
                .into_iter()
                .zip(browser_inline_tag_labels(&row.bucket_label))
            {
                let text_origin = browser_inline_tag_text_origin(chip_rect, ctx.sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: chip_rect,
                        color: blend_color(ctx.style.surface_overlay, ctx.style.bg_tertiary, 0.54),
                    }),
                );
                push_border(
                    primitives,
                    chip_rect,
                    blend_color(ctx.style.border_emphasis, ctx.style.text_muted, 0.18),
                    ctx.sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: chip_label.to_owned(),
                        position: text_origin,
                        font_size: ctx.sizing.font_meta,
                        color: ctx.style.text_primary,
                        max_width: Some((chip_rect.max.x - text_origin.x).max(4.0)),
                        align: TextAlign::Left,
                    },
                );
            }
        }
    }
    if let Some(scrollbar) = browser_scrollbar_layout(
        ctx.layout.browser_rows,
        browser_rows,
        ctx.model.browser.visible_count,
        ctx.sizing,
    ) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.track,
                color: blend_color(ctx.style.border, ctx.style.bg_secondary, 0.22),
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.thumb,
                color: blend_color(ctx.style.text_muted, ctx.style.text_primary, 0.32),
            }),
        );
    }
}

pub(super) fn render_browser_frame(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &BrowserFrameData,
) {
    for button in &data.buttons {
        let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if !button.enabled {
                    ctx.style.control_disabled_fill
                } else if button.active {
                    blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.26)
                } else {
                    ctx.style.surface_overlay
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if !button.enabled {
                ctx.style.border
            } else if button.active {
                blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32)
            } else {
                blend_color(
                    ctx.style.border_emphasis,
                    ctx.style.text_primary,
                    ctx.style.state_hover_soft,
                )
            },
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    ctx.style.text_muted
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }

    render_browser_tabs(primitives, text_runs, ctx, true);

    let toolbar = browser_toolbar_layout(ctx.layout, ctx.style, &data.buttons);
    for (index, rect) in toolbar.rating_filter_chips.iter().copied().enumerate() {
        if rect.width() <= 1.0 {
            continue;
        }
        let level = BROWSER_RATING_FILTER_LEVELS[index];
        let active = ctx.model.browser.active_rating_filters[index];
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: browser_rating_filter_chip_fill(ctx.style, level, active),
            }),
        );
        push_border(
            primitives,
            rect,
            browser_rating_filter_chip_border(ctx.style, level, active),
            ctx.sizing.border_width,
        );
    }
    if toolbar.search_field.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.search_field,
                color: ctx.style.surface_base,
            }),
        );
        push_border(
            primitives,
            toolbar.search_field,
            blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.35),
            ctx.sizing.border_width,
        );
    }
    if toolbar.activity_chip.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.activity_chip,
                color: if ctx.model.browser.busy {
                    blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.45)
                } else {
                    blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.40)
                },
            }),
        );
        push_border(
            primitives,
            toolbar.activity_chip,
            ctx.style.border,
            ctx.sizing.border_width,
        );
    }
    if toolbar.sort_chip.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.sort_chip,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            toolbar.sort_chip,
            ctx.style.border,
            ctx.sizing.border_width,
        );
    }
    for chip in &data.column_chips {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip.rect,
                color: if chip.selected {
                    match chip.column {
                        0 => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.50),
                        2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.50),
                        _ => blend_color(ctx.style.text_primary, ctx.style.bg_secondary, 0.42),
                    }
                } else {
                    match chip.column {
                        0 => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.34),
                        2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.34),
                        _ => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.28),
                    }
                },
            }),
        );
        push_border(
            primitives,
            chip.rect,
            if chip.selected {
                blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.55)
            } else {
                ctx.style.border
            },
            ctx.sizing.border_width,
        );
        let label_rect = compute_action_button_text_rect(chip.rect, ctx.sizing);
        emit_text(
            text_runs,
            TextRun {
                text: format!("{} ({})", chip.label, chip.item_count),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: if chip.selected {
                    ctx.style.text_primary
                } else {
                    ctx.style.text_muted
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
        ctx.sizing,
    );
    let search_editor_active = state.browser_search_editor_visual.is_some();
    let search_text = if ctx.model.browser.search_query.is_empty() {
        ctx.model.browser_chrome.search_placeholder.clone()
    } else {
        ctx.model.browser.search_query.clone()
    };
    if toolbar.search_field.width() > 1.0 && !search_editor_active {
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &search_text,
                    toolbar_text_layout.search_label.width().max(24.0),
                    ctx.sizing.font_meta,
                ),
                position: toolbar_text_layout.search_label.min,
                font_size: ctx.sizing.font_meta,
                color: if ctx.model.browser.search_query.is_empty() {
                    ctx.style.text_muted
                } else {
                    ctx.style.text_primary
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
                text: if ctx.model.browser.busy {
                    ctx.model.browser_chrome.activity_busy_label.clone()
                } else {
                    ctx.model.browser_chrome.activity_ready_label.clone()
                },
                position: toolbar_text_layout.activity_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(toolbar_text_layout.activity_label.width().max(20.0)),
                align: TextAlign::Center,
            },
        );
    }
    if toolbar.sort_chip.width() > 1.0 {
        let sort_label = if ctx.model.browser_chrome.sort_order_label.is_empty() {
            ctx.model
                .browser
                .sort_label
                .as_deref()
                .unwrap_or("List order")
        } else {
            ctx.model.browser_chrome.sort_order_label.as_str()
        };
        let sort_text = if ctx.model.browser_chrome.sort_prefix_label.is_empty() {
            String::from(sort_label)
        } else {
            format!(
                "{}: {}",
                ctx.model.browser_chrome.sort_prefix_label, sort_label
            )
        };
        emit_text(
            text_runs,
            TextRun {
                text: sort_text,
                position: toolbar_text_layout.sort_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(toolbar_text_layout.sort_label.width().max(20.0)),
                align: TextAlign::Center,
            },
        );
    }
}

pub(super) fn render_browser_table_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let header_text_layout =
        compute_browser_header_text_layout(ctx.layout.browser_table_header, ctx.sizing);
    let header = header_text_layout.columns;
    for separator_x in [header.index.max.x, header.sample.max.x] {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(separator_x, ctx.layout.browser_table_header.min.y),
                    Point::new(
                        (separator_x + ctx.sizing.border_width)
                            .min(ctx.layout.browser_table_header.max.x),
                        ctx.layout.browser_table_header.max.y,
                    ),
                ),
                color: ctx.style.border,
            }),
        );
    }
    emit_text(
        text_runs,
        TextRun {
            text: String::from("#"),
            position: header_text_layout.index_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(header_text_layout.index_label.width().max(12.0)),
            align: TextAlign::Right,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Sample"),
            position: header_text_layout.sample_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(header_text_layout.sample_label.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

pub(super) fn render_browser_footer(ctx: &StaticFrameCtx<'_>, text_runs: &mut impl TextRunSink) {
    let footer_text = if ctx.model.map.active {
        let mut parts = Vec::new();
        if !ctx.model.map.summary.is_empty() {
            parts.push(ctx.model.map.summary.clone());
        }
        if !ctx.model.map.cluster_label.is_empty() {
            parts.push(ctx.model.map.cluster_label.clone());
        }
        if !ctx.model.map.hover_label.is_empty() {
            parts.push(ctx.model.map.hover_label.clone());
        }
        if !ctx.model.map.viewport_label.is_empty() {
            parts.push(ctx.model.map.viewport_label.clone());
        }
        if parts.is_empty() {
            ctx.model.map.summary.clone()
        } else {
            parts.join(" | ")
        }
    } else {
        format!(
            "{} | {} selected{}",
            ctx.model.browser_chrome.item_count_label,
            ctx.model.browser.selected_path_count,
            if ctx.model.browser.busy {
                " | filtering…"
            } else {
                ""
            }
        )
    };
    let browser_footer_text =
        compute_browser_footer_text_rect(ctx.layout.browser_footer, ctx.sizing);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &footer_text,
                browser_footer_text.width().max(36.0),
                ctx.sizing.font_meta,
            ),
            position: browser_footer_text.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(browser_footer_text.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
}

pub(super) fn render_browser_tabs(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    animated: bool,
) {
    let tabs = compute_browser_tabs_rects(ctx.layout.browser_tabs, ctx.sizing);
    let wave = if animated { ctx.motion_wave * 0.1 } else { 0.0 };
    let map_active = ctx.model.map.active;
    let (samples_fill, map_fill, samples_border, map_border, samples_text_color, map_text_color) =
        if !map_active {
            (
                blend_color(
                    ctx.style.surface_overlay,
                    ctx.style.bg_tertiary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.surface_base,
                blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.42),
                ctx.style.border,
                blend_color(
                    ctx.style.accent_mint,
                    ctx.style.text_primary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.text_muted,
            )
        } else {
            (
                ctx.style.surface_base,
                blend_color(
                    ctx.style.surface_overlay,
                    ctx.style.bg_tertiary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.border,
                blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.42),
                ctx.style.text_muted,
                blend_color(
                    ctx.style.accent_mint,
                    ctx.style.text_primary,
                    ctx.style.state_selected_blend + wave,
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
        ctx.sizing.border_width,
    );
    push_border(primitives, tabs.map, map_border, ctx.sizing.border_width);
    let tabs_text_layout = compute_browser_tabs_text_layout(tabs.samples, tabs.map, ctx.sizing);
    let samples_text = format!(
        "{} ({})",
        ctx.model.browser_chrome.samples_tab_label,
        ctx.model.columns.get(1).map(|c| c.item_count).unwrap_or(0)
    );
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &samples_text,
                tabs_text_layout.samples_label.width().max(40.0),
                ctx.sizing.font_header,
            ),
            position: tabs_text_layout.samples_label.min,
            font_size: ctx.sizing.font_header,
            color: samples_text_color,
            max_width: Some(tabs_text_layout.samples_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from(ctx.model.browser_chrome.map_tab_label.as_str()),
            position: tabs_text_layout.map_label.min,
            font_size: ctx.sizing.font_header,
            color: map_text_color,
            max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
}
