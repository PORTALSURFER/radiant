//! Private retained paint-segment evidence and render-boundary policy for one
//! native window.
//!
//! Renderer payloads remain owned by the neighboring artifact store. This
//! module owns only fixed-capacity metadata, admission evidence, and the
//! immutable decision that may hand a validated boundary to assembly.

use super::{
    PaintSegmentEncodingObservation,
    runner_state::NativeTargetGeneration,
    scene::{
        ArtifactFeasibilityDisposition, ArtifactFeasibilityObservation, ArtifactFeasibilityReason,
        EncodingConservativeReason, EncodingIsolation, SafeEnclosure,
    },
};
use crate::runtime::{
    MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentObservation, PaintSegmentSpan,
};

mod admission;
mod benefit;
mod selection;

pub(in crate::gui_runtime::native_vello::generic_runtime) use admission::{
    NativePaintSegmentCacheAdmission, NativePaintSegmentRenderAdmission,
    NativePaintSegmentRenderAdmissionQuery,
};
pub(in crate::gui_runtime::native_vello::generic_runtime) use benefit::{
    NativePaintSegmentBenefitFrameEvidence, NativePaintSegmentBenefitLedger,
};
pub(super) use selection::{
    NativePaintSegmentRenderSelection, select_native_paint_segment_render_boundary,
};

#[cfg(test)]
use super::scene::PaintSegmentEncoding;

/// One renderer-safe observation that can later be compared by retained
/// storage. This is evidence only; it carries no reuse decision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePaintSegmentFingerprint {
    pub(super) identity: PaintSegmentIdentity,
    pub(super) revision: u64,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) primitive_start: u32,
    pub(super) primitive_end: u32,
    pub(super) safe_enclosure: SafeEnclosure,
    pub(super) isolation: EncodingIsolation,
    pub(super) conservative_reason: EncodingConservativeReason,
}

/// Fixed-capacity native fingerprint evidence for one fully encoded frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePaintSegmentFingerprintObservation {
    pub(super) segments: [Option<NativePaintSegmentFingerprint>; MAX_PAINT_SEGMENTS],
    pub(super) segment_count: u8,
    pub(super) conservative: bool,
}

impl NativePaintSegmentFingerprintObservation {
    pub(super) const fn unavailable() -> Self {
        Self {
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: 0,
            conservative: true,
        }
    }
}

impl Default for NativePaintSegmentFingerprintObservation {
    fn default() -> Self {
        Self::unavailable()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePaintSegmentFreshEncodingReason {
    RevisionChanged,
    NoArtifact,
    NoResident,
    NotAdmitted,
    RenderSelectionFallback,
    RequiresFreshEncoding(ArtifactFeasibilityReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePaintSegmentFallbackReason {
    PaintConservative,
    AllSegmentsImplicated,
    FeasibilityConservative,
    UnknownOrExhaustedTargetGeneration,
    SegmentCapacity,
    EmptyEvidence,
    CountMismatch,
    TrailingEvidence,
    MissingSegment,
    MissingCheckpoint,
    UnsafeEnclosure,
    UnsafeIsolation,
    ConservativeReason,
    TargetGenerationMismatch,
    IdentityMismatch,
    OrderMismatch,
    SpanMismatch,
    DuplicateIdentity,
    MalformedSpans,
}

/// The metadata-only disposition for one exact current paint-segment span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum NativePaintSegmentEligibilityDisposition {
    RetainedCandidate(NativePaintSegmentFingerprint),
    FreshEncodingRequired(NativePaintSegmentFreshEncodingReason),
}

/// Small pure observational outcome for one current paint-segment observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePaintSegmentEligibilityOutcome {
    Plan,
    FullSceneFallback(NativePaintSegmentFallbackReason),
}

/// One bounded, ordered metadata-only eligibility entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePaintSegmentEligibilityEntry {
    pub(super) span: PaintSegmentSpan,
    pub(super) disposition: NativePaintSegmentEligibilityDisposition,
}

/// Pure observational eligibility state for one current paint-segment observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentEligibilityPlan {
    pub(super) outcome: NativePaintSegmentEligibilityOutcome,
    pub(super) entries: [Option<NativePaintSegmentEligibilityEntry>; MAX_PAINT_SEGMENTS],
    pub(super) entry_count: u8,
}

impl NativePaintSegmentEligibilityPlan {
    fn plan(
        entries: [Option<NativePaintSegmentEligibilityEntry>; MAX_PAINT_SEGMENTS],
        entry_count: u8,
    ) -> Self {
        Self {
            outcome: NativePaintSegmentEligibilityOutcome::Plan,
            entries,
            entry_count,
        }
    }

    fn full_scene_fallback(reason: NativePaintSegmentFallbackReason) -> Self {
        Self {
            outcome: NativePaintSegmentEligibilityOutcome::FullSceneFallback(reason),
            entries: [None; MAX_PAINT_SEGMENTS],
            entry_count: 0,
        }
    }

    fn force_fresh_candidates(self, reason: NativePaintSegmentFreshEncodingReason) -> Self {
        let Self {
            outcome,
            mut entries,
            entry_count,
        } = self;
        if !matches!(outcome, NativePaintSegmentEligibilityOutcome::Plan) {
            return Self {
                outcome,
                entries,
                entry_count,
            };
        }
        for entry in entries.iter_mut().take(usize::from(entry_count)).flatten() {
            if matches!(
                entry.disposition,
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
            ) {
                entry.disposition =
                    NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(reason);
            }
        }
        Self {
            outcome,
            entries,
            entry_count,
        }
    }
}

impl Default for NativePaintSegmentEligibilityPlan {
    fn default() -> Self {
        Self::full_scene_fallback(
            NativePaintSegmentFallbackReason::UnknownOrExhaustedTargetGeneration,
        )
    }
}

/// Fixed-capacity evidence and counts committed by one successful assembly.
#[derive(Clone, Copy, Debug)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentBenefitAssemblyInput
{
    pub(in crate::gui_runtime::native_vello::generic_runtime) paint: PaintSegmentObservation,
    pub(in crate::gui_runtime::native_vello::generic_runtime) encoding:
        PaintSegmentEncodingObservation,
    pub(in crate::gui_runtime::native_vello::generic_runtime) feasibility:
        ArtifactFeasibilityObservation,
    pub(in crate::gui_runtime::native_vello::generic_runtime) plan:
        NativePaintSegmentEligibilityPlan,
    pub(in crate::gui_runtime::native_vello::generic_runtime) target_generation:
        NativeTargetGeneration,
    pub(in crate::gui_runtime::native_vello::generic_runtime) fresh_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime) reused_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime) append_count: usize,
}

/// Assemble renderer-local fingerprints only after the complete authoritative
/// encode.
pub(super) fn assemble_native_paint_segment_fingerprints(
    paint: PaintSegmentObservation,
    encoding: PaintSegmentEncodingObservation,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentFingerprintObservation {
    if !target_generation.is_known()
        || paint.conservative
        || encoding.conservative
        || paint.segment_count != encoding.segment_count
        || usize::from(paint.segment_count) > MAX_PAINT_SEGMENTS
    {
        return NativePaintSegmentFingerprintObservation::unavailable();
    }

    let count = usize::from(paint.segment_count);
    let mut observation = NativePaintSegmentFingerprintObservation {
        segments: [None; MAX_PAINT_SEGMENTS],
        segment_count: paint.segment_count,
        conservative: false,
    };
    for index in 0..count {
        let (Some(segment), Some(encoded)) = (paint.segments[index], encoding.segments[index])
        else {
            return NativePaintSegmentFingerprintObservation::unavailable();
        };
        if segment.identity != encoded.identity
            || encoded.conservative
            || !matches!(encoded.isolation, EncodingIsolation::SelfContained)
            || matches!(encoded.safe_enclosure, SafeEnclosure::ViewportFallback)
            || !matches!(encoded.reason, EncodingConservativeReason::None)
        {
            return NativePaintSegmentFingerprintObservation::unavailable();
        }
        observation.segments[index] = Some(NativePaintSegmentFingerprint {
            identity: segment.identity,
            revision: segment.revision,
            target_generation,
            primitive_start: encoded.primitive_start,
            primitive_end: encoded.primitive_end,
            safe_enclosure: encoded.safe_enclosure,
            isolation: encoded.isolation,
            conservative_reason: encoded.reason,
        });
    }
    observation
}

/// One native window's fixed-capacity metadata-only retained segment store.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeRetainedPaintSegmentStore {
    entries: [Option<NativePaintSegmentFingerprint>; MAX_PAINT_SEGMENTS],
}

impl Default for NativeRetainedPaintSegmentStore {
    fn default() -> Self {
        Self {
            entries: [None; MAX_PAINT_SEGMENTS],
        }
    }
}

impl NativeRetainedPaintSegmentStore {
    #[cfg(test)]
    pub(super) fn snapshot(&self) -> [Option<NativePaintSegmentFingerprint>; MAX_PAINT_SEGMENTS] {
        self.entries
    }

    pub(super) fn clear(&mut self) {
        self.entries = [None; MAX_PAINT_SEGMENTS];
    }

    /// Reconcile one complete post-encode observation atomically.
    ///
    /// Any malformed, unsafe, conservative, unavailable, or mixed-generation
    /// observation clears the whole store. A valid observation is installed in
    /// plan order; entries absent from it are evicted and no history is kept.
    pub(super) fn reconcile(&mut self, observation: NativePaintSegmentFingerprintObservation) {
        let Some(next) = self.validated_next(observation) else {
            self.clear();
            return;
        };
        self.entries = next;
    }

    fn validated_next(
        &self,
        observation: NativePaintSegmentFingerprintObservation,
    ) -> Option<[Option<NativePaintSegmentFingerprint>; MAX_PAINT_SEGMENTS]> {
        let count = usize::from(observation.segment_count);
        if observation.conservative || count > MAX_PAINT_SEGMENTS {
            return None;
        }
        if count == 0 {
            if observation.segments.iter().any(Option::is_some) {
                return None;
            }
            return Some([None; MAX_PAINT_SEGMENTS]);
        }

        let mut next: [Option<NativePaintSegmentFingerprint>; MAX_PAINT_SEGMENTS] =
            [None; MAX_PAINT_SEGMENTS];
        let mut generation = None;
        for index in 0..count {
            let fingerprint = observation.segments[index]?;
            if fingerprint.primitive_start >= fingerprint.primitive_end
                || !fingerprint.target_generation.is_known()
                || matches!(fingerprint.safe_enclosure, SafeEnclosure::ViewportFallback)
                || !matches!(fingerprint.isolation, EncodingIsolation::SelfContained)
                || !matches!(
                    fingerprint.conservative_reason,
                    EncodingConservativeReason::None
                )
            {
                return None;
            }
            if generation.is_some_and(|existing| existing != fingerprint.target_generation) {
                return None;
            }
            generation = Some(fingerprint.target_generation);
            if next[..index].iter().flatten().any(|existing| {
                existing.identity == fingerprint.identity || *existing == fingerprint
            }) {
                return None;
            }
            next[index] = Some(fingerprint);
        }
        if observation.segments[count..].iter().any(Option::is_some) {
            return None;
        }

        Some(next)
    }
}

/// Classify metadata evidence without retaining or touching renderer payloads.
#[cfg(test)]
pub(super) fn classify_native_paint_segment_eligibility(
    paint: PaintSegmentObservation,
    retained: &NativeRetainedPaintSegmentStore,
    feasibility: ArtifactFeasibilityObservation,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentEligibilityPlan {
    let mut current_spans = [None; MAX_PAINT_SEGMENTS];
    let count = usize::from(paint.segment_count).min(MAX_PAINT_SEGMENTS);
    for (index, slot) in current_spans.iter_mut().enumerate().take(count) {
        *slot = feasibility.segments[index].map(|segment| PaintSegmentSpan {
            identity: segment.identity,
            start: segment.primitive_start,
            end: segment.primitive_end,
        });
    }
    classify_native_paint_segment_eligibility_with_spans(
        paint,
        retained,
        feasibility,
        current_spans,
        paint.segment_count,
        false,
        target_generation,
    )
}

pub(super) fn classify_native_paint_segment_eligibility_with_spans(
    paint: PaintSegmentObservation,
    retained: &NativeRetainedPaintSegmentStore,
    feasibility: ArtifactFeasibilityObservation,
    current_spans: [Option<PaintSegmentSpan>; MAX_PAINT_SEGMENTS],
    current_span_count: u8,
    current_spans_malformed: bool,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentEligibilityPlan {
    let count = usize::from(paint.segment_count);
    if paint.conservative {
        return fallback(NativePaintSegmentFallbackReason::PaintConservative);
    }
    if paint.all_implicated {
        return fallback(NativePaintSegmentFallbackReason::AllSegmentsImplicated);
    }
    if feasibility.conservative {
        return fallback(NativePaintSegmentFallbackReason::FeasibilityConservative);
    }
    if !target_generation.is_known() {
        return fallback(NativePaintSegmentFallbackReason::UnknownOrExhaustedTargetGeneration);
    }
    if count > MAX_PAINT_SEGMENTS {
        return fallback(NativePaintSegmentFallbackReason::SegmentCapacity);
    }
    if count == 0 {
        return fallback(NativePaintSegmentFallbackReason::EmptyEvidence);
    }
    if current_spans_malformed || usize::from(current_span_count) != count {
        return fallback(NativePaintSegmentFallbackReason::MalformedSpans);
    }
    if current_spans[count..].iter().any(Option::is_some) {
        return fallback(NativePaintSegmentFallbackReason::TrailingEvidence);
    }
    if usize::from(feasibility.segment_count) != count
        || usize::from(feasibility.checkpoint_count) != count
    {
        return fallback(NativePaintSegmentFallbackReason::CountMismatch);
    }
    if retained.entries[count..].iter().any(Option::is_some)
        || paint.segments[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
    {
        return fallback(NativePaintSegmentFallbackReason::TrailingEvidence);
    }

    let mut entries = [None; MAX_PAINT_SEGMENTS];
    for index in 0..count {
        let Some(current) = paint.segments[index] else {
            return fallback(NativePaintSegmentFallbackReason::MissingSegment);
        };
        let Some(current_span) = current_spans[index] else {
            return fallback(NativePaintSegmentFallbackReason::MalformedSpans);
        };
        let Some(previous) = retained.entries[index] else {
            return fallback(NativePaintSegmentFallbackReason::MissingSegment);
        };
        let Some(artifact) = feasibility.segments[index] else {
            return fallback(NativePaintSegmentFallbackReason::MissingCheckpoint);
        };
        let Some(checkpoint) = feasibility.checkpoints[index] else {
            return fallback(NativePaintSegmentFallbackReason::MissingCheckpoint);
        };
        if current_span.start >= current_span.end {
            return fallback(NativePaintSegmentFallbackReason::MalformedSpans);
        }
        if current_span.identity != current.identity {
            return fallback(NativePaintSegmentFallbackReason::OrderMismatch);
        }
        if !previous.target_generation.is_known() {
            return fallback(NativePaintSegmentFallbackReason::UnknownOrExhaustedTargetGeneration);
        }
        if previous.target_generation != target_generation {
            return fallback(NativePaintSegmentFallbackReason::TargetGenerationMismatch);
        }
        if matches!(previous.safe_enclosure, SafeEnclosure::ViewportFallback) {
            return fallback(NativePaintSegmentFallbackReason::UnsafeEnclosure);
        }
        if !matches!(previous.isolation, EncodingIsolation::SelfContained) {
            return fallback(NativePaintSegmentFallbackReason::UnsafeIsolation);
        }
        if !matches!(
            previous.conservative_reason,
            EncodingConservativeReason::None
        ) {
            return fallback(NativePaintSegmentFallbackReason::ConservativeReason);
        }
        if current.identity != previous.identity {
            return fallback(NativePaintSegmentFallbackReason::OrderMismatch);
        }
        if artifact.identity != previous.identity {
            return fallback(NativePaintSegmentFallbackReason::IdentityMismatch);
        }
        if artifact.primitive_start != previous.primitive_start
            || artifact.primitive_end != previous.primitive_end
            || checkpoint.primitive_end != artifact.primitive_end
            || checkpoint.primitive_end != previous.primitive_end
            || previous.primitive_start >= previous.primitive_end
            || artifact.primitive_start >= artifact.primitive_end
        {
            return fallback(NativePaintSegmentFallbackReason::SpanMismatch);
        }
        if let ArtifactFeasibilityDisposition::RequiresFreshEncoding(reason) = artifact.disposition
            && !allowed_segment_local_reason(reason)
        {
            return fallback(NativePaintSegmentFallbackReason::FeasibilityConservative);
        }
        if duplicate_identity_before(index, current.identity, &paint.segments, |record| {
            record.identity
        }) || duplicate_identity_before(index, previous.identity, &retained.entries, |record| {
            record.identity
        }) || duplicate_identity_before(
            index,
            artifact.identity,
            &feasibility.segments,
            |record| record.identity,
        ) {
            return fallback(NativePaintSegmentFallbackReason::DuplicateIdentity);
        }

        let disposition = if current.revision != previous.revision {
            NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                NativePaintSegmentFreshEncodingReason::RevisionChanged,
            )
        } else {
            match artifact.disposition {
                ArtifactFeasibilityDisposition::ContiguousCandidate => {
                    NativePaintSegmentEligibilityDisposition::RetainedCandidate(previous)
                }
                ArtifactFeasibilityDisposition::NoArtifact => {
                    NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                        NativePaintSegmentFreshEncodingReason::NoArtifact,
                    )
                }
                ArtifactFeasibilityDisposition::RequiresFreshEncoding(reason) => {
                    NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                        NativePaintSegmentFreshEncodingReason::RequiresFreshEncoding(reason),
                    )
                }
            }
        };
        entries[index] = Some(NativePaintSegmentEligibilityEntry {
            span: current_span,
            disposition,
        });
    }

    NativePaintSegmentEligibilityPlan::plan(entries, paint.segment_count)
}

fn fallback(reason: NativePaintSegmentFallbackReason) -> NativePaintSegmentEligibilityPlan {
    NativePaintSegmentEligibilityPlan::full_scene_fallback(reason)
}

fn allowed_segment_local_reason(reason: ArtifactFeasibilityReason) -> bool {
    matches!(
        reason,
        ArtifactFeasibilityReason::CrossSegmentTransformOrStyle
            | ArtifactFeasibilityReason::UnprovableResourceLocality
            | ArtifactFeasibilityReason::UnsupportedPrimitive
    )
}

fn duplicate_identity_before<T>(
    index: usize,
    identity: PaintSegmentIdentity,
    records: &[Option<T>; MAX_PAINT_SEGMENTS],
    identity_of: impl Fn(T) -> PaintSegmentIdentity,
) -> bool
where
    T: Copy,
{
    records[..index]
        .iter()
        .flatten()
        .copied()
        .any(|record| identity_of(record) == identity)
}

#[cfg(test)]
mod tests {
    use super::super::scene::{
        ArtifactFeasibilityCheckpoint, ArtifactFeasibilityCounts, ArtifactFeasibilitySegment,
    };
    use super::*;
    use crate::runtime::{PaintSegment, PaintSegmentAnchor};

    type FingerprintMutator = (
        fn(&mut NativePaintSegmentFingerprint),
        NativePaintSegmentFallbackReason,
    );

    fn identity(key: u64) -> PaintSegmentIdentity {
        PaintSegmentIdentity {
            preceding: None,
            following: Some(PaintSegmentAnchor {
                widget_id: key,
                key,
            }),
        }
    }

    fn observation(
        identities: &[u64],
        generation: NativeTargetGeneration,
    ) -> NativePaintSegmentFingerprintObservation {
        let mut paint = PaintSegmentObservation::empty();
        paint.segment_count = identities.len() as u8;
        let mut encoding = PaintSegmentEncodingObservation {
            segment_count: paint.segment_count,
            ..PaintSegmentEncodingObservation::default()
        };
        for (index, key) in identities.iter().copied().enumerate() {
            let id = identity(key);
            paint.segments[index] = Some(PaintSegment {
                identity: id,
                owner: None,
                revision: key,
                implicated: false,
            });
            encoding.segments[index] = Some(PaintSegmentEncoding {
                identity: id,
                primitive_start: (index as u32) * 2,
                primitive_end: (index as u32) * 2 + 1,
                safe_enclosure: SafeEnclosure::Empty,
                isolation: EncodingIsolation::SelfContained,
                conservative: false,
                reason: EncodingConservativeReason::None,
            });
        }
        assemble_native_paint_segment_fingerprints(paint, encoding, generation)
    }

    fn known() -> NativeTargetGeneration {
        NativeTargetGeneration::from_test_serial(1)
    }

    fn classifier_fixture(
        identities: &[u64],
        generation: NativeTargetGeneration,
        dispositions: &[ArtifactFeasibilityDisposition],
    ) -> (
        PaintSegmentObservation,
        NativeRetainedPaintSegmentStore,
        ArtifactFeasibilityObservation,
    ) {
        assert_eq!(identities.len(), dispositions.len());
        let retained_observation = observation(identities, generation);
        let mut retained = NativeRetainedPaintSegmentStore::default();
        retained.reconcile(retained_observation);

        let mut paint = PaintSegmentObservation::empty();
        paint.segment_count = identities.len() as u8;
        let mut feasibility = ArtifactFeasibilityObservation {
            segment_count: paint.segment_count,
            checkpoint_count: paint.segment_count,
            conservative: false,
            ..ArtifactFeasibilityObservation::default()
        };
        for (index, (key, disposition)) in identities
            .iter()
            .copied()
            .zip(dispositions.iter().copied())
            .enumerate()
        {
            let id = identity(key);
            paint.segments[index] = Some(PaintSegment {
                identity: id,
                owner: None,
                revision: key,
                implicated: false,
            });
            let fingerprint = retained.snapshot()[index].unwrap();
            feasibility.segments[index] = Some(ArtifactFeasibilitySegment {
                identity: id,
                primitive_start: fingerprint.primitive_start,
                primitive_end: fingerprint.primitive_end,
                disposition,
            });
            feasibility.checkpoints[index] = Some(ArtifactFeasibilityCheckpoint {
                primitive_end: fingerprint.primitive_end,
                counts: ArtifactFeasibilityCounts::default(),
            });
        }
        (paint, retained, feasibility)
    }

    fn assert_fallback(
        paint: PaintSegmentObservation,
        retained: &NativeRetainedPaintSegmentStore,
        feasibility: ArtifactFeasibilityObservation,
        target_generation: NativeTargetGeneration,
        reason: NativePaintSegmentFallbackReason,
    ) {
        let plan = classify_native_paint_segment_eligibility(
            paint,
            retained,
            feasibility,
            target_generation,
        );
        assert_eq!(
            plan.outcome,
            NativePaintSegmentEligibilityOutcome::FullSceneFallback(reason)
        );
        assert_eq!(plan.entries, [None; MAX_PAINT_SEGMENTS]);
        assert_eq!(plan.entry_count, 0);
    }

    #[test]
    fn eligibility_classifier_builds_exact_plan_and_reports_fresh_reasons() {
        let (paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        let plan =
            classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());
        assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
        assert_eq!(plan.entry_count, 1);
        assert_eq!(
            plan.entries[0].unwrap().disposition,
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(
                retained.snapshot()[0].unwrap()
            )
        );

        let mut revision_changed = paint;
        revision_changed.segments[0].as_mut().unwrap().revision += 1;
        let plan = classify_native_paint_segment_eligibility(
            revision_changed,
            &retained,
            feasibility,
            known(),
        );
        assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
        assert_eq!(
            plan.entries[0].unwrap().disposition,
            NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                NativePaintSegmentFreshEncodingReason::RevisionChanged
            )
        );

        for (disposition, reason) in [
            (
                ArtifactFeasibilityDisposition::NoArtifact,
                NativePaintSegmentFreshEncodingReason::NoArtifact,
            ),
            (
                ArtifactFeasibilityDisposition::RequiresFreshEncoding(
                    ArtifactFeasibilityReason::CrossSegmentTransformOrStyle,
                ),
                NativePaintSegmentFreshEncodingReason::RequiresFreshEncoding(
                    ArtifactFeasibilityReason::CrossSegmentTransformOrStyle,
                ),
            ),
        ] {
            let (paint, retained, feasibility) = classifier_fixture(&[1], known(), &[disposition]);
            let plan =
                classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());
            assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
            assert_eq!(
                plan.entries[0].unwrap().disposition,
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(reason)
            );
        }
    }

    #[test]
    fn eligibility_classifier_builds_four_ordered_retained_entries() {
        let (paint, retained, feasibility) = classifier_fixture(
            &[1, 2, 3, 4],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 4],
        );
        let plan =
            classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());

        assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
        assert_eq!(plan.entry_count, 4);
        for index in 0..4 {
            let entry = plan.entries[index].expect("ordered eligibility entry");
            let current = paint.segments[index].expect("current segment");
            assert_eq!(entry.span.identity, current.identity);
            assert_eq!(entry.span.start, (index as u32) * 2);
            assert_eq!(entry.span.end, (index as u32) * 2 + 1);
            assert_eq!(
                entry.disposition,
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(
                    retained.snapshot()[index].expect("retained fingerprint")
                )
            );
        }
        assert!(plan.entries[4..].iter().all(Option::is_none));
    }

    #[test]
    fn eligibility_classifier_keeps_unrelated_entries_when_one_revision_changes() {
        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1, 2, 3, 4],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 4],
        );
        paint.segments[2].as_mut().unwrap().revision += 1;
        let plan =
            classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());

        assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
        assert_eq!(plan.entry_count, 4);
        for index in 0..4 {
            let disposition = plan.entries[index]
                .expect("ordered eligibility entry")
                .disposition;
            if index == 2 {
                assert_eq!(
                    disposition,
                    NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                        NativePaintSegmentFreshEncodingReason::RevisionChanged,
                    )
                );
            } else {
                assert_eq!(
                    disposition,
                    NativePaintSegmentEligibilityDisposition::RetainedCandidate(
                        retained.snapshot()[index].expect("retained fingerprint")
                    )
                );
            }
        }
    }

    #[test]
    fn eligibility_classifier_keeps_unrelated_entries_for_local_fresh_reasons() {
        for disposition in [
            ArtifactFeasibilityDisposition::NoArtifact,
            ArtifactFeasibilityDisposition::RequiresFreshEncoding(
                ArtifactFeasibilityReason::CrossSegmentTransformOrStyle,
            ),
            ArtifactFeasibilityDisposition::RequiresFreshEncoding(
                ArtifactFeasibilityReason::UnprovableResourceLocality,
            ),
            ArtifactFeasibilityDisposition::RequiresFreshEncoding(
                ArtifactFeasibilityReason::UnsupportedPrimitive,
            ),
        ] {
            let (paint, retained, feasibility) = classifier_fixture(
                &[1, 2, 3],
                known(),
                &[
                    ArtifactFeasibilityDisposition::ContiguousCandidate,
                    disposition,
                    ArtifactFeasibilityDisposition::ContiguousCandidate,
                ],
            );
            let plan =
                classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());

            assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
            assert_eq!(plan.entry_count, 3);
            assert!(matches!(
                plan.entries[0].expect("first entry").disposition,
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
            ));
            assert!(matches!(
                plan.entries[1].expect("middle entry").disposition,
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_)
            ));
            assert!(matches!(
                plan.entries[2].expect("last entry").disposition,
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
            ));
        }
    }

    #[test]
    fn eligibility_classifier_routes_shifted_current_spans_without_vetoing_retention() {
        let (paint, retained, feasibility) = classifier_fixture(
            &[1, 2],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 2],
        );
        let mut current_spans = [None; MAX_PAINT_SEGMENTS];
        for (index, current_span) in current_spans.iter_mut().take(2).enumerate() {
            *current_span = Some(PaintSegmentSpan {
                identity: paint.segments[index].expect("current segment").identity,
                start: 100 + index as u32 * 3,
                end: 102 + index as u32 * 3,
            });
        }
        let plan = classify_native_paint_segment_eligibility_with_spans(
            paint,
            &retained,
            feasibility,
            current_spans,
            2,
            false,
            known(),
        );

        assert_eq!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan);
        assert_eq!(plan.entry_count, 2);
        assert_eq!(
            plan.entries[0].expect("first entry").span,
            current_spans[0].unwrap()
        );
        assert_eq!(
            plan.entries[1].expect("second entry").span,
            current_spans[1].unwrap()
        );
        assert!(matches!(
            plan.entries[0].expect("first entry").disposition,
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
        ));
        assert!(matches!(
            plan.entries[1].expect("second entry").disposition,
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
        ));
    }

    #[test]
    fn eligibility_classifier_falls_back_for_conservative_and_structural_evidence() {
        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        paint.conservative = true;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::PaintConservative,
        );

        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        paint.all_implicated = true;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::AllSegmentsImplicated,
        );

        let (paint, retained, mut feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        feasibility.conservative = true;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::FeasibilityConservative,
        );

        let (paint, retained, feasibility) = classifier_fixture(
            &[1, 2],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 2],
        );
        let mut hole = paint;
        hole.segments[1] = None;
        assert_fallback(
            hole,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::MissingSegment,
        );

        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        paint.segments[1] = paint.segments[0];
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::TrailingEvidence,
        );

        let (paint, retained, mut feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        feasibility.segment_count = 0;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::CountMismatch,
        );

        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        paint.segments[0].as_mut().unwrap().identity = identity(2);
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::OrderMismatch,
        );

        let (paint, retained, mut feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        feasibility.segments[0].as_mut().unwrap().primitive_end += 1;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::SpanMismatch,
        );

        let (paint, retained, mut feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        feasibility.checkpoints[0].as_mut().unwrap().primitive_end += 1;
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::SpanMismatch,
        );
    }

    #[test]
    fn eligibility_classifier_rejects_empty_non_conservative_evidence() {
        let paint = PaintSegmentObservation::empty();
        let retained = NativeRetainedPaintSegmentStore::default();
        let feasibility = ArtifactFeasibilityObservation {
            conservative: false,
            ..ArtifactFeasibilityObservation::default()
        };

        let plan =
            classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());
        assert_eq!(
            plan.outcome,
            NativePaintSegmentEligibilityOutcome::FullSceneFallback(
                NativePaintSegmentFallbackReason::EmptyEvidence
            )
        );
        assert_eq!(plan.entries, [None; MAX_PAINT_SEGMENTS]);
        assert_eq!(plan.entry_count, 0);
        assert!(!matches!(
            plan.outcome,
            NativePaintSegmentEligibilityOutcome::Plan
        ));
    }

    #[test]
    fn eligibility_classifier_falls_back_for_identity_generation_and_safety_failures() {
        let (paint, retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        assert_fallback(
            paint,
            &retained,
            feasibility,
            NativeTargetGeneration::unknown(),
            NativePaintSegmentFallbackReason::UnknownOrExhaustedTargetGeneration,
        );
        let mut exhausted = NativeTargetGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted.advance());
        assert_fallback(
            paint,
            &retained,
            feasibility,
            exhausted,
            NativePaintSegmentFallbackReason::UnknownOrExhaustedTargetGeneration,
        );

        let (paint, mut retained, feasibility) = classifier_fixture(
            &[1],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate],
        );
        retained.entries[0].as_mut().unwrap().target_generation =
            NativeTargetGeneration::from_test_serial(2);
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::TargetGenerationMismatch,
        );

        let mutators: [FingerprintMutator; 3] = [
            (
                |fingerprint: &mut NativePaintSegmentFingerprint| {
                    fingerprint.safe_enclosure = SafeEnclosure::ViewportFallback
                },
                NativePaintSegmentFallbackReason::UnsafeEnclosure,
            ),
            (
                |fingerprint: &mut NativePaintSegmentFingerprint| {
                    fingerprint.isolation = EncodingIsolation::InheritedClip
                },
                NativePaintSegmentFallbackReason::UnsafeIsolation,
            ),
            (
                |fingerprint: &mut NativePaintSegmentFingerprint| {
                    fingerprint.conservative_reason = EncodingConservativeReason::OpenClip
                },
                NativePaintSegmentFallbackReason::ConservativeReason,
            ),
        ];
        for (mutate, reason) in mutators {
            let (paint, mut retained, feasibility) = classifier_fixture(
                &[1],
                known(),
                &[ArtifactFeasibilityDisposition::ContiguousCandidate],
            );
            mutate(retained.entries[0].as_mut().unwrap());
            assert_fallback(paint, &retained, feasibility, known(), reason);
        }

        let (mut paint, mut retained, mut feasibility) = classifier_fixture(
            &[1, 2],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 2],
        );
        retained.entries[1].as_mut().unwrap().identity = identity(1);
        feasibility.segments[1].as_mut().unwrap().identity = identity(1);
        paint.segments[1].as_mut().unwrap().identity = identity(1);
        assert_fallback(
            paint,
            &retained,
            feasibility,
            known(),
            NativePaintSegmentFallbackReason::DuplicateIdentity,
        );
    }

    #[test]
    fn eligibility_classifier_admits_no_partial_candidate_on_late_failure() {
        let (mut paint, retained, feasibility) = classifier_fixture(
            &[1, 2],
            known(),
            &[ArtifactFeasibilityDisposition::ContiguousCandidate; 2],
        );
        let retained_before = retained.snapshot();
        paint.segments[1].as_mut().unwrap().identity = identity(9);
        let plan =
            classify_native_paint_segment_eligibility(paint, &retained, feasibility, known());
        assert!(matches!(
            plan.outcome,
            NativePaintSegmentEligibilityOutcome::FullSceneFallback(
                NativePaintSegmentFallbackReason::OrderMismatch
            )
        ));
        assert_eq!(retained.snapshot(), retained_before);
        assert!(!matches!(
            plan.outcome,
            NativePaintSegmentEligibilityOutcome::Plan
        ));
        assert_eq!(plan.entries, [None; MAX_PAINT_SEGMENTS]);
        assert_eq!(plan.entry_count, 0);
    }

    #[test]
    fn complete_admission_preserves_plan_order_and_exact_matches() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        let first = observation(&[1, 2], known());
        store.reconcile(first);
        assert_eq!(store.snapshot()[0], first.segments[0]);
        assert_eq!(store.snapshot()[1], first.segments[1]);
        let second = observation(&[2, 3], known());
        store.reconcile(second);
        assert_eq!(store.snapshot()[0], second.segments[0]);
        assert_eq!(store.snapshot()[1], second.segments[1]);
    }

    #[test]
    fn valid_empty_and_removal_clear_store() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        store.reconcile(observation(&[1], known()));
        store.reconcile(observation(&[], known()));
        assert!(store.snapshot().iter().all(Option::is_none));
    }

    #[test]
    fn invalid_observation_clears_atomically() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        store.reconcile(observation(&[1, 2], known()));
        let mut invalid = observation(&[3, 4], known());
        invalid.segments[0].as_mut().unwrap().primitive_end = 0;
        store.reconcile(invalid);
        assert!(store.snapshot().iter().all(Option::is_none));
    }

    #[test]
    fn exact_fingerprint_changes_replace_and_absent_entries_evict() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        let first = observation(&[1, 2], known());
        store.reconcile(first);
        let mut changed = observation(&[1, 2], known());
        changed.segments[0].as_mut().unwrap().revision += 1;
        store.reconcile(changed);
        assert_eq!(store.snapshot()[0], changed.segments[0]);
        assert_eq!(store.snapshot()[1], changed.segments[1]);

        let removed = observation(&[1], known());
        store.reconcile(removed);
        assert_eq!(store.snapshot()[0], removed.segments[0]);
        assert!(store.snapshot()[1..].iter().all(Option::is_none));
    }

    #[test]
    fn unsafe_fields_clear_without_partial_admission() {
        let mutators: [fn(&mut NativePaintSegmentFingerprint); 3] = [
            |fingerprint: &mut NativePaintSegmentFingerprint| {
                fingerprint.isolation = EncodingIsolation::InheritedClip
            },
            |fingerprint: &mut NativePaintSegmentFingerprint| {
                fingerprint.safe_enclosure = SafeEnclosure::ViewportFallback
            },
            |fingerprint: &mut NativePaintSegmentFingerprint| {
                fingerprint.conservative_reason = EncodingConservativeReason::OpenClip
            },
        ];
        for mutate in mutators {
            let mut store = NativeRetainedPaintSegmentStore::default();
            store.reconcile(observation(&[9], known()));
            let mut invalid = observation(&[1, 2], known());
            mutate(invalid.segments[1].as_mut().unwrap());
            store.reconcile(invalid);
            assert!(store.snapshot().iter().all(Option::is_none));
        }
    }

    #[test]
    fn unavailable_holes_trailing_extras_and_conservative_clear() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        store.reconcile(observation(&[9], known()));
        store.reconcile(NativePaintSegmentFingerprintObservation::unavailable());
        assert!(store.snapshot().iter().all(Option::is_none));

        for invalid in [
            {
                let mut value = observation(&[1, 2], known());
                value.segments[1] = None;
                value
            },
            {
                let mut value = observation(&[1], known());
                value.segments[1] = value.segments[0];
                value
            },
            {
                let mut value = observation(&[1], known());
                value.conservative = true;
                value
            },
        ] {
            store.reconcile(observation(&[9], known()));
            store.reconcile(invalid);
            assert!(store.snapshot().iter().all(Option::is_none));
        }
    }

    #[test]
    fn unknown_exhausted_and_invalid_spans_clear() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        let mut unknown = observation(&[1], known());
        unknown.segments[0].as_mut().unwrap().target_generation = NativeTargetGeneration::unknown();
        store.reconcile(unknown);
        assert!(store.snapshot().iter().all(Option::is_none));

        let mut generation = NativeTargetGeneration::from_test_serial(u64::MAX);
        assert!(!generation.advance());
        let mut exhausted = observation(&[1], known());
        exhausted.segments[0].as_mut().unwrap().target_generation = generation;
        store.reconcile(exhausted);
        assert!(store.snapshot().iter().all(Option::is_none));

        let mut invalid_span = observation(&[1], known());
        invalid_span.segments[0].as_mut().unwrap().primitive_end =
            invalid_span.segments[0].unwrap().primitive_start;
        store.reconcile(invalid_span);
        assert!(store.snapshot().iter().all(Option::is_none));
    }

    #[test]
    fn fixed_capacity_admits_maximum_dense_observation() {
        let keys: Vec<_> = (0..MAX_PAINT_SEGMENTS as u64).collect();
        let mut store = NativeRetainedPaintSegmentStore::default();
        store.reconcile(observation(&keys, known()));
        assert_eq!(
            store.snapshot().iter().flatten().count(),
            MAX_PAINT_SEGMENTS
        );
        assert_eq!(store.snapshot()[0].unwrap().identity, identity(0));
        assert_eq!(
            store.snapshot()[MAX_PAINT_SEGMENTS - 1].unwrap().identity,
            identity(63)
        );
    }

    #[test]
    fn duplicate_identity_and_generation_mismatch_clear() {
        let mut store = NativeRetainedPaintSegmentStore::default();
        let mut duplicate = observation(&[1, 1], known());
        duplicate.segments[1].as_mut().unwrap().identity = duplicate.segments[0].unwrap().identity;
        store.reconcile(duplicate);
        assert!(store.snapshot().iter().all(Option::is_none));

        let mut mixed = observation(&[1, 2], known());
        mixed.segments[1].as_mut().unwrap().target_generation =
            NativeTargetGeneration::from_test_serial(2);
        store.reconcile(mixed);
        assert!(store.snapshot().iter().all(Option::is_none));
    }
}
