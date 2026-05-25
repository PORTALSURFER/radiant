use crate::gui::types::{Point, Rect, Rgba8};

use super::{BorderSides, FillCircle, FillRect, Primitive, TextRun, border_fill_rects};

/// Full frame emitted by a retained render pipeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintFrame {
    /// Root clear color.
    pub clear_color: Rgba8,
    /// Shape primitives.
    pub primitives: Vec<Primitive>,
    /// Text primitives.
    pub text_runs: Vec<TextRun>,
}

impl PaintFrame {
    /// Push a filled rectangle primitive.
    pub fn push_rect(&mut self, rect: Rect, color: Rgba8) {
        self.primitives
            .push(Primitive::Rect(FillRect { rect, color }));
    }

    /// Push a filled circle primitive.
    pub fn push_circle(&mut self, center: Point, radius: f32, color: Rgba8) {
        self.primitives.push(Primitive::Circle(FillCircle {
            center,
            radius,
            color,
        }));
    }

    /// Push filled rectangle primitives for the requested border edges.
    pub fn push_border_rects(&mut self, rect: Rect, color: Rgba8, stroke: f32, sides: BorderSides) {
        self.primitives.extend(
            border_fill_rects(rect, color, stroke, sides)
                .into_iter()
                .map(Primitive::Rect),
        );
    }
}
