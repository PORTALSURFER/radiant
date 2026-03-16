use super::*;

pub(super) fn render_sidebar_footer(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    rendered_sources: usize,
    rendered_folders: usize,
) {
    render_source_action_buttons(ctx, primitives, text_runs);
    render_sidebar_footer_text(ctx, text_runs, rendered_sources, rendered_folders);
}

fn render_source_action_buttons(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
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
}

fn render_sidebar_footer_text(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    rendered_sources: usize,
    rendered_folders: usize,
) {
    let footer_text = compute_sidebar_footer_text_layout(ctx.layout.sidebar_footer, ctx.sizing);
    let primary_width = footer_text.primary_row.width().max(56.0);
    let secondary_width = footer_text.secondary_row.width().max(56.0);
    if ctx.model.sources.rows.len() > rendered_sources {
        emit_text(
            text_runs,
            TextRun {
                text: format!("+{} more…", ctx.model.sources.rows.len() - rendered_sources),
                position: footer_text.primary_row.min,
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
                position: footer_text.secondary_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(secondary_width),
                align: TextAlign::Left,
            },
        );
        return;
    }
    if ctx.model.sources.folder_recovery.entry_count > 0 {
        emit_text(
            text_runs,
            TextRun {
                text: format!(
                    "recovery entries: {}",
                    ctx.model.sources.folder_recovery.entry_count
                ),
                position: footer_text.secondary_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(secondary_width),
                align: TextAlign::Left,
            },
        );
    }
}
