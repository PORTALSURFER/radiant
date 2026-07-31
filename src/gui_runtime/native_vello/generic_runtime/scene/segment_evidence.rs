//! Conservative, renderer-local evidence for ordinary paint-segment encoding.
//!
//! This collector is fed from the authoritative scene traversal. It observes
//! the same clip transitions and suppression decisions as encoding and never
//! changes the scene or cache behavior.

use super::{SceneClipBegin, SceneClipEnd, SceneClipState};
use crate::{
    gui::types::Rect,
    runtime::{
        MAX_PAINT_SEGMENTS, PaintPrimitive, PaintSegmentIdentity, PaintSegmentSpan,
        collect_segment_spans,
    },
};

/// Conservative region enclosing one encoded ordinary segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) enum SafeEnclosure {
    Empty,
    Bounded(Rect),
    ViewportFallback,
}

/// Whether a segment's encoding is independent of surrounding clip state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum EncodingIsolation {
    SelfContained,
    InheritedClip,
    OpenClip,
    Malformed,
}

/// Reason an encoding observation was widened conservatively.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum EncodingConservativeReason {
    None,
    UncertainPrimitive,
    NonFiniteGeometry,
    OpenClip,
    InheritedClip,
    MalformedClip,
    DuplicateAnchor,
}

/// One bounded encoding observation aligned with a [`PaintSegmentIdentity`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct PaintSegmentEncoding {
    pub(in crate::gui_runtime::native_vello) identity: PaintSegmentIdentity,
    pub(in crate::gui_runtime::native_vello) primitive_start: u32,
    pub(in crate::gui_runtime::native_vello) primitive_end: u32,
    pub(in crate::gui_runtime::native_vello) safe_enclosure: SafeEnclosure,
    pub(in crate::gui_runtime::native_vello) isolation: EncodingIsolation,
    pub(in crate::gui_runtime::native_vello) conservative: bool,
    pub(in crate::gui_runtime::native_vello) reason: EncodingConservativeReason,
}

/// Fixed-capacity observational result for one native scene traversal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct PaintSegmentEncodingObservation {
    pub(in crate::gui_runtime::native_vello) segments:
        [Option<PaintSegmentEncoding>; MAX_PAINT_SEGMENTS],
    pub(in crate::gui_runtime::native_vello) segment_count: u8,
    pub(in crate::gui_runtime::native_vello) conservative: bool,
}

impl Default for PaintSegmentEncodingObservation {
    fn default() -> Self {
        Self {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: 0,
            conservative: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SegmentAccumulator {
    start_depth: Option<usize>,
    end_depth: usize,
    enclosure: SafeEnclosure,
    has_content: bool,
    conservative: bool,
    reason: EncodingConservativeReason,
    malformed: bool,
}

impl Default for SegmentAccumulator {
    fn default() -> Self {
        Self {
            start_depth: None,
            end_depth: 0,
            enclosure: SafeEnclosure::Empty,
            has_content: false,
            conservative: false,
            reason: EncodingConservativeReason::None,
            malformed: false,
        }
    }
}

/// Event-fed collector used directly by the native scene traversal.
pub(in crate::gui_runtime::native_vello) struct PaintSegmentEvidenceCollector {
    spans: [Option<PaintSegmentSpan>; MAX_PAINT_SEGMENTS],
    segment_count: u8,
    malformed_spans: bool,
    viewport: Option<Rect>,
    accumulators: [SegmentAccumulator; MAX_PAINT_SEGMENTS],
}

impl PaintSegmentEvidenceCollector {
    pub(in crate::gui_runtime::native_vello) fn new(
        primitives: &[PaintPrimitive],
        viewport: Rect,
    ) -> Self {
        let mut spans = [None; MAX_PAINT_SEGMENTS];
        let (segment_count, malformed_spans) = collect_segment_spans(primitives, &mut spans);
        Self {
            spans,
            segment_count,
            malformed_spans,
            viewport: Some(viewport).filter(|viewport| viewport.has_finite_positive_area()),
            accumulators: [SegmentAccumulator::default(); MAX_PAINT_SEGMENTS],
        }
    }

    pub(in crate::gui_runtime::native_vello) fn observe_clip_start(
        &mut self,
        index: usize,
        depth_before: usize,
        result: SceneClipBegin,
        clip: Rect,
    ) {
        let Some(accumulator) = self.accumulator_for(index, depth_before) else {
            return;
        };
        if !clip.has_finite_positive_area() {
            accumulator.mark_fallback(EncodingConservativeReason::NonFiniteGeometry);
        }
        if matches!(result, SceneClipBegin::Suppress) {
            accumulator.mark_malformed(EncodingConservativeReason::MalformedClip);
        }
    }

    pub(in crate::gui_runtime::native_vello) fn observe_clip_end(
        &mut self,
        index: usize,
        depth_before: usize,
        depth_after: usize,
        result: SceneClipEnd,
    ) {
        let Some(accumulator) = self.accumulator_for_transition(index, depth_before, depth_after)
        else {
            return;
        };
        if matches!(result, SceneClipEnd::Unmatched) {
            accumulator.mark_malformed(EncodingConservativeReason::MalformedClip);
        }
    }

    pub(in crate::gui_runtime::native_vello) fn observe_suppressed(
        &mut self,
        index: usize,
        depth: usize,
    ) {
        let _ = self.accumulator_for(index, depth);
    }

    pub(in crate::gui_runtime::native_vello) fn observe_paint(
        &mut self,
        index: usize,
        primitive: &PaintPrimitive,
        clip_state: &SceneClipState,
    ) {
        let viewport = self.viewport;
        let Some(accumulator) = self.accumulator_for(index, clip_state.depth()) else {
            return;
        };
        accumulator.has_content = true;
        match primitive {
            PaintPrimitive::FillRect(_)
            | PaintPrimitive::FillRectBatch(_)
            | PaintPrimitive::OverlayPanel(_)
            | PaintPrimitive::Image(_) => {
                observe_finite_rects(accumulator, primitive, clip_state, viewport);
            }
            PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => {}
            _ => {
                accumulator.mark_uncertain(trusted_clip(clip_state, viewport));
            }
        }
    }

    pub(in crate::gui_runtime::native_vello) fn observe_anchor(
        &mut self,
        index: usize,
        depth: usize,
    ) {
        self.finish_at(index, depth);
    }

    pub(in crate::gui_runtime::native_vello) fn finish(
        self,
        final_depth: usize,
    ) -> PaintSegmentEncodingObservation {
        let mut this = self;
        if let Some(last) = usize::from(this.segment_count).checked_sub(1) {
            let accumulator = &mut this.accumulators[last];
            if accumulator.start_depth.is_none() {
                accumulator.start_depth = Some(final_depth);
            }
            accumulator.end_depth = final_depth;
        }
        let mut observation = PaintSegmentEncodingObservation {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: this.segment_count,
            conservative: this.malformed_spans,
        };
        for index in 0..usize::from(this.segment_count) {
            let Some(span) = this.spans[index] else {
                observation.conservative = true;
                continue;
            };
            let accumulator = this.accumulators[index];
            let (isolation, isolation_reason) = if accumulator.malformed {
                (
                    EncodingIsolation::Malformed,
                    EncodingConservativeReason::MalformedClip,
                )
            } else if accumulator.start_depth.unwrap_or(0) > 0 {
                (
                    EncodingIsolation::InheritedClip,
                    EncodingConservativeReason::InheritedClip,
                )
            } else if accumulator.end_depth > 0 {
                (
                    EncodingIsolation::OpenClip,
                    EncodingConservativeReason::OpenClip,
                )
            } else {
                (
                    EncodingIsolation::SelfContained,
                    EncodingConservativeReason::None,
                )
            };
            let mut conservative = accumulator.conservative || this.malformed_spans;
            let mut reason = accumulator.reason;
            if reason == EncodingConservativeReason::None
                && isolation_reason != EncodingConservativeReason::None
            {
                reason = isolation_reason;
                conservative = true;
            }
            if this.malformed_spans && reason == EncodingConservativeReason::None {
                reason = EncodingConservativeReason::DuplicateAnchor;
            }
            if conservative {
                observation.conservative = true;
            }
            let mut safe_enclosure = if accumulator.has_content || accumulator.conservative {
                accumulator.enclosure
            } else {
                SafeEnclosure::Empty
            };
            if this.malformed_spans
                || matches!(
                    isolation,
                    EncodingIsolation::OpenClip | EncodingIsolation::Malformed
                )
            {
                safe_enclosure = SafeEnclosure::ViewportFallback;
            }
            observation.segments[index] = Some(PaintSegmentEncoding {
                identity: span.identity,
                primitive_start: span.start,
                primitive_end: span.end,
                safe_enclosure,
                isolation,
                conservative,
                reason,
            });
        }
        observation
    }

    fn accumulator_for(&mut self, index: usize, depth: usize) -> Option<&mut SegmentAccumulator> {
        self.accumulator_for_transition(index, depth, depth)
    }

    fn accumulator_for_transition(
        &mut self,
        index: usize,
        start_depth: usize,
        end_depth: usize,
    ) -> Option<&mut SegmentAccumulator> {
        let segment_index = self.spans[..usize::from(self.segment_count)]
            .iter()
            .position(|span| {
                span.is_some_and(|span| index >= span.start as usize && index < span.end as usize)
            })?;
        let accumulator = &mut self.accumulators[segment_index];
        if accumulator.start_depth.is_none() {
            accumulator.start_depth = Some(start_depth);
        }
        accumulator.end_depth = end_depth;
        Some(accumulator)
    }

    fn finish_at(&mut self, index: usize, depth: usize) {
        for span_index in 0..usize::from(self.segment_count) {
            let Some(span) = self.spans[span_index] else {
                continue;
            };
            if index == usize::MAX || span.end as usize == index {
                let accumulator = &mut self.accumulators[span_index];
                if accumulator.start_depth.is_none() {
                    accumulator.start_depth = Some(depth);
                }
                accumulator.end_depth = depth;
            }
        }
    }
}

fn observe_finite_rects(
    accumulator: &mut SegmentAccumulator,
    primitive: &PaintPrimitive,
    clip_state: &SceneClipState,
    viewport: Option<Rect>,
) {
    let mut saw_rect = false;
    for rect in primitive.rects() {
        saw_rect = true;
        if !rect.has_finite_positive_area() {
            accumulator.mark_fallback(EncodingConservativeReason::NonFiniteGeometry);
            continue;
        }
        let Some(viewport) = viewport else {
            accumulator.mark_fallback(EncodingConservativeReason::NonFiniteGeometry);
            continue;
        };
        let Some(rect) = rect.intersection(viewport).and_then(|rect| {
            trusted_clip(clip_state, Some(viewport))
                .map_or(Some(rect), |clip| rect.intersection(clip))
        }) else {
            continue;
        };
        if !rect.has_finite_positive_area() {
            continue;
        }
        accumulator.enclosure = match accumulator.enclosure {
            SafeEnclosure::Empty => SafeEnclosure::Bounded(rect),
            SafeEnclosure::Bounded(existing) => SafeEnclosure::Bounded(existing.union(rect)),
            SafeEnclosure::ViewportFallback => SafeEnclosure::ViewportFallback,
        };
    }
    if !saw_rect {
        accumulator.enclosure = SafeEnclosure::Empty;
    }
}

fn trusted_clip(clip_state: &SceneClipState, viewport: Option<Rect>) -> Option<Rect> {
    let viewport = viewport?;
    clip_state
        .effective_rect()?
        .intersection(viewport)
        .filter(|clip| clip.has_finite_positive_area())
}

impl SegmentAccumulator {
    fn mark_uncertain(&mut self, clip: Option<Rect>) {
        if let Some(clip) = clip.filter(|clip| clip.has_finite_positive_area()) {
            if !matches!(self.enclosure, SafeEnclosure::ViewportFallback) {
                self.enclosure = SafeEnclosure::Bounded(clip);
            }
            self.conservative = true;
            self.reason = EncodingConservativeReason::UncertainPrimitive;
        } else {
            self.mark_fallback(EncodingConservativeReason::UncertainPrimitive);
        }
    }

    fn mark_fallback(&mut self, reason: EncodingConservativeReason) {
        self.enclosure = SafeEnclosure::ViewportFallback;
        self.conservative = true;
        if self.reason == EncodingConservativeReason::None {
            self.reason = reason;
        }
    }

    fn mark_malformed(&mut self, reason: EncodingConservativeReason) {
        self.malformed = true;
        self.mark_fallback(reason);
    }
}
