//! Metadata-only retained paint-segment evidence for one native window.
//!
//! This module deliberately stores fingerprints only. It does not retain a
//! Vello scene or any renderer payload and has no cache-hit or replay policy.

use super::{
    PaintSegmentEncodingObservation,
    runner_state::NativeTargetGeneration,
    scene::{EncodingConservativeReason, EncodingIsolation, SafeEnclosure},
};
use crate::runtime::{MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentObservation};

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

/// Fixed-capacity native fingerprint evidence for one fully encoded scene.
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

/// Assemble renderer-local fingerprints only after the complete scene encode.
pub(super) fn assemble_native_paint_segment_fingerprints(
    paint: PaintSegmentObservation,
    encoding: PaintSegmentEncodingObservation,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentFingerprintObservation {
    if !target_generation.is_known()
        || paint.conservative
        || encoding.conservative
        || paint.segment_count != encoding.segment_count
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

#[cfg(test)]
mod tests {
    use super::*;
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
