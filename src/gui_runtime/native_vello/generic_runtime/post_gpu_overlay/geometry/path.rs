use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    runtime::{
        PaintBrush, PaintFillPath, PaintFillRule, PaintLinearGradient, PaintPath, PaintPathCommand,
        PaintTransform,
    },
};
use lyon_tessellation::{
    FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers,
    geometry_builder::BuffersBuilder,
    math::{Point as LyonPoint, point},
    path::Path,
};

use super::{
    OPAQUE_REVEALED_FILL_ALPHA, OverlayVertex, clip_x, clip_y, rgba_to_float,
    target_has_finite_positive_size,
};
use vello_svg::usvg::tiny_skia_path::{Path as TinyPath, PathSegment};

#[derive(Clone, Copy)]
struct PathVertex {
    point: Point,
    color: [f32; 4],
}

pub(super) fn push_fill_path_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPath,
) {
    if !target_has_finite_positive_size(target_size) {
        return;
    }
    let Some(geometry) = tessellated_geometry(fill) else {
        return;
    };
    for indices in geometry.indices.chunks_exact(3) {
        if let Some(triangle) = painted_triangle(fill, &geometry, indices) {
            push_triangle(vertices, target_size, triangle);
        }
    }
}

pub(super) fn push_fill_path_vertices_in_regions(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPath,
    regions: &[UiRect],
) {
    push_fill_path_vertices_in_regions_with_opacity_policy(
        vertices,
        target_size,
        fill,
        regions,
        true,
    );
}

pub(super) fn push_fill_path_vertices_in_regions_including_opaque(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPath,
    regions: &[UiRect],
) {
    push_fill_path_vertices_in_regions_with_opacity_policy(
        vertices,
        target_size,
        fill,
        regions,
        false,
    );
}

fn push_fill_path_vertices_in_regions_with_opacity_policy(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPath,
    regions: &[UiRect],
    skip_opaque: bool,
) {
    if !target_has_finite_positive_size(target_size)
        || regions.is_empty()
        || (skip_opaque && brush_is_opaque(fill.brush))
    {
        return;
    }
    let Some(geometry) = tessellated_geometry(fill) else {
        return;
    };
    let mut input = Vec::with_capacity(8);
    let mut output = Vec::with_capacity(8);
    for indices in geometry.indices.chunks_exact(3) {
        let Some(triangle) = painted_triangle(fill, &geometry, indices) else {
            continue;
        };
        for region in regions
            .iter()
            .copied()
            .filter(|region| region.has_finite_positive_area())
        {
            input.clear();
            input.extend_from_slice(&triangle);
            for edge in ClipEdge::for_rect(region) {
                clip_polygon(&input, edge, &mut output);
                std::mem::swap(&mut input, &mut output);
                if input.is_empty() {
                    break;
                }
            }
            push_triangle_fan(vertices, target_size, &input);
        }
    }
}

fn brush_is_opaque(brush: PaintBrush) -> bool {
    match brush {
        PaintBrush::Solid(color) => color.a >= OPAQUE_REVEALED_FILL_ALPHA,
        PaintBrush::LinearGradient(gradient) => {
            gradient.start_color.a >= OPAQUE_REVEALED_FILL_ALPHA
                && gradient.end_color.a >= OPAQUE_REVEALED_FILL_ALPHA
        }
    }
}

fn tessellated_geometry(fill: &PaintFillPath) -> Option<VertexBuffers<LyonPoint, u32>> {
    if !fill.transform.is_finite() {
        return None;
    }
    let path = lyon_path(&fill.path)?;
    let mut geometry = VertexBuffers::<LyonPoint, u32>::new();
    let options = FillOptions::default().with_fill_rule(match fill.fill_rule {
        PaintFillRule::NonZero => FillRule::NonZero,
        PaintFillRule::EvenOdd => FillRule::EvenOdd,
    });
    FillTessellator::new()
        .tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex<'_>| vertex.position()),
        )
        .ok()?;

    Some(geometry)
}

fn painted_triangle(
    fill: &PaintFillPath,
    geometry: &VertexBuffers<LyonPoint, u32>,
    indices: &[u32],
) -> Option<[PathVertex; 3]> {
    let local = [
        geometry.vertices[indices[0] as usize],
        geometry.vertices[indices[1] as usize],
        geometry.vertices[indices[2] as usize],
    ];
    let painted = local.map(|point| {
        let local = Point::new(point.x, point.y);
        let transformed = transform_point(local, fill.transform);
        PathVertex {
            point: transformed,
            color: brush_color(fill.brush, transformed),
        }
    });
    (painted.iter().all(|vertex| vertex.point.is_finite())
        && painted.iter().any(|vertex| vertex.color[3] > 0.0))
    .then_some(painted)
}

fn lyon_path(path: &crate::runtime::PaintPath) -> Option<Path> {
    let mut builder = Path::builder();
    let mut open = false;
    let mut has_any_segment = false;
    for command in path.commands() {
        match *command {
            PaintPathCommand::MoveTo(to) => {
                if !to.is_finite() {
                    return None;
                }
                if open {
                    builder.end(false);
                }
                builder.begin(to_lyon(to));
                open = true;
            }
            PaintPathCommand::LineTo(to) => {
                if !to.is_finite() {
                    return None;
                }
                if open {
                    builder.line_to(to_lyon(to));
                    has_any_segment = true;
                }
            }
            PaintPathCommand::QuadTo { control, to } => {
                if !control.is_finite() || !to.is_finite() {
                    return None;
                }
                if open {
                    builder.quadratic_bezier_to(to_lyon(control), to_lyon(to));
                    has_any_segment = true;
                }
            }
            PaintPathCommand::CurveTo {
                control1,
                control2,
                to,
            } => {
                if !control1.is_finite() || !control2.is_finite() || !to.is_finite() {
                    return None;
                }
                if open {
                    builder.cubic_bezier_to(to_lyon(control1), to_lyon(control2), to_lyon(to));
                    has_any_segment = true;
                }
            }
            PaintPathCommand::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    has_any_segment.then(|| builder.build())
}

pub(super) fn paint_path_from_tiny_skia(path: &TinyPath) -> Option<PaintPath> {
    let mut commands = Vec::new();
    for segment in path.segments() {
        let command = match segment {
            PathSegment::MoveTo(point) => PaintPathCommand::MoveTo(tiny_point(point)),
            PathSegment::LineTo(point) => PaintPathCommand::LineTo(tiny_point(point)),
            PathSegment::QuadTo(control, to) => PaintPathCommand::QuadTo {
                control: tiny_point(control),
                to: tiny_point(to),
            },
            PathSegment::CubicTo(control1, control2, to) => PaintPathCommand::CurveTo {
                control1: tiny_point(control1),
                control2: tiny_point(control2),
                to: tiny_point(to),
            },
            PathSegment::Close => PaintPathCommand::Close,
        };
        commands.push(command);
    }
    (!commands.is_empty()).then(|| PaintPath::from(commands))
}

fn tiny_point(point: vello_svg::usvg::tiny_skia_path::Point) -> Point {
    Point::new(point.x, point.y)
}

fn to_lyon(value: Point) -> LyonPoint {
    point(value.x, value.y)
}

fn transform_point(point: Point, transform: PaintTransform) -> Point {
    Point::new(
        (transform.xx * point.x as f64 + transform.xy * point.y as f64 + transform.dx) as f32,
        (transform.yx * point.x as f64 + transform.yy * point.y as f64 + transform.dy) as f32,
    )
}

fn brush_color(brush: PaintBrush, point: Point) -> [f32; 4] {
    match brush {
        PaintBrush::Solid(color) => rgba_to_float(color),
        PaintBrush::LinearGradient(gradient) => gradient_color(gradient, point),
    }
}

fn gradient_color(gradient: PaintLinearGradient, point: Point) -> [f32; 4] {
    if !gradient.is_paintable() {
        return [0.0; 4];
    }
    let dx = gradient.end.x - gradient.start.x;
    let dy = gradient.end.y - gradient.start.y;
    let length_squared = dx * dx + dy * dy;
    let offset_x = point.x - gradient.start.x;
    let offset_y = point.y - gradient.start.y;
    let t = ((offset_x * dx + offset_y * dy) / length_squared).clamp(0.0, 1.0);
    lerp_color(gradient.start_color, gradient.end_color, t)
}

fn lerp_color(start: Rgba8, end: Rgba8, t: f32) -> [f32; 4] {
    let start = rgba_to_float(start);
    let end = rgba_to_float(end);
    std::array::from_fn(|index| start[index] + (end[index] - start[index]) * t)
}

fn push_triangle(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    triangle: [PathVertex; 3],
) {
    vertices.extend(triangle.map(|vertex| overlay_vertex(vertex, target_size)));
}

fn push_triangle_fan(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    polygon: &[PathVertex],
) {
    let Some((&origin, remainder)) = polygon.split_first() else {
        return;
    };
    for edge in remainder.windows(2) {
        push_triangle(vertices, target_size, [origin, edge[0], edge[1]]);
    }
}

fn overlay_vertex(vertex: PathVertex, target_size: Vector2) -> OverlayVertex {
    OverlayVertex::new(
        [
            clip_x(vertex.point.x, target_size),
            clip_y(vertex.point.y, target_size),
        ],
        vertex.color,
    )
}

#[derive(Clone, Copy)]
enum ClipEdge {
    Left(f32),
    Right(f32),
    Top(f32),
    Bottom(f32),
}

impl ClipEdge {
    fn for_rect(rect: UiRect) -> [Self; 4] {
        [
            Self::Left(rect.min.x),
            Self::Right(rect.max.x),
            Self::Top(rect.min.y),
            Self::Bottom(rect.max.y),
        ]
    }

    fn inside(self, point: Point) -> bool {
        match self {
            Self::Left(x) => point.x >= x,
            Self::Right(x) => point.x <= x,
            Self::Top(y) => point.y >= y,
            Self::Bottom(y) => point.y <= y,
        }
    }

    fn intersection(self, from: PathVertex, to: PathVertex) -> PathVertex {
        let (from_axis, to_axis, boundary) = match self {
            Self::Left(x) | Self::Right(x) => (from.point.x, to.point.x, x),
            Self::Top(y) | Self::Bottom(y) => (from.point.y, to.point.y, y),
        };
        let t = ((boundary - from_axis) / (to_axis - from_axis)).clamp(0.0, 1.0);
        PathVertex {
            point: Point::new(
                from.point.x + (to.point.x - from.point.x) * t,
                from.point.y + (to.point.y - from.point.y) * t,
            ),
            color: std::array::from_fn(|index| {
                from.color[index] + (to.color[index] - from.color[index]) * t
            }),
        }
    }
}

fn clip_polygon(input: &[PathVertex], edge: ClipEdge, output: &mut Vec<PathVertex>) {
    output.clear();
    let Some(mut previous) = input.last().copied() else {
        return;
    };
    let mut previous_inside = edge.inside(previous.point);
    for &current in input {
        let current_inside = edge.inside(current.point);
        if current_inside != previous_inside {
            output.push(edge.intersection(previous, current));
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
}
