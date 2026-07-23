use crate::{
    gui::types::{Rect as UiRect, Rgba8, Vector2},
    runtime::{PaintBrush, PaintFillPath, PaintFillRule, PaintSvg, PaintTransform},
};
use vello::kurbo::Affine;
use vello_svg::usvg;

use super::{
    OverlayVertex, intersect_rect,
    path::{paint_path_from_tiny_skia, push_fill_path_vertices_in_regions_including_opaque},
};

pub(super) fn push_svg_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    svg: &PaintSvg,
) {
    if !svg.rect.has_finite_positive_area() {
        return;
    }
    push_svg_vertices_clipped(vertices, target_size, svg, std::slice::from_ref(&svg.rect));
}

pub(super) fn push_svg_vertices_in_regions(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    svg: &PaintSvg,
    regions: &[UiRect],
) {
    let clipped_regions = regions
        .iter()
        .filter_map(|region| intersect_rect(svg.rect, *region))
        .collect::<Vec<_>>();
    push_svg_vertices_clipped(vertices, target_size, svg, &clipped_regions);
}

fn push_svg_vertices_clipped(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    svg: &PaintSvg,
    regions: &[UiRect],
) {
    if regions.is_empty() {
        return;
    }
    let size = svg.document.tree().size();
    let width = size.width();
    let height = size.height();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return;
    }
    let destination = Affine::translate((svg.rect.min.x as f64, svg.rect.min.y as f64))
        * Affine::scale_non_uniform(
            svg.rect.width() as f64 / width as f64,
            svg.rect.height() as f64 / height as f64,
        );
    push_group(
        vertices,
        target_size,
        svg,
        svg.document.tree().root(),
        destination,
        1.0,
        regions,
    );
}

fn push_group(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    svg: &PaintSvg,
    group: &usvg::Group,
    destination: Affine,
    inherited_opacity: f32,
    regions: &[UiRect],
) {
    let opacity = inherited_opacity * group.opacity().get();
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => {
                push_group(
                    vertices,
                    target_size,
                    svg,
                    child,
                    destination,
                    opacity,
                    regions,
                );
            }
            usvg::Node::Path(path) if path.is_visible() => {
                push_path(
                    vertices,
                    target_size,
                    svg,
                    path,
                    destination,
                    opacity,
                    regions,
                );
            }
            usvg::Node::Text(text) => {
                push_group(
                    vertices,
                    target_size,
                    svg,
                    text.flattened(),
                    destination,
                    opacity,
                    regions,
                );
            }
            usvg::Node::Image(_) | usvg::Node::Path(_) => {}
        }
    }
}

fn push_path(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    svg: &PaintSvg,
    path: &usvg::Path,
    destination: Affine,
    opacity: f32,
    regions: &[UiRect],
) {
    let transform = PaintTransform::new(
        (destination * vello_svg::util::to_affine(&path.abs_transform())).as_coeffs(),
    );
    let push_fill = |vertices: &mut Vec<OverlayVertex>| {
        let Some(fill) = path.fill() else {
            return;
        };
        let Some(color) = solid_color(fill.paint(), opacity * fill.opacity().get()) else {
            return;
        };
        let Some(path) = paint_path_from_tiny_skia(path.data()) else {
            return;
        };
        let fill_rule = match fill.rule() {
            usvg::FillRule::NonZero => PaintFillRule::NonZero,
            usvg::FillRule::EvenOdd => PaintFillRule::EvenOdd,
        };
        let fill = PaintFillPath::new(svg.widget_id, path, PaintBrush::solid(color))
            .transform(transform)
            .fill_rule(fill_rule);
        push_fill_path_vertices_in_regions_including_opaque(vertices, target_size, &fill, regions);
    };
    let push_stroke = |vertices: &mut Vec<OverlayVertex>| {
        let Some(stroke) = path.stroke() else {
            return;
        };
        let Some(color) = solid_color(stroke.paint(), opacity * stroke.opacity().get()) else {
            return;
        };
        let Some(outline) = path.data().stroke(&stroke.to_tiny_skia(), 1.0) else {
            return;
        };
        let Some(path) = paint_path_from_tiny_skia(&outline) else {
            return;
        };
        let fill =
            PaintFillPath::new(svg.widget_id, path, PaintBrush::solid(color)).transform(transform);
        push_fill_path_vertices_in_regions_including_opaque(vertices, target_size, &fill, regions);
    };

    match path.paint_order() {
        usvg::PaintOrder::FillAndStroke => {
            push_fill(vertices);
            push_stroke(vertices);
        }
        usvg::PaintOrder::StrokeAndFill => {
            push_stroke(vertices);
            push_fill(vertices);
        }
    }
}

fn solid_color(paint: &usvg::Paint, opacity: f32) -> Option<Rgba8> {
    let usvg::Paint::Color(color) = paint else {
        return None;
    };
    Some(Rgba8::new(
        color.red,
        color.green,
        color.blue,
        (opacity.clamp(0.0, 1.0) * u8::MAX as f32).round() as u8,
    ))
}
