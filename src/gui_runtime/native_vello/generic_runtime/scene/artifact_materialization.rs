//! Ephemeral Vello scene artifacts for an exact, validated eligibility plan.

use super::super::frame_state::NativeSceneValidityFingerprint;
use super::super::retained_paint_segments::{
    NativePaintSegmentEligibilityDisposition, NativePaintSegmentEligibilityEntry,
    NativePaintSegmentEligibilityOutcome, NativePaintSegmentEligibilityPlan,
    NativePaintSegmentFreshEncodingReason,
};
use super::super::runner_state::NativeTargetGeneration;
use super::artifact_feasibility::{
    ArtifactFeasibilityCheckpoint, ArtifactFeasibilityCounts, ArtifactFeasibilityDisposition,
    ArtifactFeasibilityObservation, ArtifactFeasibilitySegment,
};
use super::{
    EncodingConservativeReason, EncodingIsolation, PaintSegmentEncoding,
    PaintSegmentEncodingObservation, RetainedSurfaceEncodeStats, SafeEnclosure, SceneClipEnd,
    SceneClipState,
};
use crate::{
    gui::types::Vector2,
    runtime::{
        MAX_PAINT_SEGMENTS, PaintPrimitive, PaintSegmentIdentity, PaintSegmentObservation,
        PaintSegmentSpan, collect_segment_spans,
    },
};
use vello::Scene;

/// Evidence carried with one resource-free native paint payload.
///
/// The payload and this evidence are one typed operation result.  Keeping the
/// provenance beside the scene prevents an untyped scene clone from becoming
/// an authority for native assembly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentPayloadEvidence {
    pub(in crate::gui_runtime::native_vello) identity: PaintSegmentIdentity,
    pub(in crate::gui_runtime::native_vello) span: PaintSegmentSpan,
    pub(in crate::gui_runtime::native_vello) revision: u64,
    pub(in crate::gui_runtime::native_vello) target_generation: NativeTargetGeneration,
    pub(in crate::gui_runtime::native_vello) scene_validity: NativeSceneValidityFingerprint,
    pub(in crate::gui_runtime::native_vello) encoding: PaintSegmentEncoding,
    pub(in crate::gui_runtime::native_vello) counts: ArtifactFeasibilityCounts,
    pub(in crate::gui_runtime::native_vello) clip_layer_count: usize,
}

#[derive(Clone)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentPayload {
    pub(in crate::gui_runtime::native_vello) scene: Scene,
    pub(in crate::gui_runtime::native_vello) evidence: NativePaintSegmentPayloadEvidence,
}

/// Fully staged native segment assembly.  Nothing in this bundle has been
/// installed into frame state yet.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentAssemblyBundle {
    pub(in crate::gui_runtime::native_vello) scene: Scene,
    pub(in crate::gui_runtime::native_vello) stats: RetainedSurfaceEncodeStats,
    pub(in crate::gui_runtime::native_vello) materialization:
        NativePaintSegmentArtifactMaterialization,
    pub(in crate::gui_runtime::native_vello) plan: NativePaintSegmentEligibilityPlan,
    pub(in crate::gui_runtime::native_vello) paint: PaintSegmentObservation,
    pub(in crate::gui_runtime::native_vello) target_generation: NativeTargetGeneration,
    pub(in crate::gui_runtime::native_vello) fresh_count: usize,
    pub(in crate::gui_runtime::native_vello) reused_count: usize,
    pub(in crate::gui_runtime::native_vello) append_count: usize,
}

#[derive(Clone, Copy)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentAssemblyInput<'a>
{
    /// Previous committed scene, retained only as a resource-free source fence.
    /// Mixed assembly does not use it as a current-frame encoding oracle.
    pub(in crate::gui_runtime::native_vello::generic_runtime) previous_scene: &'a Scene,
    pub(in crate::gui_runtime::native_vello::generic_runtime) primitives: &'a [PaintPrimitive],
    pub(in crate::gui_runtime::native_vello::generic_runtime) viewport: Vector2,
    pub(in crate::gui_runtime::native_vello::generic_runtime) paint: PaintSegmentObservation,
    pub(in crate::gui_runtime::native_vello::generic_runtime) previous_stats:
        RetainedSurfaceEncodeStats,
    pub(in crate::gui_runtime::native_vello::generic_runtime) plan:
        NativePaintSegmentEligibilityPlan,
    pub(in crate::gui_runtime::native_vello::generic_runtime) artifacts:
        &'a NativePaintSegmentArtifactStore,
    pub(in crate::gui_runtime::native_vello::generic_runtime) scene_validity:
        NativeSceneValidityFingerprint,
    pub(in crate::gui_runtime::native_vello::generic_runtime) previous_scene_validity:
        Option<NativeSceneValidityFingerprint>,
    pub(in crate::gui_runtime::native_vello::generic_runtime) target_generation:
        NativeTargetGeneration,
}

/// One ephemeral Vello artifact materialized from a completed authoritative
/// scene encoding. The evidence is retained beside the payload so auxiliary
/// payload preparation cannot lose the identity or generation fence that
/// authorized it.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifact {
    plan_index: u8,
    payload: NativePaintSegmentPayload,
}

#[cfg(test)]
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
    InvalidCurrentObservation,
    ContextMismatch,
    #[cfg(test)]
    MissingArtifact,
    ArtifactMetadataMismatch,
    UnsupportedFreshPrimitive,
    InvalidFreshPayload,
    InvalidPayload,
    #[cfg(test)]
    CheckpointMismatch,
}

enum NativePaintSegmentArtifactAssemblyLookup<'a> {
    Exact(&'a NativePaintSegmentPayload),
    Absent,
    Invalid(NativePaintSegmentAssemblyVetoReason),
}

/// Transactional, bounded materialization result for one encoded frame.
///
/// This is intentionally ephemeral. It owns no cache admission, retention,
/// replay, or rendering policy.
#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifactMaterialization {
    plan_entry_count: u8,
    scene_validity: Option<NativeSceneValidityFingerprint>,
    artifacts: Vec<NativePaintSegmentArtifact>,
}

impl NativePaintSegmentArtifactMaterialization {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn len(&self) -> usize {
        self.artifacts.len()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn plan_entry_count_for_test(&self) -> usize {
        usize::from(self.plan_entry_count)
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
    pub(in crate::gui_runtime::native_vello) fn remove_artifact_for_test(
        &mut self,
        plan_index: usize,
    ) -> bool {
        let Some(index) = self
            .artifacts
            .iter()
            .position(|artifact| usize::from(artifact.plan_index) == plan_index)
        else {
            return false;
        };
        self.artifacts.remove(index);
        true
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn clear_artifacts_for_test(&mut self) {
        self.artifacts.clear();
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_plan_entry_count_for_test(
        &mut self,
        plan_entry_count: usize,
    ) {
        self.plan_entry_count = plan_entry_count as u8;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn duplicate_first_for_test(&mut self) {
        let Some(first) = self.artifacts.first() else {
            return;
        };
        let duplicate = NativePaintSegmentArtifact {
            plan_index: first.plan_index,
            payload: first.payload.clone(),
        };
        self.artifacts.push(duplicate);
    }
}

impl NativePaintSegmentArtifact {
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn scene_for_test(&self) -> &Scene {
        &self.payload.scene
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn scene_for_test_mut(&mut self) -> &mut Scene {
        &mut self.payload.scene
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn identity_for_test(&self) -> PaintSegmentIdentity {
        self.payload.evidence.identity
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn plan_index_for_test(&self) -> usize {
        usize::from(self.plan_index)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn span_for_test(&self) -> PaintSegmentSpan {
        self.payload.evidence.span
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn revision_for_test(&self) -> u64 {
        self.payload.evidence.revision
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn target_generation_for_test(
        &self,
    ) -> NativeTargetGeneration {
        self.payload.evidence.target_generation
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_target_generation_for_test(
        &mut self,
        target_generation: NativeTargetGeneration,
    ) {
        self.payload.evidence.target_generation = target_generation;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_revision_for_test(&mut self, revision: u64) {
        self.payload.evidence.revision = revision;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_plan_index_for_test(
        &mut self,
        plan_index: usize,
    ) {
        self.plan_index = plan_index as u8;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_scene_validity_for_test(
        &mut self,
        scene_validity: NativeSceneValidityFingerprint,
    ) {
        self.payload.evidence.scene_validity = scene_validity;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_identity_for_test(
        &mut self,
        identity: PaintSegmentIdentity,
    ) {
        self.payload.evidence.identity = identity;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_span_start_for_test(&mut self, start: u32) {
        self.payload.evidence.span.start = start;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_span_identity_for_test(
        &mut self,
        identity: PaintSegmentIdentity,
    ) {
        self.payload.evidence.span.identity = identity;
    }
}

#[cfg(test)]
impl NativePaintSegmentPayload {
    pub(in crate::gui_runtime::native_vello) fn scene_for_test(&self) -> &Scene {
        &self.scene
    }

    pub(in crate::gui_runtime::native_vello) fn scene_for_test_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
}

/// Fixed-capacity native artifacts retained for one fully encoded window.
///
/// Lookup is limited to exact metadata-matched resource-free payload clones for
/// auxiliary payload preparation. This store does not admit cache entries,
/// reconstruct authoritative scenes, or control rendering/presentation.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifactStore {
    artifacts: [Option<NativePaintSegmentArtifact>; MAX_PAINT_SEGMENTS],
    plan_entry_count: u8,
    scene_validity: Option<NativeSceneValidityFingerprint>,
}

impl Default for NativePaintSegmentArtifactStore {
    fn default() -> Self {
        Self {
            artifacts: [const { None }; MAX_PAINT_SEGMENTS],
            plan_entry_count: 0,
            scene_validity: None,
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
        if materialization.plan_entry_count == 0 {
            self.clear();
            return;
        }
        let plan_entry_count = materialization.plan_entry_count;
        let scene_validity = materialization.scene_validity;
        let Some(next) = Self::validated_next(materialization) else {
            self.clear();
            return;
        };
        self.artifacts = next;
        self.plan_entry_count = plan_entry_count;
        self.scene_validity = scene_validity;
    }

    pub(in crate::gui_runtime::native_vello) fn clear(&mut self) {
        self.artifacts = [const { None }; MAX_PAINT_SEGMENTS];
        self.plan_entry_count = 0;
        self.scene_validity = None;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn set_plan_entry_count_for_test(
        &mut self,
        plan_entry_count: usize,
    ) {
        self.plan_entry_count = plan_entry_count as u8;
    }

    pub(super) fn reusable_payload(
        &self,
        index: usize,
        entry: NativePaintSegmentEligibilityEntry,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> Option<NativePaintSegmentPayload> {
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            return None;
        };
        if index >= usize::from(self.plan_entry_count)
            || !target_generation.is_known()
            || fingerprint.revision == 0
            || fingerprint.target_generation != target_generation
            || self.scene_validity != Some(scene_validity)
        {
            return None;
        }
        let artifact = self.artifacts.get(index).and_then(Option::as_ref)?;
        if usize::from(artifact.plan_index) != index {
            return None;
        }
        let evidence = artifact.payload.evidence;
        (evidence.identity == fingerprint.identity
            && evidence.identity == entry.span.identity
            && evidence.span == entry.span
            && evidence.span.start == fingerprint.primitive_start
            && evidence.span.end == fingerprint.primitive_end
            && evidence.revision != 0
            && evidence.revision == fingerprint.revision
            && evidence.target_generation.is_known()
            && evidence.target_generation == fingerprint.target_generation
            && evidence.scene_validity == scene_validity)
            .then(|| artifact.payload.clone())
    }

    fn lookup_for_mixed_assembly(
        &self,
        index: usize,
        count: usize,
        entry: NativePaintSegmentEligibilityEntry,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> NativePaintSegmentArtifactAssemblyLookup<'_> {
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            return NativePaintSegmentArtifactAssemblyLookup::Invalid(
                NativePaintSegmentAssemblyVetoReason::MixedDisposition,
            );
        };
        if self.plan_entry_count as usize != count || self.scene_validity != Some(scene_validity) {
            return NativePaintSegmentArtifactAssemblyLookup::Invalid(
                NativePaintSegmentAssemblyVetoReason::InvalidEvidence,
            );
        }
        let Some(artifact) = self.artifacts.get(index).and_then(Option::as_ref) else {
            return NativePaintSegmentArtifactAssemblyLookup::Absent;
        };
        if usize::from(artifact.plan_index) != index {
            return NativePaintSegmentArtifactAssemblyLookup::Invalid(
                NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch,
            );
        }
        let evidence = artifact.payload.evidence;
        if evidence.identity != fingerprint.identity
            || evidence.identity != entry.span.identity
            || evidence.span != entry.span
            || evidence.revision != fingerprint.revision
            || evidence.target_generation != fingerprint.target_generation
            || evidence.target_generation != target_generation
            || !evidence.target_generation.is_known()
            || evidence.scene_validity != scene_validity
        {
            return NativePaintSegmentArtifactAssemblyLookup::Invalid(
                NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch,
            );
        }
        NativePaintSegmentArtifactAssemblyLookup::Exact(&artifact.payload)
    }

    #[cfg(test)]
    fn artifact_for_assembly(
        &self,
        index: usize,
        entry: NativePaintSegmentEligibilityEntry,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> Result<&NativePaintSegmentPayload, NativePaintSegmentAssemblyVetoReason> {
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            return Err(NativePaintSegmentAssemblyVetoReason::MixedDisposition);
        };
        let Some(artifact) = self.artifacts.get(index).and_then(Option::as_ref) else {
            return Err(NativePaintSegmentAssemblyVetoReason::MissingArtifact);
        };
        if usize::from(artifact.plan_index) != index {
            return Err(NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch);
        }
        let evidence = artifact.payload.evidence;
        if evidence.identity != fingerprint.identity
            || evidence.identity != entry.span.identity
            || evidence.span != entry.span
            || evidence.revision != fingerprint.revision
            || evidence.target_generation != fingerprint.target_generation
            || evidence.target_generation != target_generation
            || !evidence.target_generation.is_known()
            || self.scene_validity != Some(scene_validity)
        {
            return Err(NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch);
        }
        if evidence.scene_validity != scene_validity {
            return Err(NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch);
        }
        Ok(&artifact.payload)
    }

    fn has_artifact_after(&self, count: usize) -> bool {
        self.artifacts[count..].iter().any(Option::is_some)
    }

    fn has_invalid_bounded_artifact_state(&self, count: usize) -> bool {
        if count > MAX_PAINT_SEGMENTS
            || self.has_artifact_after(count)
            || self.plan_entry_count as usize > MAX_PAINT_SEGMENTS
            || (self.plan_entry_count == 0 && self.artifacts.iter().any(Option::is_some))
            || (self.plan_entry_count != 0 && self.plan_entry_count as usize != count)
        {
            return true;
        }
        let mut previous_end = None;
        let mut identities = [None; MAX_PAINT_SEGMENTS];
        let mut generation = None;
        for index in 0..count {
            let Some(artifact) = self.artifacts[index].as_ref() else {
                continue;
            };
            let evidence = artifact.payload.evidence;
            if usize::from(artifact.plan_index) != index
                || evidence.identity != evidence.span.identity
                || evidence.span.start >= evidence.span.end
                || evidence.revision == 0
                || !evidence.target_generation.is_known()
                || previous_end.is_some_and(|end| evidence.span.start < end)
                || identities[..index].contains(&Some(evidence.identity))
                || generation.is_some_and(|existing| existing != evidence.target_generation)
            {
                return true;
            }
            identities[index] = Some(evidence.identity);
            previous_end = Some(evidence.span.end);
            generation = Some(evidence.target_generation);
        }
        false
    }

    fn validated_next(
        materialization: NativePaintSegmentArtifactMaterialization,
    ) -> Option<[Option<NativePaintSegmentArtifact>; MAX_PAINT_SEGMENTS]> {
        let plan_entry_count = usize::from(materialization.plan_entry_count);
        let scene_validity = materialization.scene_validity?;
        if plan_entry_count == 0
            || plan_entry_count > MAX_PAINT_SEGMENTS
            || materialization.artifacts.len() > plan_entry_count
        {
            return None;
        }

        let mut generation = None;
        let mut plan_positions = [None; MAX_PAINT_SEGMENTS];
        let mut identities = [None; MAX_PAINT_SEGMENTS];
        for (resident_index, artifact) in materialization.artifacts.iter().enumerate() {
            let plan_index = usize::from(artifact.plan_index);
            let evidence = artifact.payload.evidence;
            let span = evidence.span;
            if plan_index >= plan_entry_count
                || plan_positions[plan_index].is_some()
                || evidence.identity != span.identity
                || evidence.revision == 0
                || !evidence.target_generation.is_known()
                || evidence.scene_validity != scene_validity
                || span.start >= span.end
                || identities[..resident_index].contains(&Some(evidence.identity))
                || generation.is_some_and(|existing| existing != evidence.target_generation)
            {
                return None;
            }
            plan_positions[plan_index] = Some(resident_index);
            identities[resident_index] = Some(evidence.identity);
            generation = Some(evidence.target_generation);
        }

        let mut previous_end = None;
        let mut next = [const { None }; MAX_PAINT_SEGMENTS];
        for resident_index in plan_positions.iter().flatten() {
            let artifact = &materialization.artifacts[*resident_index];
            if previous_end.is_some_and(|end| artifact.payload.evidence.span.start < end) {
                return None;
            }
            previous_end = Some(artifact.payload.evidence.span.end);
        }

        for artifact in materialization.artifacts.into_iter() {
            let plan_index = usize::from(artifact.plan_index);
            next[plan_index] = Some(artifact);
        }
        Some(next)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn snapshot_identities(
        &self,
    ) -> [Option<PaintSegmentIdentity>; MAX_PAINT_SEGMENTS] {
        let mut identities = [None; MAX_PAINT_SEGMENTS];
        for (index, artifact) in self.artifacts.iter().enumerate() {
            identities[index] = artifact
                .as_ref()
                .map(|artifact| artifact.payload.evidence.identity);
        }
        identities
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn plan_entry_count_for_test(&self) -> usize {
        usize::from(self.plan_entry_count)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn resident_count_for_test(&self) -> usize {
        self.artifacts.iter().flatten().count()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn artifact_for_test_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut NativePaintSegmentArtifact> {
        self.artifacts.get_mut(index)?.as_mut()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn clear_artifact_for_test(
        &mut self,
        index: usize,
    ) -> bool {
        let Some(slot) = self.artifacts.get_mut(index) else {
            return false;
        };
        slot.take().is_some()
    }
}

/// Preflight and assemble one current plan without mutating frame state.
///
/// The previous scene is required to remain a valid resource-free retained
/// source. Current payload correctness is established by typed identity,
/// revision, span, target-generation, scene-validity, append, and checkpoint
/// validation below; the bounded mixed path does not compare its new stream
/// with the previous scene.
pub(in crate::gui_runtime::native_vello::generic_runtime) fn assemble_mixed_native_paint_segment_scene(
    input: NativePaintSegmentAssemblyInput<'_>,
) -> Result<NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyVetoReason> {
    let (count, execution_plan) = preflight_mixed_plan(&input)?;
    let input = NativePaintSegmentAssemblyInput {
        plan: execution_plan,
        ..input
    };

    let selection = super::encode_native_paint_segment_payloads(
        input.primitives,
        input.viewport,
        input.paint,
        input.plan,
        input.scene_validity,
        input.target_generation,
        input.artifacts,
    );
    let (payloads, fresh_count, reused_count) = selection.into_parts();
    if payloads.len() != count || fresh_count.saturating_add(reused_count) != count {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidFreshPayload);
    }

    assemble_payload_stream(input, payloads, fresh_count, reused_count)
}

fn preflight_mixed_plan(
    input: &NativePaintSegmentAssemblyInput<'_>,
) -> Result<(usize, NativePaintSegmentEligibilityPlan), NativePaintSegmentAssemblyVetoReason> {
    let NativePaintSegmentAssemblyInput {
        previous_scene,
        primitives,
        paint,
        previous_stats,
        plan,
        artifacts,
        scene_validity,
        previous_scene_validity,
        target_generation,
        ..
    } = *input;
    let NativePaintSegmentEligibilityOutcome::Plan = plan.outcome else {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidPlan);
    };
    let count = usize::from(plan.entry_count);
    if count == 0 || count > MAX_PAINT_SEGMENTS {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidPlan);
    }
    if previous_scene_validity != Some(scene_validity) {
        return Err(NativePaintSegmentAssemblyVetoReason::ContextMismatch);
    }
    let feasibility = previous_stats.artifact_feasibility;
    if !target_generation.is_known()
        || paint.conservative
        || paint.all_implicated
        || feasibility.conservative
        || !valid_no_resource_scene(previous_scene)
    {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidCurrentObservation);
    }

    let mut current_spans = [None; MAX_PAINT_SEGMENTS];
    let (current_span_count, current_spans_malformed) =
        collect_segment_spans(primitives, &mut current_spans);
    if current_spans_malformed
        || usize::from(current_span_count) != count
        || paint.segment_count as usize != count
        || usize::from(feasibility.segment_count) != count
        || usize::from(feasibility.checkpoint_count) != count
        || plan.entries[count..].iter().any(Option::is_some)
        || paint.segments[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
        || previous_stats.segment_encoding.segment_count as usize != count
        || previous_stats.segment_encoding.segments[count..]
            .iter()
            .any(Option::is_some)
        || artifacts.has_invalid_bounded_artifact_state(count)
    {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
    }

    let mut previous_end = 0;
    let mut prior_identities = [None; MAX_PAINT_SEGMENTS];
    let mut execution_plan = plan;

    for index in 0..count {
        let (Some(entry), Some(current), Some(current_span), Some(evidence), Some(checkpoint)) = (
            plan.entries[index],
            paint.segments[index],
            current_spans[index],
            feasibility.segments[index],
            feasibility.checkpoints[index],
        ) else {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidCurrentObservation);
        };
        if current.revision == 0
            || current.identity != entry.span.identity
            || current_span != entry.span
            || entry.span.start < previous_end
            || prior_identities[..index].contains(&Some(entry.span.identity))
            || !valid_plan_and_evidence(
                entry,
                evidence,
                checkpoint,
                if index == 0 {
                    ArtifactFeasibilityCounts::default()
                } else {
                    feasibility.checkpoints[index - 1]
                        .map_or(ArtifactFeasibilityCounts::default(), |checkpoint| {
                            checkpoint.counts
                        })
                },
                previous_end,
                &prior_identities,
                target_generation,
            )
        {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
        }

        match entry.disposition {
            NativePaintSegmentEligibilityDisposition::RetainedCandidate(_) => {
                let lookup = artifacts.lookup_for_mixed_assembly(
                    index,
                    count,
                    entry,
                    scene_validity,
                    target_generation,
                );
                match lookup {
                    NativePaintSegmentArtifactAssemblyLookup::Exact(payload) => {
                        if !valid_payload_metadata(
                            payload,
                            entry,
                            current.revision,
                            scene_validity,
                            target_generation,
                        ) || !valid_no_resource_scene(&payload.scene)
                        {
                            return Err(NativePaintSegmentAssemblyVetoReason::InvalidPayload);
                        }
                    }
                    NativePaintSegmentArtifactAssemblyLookup::Absent => {
                        if let Some(entry) = execution_plan.entries[index].as_mut() {
                            entry.disposition =
                                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                                    NativePaintSegmentFreshEncodingReason::NoArtifact,
                                );
                        }
                        if !supported_fresh_span(primitives, entry.span) {
                            return Err(
                                NativePaintSegmentAssemblyVetoReason::UnsupportedFreshPrimitive,
                            );
                        }
                    }
                    NativePaintSegmentArtifactAssemblyLookup::Invalid(reason) => {
                        return Err(reason);
                    }
                }
            }
            NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_) => {
                if !supported_fresh_span(primitives, entry.span) {
                    return Err(NativePaintSegmentAssemblyVetoReason::UnsupportedFreshPrimitive);
                }
            }
        }
        previous_end = entry.span.end;
        prior_identities[index] = Some(entry.span.identity);
    }

    Ok((count, execution_plan))
}

/// Assemble one exact retained-only plan into a scratch scene.
///
/// This compatibility helper remains useful for focused retained-artifact
/// tests. Production mixed assembly uses the preflight above and the typed
/// payload stream below.
#[cfg(test)]
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
    if count == 0
        || count > MAX_PAINT_SEGMENTS
        || !target_generation.is_known()
        || feasibility.conservative
        || usize::from(feasibility.segment_count) != count
        || usize::from(feasibility.checkpoint_count) != count
        || plan.entries[count..].iter().any(Option::is_some)
        || feasibility.segments[count..].iter().any(Option::is_some)
        || feasibility.checkpoints[count..].iter().any(Option::is_some)
        || artifacts.has_invalid_bounded_artifact_state(count)
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
        if !valid_payload_metadata(
            payload,
            entry,
            match entry.disposition {
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) => {
                    fingerprint.revision
                }
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_) => 0,
            },
            scene_validity,
            target_generation,
        ) || !valid_no_resource_scene(&payload.scene)
        {
            return NativePaintSegmentAssemblyResult::Veto(
                NativePaintSegmentAssemblyVetoReason::InvalidPayload,
            );
        }

        append_payload_in_current_context(&mut scratch, &payload.scene);
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

fn assemble_payload_stream(
    input: NativePaintSegmentAssemblyInput<'_>,
    payloads: Vec<NativePaintSegmentPayload>,
    fresh_count: usize,
    reused_count: usize,
) -> Result<NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyVetoReason> {
    let NativePaintSegmentAssemblyInput {
        primitives,
        paint,
        plan,
        scene_validity,
        target_generation,
        ..
    } = input;
    let count = usize::from(plan.entry_count);
    if payloads.len() != count {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidFreshPayload);
    }

    let mut scratch = Scene::new();
    let mut previous_counts = ArtifactFeasibilityCounts::default();
    let mut previous_end = 0;
    let mut prior_identities = [None; MAX_PAINT_SEGMENTS];
    let mut segment_encoding = PaintSegmentEncodingObservation {
        segments: [None; MAX_PAINT_SEGMENTS],
        segment_count: paint.segment_count,
        conservative: false,
    };
    let mut artifact_feasibility = ArtifactFeasibilityObservation {
        segments: [None; MAX_PAINT_SEGMENTS],
        checkpoints: [None; MAX_PAINT_SEGMENTS],
        segment_count: paint.segment_count,
        checkpoint_count: paint.segment_count,
        conservative: false,
    };
    let mut clip_layer_count: usize = 0;

    for index in 0..count {
        let Some(entry) = plan.entries[index] else {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidPlan);
        };
        let Some(current) = paint.segments[index] else {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidCurrentObservation);
        };
        let payload = &payloads[index];
        if !valid_payload_metadata(
            payload,
            entry,
            current.revision,
            scene_validity,
            target_generation,
        ) || !valid_no_resource_scene(&payload.scene)
            || payload.evidence.counts != counts_from_scene(&payload.scene)
            || payload.evidence.encoding.identity != entry.span.identity
            || payload.evidence.encoding.primitive_start != entry.span.start
            || payload.evidence.encoding.primitive_end != entry.span.end
            || payload.evidence.encoding.conservative
            || !matches!(
                payload.evidence.encoding.isolation,
                EncodingIsolation::SelfContained
            )
            || matches!(
                payload.evidence.encoding.safe_enclosure,
                SafeEnclosure::ViewportFallback
            )
            || !matches!(
                payload.evidence.encoding.reason,
                EncodingConservativeReason::None
            )
            || !counts_monotonic_from(
                payload.evidence.counts,
                ArtifactFeasibilityCounts::default(),
            )
            || payload.evidence.counts.n_open_clips != 0
        {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidFreshPayload);
        }
        if entry.span.start < previous_end
            || prior_identities[..index].contains(&Some(entry.span.identity))
        {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
        }

        append_payload_in_current_context(&mut scratch, &payload.scene);
        let assembled_counts = counts_from_scene(&scratch);
        if !counts_monotonic_from(assembled_counts, previous_counts)
            || assembled_counts.n_open_clips != 0
        {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
        }
        let disposition = reconstructed_disposition(assembled_counts, previous_counts);
        let checkpoint = ArtifactFeasibilityCheckpoint {
            primitive_end: entry.span.end,
            counts: assembled_counts,
        };
        segment_encoding.segments[index] = Some(payload.evidence.encoding);
        artifact_feasibility.segments[index] = Some(ArtifactFeasibilitySegment {
            identity: entry.span.identity,
            primitive_start: entry.span.start,
            primitive_end: entry.span.end,
            disposition,
        });
        artifact_feasibility.checkpoints[index] = Some(checkpoint);
        clip_layer_count = clip_layer_count.saturating_add(payload.evidence.clip_layer_count);
        previous_counts = assembled_counts;
        previous_end = entry.span.end;
        prior_identities[index] = Some(entry.span.identity);
    }

    if !valid_no_resource_scene(&scratch)
        || fresh_count.saturating_add(reused_count) != count
        || segment_encoding.segments[count..]
            .iter()
            .any(Option::is_some)
        || artifact_feasibility.segments[count..]
            .iter()
            .any(Option::is_some)
        || artifact_feasibility.checkpoints[count..]
            .iter()
            .any(Option::is_some)
    {
        return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
    }

    let stats = RetainedSurfaceEncodeStats {
        paint_plan_primitives: primitives.len(),
        clip_layer_count,
        gpu_surface_count: primitives
            .iter()
            .filter(|primitive| primitive.gpu_surface().is_some())
            .count(),
        segment_encoding,
        artifact_feasibility,
        ..RetainedSurfaceEncodeStats::default()
    };
    let materialization = NativePaintSegmentArtifactMaterialization {
        plan_entry_count: count as u8,
        scene_validity: Some(scene_validity),
        artifacts: payloads
            .into_iter()
            .enumerate()
            .map(|(index, payload)| NativePaintSegmentArtifact {
                plan_index: index as u8,
                payload,
            })
            .collect(),
    };
    Ok(NativePaintSegmentAssemblyBundle {
        scene: scratch,
        stats,
        materialization,
        plan,
        paint,
        target_generation,
        fresh_count,
        reused_count,
        append_count: count,
    })
}

fn reconstructed_disposition(
    local: ArtifactFeasibilityCounts,
    previous: ArtifactFeasibilityCounts,
) -> ArtifactFeasibilityDisposition {
    if !counts_grew_stream_from(local, previous) {
        ArtifactFeasibilityDisposition::NoArtifact
    } else if local.transforms == previous.transforms || local.styles == previous.styles {
        ArtifactFeasibilityDisposition::RequiresFreshEncoding(
            super::artifact_feasibility::ArtifactFeasibilityReason::CrossSegmentTransformOrStyle,
        )
    } else {
        ArtifactFeasibilityDisposition::ContiguousCandidate
    }
}

fn valid_payload_metadata(
    payload: &NativePaintSegmentPayload,
    entry: NativePaintSegmentEligibilityEntry,
    revision: u64,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
) -> bool {
    let evidence = payload.evidence;
    let fingerprint_revision = match entry.disposition {
        NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) => {
            fingerprint.revision
        }
        NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_) => revision,
    };
    evidence.identity == entry.span.identity
        && evidence.span == entry.span
        && revision != 0
        && evidence.revision == fingerprint_revision
        && evidence.target_generation == target_generation
        && evidence.scene_validity == scene_validity
}

fn supported_fresh_span(primitives: &[PaintPrimitive], span: PaintSegmentSpan) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(span.start), usize::try_from(span.end)) else {
        return false;
    };
    let Some(primitives) = primitives.get(start..end) else {
        return false;
    };
    let mut clip_state = SceneClipState::default();
    for primitive in primitives {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                if !clip.rect.has_finite_positive_area() {
                    return false;
                }
                clip_state.begin(clip.rect);
            }
            PaintPrimitive::ClipEnd(_) => {
                if matches!(clip_state.end(), SceneClipEnd::Unmatched) {
                    return false;
                }
            }
            PaintPrimitive::FillRect(fill) => {
                if !fill.rect.has_finite_positive_area() {
                    return false;
                }
            }
            PaintPrimitive::FillRectBatch(fill) => {
                if fill
                    .rects
                    .iter()
                    .any(|rect| !rect.has_finite_positive_area())
                {
                    return false;
                }
            }
            PaintPrimitive::OverlayPanel(panel) => {
                if !panel.rect.has_finite_positive_area() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    clip_state.depth() == 0
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
        ) || !valid_payload_metadata(
            payload,
            entry,
            match entry.disposition {
                NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) => {
                    fingerprint.revision
                }
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_) => {
                    payload.evidence.revision
                }
            },
            scene_validity,
            target_generation,
        ) || !valid_no_resource_scene(&payload.scene)
            || payload.evidence.counts != counts_from_scene(&payload.scene)
        {
            return NativePaintSegmentArtifactMaterialization::default();
        }

        append_payload_in_current_context(&mut assembled, &payload.scene);
        let assembled_counts = counts_from_scene(&assembled);
        if assembled_counts != checkpoint.counts {
            return NativePaintSegmentArtifactMaterialization::default();
        }

        if !counts_grew_stream_from(assembled_counts, previous_counts)
            && !matches!(
                entry.disposition,
                NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_)
            )
        {
            return NativePaintSegmentArtifactMaterialization::default();
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
        .map(|(index, payload)| NativePaintSegmentArtifact {
            plan_index: index as u8,
            payload,
        })
        .collect();
    NativePaintSegmentArtifactMaterialization {
        plan_entry_count: count as u8,
        scene_validity: Some(scene_validity),
        artifacts,
    }
}

/// Append one standalone payload while preserving the stream context that a
/// single authoritative scene would have carried across the segment boundary.
/// Vello child scenes always begin with their local transform/style markers;
/// for the resource-free segment contract those markers can be redundant with
/// the already assembled context and must not be duplicated in the result.
fn append_payload_in_current_context(destination: &mut Scene, payload: &Scene) {
    let mut normalized = payload.clone();
    let destination_encoding = destination.encoding();
    let encoding = normalized.encoding_mut();
    let destination_transform = destination_encoding.transforms.last();
    let destination_style = destination_encoding.styles.last();
    let mut tag_index = 0;
    let mut transform_index = 0;
    let mut style_index = 0;

    while let Some(tag) = encoding.path_tags.get(tag_index).copied() {
        match tag.0 {
            0x20 => {
                let Some(transform) = encoding.transforms.get(transform_index) else {
                    break;
                };
                if destination_transform == Some(transform) {
                    encoding.path_tags.remove(tag_index);
                    encoding.transforms.remove(transform_index);
                } else {
                    transform_index += 1;
                    tag_index += 1;
                }
            }
            0x40 => {
                let Some(style) = encoding.styles.get(style_index) else {
                    break;
                };
                if destination_style == Some(style) {
                    encoding.path_tags.remove(tag_index);
                    encoding.styles.remove(style_index);
                } else {
                    style_index += 1;
                    tag_index += 1;
                }
            }
            _ => break,
        }
    }

    destination.append(&normalized, None);
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
