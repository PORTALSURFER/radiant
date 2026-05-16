//! Retained SVG icons for Vello-backed vector rendering.
//!
//! The public surface is [`SvgIcon`]. Parser details stay behind Radiant paint
//! primitives so application widgets can embed SVG assets without constructing
//! backend scenes directly.

use crate::gui::types::Rect;
use crate::runtime::{PaintPrimitive, PaintSvg, PaintSvgDocument, SvgParseError};
use crate::widgets::WidgetId;
use vello::kurbo::{
    Affine, BezPath, Circle as KurboCircle, Point as KurboPoint, Rect as KurboRect, Shape, Vec2,
};

/// Retained SVG icon parsed once for backend rendering.
#[derive(Clone, Debug)]
pub struct SvgIcon {
    document: PaintSvgDocument,
}

/// Parsed SVG document ready for simple mask-style rasterization.
///
/// This intentionally supports a small icon-oriented subset: view boxes,
/// groups, paths, rectangles, circles, polygons, transforms, style `fill`, and
/// `fill-rule`. Retained Vello SVG painting remains the preferred path for full
/// backend rendering.
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
    path: BezPath,
    fill_rule: SvgFillRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SvgFillRule {
    NonZero,
    EvenOdd,
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

/// Parse one icon-oriented SVG document from an asset file.
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
                path: transform * path,
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
                path: transform * path,
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
                path: transform * circle.to_path(0.1),
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
                path: transform * path,
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

/// Determine whether one point lands inside any parsed SVG shape.
pub fn point_in_svg_shapes(x: f32, y: f32, shapes: &[SvgShape]) -> bool {
    let point = KurboPoint::new(x as f64, y as f64);
    shapes.iter().any(|shape| point_in_svg_shape(point, shape))
}

fn point_in_svg_shape(point: KurboPoint, shape: &SvgShape) -> bool {
    match shape.fill_rule {
        SvgFillRule::NonZero => shape.path.contains(point),
        SvgFillRule::EvenOdd => shape.path.winding(point).abs() % 2 == 1,
    }
}

fn parse_attr_f64(node: roxmltree::Node<'_, '_>, attr: &str) -> Option<f64> {
    parse_number(node.attribute(attr)?)
}

fn parse_transform_list(raw: Option<&str>) -> Option<Affine> {
    let Some(mut remaining) = raw.map(str::trim) else {
        return Some(Affine::IDENTITY);
    };
    if remaining.is_empty() {
        return Some(Affine::IDENTITY);
    }

    let mut transform = Affine::IDENTITY;
    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
        if remaining.is_empty() {
            break;
        }
        let open = remaining.find('(')?;
        let name = remaining[..open].trim();
        let body = &remaining[open + 1..];
        let close = body.find(')')?;
        let args = &body[..close];
        remaining = &body[close + 1..];
        transform *= parse_single_transform(name, args)?;
    }
    Some(transform)
}

fn parse_single_transform(name: &str, args: &str) -> Option<Affine> {
    let values = parse_number_list(args)?;
    match name {
        "matrix" if values.len() == 6 => Some(Affine::new([
            values[0], values[1], values[2], values[3], values[4], values[5],
        ])),
        "translate" if values.len() == 1 => Some(Affine::translate(Vec2::new(values[0], 0.0))),
        "translate" if values.len() == 2 => {
            Some(Affine::translate(Vec2::new(values[0], values[1])))
        }
        "scale" if values.len() == 1 => Some(Affine::scale(values[0])),
        "scale" if values.len() == 2 => Some(Affine::scale_non_uniform(values[0], values[1])),
        "rotate" if values.len() == 1 => Some(Affine::rotate(values[0].to_radians())),
        "rotate" if values.len() == 3 => {
            let center = KurboPoint::new(values[1], values[2]);
            Some(
                Affine::translate(center.to_vec2())
                    * Affine::rotate(values[0].to_radians())
                    * Affine::translate(-center.to_vec2()),
            )
        }
        "skewX" if values.len() == 1 => Some(Affine::new([
            1.0,
            0.0,
            values[0].to_radians().tan(),
            1.0,
            0.0,
            0.0,
        ])),
        "skewY" if values.len() == 1 => Some(Affine::new([
            1.0,
            values[0].to_radians().tan(),
            0.0,
            1.0,
            0.0,
            0.0,
        ])),
        _ => None,
    }
}

fn parse_points(points: &str) -> Option<Vec<KurboPoint>> {
    let coords = parse_number_list(points)?;
    if coords.len() < 6 || coords.len() % 2 != 0 {
        return None;
    }
    Some(
        coords
            .chunks_exact(2)
            .map(|pair| KurboPoint::new(pair[0], pair[1]))
            .collect(),
    )
}

fn parse_number_list(raw: &str) -> Option<Vec<f64>> {
    let normalized = raw.replace(',', " ");
    normalized
        .split_whitespace()
        .map(parse_number)
        .collect::<Option<Vec<_>>>()
}

fn parse_number(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
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

    #[test]
    fn svg_subset_parser_supports_evenodd_cutouts() {
        let svg = r#"
            <svg viewBox="0 0 10 10" xmlns="http://www.w3.org/2000/svg">
              <path fill-rule="evenodd" d="M0 0H10V10H0ZM3 3H7V7H3Z" />
            </svg>
        "#;

        let document = parse_svg_document(svg).expect("svg should parse");

        assert_eq!(document.view_box_width, 10.0);
        assert!(point_in_svg_shapes(1.0, 1.0, &document.shapes));
        assert!(!point_in_svg_shapes(5.0, 5.0, &document.shapes));
    }

    #[test]
    fn svg_subset_parser_applies_group_transforms() {
        let svg = r#"
            <svg viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
              <g transform="translate(4 2) scale(2)">
                <rect x="1" y="1" width="3" height="3" />
              </g>
            </svg>
        "#;

        let document = parse_svg_document(svg).expect("svg should parse");

        assert!(point_in_svg_shapes(7.0, 5.0, &document.shapes));
        assert!(!point_in_svg_shapes(2.0, 2.0, &document.shapes));
    }
}
