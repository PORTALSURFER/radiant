//! Native text rendering for Vello scenes.

use super::NativeTextOptions;
use crate::gui::{
    paint::{TextAlign, TextRun},
    types::{Point, Rect, Rgba8},
};
use crate::widgets::{TextWrap, WidgetId};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
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

/// Exact renderer-local fence for one frame/plan text-input publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct NativeTextInputSnapshotFence {
    frame_token: u64,
    plan_token: u64,
}

impl NativeTextInputSnapshotFence {
    pub(crate) const fn new(frame_token: u64, plan_token: u64) -> Self {
        Self {
            frame_token,
            plan_token,
        }
    }
}

/// Checked monotonic fence allocator for native text-input frame/plan
/// preparation. Each allocation identifies one real preparation boundary; the
/// sidecar never needs to guess whether a plan belongs to a later frame.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "frame integration is intentionally deferred to the production call-path slice"
    )
)]
#[derive(Debug)]
pub(crate) struct NativeTextInputSnapshotFenceAllocator {
    next_frame_token: Option<u64>,
    next_plan_token: Option<u64>,
}

impl Default for NativeTextInputSnapshotFenceAllocator {
    fn default() -> Self {
        Self {
            next_frame_token: Some(1),
            next_plan_token: Some(1),
        }
    }
}

impl NativeTextInputSnapshotFenceAllocator {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "frame integration is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn allocate(&mut self) -> Option<NativeTextInputSnapshotFence> {
        let frame_token = self.next_frame_token?;
        let plan_token = self.next_plan_token?;
        self.next_frame_token = frame_token.checked_add(1);
        self.next_plan_token = plan_token.checked_add(1);
        Some(NativeTextInputSnapshotFence::new(frame_token, plan_token))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeTextInputFontKey {
    size_bits: u32,
    generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeTextInputConstraintsKey {
    available_width_bits: Option<u32>,
    align: TextAlign,
    wrap: TextWrap,
    break_policy_id: LineBreakPolicyId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeTextInputRectKey {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl From<Rect> for NativeTextInputRectKey {
    fn from(rect: Rect) -> Self {
        Self {
            min_x: rect.min.x.to_bits(),
            min_y: rect.min.y.to_bits(),
            max_x: rect.max.x.to_bits(),
            max_y: rect.max.y.to_bits(),
        }
    }
}

/// Private identity for one retained text-input paragraph snapshot.
///
/// The content revision and widget identity prevent cross-input reuse.  Font,
/// view constraints, and the complete input rectangle cover geometry inputs;
/// the frame/plan fence prevents this bounded sidecar from becoming an
/// unscoped cross-frame cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct NativeTextInputSnapshotKey {
    widget_id: WidgetId,
    source_identity: u64,
    content_revision: u64,
    font: NativeTextInputFontKey,
    constraints: NativeTextInputConstraintsKey,
    rect: NativeTextInputRectKey,
    fence: NativeTextInputSnapshotFence,
}

impl NativeTextInputSnapshotKey {
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn new(
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        font_generation: u64,
        available_width: Option<f32>,
        align: TextAlign,
        wrap: TextWrap,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Self {
        Self {
            widget_id,
            source_identity: model::source_identity(text),
            content_revision: model::source_revision(text, font_size.to_bits(), font_generation),
            font: NativeTextInputFontKey {
                size_bits: font_size.to_bits(),
                generation: font_generation,
            },
            constraints: NativeTextInputConstraintsKey {
                available_width_bits: available_width.map(f32::to_bits),
                align,
                wrap,
                break_policy_id: model::LINE_BREAK_POLICY_ID,
            },
            rect: rect.into(),
            fence,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RetainedTextInputSnapshot {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) key: NativeTextInputSnapshotKey,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) snapshot: Arc<ParagraphSnapshot>,
}

#[derive(Default)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "text-input publication is intentionally deferred to the production call-path slice"
    )
)]
pub(crate) struct RetainedTextInputSnapshotSidecar {
    // Text inputs are normally few, but a plan can contain more than one.
    // Keeping a fixed FIFO collection preserves all ordinary current-plan
    // inputs without turning this renderer-local seam into an unbounded cache.
    fence: Option<NativeTextInputSnapshotFence>,
    entries: VecDeque<RetainedTextInputSnapshot>,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "text-input publication is intentionally deferred to the production call-path slice"
    )
)]
pub(crate) const RETAINED_TEXT_INPUT_SNAPSHOT_CAPACITY: usize = 16;

impl RetainedTextInputSnapshotSidecar {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn begin_fence(&mut self, fence: NativeTextInputSnapshotFence) {
        if self.fence != Some(fence) {
            self.fence = Some(fence);
            self.entries.clear();
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn snapshot_for(
        &mut self,
        key: &NativeTextInputSnapshotKey,
    ) -> Option<Arc<ParagraphSnapshot>> {
        if self.fence != Some(key.fence) {
            self.entries.clear();
            return None;
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.key == *key) {
            return Some(entry.snapshot.clone());
        }
        // A changed source, widget, font, constraint, or rectangle must not
        // leave a stale entry for the same input identity available to a later
        // lookup in this preparation boundary.
        self.entries
            .retain(|entry| entry.key.widget_id != key.widget_id);
        None
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn retain(
        &mut self,
        key: NativeTextInputSnapshotKey,
        snapshot: Arc<ParagraphSnapshot>,
    ) {
        self.begin_fence(key.fence);
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            entry.snapshot = snapshot;
            return;
        }
        self.entries
            .retain(|entry| entry.key.widget_id != key.widget_id);
        if self.entries.len() >= RETAINED_TEXT_INPUT_SNAPSHOT_CAPACITY {
            self.entries.pop_front();
        }
        self.entries
            .push_back(RetainedTextInputSnapshot { key, snapshot });
    }
}

pub(crate) struct NativeTextRenderer {
    font_stack: NativeFontStack,
    layout_cache: TextLayoutCache,
    native_caret_affinities: HashMap<crate::widgets::WidgetId, CaretAffinity>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) retained_text_input_snapshot: RetainedTextInputSnapshotSidecar,
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
            retained_text_input_snapshot: RetainedTextInputSnapshotSidecar::default(),
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

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn begin_text_input_snapshot_fence(&mut self, fence: NativeTextInputSnapshotFence) {
        self.retained_text_input_snapshot.begin_fence(fence);
    }

    /// Return the current private text-input snapshot when its full fence matches.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn text_input_snapshot(
        &mut self,
        key: &NativeTextInputSnapshotKey,
    ) -> Option<Arc<ParagraphSnapshot>> {
        self.retained_text_input_snapshot.snapshot_for(key)
    }

    /// Retain one private text-input snapshot for the current frame/plan seam.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn retain_text_input_snapshot(
        &mut self,
        key: NativeTextInputSnapshotKey,
        snapshot: Arc<ParagraphSnapshot>,
    ) {
        self.retained_text_input_snapshot.retain(key, snapshot);
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "text-input publication is intentionally deferred to the production call-path slice"
        )
    )]
    pub(crate) fn retain_or_build_text_input_snapshot(
        &mut self,
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<Arc<ParagraphSnapshot>> {
        let available_width = Some(rect.width().max(0.0));
        let key = NativeTextInputSnapshotKey::new(
            widget_id,
            text,
            font_size,
            self.font_stack.generation(),
            available_width,
            TextAlign::Left,
            TextWrap::None,
            rect,
            fence,
        );
        if let Some(snapshot) = self.text_input_snapshot(&key)
            && snapshot.source_identity == key.source_identity
            && snapshot.matches_source(text, key.content_revision)
            && snapshot.is_usable_for(font_size)
            && snapshot.available_width.map(f32::to_bits) == key.constraints.available_width_bits
        {
            return Some(snapshot);
        }
        let snapshot = self
            .layout_text_view(
                text,
                font_size,
                available_width,
                TextAlign::Left,
                TextWrap::None,
            )?
            .snapshot();
        if snapshot.source_identity != key.source_identity
            || !snapshot.matches_source(text, key.content_revision)
            || !snapshot.is_usable_for(font_size)
            || snapshot.available_width.map(f32::to_bits) != key.constraints.available_width_bits
        {
            return None;
        }
        self.retain_text_input_snapshot(key, snapshot.clone());
        Some(snapshot)
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
    use super::{
        CaretAffinity, NativeTextInputSnapshotFence, NativeTextInputSnapshotFenceAllocator,
        NativeTextInputSnapshotKey, NativeTextRenderer, ParagraphSnapshot,
    };
    use super::{TextCursorStop, TextLayout, visible_face_segment};
    use crate::{
        gui::{
            paint::TextAlign,
            types::{Point, Rect},
        },
        widgets::{TextWrap, WidgetId},
    };
    use std::sync::Arc;

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
    fn retained_text_input_snapshot_reuses_arc_after_transient_view_reset() {
        let mut renderer = NativeTextRenderer::new();
        renderer
            .layout_cache
            .set_view_cache_byte_budget_override(Some(1));
        let text = "A";
        let first = renderer
            .layout_text_view(text, 20.0, Some(240.0), TextAlign::Left, TextWrap::None)
            .expect("oversized text input layout should be available")
            .snapshot();

        let key = NativeTextInputSnapshotKey::new(
            WidgetId::from(7_u32),
            text,
            20.0,
            renderer.font_stack.generation(),
            Some(240.0),
            TextAlign::Left,
            TextWrap::None,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(248.0, 38.0)),
            NativeTextInputSnapshotFence::new(4, 9),
        );
        renderer.retain_text_input_snapshot(key, first.clone());

        let replacement = renderer
            .layout_text_view(text, 20.0, Some(240.0), TextAlign::Left, TextWrap::None)
            .expect("transient view should be rebuilt")
            .snapshot();
        assert!(!Arc::ptr_eq(&first, &replacement));

        let retained = renderer
            .text_input_snapshot(&key)
            .expect("matching fence should reuse the retained snapshot");
        assert!(Arc::ptr_eq(&first, &retained));
    }

    #[test]
    fn retained_text_input_snapshot_invalidates_on_fence_mismatch() {
        let mut renderer = NativeTextRenderer::new();
        let snapshot = ParagraphSnapshot::empty("text");
        let key = NativeTextInputSnapshotKey::new(
            WidgetId::from(7_u32),
            "text",
            20.0,
            renderer.font_stack.generation(),
            Some(240.0),
            TextAlign::Left,
            TextWrap::None,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(248.0, 38.0)),
            NativeTextInputSnapshotFence::new(4, 9),
        );
        renderer.retain_text_input_snapshot(key, snapshot);

        let mismatched_key = NativeTextInputSnapshotKey::new(
            WidgetId::from(7_u32),
            "text",
            20.0,
            key.font.generation,
            Some(240.0),
            TextAlign::Left,
            TextWrap::None,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(248.0, 38.0)),
            NativeTextInputSnapshotFence::new(4, 10),
        );
        assert!(renderer.text_input_snapshot(&mismatched_key).is_none());
        assert!(renderer.text_input_snapshot(&key).is_none());
    }

    #[test]
    fn retained_text_input_snapshot_prepares_one_bounded_current_plan_entry() {
        let mut renderer = NativeTextRenderer::new();
        let mut fences = NativeTextInputSnapshotFenceAllocator::default();
        let fence = fences.allocate().expect("first preparation fence");
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(248.0, 38.0));

        renderer.begin_text_input_snapshot_fence(fence);
        let snapshot = renderer
            .retain_or_build_text_input_snapshot(WidgetId::from(7_u32), "A", 20.0, rect, fence)
            .expect("text input snapshot should be available");

        assert!(
            renderer
                .text_input_snapshot(&NativeTextInputSnapshotKey::new(
                    WidgetId::from(7_u32),
                    "A",
                    20.0,
                    renderer.font_stack.generation(),
                    Some(240.0),
                    TextAlign::Left,
                    TextWrap::None,
                    rect,
                    fence,
                ))
                .is_some()
        );
        assert_eq!(snapshot.available_width, Some(240.0));
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
