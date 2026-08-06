//! Bounded, observational evidence about whether one encoded paint segment has
//! a contiguous Vello stream shape.
//!
//! This module intentionally does not retain an encoding or any renderer
//! payload.  Checkpoints are cheap snapshots taken during the authoritative
//! scene traversal and are only evidence for a later, separately designed
//! artifact path.

use super::PaintSegmentEncodingObservation;
use crate::runtime::{
    MAX_PAINT_SEGMENTS, PaintPrimitive, PaintSegmentIdentity, PaintSegmentSpan,
    collect_segment_spans,
};
use vello::Scene;

/// Why a segment cannot be treated as a contiguous artifact candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum ArtifactFeasibilityReason {
    Capacity,
    MalformedSpans,
    CheckpointOrder,
    NonMonotonicOffsets,
    TrailingCheckpoint,
    CountMismatch,
    IdentityMismatch,
    PrimitiveSpanMismatch,
    InheritedClip,
    OpenClip,
    MalformedClip,
    ViewportFallback,
    ConservativeEvidence,
    CrossSegmentTransformOrStyle,
    UnprovableResourceLocality,
    UnsupportedPrimitive,
    OpenVelloClip,
}

/// Bounded disposition of one segment's encoded stream evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum ArtifactFeasibilityDisposition {
    NoArtifact,
    ContiguousCandidate,
    RequiresFreshEncoding(ArtifactFeasibilityReason),
}

impl ArtifactFeasibilityReason {
    /// Whether this reason means that the observation itself is unsafe to
    /// admit to the eligibility classifier. Other fresh-encoding reasons are
    /// authentic segment-local outcomes, not aggregate evidence failure.
    fn requires_conservative_fallback(self) -> bool {
        !matches!(
            self,
            Self::CrossSegmentTransformOrStyle
                | Self::UnprovableResourceLocality
                | Self::UnsupportedPrimitive
        )
    }
}

impl ArtifactFeasibilityDisposition {
    fn requires_conservative_fallback(self) -> bool {
        matches!(
            self,
            Self::RequiresFreshEncoding(reason) if reason.requires_conservative_fallback()
        )
    }
}

/// Cheap stream and resource lengths copied from one Vello encoding state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct ArtifactFeasibilityCounts {
    pub(in crate::gui_runtime::native_vello) path_tags: usize,
    pub(in crate::gui_runtime::native_vello) path_data: usize,
    pub(in crate::gui_runtime::native_vello) draw_tags: usize,
    pub(in crate::gui_runtime::native_vello) draw_data: usize,
    pub(in crate::gui_runtime::native_vello) transforms: usize,
    pub(in crate::gui_runtime::native_vello) styles: usize,
    pub(in crate::gui_runtime::native_vello) n_paths: u32,
    pub(in crate::gui_runtime::native_vello) n_path_segments: u32,
    pub(in crate::gui_runtime::native_vello) n_clips: u32,
    pub(in crate::gui_runtime::native_vello) n_open_clips: u32,
    pub(in crate::gui_runtime::native_vello) patches: usize,
    pub(in crate::gui_runtime::native_vello) color_stops: usize,
    pub(in crate::gui_runtime::native_vello) glyphs: usize,
    pub(in crate::gui_runtime::native_vello) glyph_runs: usize,
    pub(in crate::gui_runtime::native_vello) normalized_coords: usize,
}

impl ArtifactFeasibilityCounts {
    fn from_scene(scene: &Scene) -> Self {
        let encoding = scene.encoding();
        Self {
            path_tags: encoding.path_tags.len(),
            path_data: encoding.path_data.len(),
            draw_tags: encoding.draw_tags.len(),
            draw_data: encoding.draw_data.len(),
            transforms: encoding.transforms.len(),
            styles: encoding.styles.len(),
            n_paths: encoding.n_paths,
            n_path_segments: encoding.n_path_segments,
            n_clips: encoding.n_clips,
            n_open_clips: encoding.n_open_clips,
            patches: encoding.resources.patches.len(),
            color_stops: encoding.resources.color_stops.len(),
            glyphs: encoding.resources.glyphs.len(),
            glyph_runs: encoding.resources.glyph_runs.len(),
            normalized_coords: encoding.resources.normalized_coords.len(),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn monotonic_from(self, previous: Self) -> bool {
        self.path_tags >= previous.path_tags
            && self.path_data >= previous.path_data
            && self.draw_tags >= previous.draw_tags
            && self.draw_data >= previous.draw_data
            && self.transforms >= previous.transforms
            && self.styles >= previous.styles
            && self.n_paths >= previous.n_paths
            && self.n_path_segments >= previous.n_path_segments
            && self.n_clips >= previous.n_clips
            && self.n_open_clips >= previous.n_open_clips
            && self.patches >= previous.patches
            && self.color_stops >= previous.color_stops
            && self.glyphs >= previous.glyphs
            && self.glyph_runs >= previous.glyph_runs
            && self.normalized_coords >= previous.normalized_coords
    }

    pub(in crate::gui_runtime::native_vello) fn checked_delta_from(
        self,
        previous: Self,
    ) -> Option<Self> {
        if !self.monotonic_from(previous) {
            return None;
        }
        Some(Self {
            path_tags: self.path_tags.checked_sub(previous.path_tags)?,
            path_data: self.path_data.checked_sub(previous.path_data)?,
            draw_tags: self.draw_tags.checked_sub(previous.draw_tags)?,
            draw_data: self.draw_data.checked_sub(previous.draw_data)?,
            transforms: self.transforms.checked_sub(previous.transforms)?,
            styles: self.styles.checked_sub(previous.styles)?,
            n_paths: self.n_paths.checked_sub(previous.n_paths)?,
            n_path_segments: self.n_path_segments.checked_sub(previous.n_path_segments)?,
            n_clips: self.n_clips.checked_sub(previous.n_clips)?,
            n_open_clips: self.n_open_clips.checked_sub(previous.n_open_clips)?,
            patches: self.patches.checked_sub(previous.patches)?,
            color_stops: self.color_stops.checked_sub(previous.color_stops)?,
            glyphs: self.glyphs.checked_sub(previous.glyphs)?,
            glyph_runs: self.glyph_runs.checked_sub(previous.glyph_runs)?,
            normalized_coords: self
                .normalized_coords
                .checked_sub(previous.normalized_coords)?,
        })
    }

    pub(in crate::gui_runtime::native_vello) fn saturating_add(self, other: Self) -> Self {
        Self {
            path_tags: self.path_tags.saturating_add(other.path_tags),
            path_data: self.path_data.saturating_add(other.path_data),
            draw_tags: self.draw_tags.saturating_add(other.draw_tags),
            draw_data: self.draw_data.saturating_add(other.draw_data),
            transforms: self.transforms.saturating_add(other.transforms),
            styles: self.styles.saturating_add(other.styles),
            n_paths: self.n_paths.saturating_add(other.n_paths),
            n_path_segments: self.n_path_segments.saturating_add(other.n_path_segments),
            n_clips: self.n_clips.saturating_add(other.n_clips),
            n_open_clips: self.n_open_clips.saturating_add(other.n_open_clips),
            patches: self.patches.saturating_add(other.patches),
            color_stops: self.color_stops.saturating_add(other.color_stops),
            glyphs: self.glyphs.saturating_add(other.glyphs),
            glyph_runs: self.glyph_runs.saturating_add(other.glyph_runs),
            normalized_coords: self
                .normalized_coords
                .saturating_add(other.normalized_coords),
        }
    }

    fn grew_stream_from(self, previous: Self) -> bool {
        self.path_tags > previous.path_tags
            || self.path_data > previous.path_data
            || self.draw_tags > previous.draw_tags
            || self.draw_data > previous.draw_data
            || self.n_paths > previous.n_paths
            || self.n_path_segments > previous.n_path_segments
    }

    fn has_resource_growth_from(self, previous: Self) -> bool {
        self.patches > previous.patches
            || self.color_stops > previous.color_stops
            || self.glyphs > previous.glyphs
            || self.glyph_runs > previous.glyph_runs
            || self.normalized_coords > previous.normalized_coords
    }
}

/// Return the exact Vello-count delta introduced by one checked segment
/// boundary. A missing, trailing, conservative, or non-monotonic observation
/// is unavailable rather than being repaired locally.
pub(in crate::gui_runtime::native_vello) fn segment_local_count_delta(
    observation: ArtifactFeasibilityObservation,
    index: usize,
) -> Option<ArtifactFeasibilityCounts> {
    let count = usize::from(observation.segment_count);
    if observation.conservative
        || count == 0
        || count > MAX_PAINT_SEGMENTS
        || usize::from(observation.checkpoint_count) != count
        || index >= count
        || observation.segments[count..].iter().any(Option::is_some)
        || observation.checkpoints[count..].iter().any(Option::is_some)
    {
        return None;
    }
    let current = observation.checkpoints[index]?.counts;
    let previous = if index == 0 {
        ArtifactFeasibilityCounts::default()
    } else {
        observation.checkpoints[index - 1]?.counts
    };
    current.checked_delta_from(previous)
}

/// One checkpoint captured at an exact ordinary-segment boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct ArtifactFeasibilityCheckpoint {
    pub(in crate::gui_runtime::native_vello) primitive_end: u32,
    pub(in crate::gui_runtime::native_vello) counts: ArtifactFeasibilityCounts,
}

/// Bounded evidence for one plan-ordered ordinary segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct ArtifactFeasibilitySegment {
    pub(in crate::gui_runtime::native_vello) identity: PaintSegmentIdentity,
    pub(in crate::gui_runtime::native_vello) primitive_start: u32,
    pub(in crate::gui_runtime::native_vello) primitive_end: u32,
    pub(in crate::gui_runtime::native_vello) disposition: ArtifactFeasibilityDisposition,
}

/// Fixed-capacity observation retained with the ordinary scene statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct ArtifactFeasibilityObservation {
    pub(in crate::gui_runtime::native_vello) segments:
        [Option<ArtifactFeasibilitySegment>; MAX_PAINT_SEGMENTS],
    pub(in crate::gui_runtime::native_vello) checkpoints:
        [Option<ArtifactFeasibilityCheckpoint>; MAX_PAINT_SEGMENTS],
    pub(in crate::gui_runtime::native_vello) segment_count: u8,
    pub(in crate::gui_runtime::native_vello) checkpoint_count: u8,
    pub(in crate::gui_runtime::native_vello) conservative: bool,
}

impl Default for ArtifactFeasibilityObservation {
    fn default() -> Self {
        Self {
            segments: [None; MAX_PAINT_SEGMENTS],
            checkpoints: [None; MAX_PAINT_SEGMENTS],
            segment_count: 0,
            checkpoint_count: 0,
            conservative: true,
        }
    }
}

/// Event-fed observer used by the one authoritative scene encoding.
pub(in crate::gui_runtime::native_vello) struct ArtifactFeasibilityCollector {
    spans: [Option<PaintSegmentSpan>; MAX_PAINT_SEGMENTS],
    segment_count: u8,
    malformed_spans: bool,
    checkpoints: [Option<ArtifactFeasibilityCheckpoint>; MAX_PAINT_SEGMENTS],
    checkpoint_count: u8,
    previous: ArtifactFeasibilityCounts,
}

impl ArtifactFeasibilityCollector {
    pub(in crate::gui_runtime::native_vello) fn new(primitives: &[PaintPrimitive]) -> Self {
        let mut spans = [None; MAX_PAINT_SEGMENTS];
        let (segment_count, malformed_spans) = collect_segment_spans(primitives, &mut spans);
        Self {
            spans,
            segment_count,
            malformed_spans,
            checkpoints: [None; MAX_PAINT_SEGMENTS],
            checkpoint_count: 0,
            previous: ArtifactFeasibilityCounts::default(),
        }
    }

    /// Record a boundary after all text pending before that boundary has been
    /// flushed into the same authoritative scene.
    pub(in crate::gui_runtime::native_vello) fn checkpoint(
        &mut self,
        primitive_end: u32,
        scene: &Scene,
    ) {
        let Some(index) = self.spans[..usize::from(self.segment_count)]
            .iter()
            .position(|span| span.is_some_and(|span| span.end == primitive_end))
        else {
            self.malformed_spans = true;
            return;
        };
        if usize::from(self.checkpoint_count) != index
            || usize::from(self.checkpoint_count) >= MAX_PAINT_SEGMENTS
        {
            self.malformed_spans = true;
            return;
        }
        let counts = ArtifactFeasibilityCounts::from_scene(scene);
        self.checkpoints[index] = Some(ArtifactFeasibilityCheckpoint {
            primitive_end,
            counts,
        });
        self.checkpoint_count = self.checkpoint_count.saturating_add(1);
        self.previous = counts;
    }

    pub(in crate::gui_runtime::native_vello) fn finish(
        self,
        scene: &Scene,
        encoding: PaintSegmentEncodingObservation,
        primitives: &[PaintPrimitive],
    ) -> ArtifactFeasibilityObservation {
        let mut this = self;
        if let Some(last) = usize::from(this.segment_count).checked_sub(1)
            && this.checkpoint_count == this.segment_count.saturating_sub(1)
        {
            let counts = ArtifactFeasibilityCounts::from_scene(scene);
            let primitive_end = this.spans[last].map_or(0, |span| span.end);
            this.checkpoints[last] = Some(ArtifactFeasibilityCheckpoint {
                primitive_end,
                counts,
            });
            this.checkpoint_count = this.checkpoint_count.saturating_add(1);
        }

        let mut observation = ArtifactFeasibilityObservation {
            segments: [None; MAX_PAINT_SEGMENTS],
            checkpoints: this.checkpoints,
            segment_count: this.segment_count,
            checkpoint_count: this.checkpoint_count,
            conservative: this.malformed_spans,
        };
        let count = usize::from(this.segment_count);
        if count > MAX_PAINT_SEGMENTS {
            observation.conservative = true;
            if let Some(slot) = observation.segments.first_mut()
                && let Some(span) = this.spans.first().copied().flatten()
            {
                *slot = Some(evidence(span, fresh(ArtifactFeasibilityReason::Capacity)));
            }
        }
        let globally_invalid = count > MAX_PAINT_SEGMENTS
            || usize::from(this.checkpoint_count) != count
            || encoding.segment_count != this.segment_count
            || encoding.conservative
            || this.malformed_spans
            || encoding.segments[count.min(MAX_PAINT_SEGMENTS)..]
                .iter()
                .any(Option::is_some);
        if globally_invalid {
            observation.conservative = true;
        }
        for index in 0..count.min(MAX_PAINT_SEGMENTS) {
            let Some(span) = this.spans[index] else {
                observation.conservative = true;
                continue;
            };
            let Some(checkpoint) = this.checkpoints[index] else {
                observation.conservative = true;
                observation.segments[index] = Some(evidence(
                    span,
                    ArtifactFeasibilityDisposition::RequiresFreshEncoding(
                        ArtifactFeasibilityReason::CheckpointOrder,
                    ),
                ));
                continue;
            };
            let encoded = encoding.segments[index];
            let disposition = if globally_invalid {
                fresh(if count > MAX_PAINT_SEGMENTS {
                    ArtifactFeasibilityReason::Capacity
                } else if usize::from(this.checkpoint_count) != count {
                    ArtifactFeasibilityReason::CheckpointOrder
                } else if this.malformed_spans {
                    ArtifactFeasibilityReason::MalformedSpans
                } else if encoding.segments[count.min(MAX_PAINT_SEGMENTS)..]
                    .iter()
                    .any(Option::is_some)
                {
                    ArtifactFeasibilityReason::TrailingCheckpoint
                } else {
                    ArtifactFeasibilityReason::ConservativeEvidence
                })
            } else {
                validate_segment(index, span, checkpoint, encoded, &this, primitives)
            };
            if disposition.requires_conservative_fallback() {
                observation.conservative = true;
            }
            observation.segments[index] = Some(evidence(span, disposition));
        }
        if observation.segments[count.min(MAX_PAINT_SEGMENTS)..]
            .iter()
            .any(Option::is_some)
        {
            observation.conservative = true;
            if count < MAX_PAINT_SEGMENTS {
                let span = this.spans[count].unwrap_or(PaintSegmentSpan {
                    identity: PaintSegmentIdentity {
                        preceding: None,
                        following: None,
                    },
                    start: 0,
                    end: 0,
                });
                observation.segments[count] = Some(evidence(
                    span,
                    fresh(ArtifactFeasibilityReason::TrailingCheckpoint),
                ));
            }
        }
        observation
    }
}

fn evidence(
    span: PaintSegmentSpan,
    disposition: ArtifactFeasibilityDisposition,
) -> ArtifactFeasibilitySegment {
    ArtifactFeasibilitySegment {
        identity: span.identity,
        primitive_start: span.start,
        primitive_end: span.end,
        disposition,
    }
}

fn validate_segment(
    index: usize,
    span: PaintSegmentSpan,
    checkpoint: ArtifactFeasibilityCheckpoint,
    encoded: Option<super::PaintSegmentEncoding>,
    collector: &ArtifactFeasibilityCollector,
    primitives: &[PaintPrimitive],
) -> ArtifactFeasibilityDisposition {
    if checkpoint.primitive_end != span.end {
        return fresh(ArtifactFeasibilityReason::PrimitiveSpanMismatch);
    }
    let previous = if index == 0 {
        ArtifactFeasibilityCounts::default()
    } else {
        collector.checkpoints[index - 1]
            .map_or(ArtifactFeasibilityCounts::default(), |checkpoint| {
                checkpoint.counts
            })
    };
    if !checkpoint.counts.monotonic_from(previous) {
        return fresh(ArtifactFeasibilityReason::NonMonotonicOffsets);
    }
    let Some(encoded) = encoded else {
        return fresh(ArtifactFeasibilityReason::CountMismatch);
    };
    if encoded.identity != span.identity
        || encoded.primitive_start != span.start
        || encoded.primitive_end != span.end
    {
        return fresh(ArtifactFeasibilityReason::IdentityMismatch);
    }
    if encoded.conservative {
        return fresh(ArtifactFeasibilityReason::ConservativeEvidence);
    }
    match encoded.isolation {
        super::EncodingIsolation::InheritedClip => {
            return fresh(ArtifactFeasibilityReason::InheritedClip);
        }
        super::EncodingIsolation::OpenClip => return fresh(ArtifactFeasibilityReason::OpenClip),
        super::EncodingIsolation::Malformed => {
            return fresh(ArtifactFeasibilityReason::MalformedClip);
        }
        super::EncodingIsolation::SelfContained => {}
    }
    if matches!(
        encoded.safe_enclosure,
        super::SafeEnclosure::ViewportFallback
    ) {
        return fresh(ArtifactFeasibilityReason::ViewportFallback);
    }
    if checkpoint.counts.n_open_clips != previous.n_open_clips {
        return fresh(ArtifactFeasibilityReason::OpenVelloClip);
    }
    if !checkpoint.counts.grew_stream_from(previous) {
        return ArtifactFeasibilityDisposition::NoArtifact;
    }
    let slice = &primitives[span.start as usize..span.end as usize];
    if slice.iter().any(|primitive| {
        !matches!(
            primitive,
            PaintPrimitive::FillRect(_)
                | PaintPrimitive::FillRectBatch(_)
                | PaintPrimitive::OverlayPanel(_)
                | PaintPrimitive::ClipStart(_)
                | PaintPrimitive::ClipEnd(_)
        )
    }) {
        return fresh(ArtifactFeasibilityReason::UnsupportedPrimitive);
    }
    if collector.malformed_spans {
        return fresh(ArtifactFeasibilityReason::MalformedSpans);
    }
    if checkpoint.counts.has_resource_growth_from(previous) {
        return fresh(ArtifactFeasibilityReason::UnprovableResourceLocality);
    }
    if checkpoint.counts.transforms == previous.transforms
        || checkpoint.counts.styles == previous.styles
    {
        return fresh(ArtifactFeasibilityReason::CrossSegmentTransformOrStyle);
    }
    ArtifactFeasibilityDisposition::ContiguousCandidate
}

fn fresh(reason: ArtifactFeasibilityReason) -> ArtifactFeasibilityDisposition {
    ArtifactFeasibilityDisposition::RequiresFreshEncoding(reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_observation_is_bounded_and_conservative() {
        let observation = ArtifactFeasibilityObservation::default();
        assert_eq!(observation.segment_count, 0);
        assert!(observation.conservative);
        assert_eq!(observation.segments.len(), MAX_PAINT_SEGMENTS);
    }
}
