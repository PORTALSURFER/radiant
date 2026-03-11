use super::*;
use super::{SidebarFrameData, StaticFrameCtx};

pub(super) fn render_static_shell_surfaces(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
) {
    for (rect, color) in [
        (ctx.layout.top_bar, ctx.style.surface_raised),
        (ctx.layout.sidebar, ctx.style.surface_raised),
        (ctx.layout.content, ctx.style.surface_base),
        (ctx.layout.waveform_card, ctx.style.surface_raised),
        (ctx.layout.status_bar, ctx.style.surface_raised),
        (ctx.layout.browser_panel, ctx.style.surface_raised),
        (ctx.layout.browser_tabs, ctx.style.surface_overlay),
        (ctx.layout.browser_toolbar, ctx.style.surface_overlay),
        (ctx.layout.browser_table_header, ctx.style.surface_overlay),
        (ctx.layout.browser_footer, ctx.style.surface_overlay),
    ] {
        emit_primitive(primitives, Primitive::Rect(FillRect { rect, color }));
    }
}

pub(super) fn render_shell_borders(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink) {
    push_border_sides(
        primitives,
        ctx.layout.top_bar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides::ALL,
    );
    push_border_sides(
        primitives,
        ctx.layout.sidebar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: false,
            left: true,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.waveform_card,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: true,
            left: false,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.browser_panel,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: false,
            left: false,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.browser_table_header,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: true,
            bottom: true,
            left: false,
            right: false,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.status_bar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: true,
            bottom: false,
            left: true,
            right: true,
        },
    );
}

pub(super) fn render_top_bar_controls(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let top_controls = top_bar_controls_layout(ctx.layout, ctx.sizing);
    if top_controls.active {
        let top_controls_text = compute_top_bar_controls_text_layout(
            top_controls.options_label,
            top_controls.volume_value,
            top_controls.volume_label,
            ctx.sizing,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: top_controls.volume_meter,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            top_controls.volume_meter,
            ctx.style.border_emphasis,
            ctx.sizing.border_width,
        );
        let volume_level = ctx.model.volume.clamp(0.0, 1.0);
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
                color: blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.28),
            }),
        );
        emit_text(
            text_runs,
            TextRun {
                text: format!("{volume_level:.2}"),
                position: top_controls_text.volume_value.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(top_controls_text.volume_value.width().max(20.0)),
                align: TextAlign::Right,
            },
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from("Vol"),
                position: top_controls_text.volume_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(top_controls_text.volume_label.width().max(18.0)),
                align: TextAlign::Left,
            },
        );
    }
    if let Some(button_rect) =
        status_options_button_rect(ctx.layout.top_bar_action_cluster, ctx.sizing)
    {
        render_status_options_button(
            primitives,
            ctx.style,
            ctx.sizing,
            button_rect,
            state.hovered_status_options_button,
            state.status_options_button_flash_ticks > 0,
            ctx.motion_wave,
        );
    }
}

pub(super) fn render_sidebar(
    _state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &SidebarFrameData,
) {
    let sources_header = if ctx.model.sources.header.is_empty() {
        ctx.model.sources_label.as_str()
    } else {
        ctx.model.sources.header.as_str()
    };
    let sidebar_sections = sidebar_sections(ctx.layout, ctx.style, ctx.model);
    let sidebar_header_text =
        compute_sidebar_header_text_layout(ctx.layout.sidebar_header, ctx.sizing);
    let source_add_button = source_add_button_rect(ctx.layout.sidebar_header, ctx.sizing);
    let sidebar_header_title_width = source_add_button
        .map(|button| {
            (button.min.x - sidebar_header_text.title_row.min.x - ctx.sizing.text_inset_x).max(24.0)
        })
        .unwrap_or_else(|| sidebar_header_text.title_row.width().max(72.0));
    let sidebar_header_query_width = source_add_button
        .map(|button| {
            (button.min.x - sidebar_header_text.query_row.min.x - ctx.sizing.text_inset_x).max(24.0)
        })
        .unwrap_or_else(|| sidebar_header_text.query_row.width().max(72.0));
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                sources_header,
                sidebar_header_title_width,
                ctx.sizing.font_header,
            ),
            position: sidebar_header_text.title_row.min,
            font_size: ctx.sizing.font_header,
            color: ctx.style.text_primary,
            max_width: Some(sidebar_header_title_width),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: format!(
                "search: {}",
                if ctx.model.sources.search_query.is_empty() {
                    "—"
                } else {
                    ctx.model.sources.search_query.as_str()
                }
            ),
            position: sidebar_header_text.query_row.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(sidebar_header_query_width),
            align: TextAlign::Left,
        },
    );
    if let Some(button_rect) = source_add_button {
        let label_rect = compute_action_button_text_rect(button_rect, ctx.sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button_rect,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            button_rect,
            blend_color(
                ctx.style.border_emphasis,
                ctx.style.text_primary,
                ctx.style.state_hover_soft,
            ),
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from("+"),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.accent_mint,
                max_width: Some(label_rect.width().max(8.0)),
                align: TextAlign::Center,
            },
        );
    }

    let rendered_sources = data.source_row_rects.len();
    for (row_index, row_rect) in data.source_row_rects.iter().copied().enumerate() {
        let row = &ctx.model.sources.rows[row_index];
        let row_selected = row.selected
            || ctx
                .model
                .sources
                .selected_row
                .is_some_and(|selected| selected == row_index);
        let row_fill = if row_selected {
            translucent_overlay_color(
                ctx.style.bg_tertiary,
                ctx.style.grid_soft,
                ctx.style.state_selected_blend,
            )
        } else {
            ctx.style.surface_base
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
                    ctx.style.accent_mint,
                    ctx.style.text_primary,
                    ctx.motion_wave * ctx.style.state_selected_blend,
                )
            } else if row.missing {
                ctx.style.accent_warning
            } else {
                ctx.style.border
            },
            ctx.sizing.border_width,
        );
        let source_label_rect = compute_sidebar_source_row_text_rect(row_rect, ctx.sizing);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &row.label,
                    source_label_rect.width().max(24.0),
                    ctx.sizing.font_body,
                ),
                position: source_label_rect.min,
                font_size: ctx.sizing.font_body,
                color: if row_selected {
                    ctx.style.accent_mint
                } else {
                    ctx.style.text_primary
                },
                max_width: Some(source_label_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }

    let rendered_folders = data.folder_row_rects.len();
    if rendered_folders > 0 {
        if let Some(divider_rect) = compute_source_section_divider_rect(
            sidebar_sections.source_rows,
            sidebar_sections.folder_header,
            ctx.sizing,
        ) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: divider_rect,
                    color: ctx.style.source_section_divider,
                }),
            );
        }
        let folder_header_layout = compute_sidebar_folder_header_layout(
            sidebar_sections.folder_header,
            ctx.sizing,
            ctx.model.sources.folder_recovery.in_progress,
            ctx.model.sources.folder_recovery.entry_count,
        );
        if let Some(badge) = folder_header_layout.badge.as_ref() {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: badge.rect,
                    color: if badge.active {
                        ctx.style.source_recovery_badge_active
                    } else {
                        ctx.style.source_recovery_badge_idle
                    },
                }),
            );
            push_border(
                primitives,
                badge.rect,
                blend_color(
                    ctx.style.border_emphasis,
                    ctx.style.text_primary,
                    ctx.style.state_hover_soft,
                ),
                ctx.sizing.border_width,
            );
            let badge_text_rect = compute_sidebar_recovery_badge_text_rect(badge.rect, ctx.sizing);
            emit_text(
                text_runs,
                TextRun {
                    text: badge.label.clone(),
                    position: badge_text_rect.min,
                    font_size: ctx.sizing.font_meta,
                    color: ctx.style.text_primary,
                    max_width: Some(badge_text_rect.width().max(18.0)),
                    align: TextAlign::Center,
                },
            );
        }
        if folder_header_layout.title_row.width() > 8.0 {
            emit_text(
                text_runs,
                TextRun {
                    text: format!("Folders ({})", ctx.model.sources.folder_rows.len()),
                    position: folder_header_layout.title_row.min,
                    font_size: ctx.sizing.font_header,
                    color: ctx.style.text_primary,
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
                            if ctx.model.sources.folder_search_query.is_empty() {
                                "—"
                            } else {
                                ctx.model.sources.folder_search_query.as_str()
                            }
                        ),
                        position: metadata_row.min,
                        font_size: ctx.sizing.font_meta,
                        color: ctx.style.text_muted,
                        max_width: Some(metadata_row.width()),
                        align: TextAlign::Left,
                    },
                );
            }
        }
        let last_folder_row_max_y = data.folder_row_rects.last().map(|rect| rect.max.y);
        for (row_index, row_rect) in data.folder_row_rects.iter().copied().enumerate() {
            let row = &ctx.model.sources.folder_rows[row_index];
            let row_selected = row.selected || row.focused;
            let row_fill = if row.focused {
                translucent_overlay_color(
                    ctx.style.bg_tertiary,
                    ctx.style.grid_strong,
                    (ctx.style.state_hover_soft
                        + (ctx.motion_wave * ctx.style.motion_focus_wave_amp))
                        .clamp(0.0, 1.0),
                )
            } else if row_selected {
                translucent_overlay_color(
                    ctx.style.bg_tertiary,
                    ctx.style.grid_soft,
                    ctx.style.state_selected_blend,
                )
            } else {
                ctx.style.surface_base
            };
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row_rect,
                    color: row_fill,
                }),
            );
            push_browser_row_border(
                primitives,
                row_rect,
                if row.focused {
                    blend_color(
                        ctx.style.accent_warning,
                        ctx.style.text_primary,
                        ctx.motion_wave * ctx.style.state_focus_pulse_blend,
                    )
                } else if row.selected {
                    blend_color(
                        ctx.style.accent_mint,
                        ctx.style.text_primary,
                        ctx.motion_wave * ctx.style.state_selected_blend,
                    )
                } else {
                    ctx.style.border
                },
                if row.focused {
                    ctx.sizing.focus_stroke_width
                } else {
                    ctx.sizing.border_width
                },
                BorderSides {
                    top: true,
                    bottom: row.focused || Some(row_rect.max.y) == last_folder_row_max_y,
                    left: row.focused,
                    right: row.focused,
                },
            );
            let glyph = if row.is_root {
                "•"
            } else if row.has_children {
                if row.expanded { "▼" } else { "▶" }
            } else {
                "·"
            };
            let depth_indent = (row.depth as f32 * ctx.sizing.folder_indent_step)
                .min((row_rect.width() * 0.45).max(0.0));
            let label_text = format!("{glyph} {}", row.label);
            let folder_text_rect =
                compute_sidebar_folder_row_text_rect(row_rect, ctx.sizing, depth_indent);
            let folder_text_width = folder_text_rect.width().max(24.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(&label_text, folder_text_width, ctx.sizing.font_body),
                    position: folder_text_rect.min,
                    font_size: ctx.sizing.font_body,
                    color: if row.focused {
                        ctx.style.accent_warning
                    } else if row.selected {
                        ctx.style.accent_mint
                    } else {
                        ctx.style.text_primary
                    },
                    max_width: Some(folder_text_width),
                    align: TextAlign::Left,
                },
            );
        }
    }

    for button in source_action_buttons(ctx.layout, ctx.style, ctx.model) {
        let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    ctx.style.surface_overlay
                } else {
                    ctx.style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled {
                blend_color(
                    ctx.style.border_emphasis,
                    ctx.style.text_primary,
                    ctx.style.state_hover_soft,
                )
            } else {
                ctx.style.border
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

    let sidebar_footer_text =
        compute_sidebar_footer_text_layout(ctx.layout.sidebar_footer, ctx.sizing);
    let primary_width = sidebar_footer_text.primary_row.width().max(56.0);
    let secondary_width = sidebar_footer_text.secondary_row.width().max(56.0);
    if ctx.model.sources.rows.len() > rendered_sources {
        emit_text(
            text_runs,
            TextRun {
                text: format!("+{} more…", ctx.model.sources.rows.len() - rendered_sources),
                position: sidebar_footer_text.primary_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(primary_width),
                align: TextAlign::Left,
            },
        );
    }
    if ctx.model.sources.folder_rows.len() > rendered_folders {
        emit_text(
            text_runs,
            TextRun {
                text: format!(
                    "folders: +{} more…",
                    ctx.model.sources.folder_rows.len() - rendered_folders
                ),
                position: sidebar_footer_text.secondary_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(secondary_width),
                align: TextAlign::Left,
            },
        );
    } else if ctx.model.sources.folder_recovery.entry_count > 0 {
        emit_text(
            text_runs,
            TextRun {
                text: format!(
                    "recovery entries: {}",
                    ctx.model.sources.folder_recovery.entry_count
                ),
                position: sidebar_footer_text.secondary_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(secondary_width),
                align: TextAlign::Left,
            },
        );
    }
}

pub(super) fn render_modal_overlays(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    render_options_panel(primitives, text_runs, layout, style, model);
    render_progress_overlay(primitives, text_runs, layout, style, model);
    render_confirm_prompt(primitives, text_runs, layout, style, model);
    render_drag_overlay(primitives, text_runs, layout, style, model);
}
