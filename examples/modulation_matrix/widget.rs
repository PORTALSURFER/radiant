use radiant::prelude::*;
use radiant::widgets::PaintBounds;

use super::{DESTINATION_COUNT, MatrixCell, SOURCE_COUNT};

#[path = "widget/input.rs"]
mod input;

#[derive(Clone, Debug)]
pub(crate) struct ModulationMatrixWidget {
    pub(super) common: WidgetCommon,
    pub(super) amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
    pub(super) selected: MatrixCell,
    pub(super) activity_phase: f32,
    pub(crate) hover_cell: Option<MatrixCell>,
    pub(super) hover_position: Option<Point>,
    pub(super) drag_cell: Option<MatrixCell>,
}

impl ModulationMatrixWidget {
    pub(crate) fn new(
        amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
        selected: MatrixCell,
        activity_phase: f32,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            amounts,
            selected: selected.clamped(),
            activity_phase,
            hover_cell: None,
            hover_position: None,
            drag_cell: None,
        }
    }

    pub(crate) fn matrix_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 112.0, bounds.min.y + 54.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 22.0),
        )
    }

    #[cfg(test)]
    pub(crate) fn cell_rect(&self, matrix: Rect, cell: MatrixCell) -> Rect {
        super::geometry::cell_rect(matrix, cell)
    }

    #[cfg(test)]
    pub(crate) fn amount_for_position(&self, rect: Rect, position: Point) -> f32 {
        super::geometry::amount_for_position(rect, position)
    }
}
