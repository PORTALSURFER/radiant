use super::super::{
    DESTINATIONS, SOURCES,
    geometry::{destination_label_rect, source_label_rect},
    paint::push_text,
    widget::ModulationMatrixWidget,
};
use radiant::prelude::*;

pub(super) fn append_labels(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    append_source_labels(widget, primitives, bounds, matrix, theme);
    append_destination_labels(widget, primitives, matrix, theme);
}

fn append_source_labels(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    for (source, label) in SOURCES.iter().enumerate() {
        push_text(
            primitives,
            widget.common.id,
            *label,
            source_label_rect(bounds, matrix, source),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }
}

fn append_destination_labels(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    for (destination, label) in DESTINATIONS.iter().enumerate() {
        push_text(
            primitives,
            widget.common.id,
            *label,
            destination_label_rect(matrix, destination),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
}
