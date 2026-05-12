use std::sync::Arc;

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

impl PaintSvgDocument {
    /// Parse SVG source into a retained document.
    pub fn from_svg(svg: &str) -> Option<Self> {
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
        Some(Self {
            tree: Arc::new(tree),
        })
    }

    pub(crate) fn tree(&self) -> &usvg::Tree {
        &self.tree
    }
}

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
