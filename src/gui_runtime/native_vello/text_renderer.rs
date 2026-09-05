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

pub(in crate::gui_runtime::native_vello) use cache::TextLayoutProfileCounters;
use cache::{TextLayoutCache, VIEW_CACHE_BYTE_BUDGET, VIEW_CACHE_ENTRY_BUDGET};
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
    pub(crate) key: NativeTextInputSnapshotKey,
    pub(crate) snapshot: Arc<ParagraphSnapshot>,
    bytes: usize,
}

#[derive(Default)]
pub(crate) struct RetainedTextInputSnapshotSidecar {
    // Text inputs are normally few, but a plan can contain more than one. The
    // view-cache budgets bound this plan-local retention just like the normal
    // width/view cache. Replacing the fence drops every entry from the prior
    // plan.
    fence: Option<NativeTextInputSnapshotFence>,
    entries: VecDeque<RetainedTextInputSnapshot>,
    bytes: usize,
    #[cfg(test)]
    entry_budget_override: Option<usize>,
    #[cfg(test)]
    byte_budget_override: Option<usize>,
}

impl RetainedTextInputSnapshotSidecar {
    pub(crate) fn begin_fence(&mut self, fence: NativeTextInputSnapshotFence) {
        if self.fence != Some(fence) {
            self.fence = Some(fence);
            self.clear_entries();
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.fence = None;
        self.clear_entries();
    }

    fn clear_entries(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    fn entry_budget(&self) -> usize {
        #[cfg(test)]
        if let Some(budget) = self.entry_budget_override {
            return budget;
        }
        VIEW_CACHE_ENTRY_BUDGET
    }

    fn byte_budget(&self) -> usize {
        #[cfg(test)]
        if let Some(budget) = self.byte_budget_override {
            return budget;
        }
        VIEW_CACHE_BYTE_BUDGET
    }

    fn remove_entries_for_widget(&mut self, widget_id: WidgetId) {
        self.entries
            .retain(|entry| entry.key.widget_id != widget_id);
        self.recompute_bytes();
    }

    fn take_entry(&mut self, index: usize) -> Option<Arc<ParagraphSnapshot>> {
        let entry = self.entries.remove(index)?;
        let snapshot = entry.snapshot.clone();
        // VecDeque order is the deterministic LRU order: the front is the
        // least recently used entry and every successful lookup moves its
        // entry to the back.
        self.entries.push_back(entry);
        Some(snapshot)
    }

    #[cfg(test)]
    pub(super) fn set_entry_budget_override(&mut self, budget: Option<usize>) {
        self.entry_budget_override = budget;
    }

    #[cfg(test)]
    pub(super) fn set_byte_budget_override(&mut self, budget: Option<usize>) {
        self.byte_budget_override = budget;
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy keyed snapshot lookup remains covered by text-renderer tests"
        )
    )]
    pub(crate) fn snapshot_for(
        &mut self,
        key: &NativeTextInputSnapshotKey,
    ) -> Option<Arc<ParagraphSnapshot>> {
        if self.fence != Some(key.fence) {
            // A stale consumer may not erase the currently active plan. The
            // next preparation boundary owns replacement through
            // `begin_fence` (or explicit invalidation).
            return None;
        }
        if let Some(entry_index) = self.entries.iter().position(|entry| entry.key == *key) {
            return self.take_entry(entry_index);
        }
        // A changed source, widget, font, constraint, or rectangle must not
        // leave a stale entry for the same input identity available to a later
        // lookup in this preparation boundary.
        self.remove_entries_for_widget(key.widget_id);
        None
    }

    /// Return a current-fence entry without rebuilding its key from the live
    /// font stack. The seed key owns the font generation used to build the
    /// snapshot, so later consumers can safely run after lazy font discovery.
    pub(crate) fn snapshot_for_input(
        &mut self,
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        align: TextAlign,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<Arc<ParagraphSnapshot>> {
        if self.fence != Some(fence) {
            // Do not let an old pointer/IME consumer clear a newer plan's
            // entries. Fence replacement is performed only by begin_fence.
            return None;
        }
        let available_width = Some(rect.width().max(0.0));
        let source_identity = model::source_identity(text);
        let entry_index = self.entries.iter().position(|entry| {
            let key = &entry.key;
            key.widget_id == widget_id
                && key.fence == fence
                && key.source_identity == source_identity
                && key.font.size_bits == font_size.to_bits()
                && key.constraints.available_width_bits == available_width.map(f32::to_bits)
                && key.constraints.align == align
                && key.constraints.wrap == TextWrap::None
                && key.rect == rect.into()
        });
        let Some(entry_index) = entry_index else {
            self.remove_entries_for_widget(widget_id);
            return None;
        };
        let entry = &self.entries[entry_index];
        if entry
            .snapshot
            .matches_source(text, entry.key.content_revision)
            && entry.snapshot.is_usable_for(font_size)
            && entry.snapshot.available_width.map(f32::to_bits)
                == entry.key.constraints.available_width_bits
        {
            return self.take_entry(entry_index);
        }
        self.remove_entries_for_widget(widget_id);
        None
    }

    pub(crate) fn retain(
        &mut self,
        key: NativeTextInputSnapshotKey,
        snapshot: Arc<ParagraphSnapshot>,
    ) -> bool {
        self.try_retain(key, snapshot)
    }

    fn try_retain(
        &mut self,
        key: NativeTextInputSnapshotKey,
        snapshot: Arc<ParagraphSnapshot>,
    ) -> bool {
        self.begin_fence(key.fence);
        let entry_budget = self.entry_budget();
        let byte_budget = self.byte_budget();
        let bytes = snapshot.estimated_bytes();

        // An entry that cannot fit is never published. Removing the previous
        // value for this widget also prevents an older geometry from becoming
        // the only answer after a failed replacement.
        self.remove_entries_for_widget(key.widget_id);
        if entry_budget == 0 || byte_budget == 0 || bytes > byte_budget {
            return false;
        }

        while self.entries.len() >= entry_budget
            || self
                .bytes
                .saturating_add(self.bytes_for_snapshot(&snapshot))
                > byte_budget
        {
            let Some(entry) = self.entries.pop_front() else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.recompute_bytes();
        }
        if self.entries.len() >= entry_budget
            || self
                .bytes
                .saturating_add(self.bytes_for_snapshot(&snapshot))
                > byte_budget
        {
            return false;
        }

        let local_bytes = snapshot.estimated_local_bytes();
        self.entries.push_back(RetainedTextInputSnapshot {
            key,
            snapshot,
            bytes: local_bytes,
        });
        self.recompute_bytes();
        true
    }

    fn recompute_bytes(&mut self) {
        let mut retained = Vec::<Arc<super::ShapedParagraph>>::new();
        let mut bytes = 0usize;
        for entry in &self.entries {
            bytes = bytes.saturating_add(entry.bytes);
            if !retained
                .iter()
                .any(|shape| Arc::ptr_eq(shape, &entry.snapshot.shaped))
            {
                retained.push(Arc::clone(&entry.snapshot.shaped));
                bytes = bytes.saturating_add(entry.snapshot.shaped.estimated_bytes());
            }
        }
        self.bytes = bytes;
    }

    fn bytes_for_snapshot(&self, snapshot: &Arc<ParagraphSnapshot>) -> usize {
        let local_bytes = snapshot.estimated_local_bytes();
        if self
            .entries
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.snapshot.shaped, &snapshot.shaped))
        {
            local_bytes
        } else {
            local_bytes.saturating_add(snapshot.shaped.estimated_bytes())
        }
    }
}

pub(crate) struct NativeTextRenderer {
    font_stack: NativeFontStack,
    layout_cache: TextLayoutCache,
    native_caret_affinities: HashMap<crate::widgets::WidgetId, CaretAffinity>,
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

    /// Bind shaping to the environment of the published plan. A changed
    /// presentation retires native input snapshots before they can be reused.
    pub(super) fn set_application_environment(
        &mut self,
        environment: &crate::application::ApplicationEnvironment,
    ) -> bool {
        let presentation = model::TextPresentation::from_environment(environment);
        if self.layout_cache.presentation == presentation {
            return false;
        }
        self.layout_cache.presentation = presentation;
        self.invalidate_text_input_snapshots();
        self.reset_native_caret_affinities();
        true
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

    pub(crate) fn begin_text_input_snapshot_fence(&mut self, fence: NativeTextInputSnapshotFence) {
        self.retained_text_input_snapshot.begin_fence(fence);
    }

    /// Return the current private text-input snapshot when its full fence matches.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy keyed snapshot lookup remains covered by text-renderer tests"
        )
    )]
    pub(crate) fn text_input_snapshot(
        &mut self,
        key: &NativeTextInputSnapshotKey,
    ) -> Option<Arc<ParagraphSnapshot>> {
        self.retained_text_input_snapshot.snapshot_for(key)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy left-aligned snapshot lookup remains a compatibility test seam"
        )
    )]
    pub(crate) fn text_input_snapshot_for_input(
        &mut self,
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<Arc<ParagraphSnapshot>> {
        self.text_input_snapshot_for_input_aligned(
            widget_id,
            text,
            font_size,
            TextAlign::Left,
            rect,
            fence,
        )
    }

    pub(crate) fn text_input_snapshot_for_input_aligned(
        &mut self,
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        align: TextAlign,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<Arc<ParagraphSnapshot>> {
        self.retained_text_input_snapshot
            .snapshot_for_input(widget_id, text, font_size, align, rect, fence)
    }

    pub(crate) fn invalidate_text_input_snapshots(&mut self) {
        self.retained_text_input_snapshot.invalidate();
    }

    /// Retain one private text-input snapshot for the current frame/plan seam.
    pub(crate) fn retain_text_input_snapshot(
        &mut self,
        key: NativeTextInputSnapshotKey,
        snapshot: Arc<ParagraphSnapshot>,
    ) -> bool {
        self.retained_text_input_snapshot.retain(key, snapshot)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy left-aligned snapshot builder remains a compatibility test seam"
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
        self.retain_or_build_text_input_snapshot_aligned(
            widget_id,
            text,
            font_size,
            TextAlign::Left,
            rect,
            fence,
        )
    }

    pub(crate) fn retain_or_build_text_input_snapshot_aligned(
        &mut self,
        widget_id: WidgetId,
        text: &str,
        font_size: f32,
        align: TextAlign,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
    ) -> Option<Arc<ParagraphSnapshot>> {
        let available_width = Some(rect.width().max(0.0));
        if let Some(snapshot) = self
            .text_input_snapshot_for_input_aligned(widget_id, text, font_size, align, rect, fence)
        {
            return Some(snapshot);
        }
        let snapshot = self
            .layout_text_view(text, font_size, available_width, align, TextWrap::None)?
            .snapshot();
        let key = NativeTextInputSnapshotKey::new(
            widget_id,
            text,
            font_size,
            self.font_stack.generation(),
            available_width,
            align,
            TextWrap::None,
            rect,
            fence,
        );
        if snapshot.source_identity != key.source_identity
            || !snapshot.matches_source(text, key.content_revision)
            || !snapshot.is_usable_for(font_size)
            || snapshot.available_width.map(f32::to_bits) != key.constraints.available_width_bits
        {
            return None;
        }
        self.retain_text_input_snapshot(key, snapshot.clone())
            .then_some(snapshot)
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

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy width-independent layout remains covered by native text tests"
        )
    )]
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
    fn stale_text_input_snapshot_lookup_cannot_clear_current_fence() {
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
        assert!(renderer.text_input_snapshot(&key).is_some());

        renderer.begin_text_input_snapshot_fence(NativeTextInputSnapshotFence::new(4, 10));
        assert!(renderer.text_input_snapshot(&key).is_none());
    }

    #[test]
    fn stale_input_lookup_cannot_clear_current_fence_entries() {
        let mut renderer = NativeTextRenderer::new();
        let current_fence = NativeTextInputSnapshotFence::new(4, 9);
        let stale_fence = NativeTextInputSnapshotFence::new(4, 8);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));

        renderer.begin_text_input_snapshot_fence(current_fence);
        let current = renderer
            .retain_or_build_text_input_snapshot(
                WidgetId::from(1_u32),
                "value",
                14.0,
                rect,
                current_fence,
            )
            .expect("current snapshot should be retained");

        assert!(
            renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(1_u32),
                    "value",
                    14.0,
                    rect,
                    stale_fence,
                )
                .is_none()
        );
        let still_current = renderer
            .text_input_snapshot_for_input(
                WidgetId::from(1_u32),
                "value",
                14.0,
                rect,
                current_fence,
            )
            .expect("stale lookup must preserve the current snapshot");
        assert!(Arc::ptr_eq(&current, &still_current));
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
    fn text_input_snapshot_rejects_changed_logical_alignment_and_font() {
        let mut renderer = NativeTextRenderer::new();
        let fence = NativeTextInputSnapshotFence::new(4, 9);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(248.0, 38.0));
        let widget_id = WidgetId::from(7_u32);

        let left = renderer
            .retain_or_build_text_input_snapshot_aligned(
                widget_id,
                "value",
                26.0,
                TextAlign::Left,
                rect,
                fence,
            )
            .expect("logical font and left alignment should publish");
        assert!(
            renderer
                .text_input_snapshot_for_input_aligned(
                    widget_id,
                    "value",
                    26.0,
                    TextAlign::Left,
                    rect,
                    fence,
                )
                .is_some_and(|snapshot| Arc::ptr_eq(&snapshot, &left))
        );

        let right = renderer
            .retain_or_build_text_input_snapshot_aligned(
                widget_id,
                "value",
                26.0,
                TextAlign::Right,
                rect,
                fence,
            )
            .expect("changed alignment should build a new paragraph");
        assert!(!Arc::ptr_eq(&left, &right));
        assert!(
            renderer
                .text_input_snapshot_for_input_aligned(
                    widget_id,
                    "value",
                    26.0,
                    TextAlign::Left,
                    rect,
                    fence,
                )
                .is_none()
        );

        assert!(
            renderer
                .retain_or_build_text_input_snapshot_aligned(
                    widget_id,
                    "value",
                    39.0,
                    TextAlign::Right,
                    rect,
                    fence,
                )
                .is_some()
        );
    }

    #[test]
    fn text_input_snapshot_fence_allocator_is_monotonic_and_checked() {
        let mut allocator = NativeTextInputSnapshotFenceAllocator::default();
        let first = allocator.allocate().expect("first fence");
        let second = allocator.allocate().expect("second fence");

        assert_eq!(second.frame_token, first.frame_token + 1);
        assert_eq!(second.plan_token, first.plan_token + 1);

        allocator.next_frame_token = Some(u64::MAX);
        allocator.next_plan_token = Some(u64::MAX);
        assert!(allocator.allocate().is_some());
        assert!(allocator.allocate().is_none());
    }

    #[test]
    fn retained_text_input_snapshots_keep_admitted_current_plan_values_and_invalidate_old_fences() {
        let mut renderer = NativeTextRenderer::new();
        let mut fences = NativeTextInputSnapshotFenceAllocator::default();
        let fence = fences.allocate().expect("current plan fence");
        let first_rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));
        let second_rect = Rect::from_min_max(Point::new(136.0, 10.0), Point::new(256.0, 38.0));

        renderer.begin_text_input_snapshot_fence(fence);
        let empty = renderer
            .retain_or_build_text_input_snapshot(WidgetId::from(7_u32), "", 14.0, first_rect, fence)
            .expect("empty input snapshot should be retained");
        let value = renderer
            .retain_or_build_text_input_snapshot(
                WidgetId::from(8_u32),
                "value",
                14.0,
                second_rect,
                fence,
            )
            .expect("value input snapshot should be retained");

        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 2);
        assert_eq!(
            renderer.retained_text_input_snapshot.bytes,
            empty.estimated_bytes() + value.estimated_bytes()
        );
        assert!(Arc::ptr_eq(
            &empty,
            &renderer
                .text_input_snapshot_for_input(WidgetId::from(7_u32), "", 14.0, first_rect, fence,)
                .expect("empty input should remain available")
        ));
        assert!(Arc::ptr_eq(
            &value,
            &renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(8_u32),
                    "value",
                    14.0,
                    second_rect,
                    fence,
                )
                .expect("value input should remain available")
        ));

        assert!(
            renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(7_u32),
                    "changed",
                    14.0,
                    first_rect,
                    fence,
                )
                .is_none()
        );
        assert!(
            renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(8_u32),
                    "value",
                    14.0,
                    second_rect,
                    fence,
                )
                .is_some()
        );

        let next_fence = fences.allocate().expect("next plan fence");
        renderer.begin_text_input_snapshot_fence(next_fence);
        assert!(
            renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(8_u32),
                    "value",
                    14.0,
                    second_rect,
                    fence,
                )
                .is_none()
        );

        renderer.invalidate_text_input_snapshots();
        assert!(
            renderer
                .text_input_snapshot_for_input(
                    WidgetId::from(8_u32),
                    "value",
                    14.0,
                    second_rect,
                    next_fence,
                )
                .is_none()
        );
        assert_eq!(renderer.retained_text_input_snapshot.bytes, 0);
    }

    #[test]
    fn retained_text_input_snapshots_evict_least_recently_used_entries_at_entry_limit() {
        let mut renderer = NativeTextRenderer::new();
        renderer
            .retained_text_input_snapshot
            .set_entry_budget_override(Some(2));
        let fence = NativeTextInputSnapshotFence::new(4, 9);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));
        let generation = renderer.font_stack.generation();
        let first_key = retained_key(1, "first", rect, fence, generation);
        let second_key = retained_key(2, "second", rect, fence, generation);
        let third_key = retained_key(3, "third", rect, fence, generation);
        let first = ParagraphSnapshot::empty("first");
        let second = ParagraphSnapshot::empty("second");
        let third = ParagraphSnapshot::empty("third");

        renderer.retain_text_input_snapshot(first_key, first.clone());
        renderer.retain_text_input_snapshot(second_key, second);
        assert!(renderer.text_input_snapshot(&first_key).is_some());
        renderer.retain_text_input_snapshot(third_key, third.clone());

        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 2);
        assert!(renderer.text_input_snapshot(&first_key).is_some());
        assert!(renderer.text_input_snapshot(&third_key).is_some());
        assert!(renderer.text_input_snapshot(&second_key).is_none());
        assert_eq!(
            renderer.retained_text_input_snapshot.bytes,
            first.estimated_bytes() + third.estimated_bytes()
        );
    }

    #[test]
    fn retained_text_input_snapshots_evict_by_exact_byte_accounting() {
        let mut renderer = NativeTextRenderer::new();
        let fence = NativeTextInputSnapshotFence::new(4, 9);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));
        let generation = renderer.font_stack.generation();
        let first_key = retained_key(1, "first", rect, fence, generation);
        let second_key = retained_key(2, "second", rect, fence, generation);
        let first = ParagraphSnapshot::empty("first");
        let second = ParagraphSnapshot::empty("second");
        let byte_budget = first
            .estimated_bytes()
            .saturating_add(second.estimated_bytes())
            .saturating_sub(1);
        renderer
            .retained_text_input_snapshot
            .set_byte_budget_override(Some(byte_budget));

        renderer.retain_text_input_snapshot(first_key, first);
        renderer.retain_text_input_snapshot(second_key, second.clone());

        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 1);
        assert!(renderer.text_input_snapshot(&first_key).is_none());
        assert!(renderer.text_input_snapshot(&second_key).is_some());
        assert_eq!(
            renderer.retained_text_input_snapshot.bytes,
            second.estimated_bytes()
        );
    }

    #[test]
    fn retained_text_input_snapshot_replacement_updates_byte_accounting() {
        let mut renderer = NativeTextRenderer::new();
        let fence = NativeTextInputSnapshotFence::new(4, 9);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));
        let generation = renderer.font_stack.generation();
        let key = retained_key(1, "value", rect, fence, generation);
        let first = ParagraphSnapshot::empty("value");
        let replacement = ParagraphSnapshot::empty("value");
        renderer
            .retained_text_input_snapshot
            .set_byte_budget_override(Some(
                first.estimated_bytes().max(replacement.estimated_bytes()),
            ));

        renderer.retain_text_input_snapshot(key, first);
        renderer.retain_text_input_snapshot(key, replacement.clone());

        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 1);
        assert_eq!(
            renderer.retained_text_input_snapshot.bytes,
            replacement.estimated_bytes()
        );
        assert!(Arc::ptr_eq(
            &replacement,
            &renderer
                .text_input_snapshot(&key)
                .expect("replacement should remain available")
        ));
    }

    #[test]
    fn oversized_text_input_snapshot_is_declined_before_publication() {
        let mut renderer = NativeTextRenderer::new();
        renderer
            .retained_text_input_snapshot
            .set_byte_budget_override(Some(1));
        let fence = NativeTextInputSnapshotFence::new(4, 9);
        let rect = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(128.0, 38.0));
        let generation = renderer.font_stack.generation();
        let key = retained_key(1, "value", rect, fence, generation);
        let snapshot = ParagraphSnapshot::empty("value");

        assert!(
            !renderer
                .retained_text_input_snapshot
                .try_retain(key, snapshot)
        );
        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 0);
        assert_eq!(renderer.retained_text_input_snapshot.bytes, 0);
        assert!(renderer.text_input_snapshot(&key).is_none());

        assert!(
            renderer
                .retain_or_build_text_input_snapshot(
                    WidgetId::from(2_u32),
                    "value",
                    14.0,
                    rect,
                    fence,
                )
                .is_none()
        );
        assert_eq!(renderer.retained_text_input_snapshot.entries.len(), 0);
        assert_eq!(renderer.retained_text_input_snapshot.bytes, 0);
    }

    fn retained_key(
        widget_id: u64,
        text: &str,
        rect: Rect,
        fence: NativeTextInputSnapshotFence,
        font_generation: u64,
    ) -> NativeTextInputSnapshotKey {
        NativeTextInputSnapshotKey::new(
            WidgetId::from(widget_id),
            text,
            14.0,
            font_generation,
            Some(rect.width()),
            TextAlign::Left,
            TextWrap::None,
            rect,
            fence,
        )
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
