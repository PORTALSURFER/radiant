use super::{RetainedSurfaceEncodeStats, encode_image, encode_rect};
use crate::gui_runtime::native_vello::*;

const INLINE_SCENE_TEXT_RUNS: usize = 64;

pub(in crate::gui_runtime::native_vello) struct SceneTextRunBuffer<'a> {
    inline: [Option<SceneTextRun<'a>>; INLINE_SCENE_TEXT_RUNS],
    len: usize,
    overflow: Vec<SceneTextRun<'a>>,
}

impl<'a> SceneTextRunBuffer<'a> {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn new() -> Self {
        Self::with_overflow_capacity(0)
    }

    pub(in crate::gui_runtime::native_vello) fn with_overflow_capacity(
        overflow_capacity: usize,
    ) -> Self {
        Self {
            inline: [None; INLINE_SCENE_TEXT_RUNS],
            len: 0,
            overflow: Vec::with_capacity(overflow_capacity),
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn clear(&mut self) {
        for slot in &mut self.inline[..self.len] {
            *slot = None;
        }
        self.len = 0;
        self.overflow.clear();
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn push(
        &mut self,
        run: SceneTextRun<'a>,
    ) {
        if self.len < INLINE_SCENE_TEXT_RUNS {
            self.inline[self.len] = Some(run);
            self.len += 1;
            return;
        }
        self.overflow.push(run);
    }

    pub(in crate::gui_runtime::native_vello) fn is_empty(&self) -> bool {
        self.len == 0 && self.overflow.is_empty()
    }

    fn len(&self) -> usize {
        self.len + self.overflow.len()
    }

    pub(in crate::gui_runtime::native_vello) fn overflow_capacity(&self) -> usize {
        self.overflow.capacity()
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn flush_text_runs(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    text_runs: &mut SceneTextRunBuffer<'_>,
    stats: &mut RetainedSurfaceEncodeStats,
) {
    if text_runs.is_empty() {
        return;
    }
    stats.record_text_runs(text_runs.len());
    text_renderer.draw_scene_text_runs(
        scene,
        text_runs.inline[..text_runs.len].iter().flatten().copied(),
    );
    text_renderer.draw_scene_text_runs(scene, text_runs.overflow.iter().copied());
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
