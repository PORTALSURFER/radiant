//! Ephemeral Vello scene artifacts for an exact, validated eligibility plan.

use super::super::frame_state::NativeSceneValidityFingerprint;
use super::super::retained_paint_segments::{
    NativePaintSegmentEligibilityDisposition, NativePaintSegmentEligibilityEntry,
    NativePaintSegmentEligibilityOutcome, NativePaintSegmentFreshEncodingReason,
};
use super::super::runner_state::NativeTargetGeneration;
use super::artifact_feasibility::{
    ArtifactFeasibilityCounts, ArtifactFeasibilityDisposition, ArtifactFeasibilityObservation,
};
use super::{EncodingConservativeReason, EncodingIsolation, SafeEnclosure};
use crate::runtime::{MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentSpan};
use vello::Scene;

/// One ephemeral Vello artifact materialized from a completed authoritative
/// scene encoding. The evidence is retained beside the payload so auxiliary
/// payload preparation cannot lose the identity or generation fence that
/// authorized it.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifact {
    scene: Scene,
    identity: PaintSegmentIdentity,
    span: PaintSegmentSpan,
    revision: u64,
    target_generation: NativeTargetGeneration,
    scene_validity: NativeSceneValidityFingerprint,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativePaintSegmentAssemblyResult {
    Assembled(Box<Scene>),
    Veto(NativePaintSegmentAssemblyVetoReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativePaintSegmentAssemblyVetoReason
{
    InvalidPlan,
    MixedDisposition,
    InvalidEvidence,
    MissingArtifact,
    ArtifactMetadataMismatch,
    InvalidPayload,
    CheckpointMismatch,
}

/// Transactional, bounded materialization result for one encoded frame.
///
/// This is intentionally ephemeral. It owns no cache admission, retention,
/// replay, or rendering policy.
#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifactMaterialization {
    artifacts: Vec<NativePaintSegmentArtifact>,
}

impl NativePaintSegmentArtifactMaterialization {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn len(&self) -> usize {
        self.artifacts.len()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn artifacts_for_test(
        &self,
    ) -> &[NativePaintSegmentArtifact] {
        &self.artifacts
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn artifacts_for_test_mut(
        &mut self,
    ) -> &mut [NativePaintSegmentArtifact] {
        &mut self.artifacts
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn duplicate_first_for_test(&mut self) {
        let Some(first) = self.artifacts.first() else {
            return;
        };
        let duplicate = NativePaintSegmentArtifact {
            scene: first.scene.clone(),
            identity: first.identity,
            span: first.span,
            revision: first.revision,
            target_generation: first.target_generation,
            scene_validity: first.scene_validity,
        };
        self.artifacts.push(duplicate);
    }
}

impl NativePaintSegmentArtifact {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn scene_for_test(&self) -> &Scene {
        &self.scene
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn scene_for_test_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn identity_for_test(&self) -> PaintSegmentIdentity {
        self.identity
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn span_for_test(&self) -> PaintSegmentSpan {
        self.span
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn revision_for_test(&self) -> u64 {
        self.revision
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn target_generation_for_test(
        &self,
    ) -> NativeTargetGeneration {
        self.target_generation
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_target_generation_for_test(
        &mut self,
        target_generation: NativeTargetGeneration,
    ) {
        self.target_generation = target_generation;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_revision_for_test(&mut self, revision: u64) {
        self.revision = revision;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_identity_for_test(
        &mut self,
        identity: PaintSegmentIdentity,
    ) {
        self.identity = identity;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_span_start_for_test(&mut self, start: u32) {
        self.span.start = start;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_span_identity_for_test(
        &mut self,
        identity: PaintSegmentIdentity,
    ) {
        self.span.identity = identity;
    }
}

/// Fixed-capacity native artifacts retained for one fully encoded window.
///
/// Lookup is limited to exact metadata-matched resource-free payload clones for
/// auxiliary payload preparation. This store does not admit cache entries,
/// reconstruct authoritative scenes, or control rendering/presentation.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifactStore {
    artifacts: [Option<NativePaintSegmentArtifact>; MAX_PAINT_SEGMENTS],
}

impl Default for NativePaintSegmentArtifactStore {
    fn default() -> Self {
        Self {
            artifacts: [const { None }; MAX_PAINT_SEGMENTS],
        }
    }
}

impl NativePaintSegmentArtifactStore {
    /// Atomically replace the retained artifacts with one validated
    /// materialization. Invalid input clears the store rather than preserving
    /// stale artifacts.
    pub(in crate::gui_runtime::native_vello) fn reconcile(
        &mut self,
        materialization: NativePaintSegmentArtifactMaterialization,
    ) {
        if materialization.artifacts.is_empty() {
            self.clear();
            return;
        }
        let Some(next) = Self::validated_next(materialization) else {
            self.clear();
            return;
        };
        self.artifacts = next;
    }

    pub(in crate::gui_runtime::native_vello) fn clear(&mut self) {
        self.artifacts = [const { None }; MAX_PAINT_SEGMENTS];
    }

    pub(super) fn reusable_payload(
        &self,
        entry: NativePaintSegmentEligibilityEntry,
        scene_validity: NativeSceneValidityFingerprint,
    ) -> Option<Scene> {
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            return None;
        };

        self.artifacts.iter().flatten().find_map(|artifact| {
            (artifact.identity == fingerprint.identity
                && artifact.identity == entry.span.identity
                && artifact.span == entry.span
                && artifact.span.start == fingerprint.primitive_start
                && artifact.span.end == fingerprint.primitive_end
                && artifact.revision == fingerprint.revision
                && artifact.target_generation == fingerprint.target_generation
                && artifact.scene_validity == scene_validity)
                .then(|| artifact.scene.clone())
        })
    }

    fn artifact_for_assembly(
        &self,
        index: usize,
        entry: NativePaintSegmentEligibilityEntry,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> Result<&Scene, NativePaintSegmentAssemblyVetoReason> {
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            return Err(NativePaintSegmentAssemblyVetoReason::MixedDisposition);
        };
        let Some(artifact) = self.artifacts.get(index).and_then(Option::as_ref) else {
            return Err(NativePaintSegmentAssemblyVetoReason::MissingArtifact);
        };
        if artifact.identity != fingerprint.identity
            || artifact.identity != entry.span.identity
            || artifact.span != entry.span
            || artifact.revision != fingerprint.revision
            || artifact.target_generation != fingerprint.target_generation
            || artifact.target_generation != target_generation
        {
            return Err(NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch);
        }
        if artifact.scene_validity != scene_validity {
            return Err(NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch);
        }
        Ok(&artifact.scene)
    }

    fn has_artifact_after(&self, count: usize) -> bool {
        self.artifacts[count..].iter().any(Option::is_some)
    }

    fn validated_next(
        materialization: NativePaintSegmentArtifactMaterialization,
    ) -> Option<[Option<NativePaintSegmentArtifact>; MAX_PAINT_SEGMENTS]> {
        if materialization.artifacts.len() > MAX_PAINT_SEGMENTS {
            return None;
        }

        let mut generation = None;
        let mut previous_end = None;
        for (index, artifact) in materialization.artifacts.iter().enumerate() {
            let span = artifact.span;
            if artifact.identity != span.identity
                || artifact.revision == 0
                || !artifact.target_generation.is_known()
                || span.start >= span.end
                || previous_end.is_some_and(|end| span.start < end)
                || materialization.artifacts[..index]
                    .iter()
                    .any(|prior| prior.identity == artifact.identity)
                || generation.is_some_and(|existing| existing != artifact.target_generation)
            {
                return None;
            }
            generation = Some(artifact.target_generation);
            previous_end = Some(span.end);
        }

        let mut next = [const { None }; MAX_PAINT_SEGMENTS];
        for (index, artifact) in materialization.artifacts.into_iter().enumerate() {
            next[index] = Some(artifact);
        }
        Some(next)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn snapshot_identities(
        &self,
    ) -> [Option<PaintSegmentIdentity>; MAX_PAINT_SEGMENTS] {
        let mut identities = [None; MAX_PAINT_SEGMENTS];
        for (index, artifact) in self.artifacts.iter().enumerate() {
            identities[index] = artifact.as_ref().map(|artifact| artifact.identity);
        }
        identities
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn artifact_for_test_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut NativePaintSegmentArtifact> {
        self.artifacts.get_mut(index)?.as_mut()
    }
}

/// Assemble one exact retained-only plan into a scratch scene.
///
/// This is the authoritative retained path. It never asks the auxiliary
/// payload selector to encode or repair a segment: every plan entry must have
/// a matching materialized artifact, matching provenance, and matching
/// authoritative checkpoint evidence. The caller commits the returned scene
/// only after this function has validated the complete stream.
pub(in crate::gui_runtime::native_vello::generic_runtime) fn assemble_retained_native_paint_segment_scene(
    authoritative_scene: &Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
    artifacts: &NativePaintSegmentArtifactStore,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentAssemblyResult {
    let NativePaintSegmentEligibilityOutcome::Plan = plan.outcome else {
        return NativePaintSegmentAssemblyResult::Veto(
            NativePaintSegmentAssemblyVetoReason::InvalidPlan,
        );
    };
    let count = usize::from(plan.entry_count);
    if count == 0 || count > MAX_PAINT_SEGMENTS {
        return NativePaintSegmentAssemblyResult::Veto(
            NativePaintSegmentAssemblyVetoReason::InvalidPlan,
        );
    }
    if plan.entries[..count].iter().flatten().any(|entry| {
        !matches!(
            entry.disposition,
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
        )
    }) {
        return NativePaintSegmentAssemblyResult::Veto(
            NativePaintSegmentAssemblyVetoReason::MixedDisposition,
        );
    }
    if !target_generation.is_known()
        || feasibility.conservative
        || usize::from(feasibility.segment_count) != count
        || usize::from(feasibility.checkpoint_count) != count
        || plan.entries[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
        || artifacts.has_artifact_after(count)
        || !valid_no_resource_scene(authoritative_scene)
    {
        return NativePaintSegmentAssemblyResult::Veto(
            NativePaintSegmentAssemblyVetoReason::InvalidEvidence,
        );
    }

    let mut scratch = Scene::new();
    let mut previous_counts = ArtifactFeasibilityCounts::default();
    let mut previous_end = 0;
    let mut prior_identities = [None; MAX_PAINT_SEGMENTS];

    for index in 0..count {
        let (Some(entry), Some(evidence), Some(checkpoint)) = (
            plan.entries[index],
            feasibility.segments[index],
            feasibility.checkpoints[index],
        ) else {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::MissingArtifact,
            );
        };
        if !matches!(
            entry.disposition,
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(_)
        ) {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::MixedDisposition,
            );
        }
        if !valid_plan_and_evidence(
            entry,
            evidence,
            checkpoint,
            previous_counts,
            previous_end,
            &prior_identities,
            target_generation,
        ) {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::InvalidEvidence,
            );
        }

        let payload = match artifacts.artifact_for_assembly(
            index,
            entry,
            scene_validity,
            target_generation,
        ) {
            Ok(payload) => payload,
            Err(reason) => return NativePaintSegmentAssemblyResult::Veto(reason),
        };
        if !valid_no_resource_scene(payload) {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::InvalidPayload,
            );
        }

        scratch.append(payload, None);
        let assembled_counts = counts_from_scene(&scratch);
        if assembled_counts != checkpoint.counts {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::CheckpointMismatch,
            );
        }
        if !counts_grew_stream_from(assembled_counts, previous_counts) {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::InvalidEvidence,
            );
        }
        previous_counts = checkpoint.counts;
        previous_end = entry.span.end;
        prior_identities[index] = Some(entry.span.identity);
    }

    if !encoding_equivalent(&scratch, authoritative_scene) {
        return NativePaintSegmentAssemblyResult::Veto(
            NativePaintSegmentAssemblyVetoReason::InvalidPayload,
        );
    }

    NativePaintSegmentAssemblyResult::Assembled(Box::new(scratch))
}

/// Materialize artifacts from typed, independently valid scene payloads.
///
/// The payload vector is plan-ordered and contains exactly one scene per plan
/// entry. Every payload is validated before it participates in a fresh
/// assembled scene. The assembled scene must be encoding-equivalent to the
/// authoritative scene, so no artifact can escape from a partially validated
/// or merely count-compatible stream.
pub(in crate::gui_runtime::native_vello::generic_runtime) fn materialize_native_paint_segment_artifacts(
    admission: super::super::runner::NativePaintSegmentArtifactAdmission<'_>,
) -> NativePaintSegmentArtifactMaterialization {
    let (scene, feasibility, plan, payloads, scene_validity, target_generation) =
        admission.into_parts();
    let NativePaintSegmentEligibilityOutcome::Plan = plan.outcome else {
        return NativePaintSegmentArtifactMaterialization::default();
    };
    let count = usize::from(plan.entry_count);
    if count == 0
        || count > MAX_PAINT_SEGMENTS
        || payloads.len() != count
        || !target_generation.is_known()
        || feasibility.conservative
        || !valid_no_resource_scene(scene)
    {
        return NativePaintSegmentArtifactMaterialization::default();
    }
    if usize::from(feasibility.segment_count) != count
        || usize::from(feasibility.checkpoint_count) != count
        || plan.entries[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
    {
        return NativePaintSegmentArtifactMaterialization::default();
    }

    let mut assembled = Scene::new();
    let mut previous_counts = ArtifactFeasibilityCounts::default();
    let mut previous_end = 0;
    let mut prior_identities = [None; MAX_PAINT_SEGMENTS];
    let mut retained = [None; MAX_PAINT_SEGMENTS];

    for index in 0..count {
        let (Some(entry), Some(evidence), Some(checkpoint)) = (
            plan.entries[index],
            feasibility.segments[index],
            feasibility.checkpoints[index],
        ) else {
            return NativePaintSegmentArtifactMaterialization::default();
        };
        let payload = &payloads[index];
        if !valid_plan_and_evidence(
            entry,
            evidence,
            checkpoint,
            previous_counts,
            previous_end,
            &prior_identities,
            target_generation,
        ) || !valid_no_resource_scene(payload)
        {
            return NativePaintSegmentArtifactMaterialization::default();
        }

        assembled.append(payload, None);
        let assembled_counts = counts_from_scene(&assembled);
        if assembled_counts != checkpoint.counts {
            return NativePaintSegmentArtifactMaterialization::default();
        }

        if let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        {
            if !counts_grew_stream_from(assembled_counts, previous_counts) {
                return NativePaintSegmentArtifactMaterialization::default();
            }
            retained[index] = Some(fingerprint);
        }
        previous_counts = checkpoint.counts;
        previous_end = entry.span.end;
        prior_identities[index] = Some(entry.span.identity);
    }

    if !encoding_equivalent(&assembled, scene) {
        return NativePaintSegmentArtifactMaterialization::default();
    }

    let artifacts = payloads
        .into_iter()
        .enumerate()
        .filter_map(|(index, payload)| {
            let fingerprint = retained[index]?;
            let entry = plan.entries[index]?;
            Some(NativePaintSegmentArtifact {
                scene: payload,
                identity: fingerprint.identity,
                span: entry.span,
                revision: fingerprint.revision,
                target_generation: fingerprint.target_generation,
                scene_validity,
            })
        })
        .collect();
    NativePaintSegmentArtifactMaterialization { artifacts }
}

fn valid_plan_and_evidence(
    entry: super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry,
    evidence: super::artifact_feasibility::ArtifactFeasibilitySegment,
    checkpoint: super::artifact_feasibility::ArtifactFeasibilityCheckpoint,
    previous_counts: ArtifactFeasibilityCounts,
    previous_end: u32,
    prior_identities: &[Option<PaintSegmentIdentity>; MAX_PAINT_SEGMENTS],
    target_generation: NativeTargetGeneration,
) -> bool {
    if entry.span.identity != evidence.identity
        || entry.span.start != evidence.primitive_start
        || entry.span.end != evidence.primitive_end
        || entry.span.start >= entry.span.end
        || entry.span.start < previous_end
        || prior_identities.contains(&Some(entry.span.identity))
        || checkpoint.primitive_end != entry.span.end
        || checkpoint.primitive_end <= previous_end
        || !counts_monotonic_from(checkpoint.counts, previous_counts)
    {
        return false;
    }

    match entry.disposition {
        NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) => {
            evidence.disposition == ArtifactFeasibilityDisposition::ContiguousCandidate
                && fingerprint.identity == entry.span.identity
                && fingerprint.primitive_start == entry.span.start
                && fingerprint.primitive_end == entry.span.end
                && fingerprint.revision != 0
                && fingerprint.target_generation == target_generation
                && !matches!(fingerprint.safe_enclosure, SafeEnclosure::ViewportFallback)
                && matches!(fingerprint.isolation, EncodingIsolation::SelfContained)
                && matches!(
                    fingerprint.conservative_reason,
                    EncodingConservativeReason::None
                )
        }
        NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(reason) => {
            valid_fresh_reason(reason)
        }
    }
}

fn valid_fresh_reason(reason: NativePaintSegmentFreshEncodingReason) -> bool {
    matches!(
        reason,
        NativePaintSegmentFreshEncodingReason::RevisionChanged
            | NativePaintSegmentFreshEncodingReason::NoArtifact
            | NativePaintSegmentFreshEncodingReason::RequiresFreshEncoding(_)
    )
}

fn counts_from_scene(scene: &Scene) -> ArtifactFeasibilityCounts {
    let encoding = scene.encoding();
    ArtifactFeasibilityCounts {
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

fn counts_monotonic_from(
    current: ArtifactFeasibilityCounts,
    previous: ArtifactFeasibilityCounts,
) -> bool {
    current.path_tags >= previous.path_tags
        && current.path_data >= previous.path_data
        && current.draw_tags >= previous.draw_tags
        && current.draw_data >= previous.draw_data
        && current.transforms >= previous.transforms
        && current.styles >= previous.styles
        && current.n_paths >= previous.n_paths
        && current.n_path_segments >= previous.n_path_segments
        && current.n_clips >= previous.n_clips
        && current.n_open_clips >= previous.n_open_clips
        && current.patches >= previous.patches
        && current.color_stops >= previous.color_stops
        && current.glyphs >= previous.glyphs
        && current.glyph_runs >= previous.glyph_runs
        && current.normalized_coords >= previous.normalized_coords
}

fn counts_grew_stream_from(
    current: ArtifactFeasibilityCounts,
    previous: ArtifactFeasibilityCounts,
) -> bool {
    current.path_tags > previous.path_tags
        || current.path_data > previous.path_data
        || current.draw_tags > previous.draw_tags
        || current.draw_data > previous.draw_data
        || current.n_paths > previous.n_paths
        || current.n_path_segments > previous.n_path_segments
}

fn valid_no_resource_scene(scene: &Scene) -> bool {
    let encoding = scene.encoding();
    encoding.flags == 0
        && encoding.n_open_clips == 0
        && encoding.resources.patches.is_empty()
        && encoding.resources.color_stops.is_empty()
        && encoding.resources.glyphs.is_empty()
        && encoding.resources.glyph_runs.is_empty()
        && encoding.resources.normalized_coords.is_empty()
}

fn encoding_equivalent(actual: &Scene, expected: &Scene) -> bool {
    let actual = actual.encoding();
    let expected = expected.encoding();
    actual.path_tags == expected.path_tags
        && actual.path_data == expected.path_data
        && actual.draw_tags == expected.draw_tags
        && actual.draw_data == expected.draw_data
        && actual.transforms == expected.transforms
        && actual.styles == expected.styles
        && actual.n_paths == expected.n_paths
        && actual.n_path_segments == expected.n_path_segments
        && actual.n_clips == expected.n_clips
        && actual.n_open_clips == expected.n_open_clips
        && actual.flags == expected.flags
        && actual.resources.patches.is_empty()
        && actual.resources.color_stops.is_empty()
        && actual.resources.glyphs.is_empty()
        && actual.resources.glyph_runs.is_empty()
        && actual.resources.normalized_coords.is_empty()
        && expected.resources.patches.is_empty()
        && expected.resources.color_stops.is_empty()
        && expected.resources.glyphs.is_empty()
        && expected.resources.glyph_runs.is_empty()
        && expected.resources.normalized_coords.is_empty()
}
