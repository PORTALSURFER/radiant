use super::*;

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

fn render_section_focus_surface(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    style: &StyleTokens,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: translucent_overlay_color(
                style.bg_tertiary,
                style.accent_warning,
                (style.state_focus_pulse_blend * 0.12).clamp(0.06, 0.16),
            ),
        }),
    );
    push_border(
        primitives,
        rect,
        blend_color(
            style.accent_warning,
            style.text_primary,
            style.state_focus_pulse_blend,
        ),
        style.sizing.focus_stroke_width,
    );
}

fn union_rect(first: Rect, second: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(first.min.x.min(second.min.x), first.min.y.min(second.min.y)),
        Point::new(first.max.x.max(second.max.x), first.max.y.max(second.max.y)),
    )
}

fn render_sidebar_section_focus_overlay(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    let sections = sidebar_sections(layout, style, model);
    match model.focus_context {
        crate::app::FocusContextModel::SourcesList => {
            render_section_focus_surface(primitives, sections.source_rows, style);
        }
        crate::app::FocusContextModel::SourceFolders => {
            render_section_focus_surface(
                primitives,
                union_rect(sections.folder_header, sections.folder_rows),
                style,
            );
        }
        _ => {}
    }
}

pub(super) fn render_waveform_focus_overlay(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    if matches!(model.focus_context, crate::app::FocusContextModel::Waveform) {
        render_section_focus_surface(primitives, layout.waveform_card, style);
    }
}

fn render_panel_focus_surface(
    rect: Rect,
    style: &StyleTokens,
    primitives: &mut impl PrimitiveSink,
) {
    render_section_focus_surface(primitives, rect, style);
}

pub(super) fn render_source_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    if matches!(
        model.focus_context,
        crate::app::FocusContextModel::SourcesList
    ) {
        render_sidebar_section_focus_overlay(layout, style, model, primitives);
    }
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

pub(super) fn render_folder_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    if matches!(
        model.focus_context,
        crate::app::FocusContextModel::SourceFolders
    ) {
        render_sidebar_section_focus_overlay(layout, style, model, primitives);
    }
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

pub(super) fn render_browser_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    if matches!(
        model.focus_context,
        crate::app::FocusContextModel::SampleBrowser
    ) {
        render_panel_focus_surface(layout.browser_panel, style, primitives);
    }
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
