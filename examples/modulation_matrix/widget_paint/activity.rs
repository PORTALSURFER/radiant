use super::super::{
    DESTINATION_COUNT, MatrixCell, SOURCE_COUNT,
    geometry::cell_rect,
    paint::{push_rect, translucent},
    widget::ModulationMatrixWidget,
};
use radiant::prelude::*;

pub(super) fn append_activity_pulses(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    for source in 0..SOURCE_COUNT {
        for destination in 0..DESTINATION_COUNT {
            append_activity_pulse(widget, primitives, matrix, source, destination, theme);
        }
    }
}

fn append_activity_pulse(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    source: usize,
    destination: usize,
    theme: &ThemeTokens,
) {
    let amount = widget.amounts[source][destination];
    if amount.abs() < 0.20 {
        return;
    }
    let rect = cell_rect(
        matrix,
        MatrixCell {
            source,
            destination,
        },
    );
    let phase = (widget.activity_phase + source as f32 * 0.11 + destination as f32 * 0.07).fract();
    let x = rect.min.x + 8.0 + (rect.width() - 16.0) * phase;
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_size(
            Point::new(x, rect.min.y + 7.0),
            Vector2::new(4.0, rect.height() - 14.0),
        ),
        translucent(theme.text_primary, 70),
    );
}
