//! SVG document paint primitive for backend-neutral frames.

use crate::{gui::types::Rect, runtime::PaintSvgDocument};

/// Parsed SVG document drawn into a destination rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawSvg {
    /// SVG document to render.
    pub document: PaintSvgDocument,
    /// Destination bounds in logical points.
    pub rect: Rect,
}
