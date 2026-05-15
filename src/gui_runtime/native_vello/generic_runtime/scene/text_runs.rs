use super::RetainedSurfaceEncodeStats;
use crate::gui_runtime::native_vello::*;
use std::mem::ManuallyDrop;

const INLINE_SCENE_TEXT_RUNS: usize = 64;

pub(in crate::gui_runtime::native_vello) struct SceneTextRunBuffer<'a> {
    inline: [Option<SceneTextRun<'a>>; INLINE_SCENE_TEXT_RUNS],
    len: usize,
    overflow: Vec<SceneTextRun<'a>>,
}

impl Default for SceneTextRunBuffer<'static> {
    fn default() -> Self {
        Self::with_overflow_capacity(0)
    }
}

impl<'a> SceneTextRunBuffer<'a> {
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
        // `len` is the replay boundary, so stale inline copies are intentionally
        // left in place to avoid per-flush slot writes around clip layers.
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

    fn has_overflow(&self) -> bool {
        !self.overflow.is_empty()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn overflow_capacity(&self) -> usize {
        self.overflow.capacity()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn first_for_test(
        &self,
    ) -> Option<SceneTextRun<'a>> {
        self.inline[..self.len]
            .iter()
            .flatten()
            .next()
            .copied()
            .or_else(|| self.overflow.first().copied())
    }

    pub(in crate::gui_runtime::native_vello) fn rebind<'next>(
        mut self,
    ) -> SceneTextRunBuffer<'next> {
        self.clear();
        let mut this = ManuallyDrop::new(self);
        let capacity = this.overflow.capacity();
        let ptr = this.overflow.as_mut_ptr();
        // The buffer is always cleared before rebinding, so the overflow vector
        // contains no borrowed text runs when its allocation is reused for the
        // next paint plan lifetime.
        let overflow =
            unsafe { Vec::from_raw_parts(ptr.cast::<SceneTextRun<'next>>(), 0, capacity) };
        SceneTextRunBuffer {
            inline: [None; INLINE_SCENE_TEXT_RUNS],
            len: 0,
            overflow,
        }
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
    if text_runs.has_overflow() {
        text_renderer.draw_scene_text_runs(scene, text_runs.overflow.iter().copied());
    }
    text_runs.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rgba8};

    fn text_run(text: &str) -> SceneTextRun<'_> {
        SceneTextRun {
            text,
            position: Point::new(0.0, 0.0),
            font_size: 12.0,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            max_width: None,
            align: TextAlign::Left,
        }
    }

    #[test]
    fn text_run_buffer_reports_overflow_only_after_inline_capacity() {
        let mut text_runs = SceneTextRunBuffer::new();

        for _ in 0..INLINE_SCENE_TEXT_RUNS {
            text_runs.push(text_run("inline"));
        }

        assert_eq!(text_runs.len(), INLINE_SCENE_TEXT_RUNS);
        assert!(!text_runs.has_overflow());

        text_runs.push(text_run("overflow"));

        assert_eq!(text_runs.len(), INLINE_SCENE_TEXT_RUNS + 1);
        assert!(text_runs.has_overflow());
    }

    #[test]
    fn clear_resets_inline_and_overflow_runs_for_reuse() {
        let mut text_runs = SceneTextRunBuffer::new();
        for _ in 0..=INLINE_SCENE_TEXT_RUNS {
            text_runs.push(text_run("run"));
        }

        text_runs.clear();

        assert!(text_runs.is_empty());
        assert!(!text_runs.has_overflow());
        text_runs.push(text_run("next"));
        assert_eq!(text_runs.len(), 1);
        assert!(!text_runs.has_overflow());
    }

    #[test]
    fn rebind_preserves_overflow_storage_for_next_paint_plan_lifetime() {
        let mut text_runs = SceneTextRunBuffer::with_overflow_capacity(8);
        for _ in 0..=INLINE_SCENE_TEXT_RUNS {
            text_runs.push(text_run("run"));
        }
        let capacity = text_runs.overflow_capacity();

        let rebound: SceneTextRunBuffer<'static> = text_runs.rebind();

        assert!(rebound.is_empty());
        assert!(rebound.overflow_capacity() >= capacity);
    }
}
