//! Private SVG document projection for retained vector icons.

use std::sync::Arc;

use vello_svg::{
    usvg::{self, FillRule, Node},
    util::{to_affine, to_bez_path},
};

use crate::runtime::PaintPath;

/// Parsed SVG document ready for backend-neutral vector painting.
#[derive(Clone, Debug)]
pub(super) struct SvgDocument {
    /// The normalized document width produced by `usvg`.
    pub(super) width: f32,
    /// The normalized document height produced by `usvg`.
    pub(super) height: f32,
    /// Filled shapes emitted by the document.
    pub(super) shapes: Vec<SvgShape>,
}

impl SvgDocument {
    pub(super) fn parse(svg: &str) -> Option<Self> {
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
        let mut shapes = Vec::new();
        collect_shapes(tree.root(), &mut shapes);
        if shapes.is_empty() {
            return None;
        }

        let size = tree.size();
        Some(Self {
            width: size.width(),
            height: size.height(),
            shapes,
        })
    }
}

/// One filled SVG shape retained for vector painting.
#[derive(Clone, Debug)]
pub(super) struct SvgShape {
    pub(super) path: PaintPath,
    pub(super) fill_rule: SvgFillRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SvgFillRule {
    NonZero,
    EvenOdd,
}

fn collect_shapes(group: &usvg::Group, shapes: &mut Vec<SvgShape>) {
    for node in group.children() {
        match node {
            Node::Group(group) => collect_shapes(group, shapes),
            Node::Path(path) if path.is_visible() => {
                let Some(fill) = path.fill() else {
                    continue;
                };
                shapes.push(SvgShape {
                    path: Arc::new(to_affine(&path.abs_transform()) * to_bez_path(path)),
                    fill_rule: fill.rule().into(),
                });
            }
            _ => {}
        }
    }
}

impl From<FillRule> for SvgFillRule {
    fn from(value: FillRule) -> Self {
        match value {
            FillRule::NonZero => Self::NonZero,
            FillRule::EvenOdd => Self::EvenOdd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vello::kurbo::{Rect as KurboRect, Shape};

    #[test]
    fn parses_usvg_normalized_shape_icons() {
        let svg = r#"
            <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <g transform="matrix(2,0,0,2,4,4)" style="fill-rule:evenodd">
                <path d="M 0 0 L 2 0 L 2 2 L 0 2 Z" />
              </g>
            </svg>
        "#;

        let document = SvgDocument::parse(svg).expect("document should parse");

        assert_eq!(document.width, 16.0);
        assert_eq!(document.height, 16.0);
        assert_eq!(document.shapes.len(), 1);
        assert_eq!(document.shapes[0].fill_rule, SvgFillRule::EvenOdd);
        assert_eq!(
            document.shapes[0].path.bounding_box(),
            KurboRect::new(4.0, 4.0, 8.0, 8.0)
        );
    }

    #[test]
    fn supports_shape_elements_normalized_by_usvg() {
        let svg = r#"
            <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <rect x="1" y="2" width="3" height="4" />
              <circle cx="10" cy="10" r="2" />
              <polygon points="6,1 9,1 9,4" />
            </svg>
        "#;

        let document = SvgDocument::parse(svg).expect("document should parse");

        assert_eq!(document.shapes.len(), 3);
    }

    #[test]
    fn rejects_documents_with_no_supported_filled_shapes() {
        let svg = r#"
            <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <defs><path d="M0 0 L1 0 L1 1 Z" /></defs>
            </svg>
        "#;

        assert!(SvgDocument::parse(svg).is_none());
    }
}
