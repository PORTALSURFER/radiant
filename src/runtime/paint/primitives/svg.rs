use std::{fmt, sync::Arc};

use vello_svg::usvg;

use crate::{gui::types::Rect, widgets::WidgetId};

/// Retained SVG document parsed once for backend rendering.
///
/// Radiant keeps this type opaque so widgets can embed SVG assets without
/// depending on a renderer scene. The native Vello backend consumes the parsed
/// tree directly through `vello_svg`.
#[derive(Clone, Debug)]
pub struct PaintSvgDocument {
    tree: Arc<usvg::Tree>,
}

/// Error returned when SVG source cannot be parsed into a retained document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgParseError {
    message: String,
}

impl PaintSvgDocument {
    /// Parse SVG source into a retained document.
    pub fn from_svg(svg: &str) -> Option<Self> {
        Self::try_from_svg(svg).ok()
    }

    /// Parse SVG source into a retained document with diagnostics.
    pub fn try_from_svg(svg: &str) -> Result<Self, SvgParseError> {
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|error| {
            SvgParseError {
                message: error.to_string(),
            }
        })?;
        Ok(Self {
            tree: Arc::new(tree),
        })
    }

    pub(crate) fn tree(&self) -> &usvg::Tree {
        &self.tree
    }
}

impl SvgParseError {
    /// Return the parser diagnostic message.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl fmt::Display for SvgParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message.as_str())
    }
}

impl std::error::Error for SvgParseError {}

impl PartialEq for PaintSvgDocument {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.tree, &other.tree)
    }
}

/// SVG document paint primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintSvg {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Retained SVG document to render.
    pub document: PaintSvgDocument,
    /// Destination rectangle.
    pub rect: Rect,
}
