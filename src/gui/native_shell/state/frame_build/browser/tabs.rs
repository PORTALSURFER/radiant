use super::*;

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
