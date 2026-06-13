mod chrome;
mod geometry;
mod label;
mod paint;
mod palette;
mod state;

mod fixtures {
    pub(super) use super::super::super::{
        DenseRowChromeParts, DenseRowLabelParts, DenseRowMarkerEdge, DenseRowMarkerParts,
        DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette, DenseRowVisualState,
        dense_row_drop_outline_from_style, dense_row_fill_color, dense_row_inset_rect,
        dense_row_label_font_size, dense_row_palette_from_style, dense_row_tree_guide_color,
        dense_row_vertical_marker_rect, push_dense_row_chrome, push_dense_row_fill,
        push_dense_row_inset_stroke, push_dense_row_label, push_dense_row_labeled_chrome,
        push_dense_row_vertical_marker,
    };
    pub(super) use crate::gui::types::{Point, Rect, Rgba8, Vector2};
    pub(super) use crate::runtime::{PaintPrimitive, PaintStrokeRect};
    pub(super) use crate::theme::ThemeTokens;
    pub(super) use crate::widgets::{WidgetProminence, WidgetStyle, WidgetTone};

    pub(super) const SELECTED: Rgba8 = Rgba8 {
        r: 1,
        g: 0,
        b: 0,
        a: 255,
    };
    pub(super) const HOVERED: Rgba8 = Rgba8 {
        r: 2,
        g: 0,
        b: 0,
        a: 255,
    };
    pub(super) const PRESSED: Rgba8 = Rgba8 {
        r: 3,
        g: 0,
        b: 0,
        a: 255,
    };
    pub(super) const ACTIVE: Rgba8 = Rgba8 {
        r: 4,
        g: 0,
        b: 0,
        a: 255,
    };
    pub(super) const CANDIDATE: Rgba8 = Rgba8 {
        r: 5,
        g: 0,
        b: 0,
        a: 255,
    };

    pub(super) fn palette() -> DenseRowPalette {
        DenseRowPalette::new()
            .selected(SELECTED)
            .hovered(HOVERED)
            .pressed(PRESSED)
            .active_target(ACTIVE)
            .candidate_hovered(CANDIDATE)
    }
}
