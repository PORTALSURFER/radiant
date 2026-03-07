//! Visual state and overlay rendering for active single-line shell text fields.

use super::*;

/// Precomputed visual state for one active shell text field.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextFieldVisualState {
    /// Visible text substring that fits the field width.
    pub(crate) text: String,
    /// Caret x-offset inside the field text rect.
    pub(crate) caret_offset: f32,
    /// Selected x-span inside the field text rect, when any.
    pub(crate) selection_offsets: Option<(f32, f32)>,
}

/// Render the active browser-search editor fill, selection, text, and caret.
pub(crate) fn render_active_browser_search_editor(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    search_field_rect: Rect,
    search_text_rect: Rect,
    visual: &TextFieldVisualState,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: search_field_rect,
            color: browser_search_field_active_fill(style),
        }),
    );
    push_border(
        primitives,
        search_field_rect,
        browser_search_field_active_border(style),
        sizing.border_width,
    );
    if let Some((start, end)) = visual.selection_offsets
        && end > start
    {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(search_text_rect.min.x + start, search_text_rect.min.y),
                    Point::new(search_text_rect.min.x + end, search_text_rect.max.y),
                ),
                color: browser_search_selection_fill(style),
            }),
        );
    }
    if !visual.text.is_empty() {
        emit_text(
            text_runs,
            TextRun {
                text: visual.text.clone(),
                position: search_text_rect.min,
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(search_text_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    let caret_rect = Rect::from_min_max(
        Point::new(
            search_text_rect.min.x + visual.caret_offset,
            search_text_rect.min.y,
        ),
        Point::new(
            search_text_rect.min.x + visual.caret_offset + sizing.border_width.max(1.0),
            search_text_rect.max.y,
        ),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: caret_rect,
            color: browser_search_caret_color(style),
        }),
    );
}

pub(crate) fn text_field_visual_signature(visual: Option<&TextFieldVisualState>) -> u64 {
    let Some(visual) = visual else {
        return 0;
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    visual.text.hash(&mut hasher);
    visual.caret_offset.to_bits().hash(&mut hasher);
    visual
        .selection_offsets
        .map(|(start, end)| (start.to_bits(), end.to_bits()))
        .hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn browser_search_field_active_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(style.surface_base, style.highlight_orange_soft, 0.22)
}

pub(crate) fn browser_search_field_active_border(style: &StyleTokens) -> Rgba8 {
    blend_color(style.border_emphasis, style.highlight_orange, 0.6)
}

fn browser_search_selection_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(style.highlight_orange_soft, style.text_primary, 0.22)
}

fn browser_search_caret_color(style: &StyleTokens) -> Rgba8 {
    blend_color(style.text_primary, style.highlight_orange, 0.24)
}
