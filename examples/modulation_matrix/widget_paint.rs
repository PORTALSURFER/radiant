use radiant::prelude::*;

use super::{MatrixCell, widget::ModulationMatrixWidget};

#[path = "widget_paint/activity.rs"]
mod activity;
#[path = "widget_paint/cell.rs"]
mod cell;
#[path = "widget_paint/labels.rs"]
mod labels;
#[path = "widget_paint/overlay.rs"]
mod overlay;

pub(super) fn append_labels(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    labels::append_labels(widget, primitives, bounds, matrix, theme);
}

pub(super) fn append_cell(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    matrix_cell: MatrixCell,
    theme: &ThemeTokens,
) {
    cell::append_cell(widget, primitives, matrix, matrix_cell, theme);
}

pub(super) fn append_overlay_guides(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    overlay::append_overlay_guides(widget, primitives, matrix, theme);
}

pub(super) fn append_activity_pulses(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    activity::append_activity_pulses(widget, primitives, matrix, theme);
}
