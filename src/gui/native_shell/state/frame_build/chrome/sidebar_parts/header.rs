use super::*;

pub(super) fn render_sidebar_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sources_header = if ctx.model.sources.header.is_empty() {
        ctx.model.sources_label.as_str()
    } else {
        ctx.model.sources.header.as_str()
    };
    let header_text = compute_sidebar_header_text_layout(ctx.layout.sidebar_header, ctx.sizing);
    let add_button = source_add_button_rect(ctx.layout.sidebar_header, ctx.sizing);
    let title_width = add_button
        .map(|button| {
            (button.min.x - header_text.title_row.min.x - ctx.sizing.text_inset_x).max(24.0)
        })
        .unwrap_or_else(|| header_text.title_row.width().max(72.0));
    let query_width = add_button
        .map(|button| {
            (button.min.x - header_text.query_row.min.x - ctx.sizing.text_inset_x).max(24.0)
        })
        .unwrap_or_else(|| header_text.query_row.width().max(72.0));
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(sources_header, title_width, ctx.sizing.font_header),
            position: header_text.title_row.min,
            font_size: ctx.sizing.font_header,
            color: ctx.style.text_primary,
            max_width: Some(title_width),
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
            position: header_text.query_row.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(query_width),
            align: TextAlign::Left,
        },
    );
    render_source_add_button(ctx, primitives, text_runs, add_button);
}

fn render_source_add_button(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    button_rect: Option<Rect>,
) {
    let Some(button_rect) = button_rect else {
        return;
    };
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
