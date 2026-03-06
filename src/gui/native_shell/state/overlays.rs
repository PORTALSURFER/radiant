//! Overlay geometry helpers and overlay paint builders for the native shell state.

use super::*;

pub(super) fn progress_cancel_button(
    layout: &ShellLayout,
    style: &StyleTokens,
    modal: bool,
) -> Rect {
    compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        modal,
        0.0,
    )
    .sections
    .cancel_button
}

pub(super) fn prompt_buttons(layout: &ShellLayout, style: &StyleTokens) -> (Rect, Rect) {
    let sections = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        false,
        false,
    )
    .sections;
    (sections.confirm_button, sections.cancel_button)
}

pub(super) fn prompt_input_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<Rect> {
    if model.confirm_prompt.input_value.is_none() {
        return None;
    }
    compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        true,
        model.confirm_prompt.target_label.is_some(),
    )
    .sections
    .input
}

pub(super) fn drag_overlay_rect(layout: &ShellLayout, style: &StyleTokens) -> Rect {
    compute_drag_overlay_visual_layout(layout.content, layout.status_bar, style.sizing).banner
}

pub(super) fn render_progress_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.progress_overlay.visible {
        return;
    }
    let sizing = style.sizing;
    let fraction = if model.progress_overlay.total == 0 {
        0.0
    } else {
        (model.progress_overlay.completed as f32 / model.progress_overlay.total as f32)
            .clamp(0.0, 1.0)
    };
    let progress_visuals = compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        sizing,
        model.progress_overlay.modal,
        fraction,
    );
    if let Some(scrim_rect) = progress_visuals.scrim {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrim_rect,
                color: Rgba8 {
                    r: style.bg_primary.r,
                    g: style.bg_primary.g,
                    b: style.bg_primary.b,
                    a: style.scrim_soft_alpha,
                },
            }),
        );
    }
    let overlay_sections = progress_visuals.sections;
    let progress_text_layout = compute_progress_overlay_text_layout(
        overlay_sections,
        sizing,
        model.progress_overlay.detail.is_some(),
        model.progress_overlay.cancelable,
    );
    let rect = overlay_sections.dialog;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: style.surface_overlay,
        }),
    );
    push_border(primitives, rect, style.border, sizing.border_width);

    emit_text(
        text_runs,
        TextRun {
            text: model.progress_overlay.title.clone(),
            position: progress_text_layout.title.min,
            font_size: sizing.font_header,
            color: style.text_primary,
            max_width: Some(progress_text_layout.title.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    if let (Some(detail), Some(detail_rect)) = (
        model.progress_overlay.detail.as_deref(),
        progress_text_layout.detail,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: detail.to_string(),
                position: detail_rect.min,
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(detail_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    let bar_rect = overlay_sections.progress_bar;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: bar_rect,
            color: style.grid_soft,
        }),
    );
    if let Some(fill_rect) = progress_visuals.progress_fill {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: fill_rect,
                color: style.accent_mint,
            }),
        );
    }
    push_border(primitives, bar_rect, style.border, sizing.border_width);

    emit_text(
        text_runs,
        TextRun {
            text: format!(
                "{} / {}",
                model.progress_overlay.completed, model.progress_overlay.total
            ),
            position: progress_text_layout.counter.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(progress_text_layout.counter.width().max(24.0)),
            align: TextAlign::Right,
        },
    );

    if model.progress_overlay.cancelable {
        let button = overlay_sections.cancel_button;
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button,
                color: if model.progress_overlay.cancel_requested {
                    style.grid_soft
                } else {
                    style.bg_tertiary
                },
            }),
        );
        push_border(
            primitives,
            button,
            if model.progress_overlay.cancel_requested {
                style.border
            } else {
                style.accent_warning
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: if model.progress_overlay.cancel_requested {
                    String::from("Cancelling")
                } else {
                    String::from("Cancel")
                },
                position: progress_text_layout.cancel_label.min,
                font_size: sizing.font_meta,
                color: if model.progress_overlay.cancel_requested {
                    style.text_muted
                } else {
                    style.text_primary
                },
                max_width: Some(progress_text_layout.cancel_label.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}

pub(super) fn render_confirm_prompt(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.confirm_prompt.visible {
        return;
    }
    let sizing = style.sizing;
    let confirm_enabled = !prompt_has_validation_error(model);
    let has_target_label = model.confirm_prompt.target_label.is_some();
    let has_input = model.confirm_prompt.input_value.is_some();
    let prompt_visuals = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        sizing,
        has_input,
        has_target_label,
    );
    let prompt_sections = prompt_visuals.sections;
    let prompt_text_layout = compute_prompt_overlay_text_layout(
        prompt_sections,
        sizing,
        has_target_label,
        model.confirm_prompt.input_error.is_some(),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: prompt_visuals.scrim,
            color: Rgba8 {
                r: style.bg_primary.r,
                g: style.bg_primary.g,
                b: style.bg_primary.b,
                a: style.scrim_modal_alpha,
            },
        }),
    );
    let dialog = prompt_sections.dialog;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: dialog,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        dialog,
        style.accent_warning,
        sizing.border_width,
    );

    emit_text(
        text_runs,
        TextRun {
            text: model.confirm_prompt.title.clone(),
            position: prompt_text_layout.title.min,
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(prompt_text_layout.title.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: model.confirm_prompt.message.clone(),
            position: prompt_text_layout.message.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(prompt_text_layout.message.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    if let (Some(target), Some(target_rect)) = (
        model.confirm_prompt.target_label.as_deref(),
        prompt_text_layout.target,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: target.to_string(),
                position: target_rect.min,
                font_size: sizing.font_meta,
                color: style.accent_copper,
                max_width: Some(target_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    if let Some(input_rect) = prompt_sections.input {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: input_rect,
                color: style.surface_base,
            }),
        );
        push_border(
            primitives,
            input_rect,
            if model.confirm_prompt.input_error.is_some() {
                style.accent_warning
            } else {
                style.accent_copper
            },
            sizing.border_width,
        );
        let input_text = model
            .confirm_prompt
            .input_value
            .as_deref()
            .unwrap_or_default();
        let (text, color) = if input_text.is_empty() {
            (
                model
                    .confirm_prompt
                    .input_placeholder
                    .as_deref()
                    .unwrap_or("Type here…"),
                style.text_muted,
            )
        } else {
            (input_text, style.text_primary)
        };
        let input_text_rect = prompt_text_layout
            .input_text
            .unwrap_or(Rect::from_min_max(input_rect.min, input_rect.min));
        let input_text_width = prompt_text_layout
            .input_text
            .map(|line_rect: Rect| line_rect.width().max(24.0))
            .unwrap_or((input_rect.width() - (sizing.text_inset_x * 2.0)).max(24.0));
        emit_text(
            text_runs,
            TextRun {
                text: text.to_string(),
                position: input_text_rect.min,
                font_size: sizing.font_meta,
                color,
                max_width: Some(input_text_width),
                align: TextAlign::Left,
            },
        );
        if let (Some(error), Some(error_rect)) = (
            model.confirm_prompt.input_error.as_deref(),
            prompt_text_layout.input_error,
        ) {
            emit_text(
                text_runs,
                TextRun {
                    text: error.to_string(),
                    position: error_rect.min,
                    font_size: sizing.font_meta,
                    color: style.accent_warning,
                    max_width: Some(error_rect.width().max(24.0)),
                    align: TextAlign::Left,
                },
            );
        }
    }
    let confirm_button = prompt_sections.confirm_button;
    let cancel_button = prompt_sections.cancel_button;
    for (index, (rect, label, color)) in [
        (
            confirm_button,
            if model.confirm_prompt.confirm_label.is_empty() {
                "Confirm"
            } else {
                model.confirm_prompt.confirm_label.as_str()
            },
            style.accent_mint,
        ),
        (
            cancel_button,
            if model.confirm_prompt.cancel_label.is_empty() {
                "Cancel"
            } else {
                model.confirm_prompt.cancel_label.as_str()
            },
            style.text_muted,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let enabled = if index == 0 { confirm_enabled } else { true };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: if enabled {
                    style.surface_overlay
                } else {
                    style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            rect,
            if enabled { color } else { style.border },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: label.to_string(),
                position: if index == 0 {
                    prompt_text_layout.confirm_label.min
                } else {
                    prompt_text_layout.cancel_label.min
                },
                font_size: sizing.font_meta,
                color: if !enabled {
                    style.text_muted
                } else if index == 0 {
                    style.text_primary
                } else {
                    style.text_muted
                },
                max_width: Some(if index == 0 {
                    prompt_text_layout.confirm_label.width().max(12.0)
                } else {
                    prompt_text_layout.cancel_label.width().max(12.0)
                }),
                align: TextAlign::Center,
            },
        );
    }
}

pub(super) fn render_drag_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.drag_overlay.active {
        return;
    }
    let sizing = style.sizing;
    let rect = drag_overlay_rect(layout, style);
    let drag_text_layout = compute_drag_overlay_text_layout(rect, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        rect,
        if model.drag_overlay.valid_target {
            style.accent_mint
        } else {
            style.accent_warning
        },
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if model.drag_overlay.target_label.is_empty() {
                model.drag_overlay.label.clone()
            } else {
                format!(
                    "{} -> {}",
                    model.drag_overlay.label, model.drag_overlay.target_label
                )
            },
            position: drag_text_layout.label.min,
            font_size: sizing.font_meta,
            color: if model.drag_overlay.valid_target {
                style.text_primary
            } else {
                style.accent_warning
            },
            max_width: Some(drag_text_layout.label.width().max(24.0)),
            align: TextAlign::Center,
        },
    );
}

pub(super) fn style_for_layout(layout: &ShellLayout) -> StyleTokens {
    StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
}

pub(super) fn prompt_has_validation_error(model: &AppModel) -> bool {
    model
        .confirm_prompt
        .input_error
        .as_ref()
        .is_some_and(|error| !error.trim().is_empty())
}

pub(super) fn push_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
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
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
            color,
        }),
    );
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

pub(super) fn build_stacked_rows(
    column: Rect,
    rows: usize,
    gap: f32,
    row_height: f32,
) -> Vec<Rect> {
    if rows == 0 {
        return Vec::new();
    }
    let row_height = row_height.max(8.0).round().max(1.0);
    let gap = gap.max(0.0);
    let stride = row_height + gap;
    let column_min_y = column.min.y.round();
    let column_max_y = column.max.y.round().max(column_min_y);
    let mut output = Vec::with_capacity(rows);
    for index in 0..rows {
        let y = (column_min_y + (index as f32 * stride)).round();
        let max_y = (y + row_height).min(column_max_y);
        if max_y <= y {
            break;
        }
        output.push(Rect::from_min_max(
            Point::new(column.min.x, y),
            Point::new(column.max.x, max_y),
        ));
        if max_y >= column_max_y {
            break;
        }
    }
    output
}
