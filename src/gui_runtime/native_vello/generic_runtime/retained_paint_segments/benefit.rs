//! Bounded, private evidence about the work shape of native paint segments.
//!
//! This ledger observes only completed authoritative encodes and atomically
//! committed retained/mixed assemblies. It has no cache-admission or reuse
//! decision and owns no renderer payload.

use super::super::runner_state::NativeTargetGeneration;
use super::super::scene::{
    ArtifactFeasibilityCounts, ArtifactFeasibilityObservation, EncodingConservativeReason,
    EncodingIsolation, PaintSegmentEncodingObservation, SafeEnclosure, segment_local_count_delta,
};
use super::{
    NativePaintSegmentBenefitAssemblyInput, NativePaintSegmentEligibilityDisposition,
    NativePaintSegmentEligibilityOutcome,
};
use crate::runtime::{
    MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentObservation, PaintSegmentSpan,
};

/// One accepted segment-level benefit observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativePaintSegmentBenefitOutcome {
    FreshEncoding,
    AssemblyVetoFullEncodeRepair,
    SuccessfulRetainedReuse,
    SuccessfulMixedReuse,
    SuccessfulMixedFreshEncoding,
}

/// The evidence kept for the latest observation of one stable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePaintSegmentBenefitSample {
    identity: PaintSegmentIdentity,
    span: PaintSegmentSpan,
    revision: u64,
    target_generation: NativeTargetGeneration,
    outcome: NativePaintSegmentBenefitOutcome,
    /// Exact Vello count delta for this segment's checked boundary. For a
    /// reuse outcome this is the shape of the encoding work avoided; it is not
    /// a byte, timing, GPU-cost, or reuse-authority measurement.
    encoding_counts: ArtifactFeasibilityCounts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePaintSegmentBenefitObservation {
    segments: [Option<NativePaintSegmentBenefitSample>; MAX_PAINT_SEGMENTS],
    segment_count: u8,
    target_generation: NativeTargetGeneration,
}

/// Crate-private aggregate evidence for one stable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentBenefitSummary {
    pub(in crate::gui_runtime::native_vello::generic_runtime) identity: PaintSegmentIdentity,
    pub(in crate::gui_runtime::native_vello::generic_runtime) span: PaintSegmentSpan,
    pub(in crate::gui_runtime::native_vello::generic_runtime) target_generation:
        NativeTargetGeneration,
    pub(in crate::gui_runtime::native_vello::generic_runtime) latest_revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) latest_outcome:
        NativePaintSegmentBenefitOutcome,
    pub(in crate::gui_runtime::native_vello::generic_runtime) observation_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) fresh_encoding_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) retained_reuse_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) mixed_reuse_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) mixed_fresh_encoding_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) assembly_veto_full_encode_count: u64,
    /// Number of segment encoding operations avoided by committed reuse.
    pub(in crate::gui_runtime::native_vello::generic_runtime) avoided_encoding_count: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) fresh_encoding_counts:
        ArtifactFeasibilityCounts,
    pub(in crate::gui_runtime::native_vello::generic_runtime) avoided_encoding_counts:
        ArtifactFeasibilityCounts,
    last_observation_epoch: u64,
    latest_sample: NativePaintSegmentBenefitSample,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePaintSegmentBenefitEntry {
    summary: NativePaintSegmentBenefitSummary,
}

/// Fixed-capacity, per-window benefit evidence.
///
/// The recent window is measured in accepted frame observations rather than
/// wall-clock time. Once the fixed window is full, the next accepted frame
/// deterministically starts a new window. Invalid evidence clears the ledger
/// before any part of that frame can be published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentBenefitLedger {
    entries: [Option<NativePaintSegmentBenefitEntry>; MAX_PAINT_SEGMENTS],
    observation_epoch: u64,
    window_start_epoch: u64,
    target_generation: Option<NativeTargetGeneration>,
    available: bool,
}

impl Default for NativePaintSegmentBenefitLedger {
    fn default() -> Self {
        Self {
            entries: [None; MAX_PAINT_SEGMENTS],
            observation_epoch: 0,
            window_start_epoch: 0,
            target_generation: None,
            available: false,
        }
    }
}

impl NativePaintSegmentBenefitLedger {
    const HISTORY_WINDOW: u64 = MAX_PAINT_SEGMENTS as u64;

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn clear(&mut self) {
        self.entries = [None; MAX_PAINT_SEGMENTS];
        self.target_generation = None;
        self.available = false;
        self.window_start_epoch = self.observation_epoch;
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_full_encode(
        &mut self,
        paint: PaintSegmentObservation,
        encoding: PaintSegmentEncodingObservation,
        feasibility: ArtifactFeasibilityObservation,
        target_generation: NativeTargetGeneration,
        assembly_vetoed: bool,
    ) {
        let outcome = if assembly_vetoed {
            NativePaintSegmentBenefitOutcome::AssemblyVetoFullEncodeRepair
        } else {
            NativePaintSegmentBenefitOutcome::FreshEncoding
        };
        let Some(observation) =
            build_full_encode_observation(paint, encoding, feasibility, target_generation, outcome)
        else {
            self.record_unavailable();
            return;
        };
        self.record_observation(observation);
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_successful_assembly(
        &mut self,
        input: NativePaintSegmentBenefitAssemblyInput,
    ) {
        let Some(observation) = build_assembly_observation(input) else {
            self.record_unavailable();
            return;
        };
        self.record_observation(observation);
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_unavailable(&mut self) {
        self.clear();
    }

    fn record_observation(&mut self, observation: NativePaintSegmentBenefitObservation) {
        if !valid_observation(observation) {
            self.record_unavailable();
            return;
        }
        if self
            .target_generation
            .is_some_and(|generation| generation != observation.target_generation)
        {
            self.record_unavailable();
            return;
        }

        self.observation_epoch = self.observation_epoch.saturating_add(1);
        if !self.available {
            self.window_start_epoch = self.observation_epoch;
            self.available = true;
        } else if self
            .observation_epoch
            .saturating_sub(self.window_start_epoch)
            >= Self::HISTORY_WINDOW
        {
            self.entries = [None; MAX_PAINT_SEGMENTS];
            self.window_start_epoch = self.observation_epoch;
        }
        self.target_generation = Some(observation.target_generation);

        for sample in observation
            .segments
            .into_iter()
            .take(usize::from(observation.segment_count))
            .flatten()
        {
            self.record_sample(sample);
        }
    }

    fn record_sample(&mut self, sample: NativePaintSegmentBenefitSample) {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.summary.identity == sample.identity))
            .unwrap_or_else(|| self.oldest_or_empty_index());
        let summary = self.entries[index].map(|entry| entry.summary).unwrap_or(
            NativePaintSegmentBenefitSummary {
                identity: sample.identity,
                span: sample.span,
                target_generation: sample.target_generation,
                latest_revision: 0,
                latest_outcome: sample.outcome,
                observation_count: 0,
                fresh_encoding_count: 0,
                retained_reuse_count: 0,
                mixed_reuse_count: 0,
                mixed_fresh_encoding_count: 0,
                assembly_veto_full_encode_count: 0,
                avoided_encoding_count: 0,
                fresh_encoding_counts: ArtifactFeasibilityCounts::default(),
                avoided_encoding_counts: ArtifactFeasibilityCounts::default(),
                last_observation_epoch: 0,
                latest_sample: sample,
            },
        );
        let mut summary = summary;
        summary.span = sample.span;
        summary.target_generation = sample.target_generation;
        summary.latest_revision = sample.revision;
        summary.latest_outcome = sample.outcome;
        summary.observation_count = summary.observation_count.saturating_add(1);
        summary.last_observation_epoch = self.observation_epoch;
        summary.latest_sample = sample;
        match sample.outcome {
            NativePaintSegmentBenefitOutcome::FreshEncoding => {
                summary.fresh_encoding_count = summary.fresh_encoding_count.saturating_add(1);
                summary.fresh_encoding_counts = summary
                    .fresh_encoding_counts
                    .saturating_add(sample.encoding_counts);
            }
            NativePaintSegmentBenefitOutcome::AssemblyVetoFullEncodeRepair => {
                summary.fresh_encoding_count = summary.fresh_encoding_count.saturating_add(1);
                summary.assembly_veto_full_encode_count =
                    summary.assembly_veto_full_encode_count.saturating_add(1);
                summary.fresh_encoding_counts = summary
                    .fresh_encoding_counts
                    .saturating_add(sample.encoding_counts);
            }
            NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse => {
                summary.retained_reuse_count = summary.retained_reuse_count.saturating_add(1);
                summary.avoided_encoding_count = summary.avoided_encoding_count.saturating_add(1);
                summary.avoided_encoding_counts = summary
                    .avoided_encoding_counts
                    .saturating_add(sample.encoding_counts);
            }
            NativePaintSegmentBenefitOutcome::SuccessfulMixedReuse => {
                summary.mixed_reuse_count = summary.mixed_reuse_count.saturating_add(1);
                summary.avoided_encoding_count = summary.avoided_encoding_count.saturating_add(1);
                summary.avoided_encoding_counts = summary
                    .avoided_encoding_counts
                    .saturating_add(sample.encoding_counts);
            }
            NativePaintSegmentBenefitOutcome::SuccessfulMixedFreshEncoding => {
                summary.mixed_fresh_encoding_count =
                    summary.mixed_fresh_encoding_count.saturating_add(1);
                summary.fresh_encoding_count = summary.fresh_encoding_count.saturating_add(1);
                summary.fresh_encoding_counts = summary
                    .fresh_encoding_counts
                    .saturating_add(sample.encoding_counts);
            }
        }
        self.entries[index] = Some(NativePaintSegmentBenefitEntry { summary });
    }

    fn oldest_or_empty_index(&self) -> usize {
        let mut oldest_index = 0;
        let mut oldest_epoch = u64::MAX;
        for (index, entry) in self.entries.iter().enumerate() {
            let Some(entry) = entry else {
                return index;
            };
            if entry.summary.last_observation_epoch < oldest_epoch {
                oldest_epoch = entry.summary.last_observation_epoch;
                oldest_index = index;
            }
        }
        oldest_index
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn snapshot_for_test(
        &self,
    ) -> [Option<NativePaintSegmentBenefitSummary>; MAX_PAINT_SEGMENTS] {
        self.entries.map(|entry| entry.map(|entry| entry.summary))
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn available_for_test(
        &self,
    ) -> bool {
        self.available
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn observation_epoch_for_test(
        &self,
    ) -> u64 {
        self.observation_epoch
    }
}

fn build_full_encode_observation(
    paint: PaintSegmentObservation,
    encoding: PaintSegmentEncodingObservation,
    feasibility: ArtifactFeasibilityObservation,
    target_generation: NativeTargetGeneration,
    outcome: NativePaintSegmentBenefitOutcome,
) -> Option<NativePaintSegmentBenefitObservation> {
    let count = validate_common_evidence(paint, encoding, feasibility, target_generation)?;
    let mut observation = NativePaintSegmentBenefitObservation {
        segments: [None; MAX_PAINT_SEGMENTS],
        segment_count: paint.segment_count,
        target_generation,
    };
    for index in 0..count {
        let current = paint.segments[index]?;
        let encoded = encoding.segments[index]?;
        let evidence = feasibility.segments[index]?;
        let counts = segment_local_count_delta(feasibility, index)?;
        let span = PaintSegmentSpan {
            identity: evidence.identity,
            start: evidence.primitive_start,
            end: evidence.primitive_end,
        };
        if current.revision == 0
            || current.identity != span.identity
            || encoded.identity != span.identity
            || encoded.primitive_start != span.start
            || encoded.primitive_end != span.end
        {
            return None;
        }
        observation.segments[index] = Some(NativePaintSegmentBenefitSample {
            identity: current.identity,
            span,
            revision: current.revision,
            target_generation,
            outcome,
            encoding_counts: counts,
        });
    }
    Some(observation)
}

fn build_assembly_observation(
    input: NativePaintSegmentBenefitAssemblyInput,
) -> Option<NativePaintSegmentBenefitObservation> {
    let NativePaintSegmentBenefitAssemblyInput {
        paint,
        encoding,
        feasibility,
        plan,
        target_generation,
        fresh_count,
        reused_count,
        append_count,
    } = input;
    if !matches!(plan.outcome, NativePaintSegmentEligibilityOutcome::Plan)
        || paint.all_implicated
        || append_count == 0
    {
        return None;
    }
    let count = validate_common_evidence(paint, encoding, feasibility, target_generation)?;
    if usize::from(plan.entry_count) != count
        || plan.entries[count..].iter().any(Option::is_some)
        || fresh_count.checked_add(reused_count) != Some(count)
        || append_count != count
    {
        return None;
    }

    let mut expected_fresh = 0usize;
    let mut expected_reused = 0usize;
    let mixed = fresh_count > 0 && reused_count > 0;
    let mut observation = NativePaintSegmentBenefitObservation {
        segments: [None; MAX_PAINT_SEGMENTS],
        segment_count: paint.segment_count,
        target_generation,
    };
    for index in 0..count {
        let entry = plan.entries[index]?;
        let current = paint.segments[index]?;
        let encoded = encoding.segments[index]?;
        let evidence = feasibility.segments[index]?;
        let counts = segment_local_count_delta(feasibility, index)?;
        let span = PaintSegmentSpan {
            identity: evidence.identity,
            start: evidence.primitive_start,
            end: evidence.primitive_end,
        };
        if current.revision == 0
            || current.identity != entry.span.identity
            || current.identity != span.identity
            || entry.span != span
            || encoded.identity != span.identity
            || encoded.primitive_start != span.start
            || encoded.primitive_end != span.end
        {
            return None;
        }
        let outcome = match entry.disposition {
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) => {
                if fingerprint.identity != entry.span.identity
                    || fingerprint.primitive_start != entry.span.start
                    || fingerprint.primitive_end != entry.span.end
                    || fingerprint.revision == 0
                    || fingerprint.revision != current.revision
                    || fingerprint.target_generation != target_generation
                    || matches!(fingerprint.safe_enclosure, SafeEnclosure::ViewportFallback)
                    || !matches!(fingerprint.isolation, EncodingIsolation::SelfContained)
                    || !matches!(
                        fingerprint.conservative_reason,
                        EncodingConservativeReason::None
                    )
                {
                    return None;
                }
                expected_reused = expected_reused.saturating_add(1);
                if mixed {
                    NativePaintSegmentBenefitOutcome::SuccessfulMixedReuse
                } else {
                    NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse
                }
            }
            NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_) => {
                expected_fresh = expected_fresh.saturating_add(1);
                NativePaintSegmentBenefitOutcome::SuccessfulMixedFreshEncoding
            }
        };
        observation.segments[index] = Some(NativePaintSegmentBenefitSample {
            identity: current.identity,
            span,
            revision: current.revision,
            target_generation,
            outcome,
            encoding_counts: counts,
        });
    }
    if expected_fresh != fresh_count || expected_reused != reused_count {
        return None;
    }
    Some(observation)
}

fn validate_common_evidence(
    paint: PaintSegmentObservation,
    encoding: PaintSegmentEncodingObservation,
    feasibility: ArtifactFeasibilityObservation,
    target_generation: NativeTargetGeneration,
) -> Option<usize> {
    let count = usize::from(paint.segment_count);
    if !target_generation.is_known()
        || paint.conservative
        || encoding.conservative
        || feasibility.conservative
        || count == 0
        || count > MAX_PAINT_SEGMENTS
        || encoding.segment_count != paint.segment_count
        || feasibility.segment_count as usize != count
        || feasibility.checkpoint_count as usize != count
        || paint.segments[count..].iter().any(Option::is_some)
        || encoding.segments[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
    {
        return None;
    }

    let mut previous_end = 0;
    let mut identities = [None; MAX_PAINT_SEGMENTS];
    for index in 0..count {
        let current = paint.segments[index]?;
        let encoded = encoding.segments[index]?;
        let evidence = feasibility.segments[index]?;
        let checkpoint = feasibility.checkpoints[index]?;
        let span = PaintSegmentSpan {
            identity: evidence.identity,
            start: evidence.primitive_start,
            end: evidence.primitive_end,
        };
        if current.identity != encoded.identity
            || current.identity != span.identity
            || span.start >= span.end
            || span.start < previous_end
            || checkpoint.primitive_end != span.end
            || checkpoint.primitive_end <= previous_end
            || identities[..index].contains(&Some(span.identity))
            || encoded.primitive_start != span.start
            || encoded.primitive_end != span.end
            || encoded.conservative
            || !matches!(encoded.isolation, EncodingIsolation::SelfContained)
            || matches!(encoded.safe_enclosure, SafeEnclosure::ViewportFallback)
            || !matches!(encoded.reason, EncodingConservativeReason::None)
            || segment_local_count_delta(feasibility, index).is_none()
        {
            return None;
        }
        identities[index] = Some(span.identity);
        previous_end = span.end;
    }
    Some(count)
}

fn valid_observation(observation: NativePaintSegmentBenefitObservation) -> bool {
    let count = usize::from(observation.segment_count);
    if !observation.target_generation.is_known()
        || count == 0
        || count > MAX_PAINT_SEGMENTS
        || observation.segments[count..].iter().any(Option::is_some)
    {
        return false;
    }
    let mut identities = [None; MAX_PAINT_SEGMENTS];
    for index in 0..count {
        let Some(sample) = observation.segments[index] else {
            return false;
        };
        if sample.identity != sample.span.identity
            || sample.span.start >= sample.span.end
            || sample.revision == 0
            || sample.target_generation != observation.target_generation
            || identities[..index].contains(&Some(sample.identity))
        {
            return false;
        }
        identities[index] = Some(sample.identity);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::{
        NativePaintSegmentEligibilityPlan, NativePaintSegmentFingerprint,
        NativePaintSegmentFreshEncodingReason,
    };
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::scene::{
        ArtifactFeasibilityCheckpoint, ArtifactFeasibilityDisposition, ArtifactFeasibilitySegment,
        PaintSegmentEncoding,
    };
    use crate::runtime::{PaintSegment, PaintSegmentAnchor};

    fn identity(key: u64) -> PaintSegmentIdentity {
        PaintSegmentIdentity {
            preceding: None,
            following: Some(PaintSegmentAnchor {
                widget_id: key,
                key,
            }),
        }
    }

    fn evidence(
        ids: &[u64],
        revisions: &[u64],
        generation: NativeTargetGeneration,
    ) -> (
        PaintSegmentObservation,
        PaintSegmentEncodingObservation,
        ArtifactFeasibilityObservation,
    ) {
        let count = ids.len();
        let mut paint = PaintSegmentObservation::empty();
        paint.segment_count = count as u8;
        let mut encoding = PaintSegmentEncodingObservation {
            segment_count: count as u8,
            ..PaintSegmentEncodingObservation::default()
        };
        let mut feasibility = ArtifactFeasibilityObservation {
            segments: [None; MAX_PAINT_SEGMENTS],
            checkpoints: [None; MAX_PAINT_SEGMENTS],
            segment_count: count as u8,
            checkpoint_count: count as u8,
            conservative: false,
        };
        let mut counts = ArtifactFeasibilityCounts::default();
        for index in 0..count {
            let identity = identity(ids[index]);
            let start = (index as u32) * 3;
            let end = start + 2;
            paint.segments[index] = Some(PaintSegment {
                identity,
                owner: None,
                revision: revisions[index],
                implicated: false,
            });
            encoding.segments[index] = Some(PaintSegmentEncoding {
                identity,
                primitive_start: start,
                primitive_end: end,
                safe_enclosure: SafeEnclosure::Empty,
                isolation: EncodingIsolation::SelfContained,
                conservative: false,
                reason: EncodingConservativeReason::None,
            });
            counts.draw_tags += 2;
            counts.draw_data += 3;
            feasibility.segments[index] = Some(ArtifactFeasibilitySegment {
                identity,
                primitive_start: start,
                primitive_end: end,
                disposition: ArtifactFeasibilityDisposition::ContiguousCandidate,
            });
            feasibility.checkpoints[index] = Some(ArtifactFeasibilityCheckpoint {
                primitive_end: end,
                counts,
            });
        }
        let _ = generation;
        (paint, encoding, feasibility)
    }

    fn plan_for(
        ids: &[u64],
        revisions: &[u64],
        generation: NativeTargetGeneration,
        fresh_index: Option<usize>,
    ) -> NativePaintSegmentEligibilityPlan {
        let mut plan = NativePaintSegmentEligibilityPlan {
            outcome: NativePaintSegmentEligibilityOutcome::Plan,
            entries: [None; MAX_PAINT_SEGMENTS],
            entry_count: ids.len() as u8,
        };
        for index in 0..ids.len() {
            let identity = identity(ids[index]);
            let span = PaintSegmentSpan {
                identity,
                start: (index as u32) * 3,
                end: (index as u32) * 3 + 2,
            };
            let disposition = if Some(index) == fresh_index {
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    NativePaintSegmentFreshEncodingReason::RevisionChanged,
                )
            } else {
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(
                    NativePaintSegmentFingerprint {
                        identity,
                        revision: revisions[index],
                        target_generation: generation,
                        primitive_start: span.start,
                        primitive_end: span.end,
                        safe_enclosure: SafeEnclosure::Empty,
                        isolation: EncodingIsolation::SelfContained,
                        conservative_reason: EncodingConservativeReason::None,
                    },
                )
            };
            plan.entries[index] =
                Some(super::super::NativePaintSegmentEligibilityEntry { span, disposition });
        }
        plan
    }

    #[test]
    fn full_encode_records_fresh_work_and_exact_segment_count_delta() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let (paint, encoding, feasibility) = evidence(&[1, 2], &[3, 4], generation);
        let mut ledger = NativePaintSegmentBenefitLedger::default();

        ledger.record_full_encode(paint, encoding, feasibility, generation, false);

        let summaries: Vec<_> = ledger.snapshot_for_test().into_iter().flatten().collect();
        assert_eq!(summaries.len(), 2);
        for summary in summaries {
            assert_eq!(summary.fresh_encoding_count, 1);
            assert_eq!(summary.avoided_encoding_count, 0);
            assert_eq!(summary.fresh_encoding_counts.draw_tags, 2);
            assert_eq!(summary.fresh_encoding_counts.draw_data, 3);
            assert_eq!(
                summary.latest_outcome,
                NativePaintSegmentBenefitOutcome::FreshEncoding
            );
        }
    }

    #[test]
    fn committed_mixed_assembly_records_reuse_and_fresh_segments_atomically() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let (paint, encoding, feasibility) = evidence(&[1, 2, 3], &[1, 2, 1], generation);
        let plan = plan_for(&[1, 2, 3], &[1, 2, 1], generation, Some(1));
        let mut ledger = NativePaintSegmentBenefitLedger::default();

        ledger.record_successful_assembly(NativePaintSegmentBenefitAssemblyInput {
            paint,
            encoding,
            feasibility,
            plan,
            target_generation: generation,
            fresh_count: 1,
            reused_count: 2,
            append_count: 3,
        });

        let summaries = ledger.snapshot_for_test();
        let first = summaries[0].expect("first segment");
        let middle = summaries[1].expect("fresh segment");
        assert_eq!(first.retained_reuse_count, 0);
        assert_eq!(first.mixed_reuse_count, 1);
        assert_eq!(first.avoided_encoding_count, 1);
        assert_eq!(first.avoided_encoding_counts.draw_tags, 2);
        assert_eq!(
            middle.latest_outcome,
            NativePaintSegmentBenefitOutcome::SuccessfulMixedFreshEncoding
        );
        assert_eq!(middle.fresh_encoding_count, 1);
        assert_eq!(middle.avoided_encoding_count, 0);
    }

    #[test]
    fn veto_repair_is_fresh_work_and_never_successful_reuse() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let (paint, encoding, feasibility) = evidence(&[1], &[1], generation);
        let mut ledger = NativePaintSegmentBenefitLedger::default();

        ledger.record_full_encode(paint, encoding, feasibility, generation, true);

        let summary = ledger.snapshot_for_test()[0].expect("repair sample");
        assert_eq!(summary.fresh_encoding_count, 1);
        assert_eq!(summary.assembly_veto_full_encode_count, 1);
        assert_eq!(summary.avoided_encoding_count, 0);
        assert_eq!(
            summary.latest_outcome,
            NativePaintSegmentBenefitOutcome::AssemblyVetoFullEncodeRepair
        );
    }

    #[test]
    fn malformed_or_mixed_generation_evidence_clears_without_partial_publish() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let (paint, encoding, feasibility) = evidence(&[1, 2], &[1, 1], generation);
        let mut ledger = NativePaintSegmentBenefitLedger::default();
        ledger.record_full_encode(paint, encoding, feasibility, generation, false);
        assert!(ledger.available_for_test());

        let mut malformed = encoding;
        malformed.segments[1].as_mut().unwrap().primitive_end += 1;
        ledger.record_full_encode(paint, malformed, feasibility, generation, false);
        assert!(!ledger.available_for_test());
        assert!(ledger.snapshot_for_test().iter().all(Option::is_none));

        let next_generation = NativeTargetGeneration::from_test_serial(2);
        let mut generation_mismatch_ledger = NativePaintSegmentBenefitLedger::default();
        generation_mismatch_ledger.record_full_encode(
            paint,
            encoding,
            feasibility,
            generation,
            false,
        );
        assert!(generation_mismatch_ledger.available_for_test());
        generation_mismatch_ledger.record_full_encode(
            paint,
            encoding,
            feasibility,
            next_generation,
            false,
        );
        assert!(!generation_mismatch_ledger.available_for_test());
        assert!(
            generation_mismatch_ledger
                .snapshot_for_test()
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn recent_window_expires_deterministically_without_growth() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let (paint, encoding, feasibility) = evidence(&[1], &[1], generation);
        let mut ledger = NativePaintSegmentBenefitLedger::default();
        for _ in 0..NativePaintSegmentBenefitLedger::HISTORY_WINDOW {
            ledger.record_full_encode(paint, encoding, feasibility, generation, false);
        }
        assert_eq!(
            ledger.snapshot_for_test()[0]
                .expect("bounded sample")
                .fresh_encoding_count,
            NativePaintSegmentBenefitLedger::HISTORY_WINDOW
        );
        ledger.record_full_encode(paint, encoding, feasibility, generation, false);
        assert_eq!(
            ledger.snapshot_for_test()[0]
                .expect("new window sample")
                .fresh_encoding_count,
            1
        );
        assert_eq!(
            ledger.observation_epoch_for_test(),
            NativePaintSegmentBenefitLedger::HISTORY_WINDOW + 1
        );
    }
}
