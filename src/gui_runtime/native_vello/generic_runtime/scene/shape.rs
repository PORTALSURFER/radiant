use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8},
    gui_runtime::native_vello::{color_from_rgba, to_kurbo_rect},
    runtime::{PaintFillRule, PaintPath, PaintTransform},
};
use kurbo::Stroke;
use vello::{Scene, kurbo::Affine, peniko::Fill};

mod geometry;

use geometry::{paintable_stroke_width, polygon_path, polyline_path, to_kurbo_path};

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_path_fill(
    scene: &mut Scene,
    color: Rgba8,
    transform: PaintTransform,
    fill_rule: PaintFillRule,
    path: &PaintPath,
) {
    if !transform.is_finite() {
        return;
    }
    let Some(path) = to_kurbo_path(path) else {
        return;
    };

    scene.fill(
        match fill_rule {
            PaintFillRule::NonZero => Fill::NonZero,
            PaintFillRule::EvenOdd => Fill::EvenOdd,
        },
        Affine::new(transform.coefficients()),
        color_from_rgba(color),
        None,
        &path,
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_rect(
    scene: &mut Scene,
    color: Rgba8,
    rect: UiRect,
) {
    if !rect.has_finite_positive_area() {
        return;
    }
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_rect_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    rect: UiRect,
) {
    if !rect.has_finite_positive_area() || !paintable_stroke_width(width) {
        return;
    }
    scene.stroke(
        &Stroke::new(width as f64),
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polygon_fill(
    scene: &mut Scene,
    color: Rgba8,
    points: &[Point],
) {
    if let Some(path) = polygon_path(points) {
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polygon_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    points: &[Point],
) {
    if !paintable_stroke_width(width) {
        return;
    }
    if let Some(path) = polygon_path(points) {
        scene.stroke(
            &Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polyline_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    points: &[Point],
) {
    if !paintable_stroke_width(width) {
        return;
    }
    if let Some(path) = polyline_path(points) {
        scene.stroke(
            &Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}
