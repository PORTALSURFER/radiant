//! XML-backed SVG subset parsing for retained icon rasterizers.
//!
//! The toolbar loader only needs filled vector glyphs, but users edit these
//! assets in regular tools such as Inkscape. This parser accepts the common
//! SVG subset those tools emit for simple icons, including group transforms
//! and path data, and converts the result into retained bezier paths.

mod transform;

use vello::kurbo::{
    Affine, BezPath, Circle as KurboCircle, Point as KurboPoint, Rect as KurboRect, Shape, Vec2,
};

use crate::gui::types::Rgba8;
use crate::runtime::{PaintFillPath, PaintFillRule, PaintPath, PaintPrimitive};
use crate::widgets::WidgetId;
use transform::{parse_attr_f64, parse_number_list, parse_points, parse_transform_list};

/// Parsed SVG document ready for rasterization.
#[derive(Clone, Debug)]
pub struct SvgDocument {
    /// The minimum x coordinate in the declared view box.
    pub view_box_min_x: f32,
    /// The minimum y coordinate in the declared view box.
    pub view_box_min_y: f32,
    /// The width of the declared view box.
    pub view_box_width: f32,
    /// The height of the declared view box.
    pub view_box_height: f32,
    /// The transformed filled shapes emitted by the document.
    pub shapes: Vec<SvgShape>,
}

/// One rasterizable filled SVG shape.
#[derive(Clone, Debug)]
pub struct SvgShape {
    path: PaintPath,
    fill_rule: SvgFillRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SvgFillRule {
    NonZero,
    EvenOdd,
}

/// Retained vector SVG icon parsed into backend-neutral paint paths.
#[derive(Clone, Debug)]
pub struct SvgIcon {
    document: SvgDocument,
}

impl SvgIcon {
    /// Parse an SVG icon from embedded source text.
    pub fn from_svg(svg: &str) -> Option<Self> {
        Some(Self {
            document: parse_svg_document(svg)?,
        })
    }

    /// Append this icon as filled vector paint primitives inside `rect`.
    pub fn append_fill_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        rect: crate::gui::types::Rect,
        color: Rgba8,
    ) {
        let scale_x = rect.width() / self.document.view_box_width.max(f32::EPSILON);
        let scale_y = rect.height() / self.document.view_box_height.max(f32::EPSILON);
        let transform = Affine::translate(Vec2::new(rect.min.x as f64, rect.min.y as f64))
            * Affine::scale_non_uniform(scale_x as f64, scale_y as f64)
            * Affine::translate(Vec2::new(
                -self.document.view_box_min_x as f64,
                -self.document.view_box_min_y as f64,
            ));
        for shape in &self.document.shapes {
            primitives.push(PaintPrimitive::FillPath(PaintFillPath {
                widget_id,
                path: std::sync::Arc::clone(&shape.path),
                transform,
                fill_rule: shape.fill_rule.into(),
                color,
            }));
        }
    }
}

impl From<SvgFillRule> for PaintFillRule {
    fn from(value: SvgFillRule) -> Self {
        match value {
            SvgFillRule::NonZero => Self::NonZero,
            SvgFillRule::EvenOdd => Self::EvenOdd,
        }
    }
}

/// Parse one SVG document from an asset file.
pub fn parse_svg_document(svg: &str) -> Option<SvgDocument> {
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

    Some(SvgDocument {
        view_box_min_x: view_box_values[0] as f32,
        view_box_min_y: view_box_values[1] as f32,
        view_box_width: view_box_values[2] as f32,
        view_box_height: view_box_values[3] as f32,
        shapes,
    })
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
                path: std::sync::Arc::new(transform * path),
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
                path: std::sync::Arc::new(transform * path),
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
                path: std::sync::Arc::new(transform * circle.to_path(0.1)),
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
                path: std::sync::Arc::new(transform * path),
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

        let document = parse_svg_document(svg).expect("document should parse");

        assert_eq!(document.shapes.len(), 1);
        assert_eq!(
            document.shapes[0].path.bounding_box(),
            KurboRect::new(4.0, 4.0, 8.0, 8.0)
        );
    }

    #[test]
    fn svg_icon_appends_scaled_fill_path_primitives() {
        let svg = r#"
            <svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg">
              <rect x="0" y="0" width="2" height="4" />
            </svg>
        "#;
        let icon = SvgIcon::from_svg(svg).expect("icon should parse");
        let mut primitives = Vec::new();

        icon.append_fill_paint(
            &mut primitives,
            9,
            crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(10.0, 20.0),
                crate::gui::types::Vector2::new(8.0, 8.0),
            ),
            Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
        );
        icon.append_fill_paint(
            &mut primitives,
            9,
            crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(10.0, 20.0),
                crate::gui::types::Vector2::new(8.0, 8.0),
            ),
            Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
        );

        let [
            PaintPrimitive::FillPath(fill),
            PaintPrimitive::FillPath(second_fill),
        ] = primitives.as_slice()
        else {
            panic!("svg icon should append filled paths");
        };
        assert_eq!(fill.widget_id, 9);
        assert_eq!(fill.color.r, 10);
        assert!(std::sync::Arc::ptr_eq(&fill.path, &second_fill.path));
        let transformed_path = fill.transform * (*fill.path).clone();
        assert_eq!(
            transformed_path.bounding_box(),
            KurboRect::new(10.0, 20.0, 14.0, 28.0)
        );
    }

    #[test]
    fn rejects_documents_with_no_supported_filled_shapes() {
        let svg = r#"
            <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <defs><path d="M0 0 L1 0 L1 1 Z" /></defs>
            </svg>
        "#;

        assert!(parse_svg_document(svg).is_none());
    }
}
