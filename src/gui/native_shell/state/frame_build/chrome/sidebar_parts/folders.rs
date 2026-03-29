use super::*;

mod header;
mod rows;

pub(super) fn render_folder_section(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &SidebarFrameData,
) -> usize {
    if data.folder_rows.is_empty() {
        return 0;
    }
    let sections = sidebar_sections(ctx.layout, ctx.style, ctx.model);
    render_source_section_divider(ctx, primitives, sections);
    header::render_folder_header(ctx, primitives, text_runs, sections.folder_header);
    rows::render_folder_rows(ctx, primitives, text_runs, data);
    if let Some(scrollbar) = folder_scrollbar_layout(
        sections.folder_rows,
        &data.folder_rows,
        ctx.model.sources.folder_rows.len(),
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
    data.folder_rows.len()
}

fn render_source_section_divider(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    sections: SidebarSections,
) {
    let Some(divider_rect) = compute_source_section_divider_rect(
        sections.source_rows,
        sections.folder_header,
        ctx.sizing,
    ) else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: divider_rect,
            color: ctx.style.source_section_divider,
        }),
    );
}
