//! Private, observational admission state for native paint segments.
//!
//! This policy consumes only exact latest-frame benefit evidence. It does not
//! authorize reuse, retain renderer payloads, or participate in scene
//! eligibility or assembly.

use super::super::runner_state::NativeTargetGeneration;
use super::NativePaintSegmentBenefitFrameEvidence;
use super::benefit::NativePaintSegmentBenefitFrameSegment;
use crate::runtime::{MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentSpan};

const REUSE_OBSERVATIONS_TO_ADMIT: u8 = 2;
const LOW_BENEFIT_OBSERVATIONS_TO_DEMOTE: u8 = 3;
const CONFIDENCE_CAP: u8 = 8;
const PROMOTION_OBSERVATION_WINDOW_EPOCHS: u64 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativePaintSegmentCacheAdmissionState {
    Warming,
    Admitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePaintSegmentCacheAdmissionEntry {
    identity: PaintSegmentIdentity,
    span: PaintSegmentSpan,
    revision: u64,
    target_generation: NativeTargetGeneration,
    state: NativePaintSegmentCacheAdmissionState,
    confidence: u8,
    beneficial_reuse_observations: u8,
    low_benefit_observations: u8,
    promotion_window_start_epoch: u64,
}

/// Fixed-capacity, per-window observational cache-admission state.
///
/// The state is keyed by exact segment identity and target generation. It is
/// intentionally not consulted by any rendering path in this slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentCacheAdmission {
    entries: [Option<NativePaintSegmentCacheAdmissionEntry>; MAX_PAINT_SEGMENTS],
    target_generation: Option<NativeTargetGeneration>,
    last_processed_epoch: u64,
}

impl Default for NativePaintSegmentCacheAdmission {
    fn default() -> Self {
        Self {
            entries: [None; MAX_PAINT_SEGMENTS],
            target_generation: None,
            last_processed_epoch: 0,
        }
    }
}

impl NativePaintSegmentCacheAdmission {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn clear(&mut self) {
        self.entries = [None; MAX_PAINT_SEGMENTS];
        self.target_generation = None;
    }

    /// Return whether one exact current artifact may be published into the
    /// sparse materialization store.
    ///
    /// Warming is intentionally publication-eligible: it seeds the store so a
    /// later frame can observe non-zero retained reuse and promote the exact
    /// tuple to `Admitted`. This query remains observational; publication does
    /// not authorize reuse, assembly, rendering, or presentation.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn publication_eligible(
        &self,
        identity: PaintSegmentIdentity,
        span: PaintSegmentSpan,
        revision: u64,
        target_generation: NativeTargetGeneration,
    ) -> bool {
        if revision == 0
            || !target_generation.is_known()
            || span.identity != identity
            || span.start >= span.end
        {
            return false;
        }
        self.target_generation == Some(target_generation)
            && self.entries.iter().flatten().any(|entry| {
                entry.identity == identity
                    && entry.span == span
                    && entry.revision == revision
                    && entry.target_generation == target_generation
                    && matches!(
                        entry.state,
                        NativePaintSegmentCacheAdmissionState::Warming
                            | NativePaintSegmentCacheAdmissionState::Admitted
                    )
            })
    }

    /// Reconcile one exact latest-frame ledger projection atomically.
    ///
    /// Repeated processing of one accepted epoch is a no-op. Older epochs,
    /// unavailable evidence, malformed frames, generation fences, and an
    /// assembly veto all clear state without carrying a partial batch forward.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn reconcile(
        &mut self,
        evidence: NativePaintSegmentBenefitFrameEvidence,
    ) {
        if !evidence.available {
            self.clear();
            return;
        }
        if evidence.epoch < self.last_processed_epoch {
            self.clear();
            return;
        }
        if evidence.epoch == self.last_processed_epoch {
            return;
        }
        if !valid_evidence(evidence) {
            self.clear_for_epoch(evidence.epoch, None);
            return;
        }
        if self
            .target_generation
            .is_some_and(|generation| generation != evidence.target_generation)
        {
            self.clear_for_epoch(evidence.epoch, Some(evidence.target_generation));
            return;
        }
        if evidence.segments[..usize::from(evidence.segment_count)]
            .iter()
            .flatten()
            .any(|sample| sample.is_assembly_veto())
        {
            self.clear_for_epoch(evidence.epoch, Some(evidence.target_generation));
            return;
        }

        let count = usize::from(evidence.segment_count);
        let mut next = [None; MAX_PAINT_SEGMENTS];
        for (index, slot) in next.iter_mut().enumerate().take(count) {
            let Some(sample) = evidence.segments[index] else {
                self.clear_for_epoch(evidence.epoch, None);
                return;
            };
            let previous = self
                .entries
                .iter()
                .copied()
                .flatten()
                .find(|entry| entry.identity == sample.identity);
            *slot = Some(observe_sample(previous, sample, evidence.epoch));
        }
        self.entries = next;
        self.target_generation = Some(evidence.target_generation);
        self.last_processed_epoch = evidence.epoch;
    }

    fn clear_for_epoch(&mut self, epoch: u64, target_generation: Option<NativeTargetGeneration>) {
        self.entries = [None; MAX_PAINT_SEGMENTS];
        self.target_generation = target_generation;
        self.last_processed_epoch = self.last_processed_epoch.max(epoch);
    }

    #[cfg(test)]
    fn snapshot_for_test(
        &self,
    ) -> [Option<NativePaintSegmentCacheAdmissionEntrySnapshot>; MAX_PAINT_SEGMENTS] {
        self.entries.map(|entry| {
            entry.map(|entry| NativePaintSegmentCacheAdmissionEntrySnapshot {
                identity: entry.identity,
                span: entry.span,
                revision: entry.revision,
                target_generation: entry.target_generation,
                state: entry.state,
                confidence: entry.confidence,
                beneficial_reuse_observations: entry.beneficial_reuse_observations,
                low_benefit_observations: entry.low_benefit_observations,
                promotion_window_start_epoch: entry.promotion_window_start_epoch,
            })
        })
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn has_entries_for_test(
        &self,
    ) -> bool {
        self.entries.iter().any(Option::is_some)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn admitted_for_test(
        &self,
        identity: PaintSegmentIdentity,
    ) -> bool {
        self.entries.iter().flatten().any(|entry| {
            entry.identity == identity
                && matches!(entry.state, NativePaintSegmentCacheAdmissionState::Admitted)
        })
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn add_warming_for_test(
        &mut self,
        identity: PaintSegmentIdentity,
        span: PaintSegmentSpan,
        revision: u64,
        target_generation: NativeTargetGeneration,
    ) {
        if revision == 0
            || !target_generation.is_known()
            || span.identity != identity
            || span.start >= span.end
            || self
                .target_generation
                .is_some_and(|existing| existing != target_generation)
        {
            return;
        }
        let Some(slot) = self.entries.iter_mut().find(|slot| slot.is_none()) else {
            return;
        };
        *slot = Some(NativePaintSegmentCacheAdmissionEntry {
            identity,
            span,
            revision,
            target_generation,
            state: NativePaintSegmentCacheAdmissionState::Warming,
            confidence: 0,
            beneficial_reuse_observations: 0,
            low_benefit_observations: 0,
            promotion_window_start_epoch: 0,
        });
        self.target_generation = Some(target_generation);
    }
}

fn valid_evidence(evidence: NativePaintSegmentBenefitFrameEvidence) -> bool {
    let count = usize::from(evidence.segment_count);
    if evidence.epoch == 0
        || !evidence.target_generation.is_known()
        || count == 0
        || count > MAX_PAINT_SEGMENTS
        || evidence.segments[count..].iter().any(Option::is_some)
    {
        return false;
    }

    let mut identities = [None; MAX_PAINT_SEGMENTS];
    let mut previous_end = 0;
    for index in 0..count {
        let Some(sample) = evidence.segments[index] else {
            return false;
        };
        if sample.identity != sample.span.identity
            || sample.span.start >= sample.span.end
            || sample.span.start < previous_end
            || sample.revision == 0
            || sample.target_generation != evidence.target_generation
            || identities[..index].contains(&Some(sample.identity))
        {
            return false;
        }
        identities[index] = Some(sample.identity);
        previous_end = sample.span.end;
    }
    true
}

fn observe_sample(
    previous: Option<NativePaintSegmentCacheAdmissionEntry>,
    sample: NativePaintSegmentBenefitFrameSegment,
    epoch: u64,
) -> NativePaintSegmentCacheAdmissionEntry {
    let same_shape = previous.is_some_and(|entry| {
        entry.identity == sample.identity
            && entry.span == sample.span
            && entry.revision == sample.revision
            && entry.target_generation == sample.target_generation
    });
    let mut entry = if same_shape {
        previous.unwrap_or_else(|| new_entry(sample))
    } else {
        new_entry(sample)
    };
    entry.identity = sample.identity;
    entry.span = sample.span;
    entry.revision = sample.revision;
    entry.target_generation = sample.target_generation;

    if matches!(entry.state, NativePaintSegmentCacheAdmissionState::Warming)
        && entry.promotion_window_start_epoch != 0
        && epoch.saturating_sub(entry.promotion_window_start_epoch)
            >= PROMOTION_OBSERVATION_WINDOW_EPOCHS
    {
        entry.beneficial_reuse_observations = 0;
        entry.confidence = 0;
        entry.low_benefit_observations = 0;
        entry.promotion_window_start_epoch = 0;
    }

    if sample.is_beneficial_non_zero_work() {
        if matches!(entry.state, NativePaintSegmentCacheAdmissionState::Warming)
            && entry.promotion_window_start_epoch == 0
        {
            entry.promotion_window_start_epoch = epoch;
        }
        entry.beneficial_reuse_observations = entry.beneficial_reuse_observations.saturating_add(1);
        entry.confidence = entry.confidence.saturating_add(1).min(CONFIDENCE_CAP);
        entry.low_benefit_observations = 0;
        if entry.beneficial_reuse_observations >= REUSE_OBSERVATIONS_TO_ADMIT
            && entry.confidence >= REUSE_OBSERVATIONS_TO_ADMIT
        {
            entry.state = NativePaintSegmentCacheAdmissionState::Admitted;
        }
    } else {
        // Fresh encoding and zero-work reuse are low-benefit evidence. They
        // lower confidence but do not immediately demote an admitted entry.
        entry.confidence = entry.confidence.saturating_sub(1);
        entry.low_benefit_observations = entry.low_benefit_observations.saturating_add(1);
        if entry.low_benefit_observations >= LOW_BENEFIT_OBSERVATIONS_TO_DEMOTE {
            entry.state = NativePaintSegmentCacheAdmissionState::Warming;
            entry.confidence = 0;
            entry.beneficial_reuse_observations = 0;
            entry.low_benefit_observations = 0;
            entry.promotion_window_start_epoch = 0;
        }
    }
    entry
}

fn new_entry(
    sample: NativePaintSegmentBenefitFrameSegment,
) -> NativePaintSegmentCacheAdmissionEntry {
    NativePaintSegmentCacheAdmissionEntry {
        identity: sample.identity,
        span: sample.span,
        revision: sample.revision,
        target_generation: sample.target_generation,
        state: NativePaintSegmentCacheAdmissionState::Warming,
        confidence: 0,
        beneficial_reuse_observations: 0,
        low_benefit_observations: 0,
        promotion_window_start_epoch: 0,
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePaintSegmentCacheAdmissionEntrySnapshot {
    identity: PaintSegmentIdentity,
    span: PaintSegmentSpan,
    revision: u64,
    target_generation: NativeTargetGeneration,
    state: NativePaintSegmentCacheAdmissionState,
    confidence: u8,
    beneficial_reuse_observations: u8,
    low_benefit_observations: u8,
    promotion_window_start_epoch: u64,
}

#[cfg(test)]
mod native_paint_segment_cache_admission {
    use super::super::benefit::NativePaintSegmentBenefitOutcome;
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::scene::ArtifactFeasibilityCounts;
    use crate::runtime::PaintSegmentAnchor;

    fn identity(key: u64) -> PaintSegmentIdentity {
        PaintSegmentIdentity {
            preceding: None,
            following: Some(PaintSegmentAnchor {
                widget_id: key,
                key,
            }),
        }
    }

    fn known(serial: u64) -> NativeTargetGeneration {
        NativeTargetGeneration::from_test_serial(serial)
    }

    fn frame(
        epoch: u64,
        generation: NativeTargetGeneration,
        ids: &[u64],
        revisions: &[u64],
        outcomes: &[NativePaintSegmentBenefitOutcome],
        non_zero_work: &[bool],
    ) -> NativePaintSegmentBenefitFrameEvidence {
        assert_eq!(ids.len(), revisions.len());
        assert_eq!(ids.len(), outcomes.len());
        assert_eq!(ids.len(), non_zero_work.len());
        assert!(ids.len() <= MAX_PAINT_SEGMENTS);
        let mut evidence = NativePaintSegmentBenefitFrameEvidence {
            epoch,
            segments: [None; MAX_PAINT_SEGMENTS],
            segment_count: ids.len() as u8,
            target_generation: generation,
            available: true,
        };
        for index in 0..ids.len() {
            let segment_identity = identity(ids[index]);
            let span = PaintSegmentSpan {
                identity: segment_identity,
                start: (index as u32) * 3,
                end: (index as u32) * 3 + 2,
            };
            evidence.segments[index] = Some(NativePaintSegmentBenefitFrameSegment {
                identity: segment_identity,
                span,
                revision: revisions[index],
                target_generation: generation,
                outcome: outcomes[index],
                encoding_counts: if non_zero_work[index] {
                    ArtifactFeasibilityCounts {
                        draw_tags: 1,
                        ..ArtifactFeasibilityCounts::default()
                    }
                } else {
                    ArtifactFeasibilityCounts::default()
                },
            });
        }
        evidence
    }

    fn single(
        epoch: u64,
        generation: NativeTargetGeneration,
        id: u64,
        revision: u64,
        outcome: NativePaintSegmentBenefitOutcome,
        non_zero_work: bool,
    ) -> NativePaintSegmentBenefitFrameEvidence {
        frame(
            epoch,
            generation,
            &[id],
            &[revision],
            &[outcome],
            &[non_zero_work],
        )
    }

    fn beneficial(
        epoch: u64,
        generation: NativeTargetGeneration,
    ) -> NativePaintSegmentBenefitFrameEvidence {
        single(
            epoch,
            generation,
            1,
            1,
            NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse,
            true,
        )
    }

    fn promote(policy: &mut NativePaintSegmentCacheAdmission, generation: NativeTargetGeneration) {
        promote_at(policy, generation, 1);
    }

    fn promote_at(
        policy: &mut NativePaintSegmentCacheAdmission,
        generation: NativeTargetGeneration,
        first_epoch: u64,
    ) {
        policy.reconcile(beneficial(first_epoch, generation));
        policy.reconcile(beneficial(first_epoch + 1, generation));
    }

    #[test]
    fn cold_start_requires_two_beneficial_non_zero_work_reuses() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();

        policy.reconcile(beneficial(1, generation));
        assert!(!policy.admitted_for_test(identity(1)));
        let warming_entry = policy.snapshot_for_test()[0];
        assert!(warming_entry.is_some(), "warming entry");
        if let Some(entry) = warming_entry {
            assert_eq!(entry.beneficial_reuse_observations, 1);
        }

        policy.reconcile(beneficial(2, generation));
        assert!(policy.admitted_for_test(identity(1)));
    }

    #[test]
    fn publication_requires_exact_warming_or_admitted_tuple() {
        let generation = known(1);
        let segment_identity = identity(1);
        let span = PaintSegmentSpan {
            identity: segment_identity,
            start: 0,
            end: 2,
        };
        let mut policy = NativePaintSegmentCacheAdmission::default();

        assert!(!policy.publication_eligible(segment_identity, span, 1, generation,));

        policy.reconcile(beneficial(1, generation));
        assert!(policy.publication_eligible(segment_identity, span, 1, generation,));
        assert!(!policy.publication_eligible(segment_identity, span, 2, generation,));
        let other_identity = identity(2);
        assert!(!policy.publication_eligible(
            other_identity,
            PaintSegmentSpan {
                identity: other_identity,
                ..span
            },
            1,
            generation,
        ));
        assert!(!policy.publication_eligible(
            segment_identity,
            PaintSegmentSpan { end: 3, ..span },
            1,
            generation,
        ));
        assert!(!policy.publication_eligible(segment_identity, span, 1, known(2),));
        assert!(!policy.publication_eligible(segment_identity, span, 0, generation,));
        assert!(!policy.publication_eligible(
            segment_identity,
            span,
            1,
            NativeTargetGeneration::unknown(),
        ));

        policy.reconcile(beneficial(2, generation));
        assert!(policy.publication_eligible(segment_identity, span, 1, generation,));
    }

    #[test]
    fn zero_work_reuse_never_admits() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        for epoch in 1..=8 {
            policy.reconcile(single(
                epoch,
                generation,
                1,
                1,
                NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse,
                false,
            ));
        }
        assert!(!policy.admitted_for_test(identity(1)));
    }

    #[test]
    fn fresh_encoding_lowers_confidence_and_delays_promotion() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        policy.reconcile(beneficial(1, generation));
        let first_entry = policy.snapshot_for_test()[0];
        assert!(first_entry.is_some(), "first confidence entry");
        if let Some(entry) = first_entry {
            assert_eq!(entry.confidence, 1);
        }

        policy.reconcile(single(
            2,
            generation,
            1,
            1,
            NativePaintSegmentBenefitOutcome::FreshEncoding,
            true,
        ));
        let fresh_entry = policy.snapshot_for_test()[0];
        assert!(fresh_entry.is_some(), "fresh confidence entry");
        if let Some(entry) = fresh_entry {
            assert_eq!(entry.confidence, 0);
        }
        policy.reconcile(beneficial(3, generation));
        assert!(!policy.admitted_for_test(identity(1)));
    }

    #[test]
    fn intermittent_warming_evidence_expires_across_promotion_windows() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        for epoch in 1..=28 {
            let outcome = if (epoch - 1) % 3 == 0 {
                NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse
            } else {
                NativePaintSegmentBenefitOutcome::FreshEncoding
            };
            policy.reconcile(single(epoch, generation, 1, 1, outcome, true));
        }

        assert!(!policy.admitted_for_test(identity(1)));
        let entry = policy.snapshot_for_test()[0];
        assert!(entry.is_some(), "intermittent warming entry");
        if let Some(entry) = entry {
            assert_eq!(entry.state, NativePaintSegmentCacheAdmissionState::Warming);
            assert_eq!(entry.beneficial_reuse_observations, 1);
            assert_eq!(entry.confidence, 1);
            assert_eq!(entry.low_benefit_observations, 0);
        }
    }

    #[test]
    fn short_low_benefit_burst_does_not_demote_admitted_state() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        promote(&mut policy, generation);
        for epoch in 3..=4 {
            policy.reconcile(single(
                epoch,
                generation,
                1,
                1,
                NativePaintSegmentBenefitOutcome::FreshEncoding,
                true,
            ));
        }
        assert!(policy.admitted_for_test(identity(1)));
    }

    #[test]
    fn sustained_low_benefit_demotes_deterministically() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        promote(&mut policy, generation);
        for epoch in 3..=5 {
            policy.reconcile(single(
                epoch,
                generation,
                1,
                1,
                NativePaintSegmentBenefitOutcome::FreshEncoding,
                true,
            ));
        }
        assert!(!policy.admitted_for_test(identity(1)));
        let demoted_entry = policy.snapshot_for_test()[0];
        assert!(demoted_entry.is_some(), "demoted entry remains bounded");
        if let Some(entry) = demoted_entry {
            assert_eq!(entry.state, NativePaintSegmentCacheAdmissionState::Warming);
            assert!(policy.publication_eligible(
                entry.identity,
                entry.span,
                entry.revision,
                entry.target_generation,
            ));
        }
    }

    #[test]
    fn duplicate_epoch_is_idempotent_and_stale_epoch_clears() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        let first = beneficial(1, generation);
        policy.reconcile(first);
        policy.reconcile(first);
        let one_observation = policy.snapshot_for_test()[0];
        assert!(one_observation.is_some(), "one observation");
        if let Some(entry) = one_observation {
            assert_eq!(entry.beneficial_reuse_observations, 1);
        }
        policy.reconcile(beneficial(2, generation));
        assert!(policy.admitted_for_test(identity(1)));
        policy.reconcile(single(
            1,
            generation,
            1,
            1,
            NativePaintSegmentBenefitOutcome::FreshEncoding,
            true,
        ));
        assert!(!policy.has_entries_for_test());
    }

    #[test]
    fn invalid_duplicate_generation_veto_and_unavailable_evidence_clear_batch() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        promote(&mut policy, generation);

        let mut duplicate = frame(
            3,
            generation,
            &[1, 1],
            &[1, 1],
            &[
                NativePaintSegmentBenefitOutcome::FreshEncoding,
                NativePaintSegmentBenefitOutcome::FreshEncoding,
            ],
            &[true, true],
        );
        policy.reconcile(duplicate);
        assert!(!policy.has_entries_for_test());

        promote_at(&mut policy, generation, 4);
        let mut mixed = beneficial(6, generation);
        let mixed_segment = mixed.segments[0].as_mut();
        assert!(mixed_segment.is_some(), "mixed segment");
        if let Some(segment) = mixed_segment {
            segment.target_generation = known(2);
        }
        policy.reconcile(mixed);
        assert!(!policy.has_entries_for_test());

        promote_at(&mut policy, generation, 7);
        policy.reconcile(single(
            9,
            generation,
            1,
            1,
            NativePaintSegmentBenefitOutcome::AssemblyVetoFullEncodeRepair,
            true,
        ));
        assert!(!policy.has_entries_for_test());

        promote_at(&mut policy, generation, 10);
        policy.reconcile(NativePaintSegmentBenefitFrameEvidence::unavailable(12));
        assert!(!policy.has_entries_for_test());

        let duplicate_segment = duplicate.segments[0].as_mut();
        assert!(duplicate_segment.is_some(), "duplicate segment");
        if let Some(segment) = duplicate_segment {
            segment.span.end = 0;
        }
        duplicate.epoch = 13;
        policy.reconcile(duplicate);
        assert!(!policy.has_entries_for_test());
    }

    #[test]
    fn target_generation_change_clears_before_accepting_new_generation() {
        let generation_one = known(1);
        let generation_two = known(2);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        promote(&mut policy, generation_one);

        policy.reconcile(beneficial(3, generation_two));
        assert!(!policy.has_entries_for_test());

        policy.reconcile(beneficial(4, generation_two));
        policy.reconcile(beneficial(5, generation_two));
        assert!(policy.admitted_for_test(identity(1)));
    }

    #[test]
    fn disappearance_removes_identity_without_transferring_scores() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        let ids = [1, 2];
        let outcomes = [
            NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse,
            NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse,
        ];
        let work = [true, true];
        policy.reconcile(frame(1, generation, &ids, &[1, 1], &outcomes, &work));
        policy.reconcile(frame(2, generation, &ids, &[1, 1], &outcomes, &work));
        assert!(policy.admitted_for_test(identity(2)));

        policy.reconcile(single(
            3,
            generation,
            1,
            1,
            NativePaintSegmentBenefitOutcome::FreshEncoding,
            true,
        ));
        assert!(!policy.admitted_for_test(identity(2)));
        let removed_span = PaintSegmentSpan {
            identity: identity(2),
            start: 3,
            end: 5,
        };
        assert!(!policy.publication_eligible(identity(2), removed_span, 1, generation,));
        assert!(policy.has_entries_for_test());
    }

    #[test]
    fn revision_and_span_changes_never_authorize_reuse() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        promote(&mut policy, generation);

        let mut changed = beneficial(3, generation);
        let changed_sample = changed.segments[0].as_mut();
        assert!(changed_sample.is_some(), "changed evidence");
        if let Some(sample) = changed_sample {
            sample.revision = 2;
            sample.span.end = 3;
        }
        policy.reconcile(changed);
        assert!(!policy.admitted_for_test(identity(1)));
        let changed_entry = policy.snapshot_for_test()[0];
        assert!(changed_entry.is_some(), "changed evidence entry");
        if let Some(entry) = changed_entry {
            assert_eq!(entry.beneficial_reuse_observations, 1);
        }
    }

    #[test]
    fn capacity_and_counters_saturate_without_growth() {
        let generation = known(1);
        let mut policy = NativePaintSegmentCacheAdmission::default();
        let mut ids = [0; MAX_PAINT_SEGMENTS];
        let revisions = [1; MAX_PAINT_SEGMENTS];
        let outcomes =
            [NativePaintSegmentBenefitOutcome::SuccessfulRetainedReuse; MAX_PAINT_SEGMENTS];
        let work = [true; MAX_PAINT_SEGMENTS];
        for (index, id) in ids.iter_mut().enumerate() {
            *id = index as u64 + 1;
        }
        policy.reconcile(frame(1, generation, &ids, &revisions, &outcomes, &work));
        assert!(policy.has_entries_for_test());
        assert_eq!(
            policy.snapshot_for_test().iter().flatten().count(),
            MAX_PAINT_SEGMENTS
        );

        for epoch in 2..=300 {
            policy.reconcile(beneficial(epoch, generation));
        }
        let saturated_entry = policy.snapshot_for_test()[0];
        assert!(saturated_entry.is_some(), "saturated entry");
        if let Some(entry) = saturated_entry {
            assert_eq!(entry.beneficial_reuse_observations, u8::MAX);
            assert_eq!(entry.confidence, CONFIDENCE_CAP);
        }

        let mut overflow = beneficial(301, generation);
        overflow.segment_count = (MAX_PAINT_SEGMENTS + 1) as u8;
        policy.reconcile(overflow);
        assert!(!policy.has_entries_for_test());
    }
}
