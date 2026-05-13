use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_svg(
    scene: &mut Scene,
    svg: &PaintSvg,
) {
    let size = svg.document.tree().size();
    let width = size.width().max(f32::EPSILON);
    let height = size.height().max(f32::EPSILON);
    if svg.rect.width() <= 0.0 || svg.rect.height() <= 0.0 {
        return;
    }

    let transform = Affine::translate((svg.rect.min.x as f64, svg.rect.min.y as f64))
        * Affine::scale_non_uniform(
            svg.rect.width() as f64 / width as f64,
            svg.rect.height() as f64 / height as f64,
        );
    let source_bounds = vello::kurbo::Rect::new(0.0, 0.0, width as f64, height as f64);
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
