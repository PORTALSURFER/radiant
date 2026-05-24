use crate::{gui::types::Rect as UiRect, runtime::PaintSvg};
use kurbo::Rect as KurboRect;
use vello::{
    Scene,
    kurbo::Affine,
    peniko::{BlendMode, Fill},
};

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_svg(
    scene: &mut Scene,
    svg: &PaintSvg,
) {
    let size = svg.document.tree().size();
    let Some((transform, source_bounds)) = svg_transform(svg.rect, size.width(), size.height())
    else {
        return;
    };

    scene.push_layer(
        Fill::NonZero,
        BlendMode::default(),
        1.0,
        transform,
        &source_bounds,
    );
    vello_svg::append_tree_with(scene, svg.document.tree(), &mut |_, _| {});
    scene.pop_layer();
}

fn svg_transform(
    rect: UiRect,
    document_width: f32,
    document_height: f32,
) -> Option<(Affine, KurboRect)> {
    if !rect.has_finite_positive_area()
        || !document_width.is_finite()
        || !document_height.is_finite()
    {
        return None;
    }
    let width = document_width.max(f32::EPSILON);
    let height = document_height.max(f32::EPSILON);
    let scale_x = rect.width() as f64 / width as f64;
    let scale_y = rect.height() as f64 / height as f64;
    if !scale_x.is_finite() || !scale_y.is_finite() {
        return None;
    }
    Some((
        Affine::translate((rect.min.x as f64, rect.min.y as f64))
            * Affine::scale_non_uniform(scale_x, scale_y),
        KurboRect::new(0.0, 0.0, width as f64, height as f64),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Point;

    #[test]
    fn svg_transform_rejects_nonfinite_or_empty_geometry() {
        let rect = UiRect::from_min_max(Point::new(10.0, 20.0), Point::new(30.0, 60.0));
        let empty = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(0.0, 1.0));
        let nonfinite = UiRect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(1.0, 1.0));

        assert!(svg_transform(rect, 100.0, 50.0).is_some());
        assert!(svg_transform(empty, 100.0, 50.0).is_none());
        assert!(svg_transform(nonfinite, 100.0, 50.0).is_none());
        assert!(svg_transform(rect, f32::NAN, 50.0).is_none());
        assert!(svg_transform(rect, 100.0, f32::INFINITY).is_none());
    }
}
