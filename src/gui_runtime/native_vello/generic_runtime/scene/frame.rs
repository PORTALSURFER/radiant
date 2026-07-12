use super::{encode_image, encode_rect, encode_svg};
use crate::{
    gui::{
        paint::{PaintFrame, Primitive},
        types::{Point, Rect as UiRect},
    },
    gui_runtime::native_vello::{NativeTextRenderer, color_from_rgba, to_kurbo_rect},
    runtime::PaintSvg,
};
use std::sync::Arc;
use vello::{
    Scene,
    kurbo::{Affine, Circle, Point as KurboPoint},
    peniko::{Fill, Gradient},
};

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_paint_frame_to_scene(
    frame: &PaintFrame,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
) {
    for primitive in frame.primitives.iter() {
        match primitive {
            Primitive::Rect(fill) => encode_rect(scene, fill.color, fill.rect),
            Primitive::Circle(fill) => {
                if !paintable_circle(fill.center, fill.radius) {
                    continue;
                }
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(fill.color),
                    None,
                    &Circle::new(
                        (fill.center.x as f64, fill.center.y as f64),
                        fill.radius as f64,
                    ),
                );
            }
            Primitive::LinearGradient(fill) => {
                if !paintable_gradient(fill.rect, fill.start, fill.end) {
                    continue;
                }
                let mut gradient = Gradient::new_linear(
                    KurboPoint::new(fill.start.x as f64, fill.start.y as f64),
                    KurboPoint::new(fill.end.x as f64, fill.end.y as f64),
                );
                gradient
                    .stops
                    .push((0.0, color_from_rgba(fill.start_color)).into());
                gradient
                    .stops
                    .push((1.0, color_from_rgba(fill.end_color)).into());
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &gradient,
                    None,
                    &to_kurbo_rect(fill.rect),
                );
            }
            Primitive::Image(draw) => {
                encode_image(
                    scene,
                    Arc::clone(draw.image.shared_pixels()),
                    draw.image.width(),
                    draw.image.height(),
                    None,
                    draw.rect,
                );
            }
            Primitive::Svg(draw) => {
                encode_svg(
                    scene,
                    &PaintSvg {
                        widget_id: 0,
                        document: draw.document.clone(),
                        rect: draw.rect,
                    },
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &frame.text_runs);
}

fn paintable_circle(center: Point, radius: f32) -> bool {
    center.is_finite() && radius.is_finite() && radius > 0.0
}

fn paintable_gradient(rect: UiRect, start: Point, end: Point) -> bool {
    rect.has_finite_positive_area() && start.is_finite() && end.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paintable_circle_rejects_nonfinite_or_empty_geometry() {
        assert!(paintable_circle(Point::new(1.0, 2.0), 3.0));
        assert!(!paintable_circle(Point::new(f32::NAN, 2.0), 3.0));
        assert!(!paintable_circle(Point::new(1.0, 2.0), 0.0));
        assert!(!paintable_circle(Point::new(1.0, 2.0), f32::INFINITY));
    }

    #[test]
    fn paintable_gradient_rejects_invalid_rect_or_points() {
        let rect = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let empty = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(0.0, 10.0));

        assert!(paintable_gradient(
            rect,
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0)
        ));
        assert!(!paintable_gradient(
            empty,
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0)
        ));
        assert!(!paintable_gradient(
            rect,
            Point::new(f32::NAN, 0.0),
            Point::new(10.0, 10.0)
        ));
    }
}
