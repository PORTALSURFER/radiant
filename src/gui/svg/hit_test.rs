use kurbo::{Point as KurboPoint, Shape};

use super::{SvgFillRule, SvgShape};

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
