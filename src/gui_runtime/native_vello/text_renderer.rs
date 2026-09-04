//! Native text rendering for Vello scenes.

use super::NativeTextOptions;
use crate::gui::{
    paint::{TextAlign, TextRun},
    types::{Point, Rgba8},
};
use crate::widgets::TextWrap;
use std::collections::HashMap;
use vello::{Glyph, Scene, peniko::Fill};

mod cache;
mod encoding;
mod font;
mod layout;
mod model;
mod renderability;

use cache::TextLayoutCache;
pub(in crate::gui_runtime::native_vello) use cache::TextLayoutProfileCounters;
pub(super) use encoding::{color_from_rgba, icon_from_rgba, to_kurbo_rect};
use font::NativeFontStack;
pub(in crate::gui_runtime::native_vello) use model::{
    BidiDirection, BidiRun, CaretAffinity, CaretStopGeometry, GlyphPlacement, GraphemeBoundary,
    GraphemeGeometry, LineBreakKind, LineBreakPolicyId, LineBreakRecord, ParagraphSnapshot,
    ResolvedFontRun, ScalarBoundary, SceneTextRun, ShapeClusterRange, ShapedParagraph,
    SnapshotQuality, TextLayout, TextLayoutKey, TextQuality, TextViewKey, Utf8ByteOffset,
};
#[cfg(test)]
pub(in crate::gui_runtime::native_vello) use model::{GlyphLayout, TextCursorStop};
pub(in crate::gui_runtime::native_vello) use renderability::font_size_is_renderable;
use renderability::text_run_is_renderable;
use renderability::text_run_parts_are_renderable;

pub(super) struct NativeTextRenderer {
    font_stack: NativeFontStack,
    layout_cache: TextLayoutCache,
    native_caret_affinities: HashMap<crate::widgets::WidgetId, CaretAffinity>,
}

impl NativeTextRenderer {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::with_options(&NativeTextOptions::default())
    }

    pub(super) fn with_options(options: &NativeTextOptions) -> Self {
        let font_stack = NativeFontStack::with_options(options);
        if font_stack.is_empty() {
            tracing::warn!(
                "Native vello text renderer found no fallback font; text runs will be skipped"
            );
        }
        Self {
            font_stack,
            layout_cache: TextLayoutCache::new(),
            native_caret_affinities: HashMap::new(),
        }
    }

    pub(super) fn set_native_caret_affinity(
        &mut self,
        widget_id: crate::widgets::WidgetId,
        affinity: CaretAffinity,
    ) {
        self.native_caret_affinities.insert(widget_id, affinity);
    }

    pub(super) fn native_caret_affinity(
        &self,
        widget_id: crate::widgets::WidgetId,
    ) -> CaretAffinity {
        self.native_caret_affinities
            .get(&widget_id)
            .copied()
            .unwrap_or(CaretAffinity::Downstream)
    }

    pub(super) fn reset_native_caret_affinities(&mut self) {
        self.native_caret_affinities.clear();
    }

    pub(super) fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        self.draw_scene_text_runs(scene, text_runs.iter().map(SceneTextRun::from));
    }

    pub(super) fn draw_scene_text_runs(
        &mut self,
        scene: &mut Scene,
        text_runs: impl IntoIterator<Item = SceneTextRun>,
    ) {
        let layout_cache = &mut self.layout_cache;
        for run in text_runs {
            if !text_run_is_renderable(&run) {
                continue;
            }
            draw_text_run_with_font(
                scene,
                &mut self.font_stack,
                layout_cache,
                run.text.as_ref(),
                TextRunParts {
                    position: run.position,
                    font_size: run.font_size,
                    color: run.color,
                    max_width: run.max_width,
                    align: run.align,
                    wrap: run.wrap,
                },
            );
        }
    }

    pub(super) fn draw_text_run(&mut self, scene: &mut Scene, text: &str, parts: TextRunParts) {
        if !text_run_parts_are_renderable(text, parts.position, parts.font_size, parts.max_width) {
            return;
        }
        draw_text_run_with_font(
            scene,
            &mut self.font_stack,
            &mut self.layout_cache,
            text,
            parts,
        );
    }

    pub(super) fn layout_text(&mut self, text: &str, font_size: f32) -> Option<&TextLayout> {
        if !font_size_is_renderable(font_size) {
            return None;
        }
        self.layout_cache
            .layout_for(&mut self.font_stack, text, font_size)
    }

    pub(super) fn layout_text_view(
        &mut self,
        text: &str,
        font_size: f32,
        available_width: Option<f32>,
        align: TextAlign,
        wrap: TextWrap,
    ) -> Option<&TextLayout> {
        if !font_size_is_renderable(font_size) {
            return None;
        }
        self.layout_cache.layout_for_view(
            &mut self.font_stack,
            text,
            font_size,
            available_width,
            align,
            wrap,
        )
    }

    pub(super) fn draw_paragraph_snapshot(
        &mut self,
        scene: &mut Scene,
        snapshot: &ParagraphSnapshot,
        paint: TextSnapshotPaint,
    ) {
        if !font_size_is_renderable(paint.font_size)
            || !paint.position.x.is_finite()
            || !paint.position.y.is_finite()
            || !paint.clip_width.is_finite()
            || paint.clip_width <= 0.0
            || !paint.scroll_x.is_finite()
        {
            return;
        }
        if !snapshot.is_usable_for(paint.font_size) {
            return;
        }
        draw_snapshot_glyphs(
            scene,
            &mut self.font_stack,
            snapshot,
            TextSnapshotPaint {
                scroll_x: paint.scroll_x.max(0.0),
                ..paint
            },
        );
    }

    pub(super) fn take_layout_profile_counters(&mut self) -> TextLayoutProfileCounters {
        self.layout_cache.take_profile_counters()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TextRunParts {
    pub(super) position: Point,
    pub(super) font_size: f32,
    pub(super) color: Rgba8,
    pub(super) max_width: Option<f32>,
    pub(super) align: TextAlign,
    pub(super) wrap: TextWrap,
}

fn draw_text_run_with_font(
    scene: &mut Scene,
    font_stack: &mut NativeFontStack,
    layout_cache: &mut TextLayoutCache,
    text: &str,
    parts: TextRunParts,
) {
    let Some(layout) = layout_cache.layout_for_view(
        font_stack,
        text,
        parts.font_size,
        parts.max_width,
        parts.align,
        parts.wrap,
    ) else {
        return;
    };
    let clip_width = parts.max_width.unwrap_or(f32::INFINITY);
    draw_snapshot_glyphs(
        scene,
        font_stack,
        &layout.snapshot,
        TextSnapshotPaint {
            position: parts.position,
            font_size: parts.font_size,
            color: parts.color,
            clip_width,
            scroll_x: 0.0,
        },
    );
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TextSnapshotPaint {
    pub(super) position: Point,
    pub(super) font_size: f32,
    pub(super) color: Rgba8,
    pub(super) clip_width: f32,
    pub(super) scroll_x: f32,
}

fn draw_snapshot_glyphs(
    scene: &mut Scene,
    font_stack: &mut NativeFontStack,
    snapshot: &ParagraphSnapshot,
    paint: TextSnapshotPaint,
) {
    if !snapshot.is_usable_for(paint.font_size) {
        return;
    }
    let baseline = paint.position.y + paint.font_size;
    let right = paint.scroll_x + paint.clip_width;
    let mut segment_face = None;
    let mut segment = Vec::new();

    let flush = |scene: &mut Scene, face_index: Option<usize>, glyphs: &mut Vec<Glyph>| {
        let Some(face_index) = face_index else {
            glyphs.clear();
            return;
        };
        let Some(font_data) = font_stack.face(face_index) else {
            glyphs.clear();
            return;
        };
        scene
            .draw_glyphs(font_data)
            .font_size(paint.font_size)
            .brush(color_from_rgba(paint.color))
            .draw(Fill::NonZero, glyphs.drain(..));
    };

    for glyph in &snapshot.glyphs {
        let glyph_left = glyph.x.min(glyph.x + glyph.advance);
        let glyph_right = glyph.x.max(glyph.x + glyph.advance);
        let visible = glyph_right >= paint.scroll_x && glyph_left <= right;
        if !visible {
            flush(scene, segment_face.take(), &mut segment);
            continue;
        }
        if segment_face != Some(glyph.face_index) {
            flush(scene, segment_face.replace(glyph.face_index), &mut segment);
        }
        segment.push(Glyph {
            id: glyph.glyph_id,
            x: paint.position.x + glyph.x + glyph.x_offset - paint.scroll_x,
            y: baseline + glyph.y_offset,
        });
    }
    flush(scene, segment_face, &mut segment);
}

#[cfg(test)]
fn visible_face_segment(
    glyphs: &[GlyphLayout],
    start: usize,
    clip_width: f32,
) -> Option<(usize, usize)> {
    let first = glyphs.get(start)?;
    if first.x > clip_width {
        return None;
    }
    let face_index = first.face_index;
    let mut end = start + 1;
    while end < glyphs.len() && glyphs[end].face_index == face_index && glyphs[end].x <= clip_width
    {
        end += 1;
    }
    Some((face_index, end))
}

#[cfg(test)]
mod tests {
    use super::GlyphLayout;
    use super::{CaretAffinity, NativeTextRenderer};
    use super::{TextCursorStop, TextLayout, visible_face_segment};
    use crate::widgets::WidgetId;

    #[test]
    fn native_pointer_affinity_resets_to_downstream() {
        let mut renderer = NativeTextRenderer::new();
        renderer.set_native_caret_affinity(WidgetId::from(7_u32), CaretAffinity::Upstream);
        assert_eq!(
            renderer.native_caret_affinity(WidgetId::from(7_u32)),
            CaretAffinity::Upstream
        );
        renderer.reset_native_caret_affinities();
        assert_eq!(
            renderer.native_caret_affinity(WidgetId::from(7_u32)),
            CaretAffinity::Downstream
        );
    }

    #[test]
    fn empty_layout_preserves_terminal_cursor_stop() {
        let layout = TextLayout::empty_for("tempo");
        assert_eq!(layout.width, 0.0);
        assert!(layout.glyphs.is_empty());
        assert_eq!(
            layout.cursor_stops,
            vec![TextCursorStop {
                byte_index: 5,
                x: 0.0,
            }]
        );
    }

    #[test]
    fn visible_glyph_segments_follow_face_boundaries_and_clip_width() {
        let glyphs = vec![
            GlyphLayout {
                face_index: 0,
                id: 1,
                x: 0.0,
            },
            GlyphLayout {
                face_index: 1,
                id: 2,
                x: 5.0,
            },
            GlyphLayout {
                face_index: 0,
                id: 3,
                x: 10.0,
            },
        ];
        let mut start = 0;
        let mut segments = Vec::new();
        while let Some((face, end)) = visible_face_segment(&glyphs, start, 9.0) {
            segments.push((face, start..end));
            start = end;
        }

        assert_eq!(
            glyphs.iter().map(|glyph| glyph.id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(segments, vec![(0, 0..1), (1, 1..2)]);
    }
}
