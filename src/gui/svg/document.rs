//! Private SVG document parser for retained vector icons.

use std::sync::Arc;
use vello::kurbo::{
    Affine, BezPath, Circle as KurboCircle, Point as KurboPoint, Rect as KurboRect, Shape,
};

use crate::runtime::PaintPath;

use super::transform::{parse_attr_f64, parse_number_list, parse_points, parse_transform_list};

/// Parsed SVG document ready for vector painting.
#[derive(Clone, Debug)]
pub(super) struct SvgDocument {
    /// The minimum x coordinate in the declared view box.
    pub(super) view_box_min_x: f32,
    /// The minimum y coordinate in the declared view box.
    pub(super) view_box_min_y: f32,
    /// The width of the declared view box.
    pub(super) view_box_width: f32,
    /// The height of the declared view box.
    pub(super) view_box_height: f32,
    /// The transformed filled shapes emitted by the document.
    pub(super) shapes: Vec<SvgShape>,
}

impl SvgDocument {
    pub(super) fn parse(svg: &str) -> Option<Self> {
        let document = roxmltree::Document::parse(svg).ok()?;
        let root = document.root_element();
        if root.tag_name().name() != "svg" {
            return None;
        }

        let view_box_values = parse_number_list(root.attribute("viewBox")?)?;
        if view_box_values.len() != 4 {
            return None;
        }

        let mut shapes = Vec::new();
        collect_shapes(
            root,
            Affine::IDENTITY,
            resolve_fill_rule(root, SvgFillRule::NonZero),
            &mut shapes,
        )?;
        if shapes.is_empty() {
            return None;
        }

        Some(Self {
            view_box_min_x: view_box_values[0] as f32,
            view_box_min_y: view_box_values[1] as f32,
            view_box_width: view_box_values[2] as f32,
            view_box_height: view_box_values[3] as f32,
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

fn collect_shapes(
    node: roxmltree::Node<'_, '_>,
    inherited_transform: Affine,
    inherited_fill_rule: SvgFillRule,
    shapes: &mut Vec<SvgShape>,
) -> Option<()> {
    let local_transform = parse_transform_list(node.attribute("transform"))?;
    let transform = inherited_transform * local_transform;
    let fill_rule = resolve_fill_rule(node, inherited_fill_rule);

    match node.tag_name().name() {
        "svg" | "g" => {
            for child in node.children().filter(roxmltree::Node::is_element) {
                collect_shapes(child, transform, fill_rule, shapes)?;
            }
        }
        "path" => {
            if !shape_is_filled(node) {
                return Some(());
            }
            let path = BezPath::from_svg(node.attribute("d")?).ok()?;
            shapes.push(SvgShape {
                path: Arc::new(transform * path),
                fill_rule,
            });
        }
        "rect" => {
            if !shape_is_filled(node) {
                return Some(());
            }
            let x = parse_attr_f64(node, "x").unwrap_or(0.0);
            let y = parse_attr_f64(node, "y").unwrap_or(0.0);
            let width = parse_attr_f64(node, "width")?;
            let height = parse_attr_f64(node, "height")?;
            let path = KurboRect::new(x, y, x + width, y + height).to_path(0.1);
            shapes.push(SvgShape {
                path: Arc::new(transform * path),
                fill_rule,
            });
        }
        "circle" => {
            if !shape_is_filled(node) {
                return Some(());
            }
            let circle = KurboCircle::new(
                KurboPoint::new(parse_attr_f64(node, "cx")?, parse_attr_f64(node, "cy")?),
                parse_attr_f64(node, "r")?,
            );
            shapes.push(SvgShape {
                path: Arc::new(transform * circle.to_path(0.1)),
                fill_rule,
            });
        }
        "polygon" => {
            if !shape_is_filled(node) {
                return Some(());
            }
            let points = parse_points(node.attribute("points")?)?;
            let mut path = BezPath::new();
            let first = points.first()?;
            path.move_to(*first);
            for point in points.iter().skip(1) {
                path.line_to(*point);
            }
            path.close_path();
            shapes.push(SvgShape {
                path: Arc::new(transform * path),
                fill_rule,
            });
        }
        _ => {}
    }

    Some(())
}

fn resolve_fill_rule(node: roxmltree::Node<'_, '_>, inherited: SvgFillRule) -> SvgFillRule {
    node.attribute("fill-rule")
        .and_then(parse_fill_rule)
        .or_else(|| extract_style_property(node, "fill-rule").and_then(parse_fill_rule))
        .unwrap_or(inherited)
}

fn parse_fill_rule(raw: &str) -> Option<SvgFillRule> {
    match raw.trim() {
        "evenodd" => Some(SvgFillRule::EvenOdd),
        "nonzero" => Some(SvgFillRule::NonZero),
        _ => None,
    }
}

fn shape_is_filled(node: roxmltree::Node<'_, '_>) -> bool {
    let fill = node
        .attribute("fill")
        .or_else(|| extract_style_property(node, "fill"));
    !matches!(fill.map(str::trim), Some("none"))
}

fn extract_style_property<'a>(node: roxmltree::Node<'a, 'a>, property: &str) -> Option<&'a str> {
    let style = node.attribute("style")?;
    style.split(';').find_map(|entry| {
        let (name, value) = entry.split_once(':')?;
        (name.trim() == property).then_some(value.trim())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_transformed_path_icons() {
        let svg = r#"
            <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <g transform="matrix(2,0,0,2,4,4)" style="fill-rule:evenodd">
                <path d="M 0 0 L 2 0 L 2 2 L 0 2 Z" />
              </g>
            </svg>
        "#;

        let document = SvgDocument::parse(svg).expect("document should parse");

        assert_eq!(document.shapes.len(), 1);
        assert_eq!(
            document.shapes[0].path.bounding_box(),
            KurboRect::new(4.0, 4.0, 8.0, 8.0)
        );
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
