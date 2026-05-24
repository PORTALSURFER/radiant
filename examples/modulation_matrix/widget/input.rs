use radiant::prelude::*;

use super::super::{
    DESTINATION_COUNT, MatrixCell, MatrixMessage, SOURCE_COUNT,
    geometry::{amount_for_position, cell_at_position, cell_rect},
    paint::{push_rect, push_stroke},
    widget_paint::{append_activity_pulses, append_cell, append_labels, append_overlay_guides},
};
use super::ModulationMatrixWidget;

impl Widget for ModulationMatrixWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let matrix = self.matrix_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(bounds, matrix, position)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if matrix.contains(position) => self.handle_primary_press(matrix, position),
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => self.finish_drag(matrix, position),
            WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(MatrixMessage::ClearSelected))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        self.drag_cell.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_cell = previous.hover_cell;
            self.hover_position = previous.hover_position;
            self.drag_cell = previous.drag_cell;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let matrix = self.matrix_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        append_labels(self, primitives, bounds, matrix, theme);
        for source in 0..SOURCE_COUNT {
            for destination in 0..DESTINATION_COUNT {
                append_cell(
                    self,
                    primitives,
                    matrix,
                    MatrixCell {
                        source,
                        destination,
                    },
                    theme,
                );
            }
        }
        push_stroke(
            primitives,
            self.common.id,
            matrix,
            theme.border_emphasis,
            1.0,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let matrix = self.matrix_rect(bounds);
        append_overlay_guides(self, primitives, matrix, theme);
        append_activity_pulses(self, primitives, matrix, theme);
    }
}

impl ModulationMatrixWidget {
    fn handle_pointer_move(
        &mut self,
        bounds: Rect,
        matrix: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.hover_position = matrix.contains(position).then_some(position);
        self.hover_cell = cell_at_position(matrix, position);
        self.drag_cell
            .map(|cell| self.set_amount_output(matrix, cell, position))
    }

    fn handle_primary_press(&mut self, matrix: Rect, position: Point) -> Option<WidgetOutput> {
        let cell = cell_at_position(matrix, position)?;
        self.selected = cell;
        self.hover_cell = Some(cell);
        self.drag_cell = Some(cell);
        Some(self.set_amount_output(matrix, cell, position))
    }

    fn finish_drag(&mut self, matrix: Rect, position: Point) -> Option<WidgetOutput> {
        let drag = self.drag_cell.take();
        self.hover_cell = cell_at_position(matrix, position);
        drag.map(|cell| self.set_amount_output(matrix, cell, position))
    }

    fn set_amount_output(&self, matrix: Rect, cell: MatrixCell, position: Point) -> WidgetOutput {
        WidgetOutput::custom(MatrixMessage::SetAmount {
            cell,
            amount: amount_for_position(cell_rect(matrix, cell), position),
        })
    }
}
