use super::*;

pub(super) fn render_browser_rows_window(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    browser_rows: &[CachedBrowserRow],
) {
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows {
        let row_border_stroke = browser_row_border_stroke(ctx.layout);
        let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
        let row_columns = row.text_layout.columns;
        let similarity_active =
            ctx.model.browser.similarity_filtered || ctx.model.browser.duplicate_cleanup_active;
        let similarity_button = (!ctx.model.browser.duplicate_cleanup_active)
            .then_some(row)
            .filter(|row| row.focused)
            .and_then(|row| browser_similarity_button_rect(row.rect, ctx.sizing));
        let similarity_button_reserved_width =
            browser_similarity_button_reserved_width(similarity_button.is_some(), ctx.sizing);
        let similarity_strength_reserved_width = browser_similarity_strength_reserved_width(
            row.similarity_display_strength.is_some(),
            ctx.sizing,
        );
        let base_fill = if row.marked && similarity_active {
            browser_marked_similarity_row_fill(ctx.style, row.visible_row, row.visible_row == 0)
        } else if row.marked {
            browser_marked_row_fill(ctx.style, row.visible_row)
        } else if similarity_active {
            browser_similarity_row_fill(ctx.style, row.visible_row, row.visible_row == 0)
        } else {
            browser_row_stripe_fill(ctx.style, row.visible_row)
        };
        let age_marker_reserved_width = browser_playback_age_marker_reserved_width(
            row.rect,
            ctx.sizing,
            similarity_button_reserved_width,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row.rect,
                color: base_fill,
            }),
        );
        if let Some(marker_rect) =
            browser_playback_age_marker_rect(row.rect, ctx.sizing, similarity_button_reserved_width)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: marker_rect,
                    color: browser_playback_age_marker_color(ctx.style, row.playback_age_bucket),
                }),
            );
        }
        let marked_marker_width = if row.marked { 4.0 } else { 0.0 };
        if row.marked {
            if let Some(marker_rect) = browser_locked_marker_rect(row.rect, ctx.sizing, 0.0) {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: marker_rect,
                        color: ctx.style.highlight_cyan,
                    }),
                );
            }
        }
        if row.locked {
            if let Some(marker_rect) =
                browser_locked_marker_rect(row.rect, ctx.sizing, marked_marker_width)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: marker_rect,
                        color: ctx.style.accent_mint,
                    }),
                );
            }
        }
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
            ctx.style.border,
            row_border_stroke,
            BorderSides {
                top: true,
                bottom: Some(row.rect.max.y) == last_row_max_y,
                left: false,
                right: false,
            },
        );
        let chip_rect = row.text_layout.bucket_chip;
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
                text: row.visible_row_label.clone(),
                position: row.text_layout.index_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(row.text_layout.index_label.width().max(12.0)),
                align: TextAlign::Right,
            },
        );
        let mut label_position = row.text_layout.sample_label.min;
        let mut label_max_width = row.text_layout.sample_label.width().max(20.0);
        if similarity_button_reserved_width > 0.0 {
            label_position.x = (label_position.x + similarity_button_reserved_width)
                .min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        if age_marker_reserved_width > 0.0 {
            label_position.x = (label_position.x + age_marker_reserved_width)
                .min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        if row.missing {
            let marker_advance =
                browser_missing_marker_advance(ctx.sizing.font_body).min(label_max_width.max(0.0));
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(BROWSER_MISSING_SAMPLE_MARKER),
                    position: label_position,
                    font_size: ctx.sizing.font_body,
                    color: ctx.style.accent_trash,
                    max_width: Some(marker_advance),
                    align: TextAlign::Left,
                },
            );
            label_position.x =
                (label_position.x + marker_advance).min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        let inline_tag_reserved_width =
            browser_inline_tag_reserved_width_for_labels(&row.inline_tag_labels, ctx.sizing);
        let rating_reserved_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, ctx.sizing);
        let rating_indicator_layout = browser_rating_indicator_layout(
            BrowserRatingIndicatorAnchor {
                sample_label: row.text_layout.sample_label,
                label_origin_x: label_position.x,
                label_rendered_width: row.label_rendered_width.min(label_max_width.max(0.0)),
                right_limit_x: row.text_layout.sample_label.max.x
                    - inline_tag_reserved_width
                    - similarity_strength_reserved_width,
            },
            row.rating_level,
            row.locked,
            ctx.sizing,
        );
        if let Some(indicators) = rating_indicator_layout {
            label_max_width = (label_max_width
                - rating_reserved_width
                - inline_tag_reserved_width
                - similarity_strength_reserved_width)
                .max(4.0);
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
            label_max_width =
                (label_max_width - inline_tag_reserved_width - similarity_strength_reserved_width)
                    .max(4.0);
        }
        emit_text(
            text_runs,
            TextRun {
                text: row.label.clone(),
                position: label_position,
                font_size: ctx.sizing.font_body,
                color: ctx.style.text_primary,
                max_width: Some(label_max_width),
                align: TextAlign::Left,
            },
        );
        if !row.bucket_label.is_empty() {
            for (chip_rect, chip_label) in row
                .inline_tag_rects
                .iter()
                .copied()
                .zip(row.inline_tag_labels.iter())
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
                        text: chip_label.clone(),
                        position: text_origin,
                        font_size: ctx.sizing.font_meta,
                        color: ctx.style.text_primary,
                        max_width: Some((chip_rect.max.x - text_origin.x).max(4.0)),
                        align: TextAlign::Left,
                    },
                );
            }
        }
        if let Some(strength) = row.similarity_display_strength {
            if let Some(track_rect) =
                browser_similarity_strength_track_rect(row.text_layout.sample_label, ctx.sizing)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: track_rect,
                        color: translucent_overlay_color(
                            ctx.style.surface_overlay,
                            ctx.style.text_muted,
                            0.12,
                        ),
                    }),
                );
                if let Some(fill_rect) = browser_similarity_strength_fill_rect(track_rect, strength)
                {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: fill_rect,
                            color: blend_color(
                                ctx.style.highlight_cyan_soft,
                                ctx.style.highlight_cyan,
                                0.38,
                            ),
                        }),
                    );
                }
            }
        }
        if let Some(button_rect) = similarity_button {
            let button_active = similarity_active && row.visible_row == 0;
            render_browser_similarity_button(
                primitives,
                button_rect,
                ctx.style,
                ctx.sizing,
                button_active,
                if button_active {
                    ctx.style.text_primary
                } else {
                    ctx.style.text_muted
                },
            );
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
