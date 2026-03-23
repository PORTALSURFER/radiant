use super::*;
use crate::app::SourceRowModel;

pub(super) fn render_source_rows(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &SidebarFrameData,
) -> usize {
    for (row_index, row_rect) in data.source_row_rects.iter().copied().enumerate() {
        let row = &ctx.model.sources.rows[row_index];
        let row_selected = row.selected
            || ctx
                .model
                .sources
                .selected_row
                .is_some_and(|selected| selected == row_index);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row_rect,
                color: source_row_fill(ctx, row_selected),
            }),
        );
        push_border(
            primitives,
            row_rect,
            source_row_border(ctx, row, row_selected),
            ctx.sizing.border_width,
        );
        emit_source_row_label(ctx, text_runs, row_rect, row, row_selected);
    }
    data.source_row_rects.len()
}

fn source_row_fill(ctx: &StaticFrameCtx<'_>, row_selected: bool) -> Rgba8 {
    if row_selected {
        translucent_overlay_color(
            ctx.style.bg_tertiary,
            ctx.style.grid_soft,
            ctx.style.state_selected_blend,
        )
    } else {
        ctx.style.surface_base
    }
}

fn source_row_border(ctx: &StaticFrameCtx<'_>, row: &SourceRowModel, row_selected: bool) -> Rgba8 {
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
    }
}

fn emit_source_row_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    row_rect: Rect,
    row: &SourceRowModel,
    row_selected: bool,
) {
    let label_rect = compute_sidebar_source_row_text_rect(row_rect, ctx.sizing);
    let label_width = label_rect.width().max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&row.label, label_width, ctx.sizing.font_body),
            position: label_rect.min,
            font_size: ctx.sizing.font_body,
            color: if row_selected {
                ctx.style.accent_mint
            } else {
                ctx.style.text_primary
            },
            max_width: Some(label_width),
            align: TextAlign::Left,
        },
    );
}
