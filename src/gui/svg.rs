//! Retained SVG icons for Vello-backed vector rendering.
//!
//! The public surface is [`SvgIcon`]. Parser details stay behind Radiant paint
//! primitives so application widgets can embed SVG assets without constructing
//! backend scenes directly.

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_icon_appends_retained_svg_primitive() {
        let svg = r##"
            <svg viewBox="0 0 4 4" color="#0a141e" xmlns="http://www.w3.org/2000/svg">
              <rect x="0" y="0" width="2" height="4" fill="currentColor" />
            </svg>
        "##;
        let icon = SvgIcon::from_svg(svg).expect("icon should parse");
        let mut primitives = Vec::new();

        icon.append_paint(
            &mut primitives,
            9,
            Rect::from_min_size(
                crate::gui::types::Point::new(10.0, 20.0),
                crate::gui::types::Vector2::new(8.0, 8.0),
            ),
        );

        let [PaintPrimitive::Svg(svg)] = primitives.as_slice() else {
            panic!("svg icon should append a retained SVG primitive");
        };
        assert_eq!(svg.widget_id, 9);
        assert_eq!(svg.rect.min.x, 10.0);
    }

    #[test]
    fn svg_icon_try_from_svg_reports_parse_errors() {
        let error = SvgIcon::try_from_svg("<svg><").expect_err("invalid svg should fail");

        assert!(!error.message().is_empty());
        assert_eq!(error.to_string(), error.message());
        assert!(SvgIcon::from_svg("<svg><").is_none());
    }
}
