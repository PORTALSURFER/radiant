use crate::gui::types::Rect;
use crate::runtime::{PaintPrimitive, PaintSvg, PaintSvgDocument, SvgParseError};
use crate::widgets::WidgetId;

/// Retained SVG icon parsed once for backend rendering.
#[derive(Clone, Debug)]
pub struct SvgIcon {
    document: PaintSvgDocument,
}

impl SvgIcon {
    /// Parse an SVG icon from embedded source text.
    pub fn from_svg(svg: &str) -> Option<Self> {
        Self::try_from_svg(svg).ok()
    }

    /// Parse an SVG icon from embedded source text with diagnostics.
    pub fn try_from_svg(svg: &str) -> Result<Self, SvgParseError> {
        Ok(Self {
            document: PaintSvgDocument::try_from_svg(svg)?,
        })
    }

    /// Append this icon as a retained SVG paint primitive inside `rect`.
    pub fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        rect: Rect,
    ) {
        primitives.push(PaintPrimitive::Svg(PaintSvg {
            widget_id,
            document: self.document.clone(),
            rect,
        }));
    }
}
