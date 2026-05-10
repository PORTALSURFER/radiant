use super::{RetainedSurfaceEncodeStats, encode_image, encode_rect};
use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn flush_text_runs(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    text_runs: &mut Vec<SceneTextRun<'_>>,
    stats: &mut RetainedSurfaceEncodeStats,
) {
    if text_runs.is_empty() {
        return;
    }
    stats.record_text_runs(text_runs.len());
    text_renderer.draw_scene_text_runs(scene, text_runs.iter().copied());
    text_runs.clear();
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_paint_frame_to_scene(
    frame: &PaintFrame,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
) {
    for primitive in frame.primitives.iter() {
        match primitive {
            Primitive::Rect(fill) => encode_rect(scene, fill.color, fill.rect),
            Primitive::Circle(fill) => {
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
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    None,
                    draw.rect,
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &frame.text_runs);
}
