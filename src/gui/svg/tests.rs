use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::PaintPrimitive;

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
        Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(8.0, 8.0)),
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
