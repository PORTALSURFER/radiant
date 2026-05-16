//! Backend-neutral paint primitives and frame payloads for renderer adapters.

mod frame;
mod image;
mod primitive;
mod shapes;
mod svg;
mod text;
mod text_field;

pub use frame::PaintFrame;
pub use image::DrawImage;
pub use primitive::Primitive;
pub use shapes::{
    BorderSides, FillCircle, FillLinearGradient, FillRect, border_fill_rects,
    push_border_fill_rects,
};
pub use svg::DrawSvg;
pub use text::{TextAlign, TextRun};
pub use text_field::{TextFieldPaint, TextFieldPaintOutput, text_field_paint};

#[cfg(test)]
mod tests;
