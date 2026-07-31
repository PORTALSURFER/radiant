//! Ephemeral Vello scene artifacts for an exact, validated eligibility plan.

use super::super::retained_paint_segments::{
    NativePaintSegmentEligibilityDisposition, NativePaintSegmentEligibilityOutcome,
    NativePaintSegmentEligibilityPlan, NativePaintSegmentFreshEncodingReason,
};
use super::super::runner_state::NativeTargetGeneration;
use super::artifact_feasibility::{
    ArtifactFeasibilityCounts, ArtifactFeasibilityDisposition, ArtifactFeasibilityObservation,
};
use super::{EncodingConservativeReason, EncodingIsolation, SafeEnclosure};
use crate::runtime::{MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentSpan};
use vello::Scene;

/// One ephemeral Vello artifact materialized from a completed authoritative
/// scene encoding. The evidence is retained beside the payload so a later
/// consumer cannot lose the identity or generation fence that authorized it.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifact {
    pub(in crate::gui_runtime::native_vello) scene: Scene,
    pub(in crate::gui_runtime::native_vello) identity: PaintSegmentIdentity,
    pub(in crate::gui_runtime::native_vello) span: PaintSegmentSpan,
    pub(in crate::gui_runtime::native_vello) revision: u64,
    pub(in crate::gui_runtime::native_vello) target_generation: NativeTargetGeneration,
}

/// Transactional, bounded materialization result for one encoded frame.
///
/// This is intentionally ephemeral. It owns no cache admission, retention,
/// replay, or rendering policy.
#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentArtifactMaterialization {
    pub(in crate::gui_runtime::native_vello) artifacts: Vec<NativePaintSegmentArtifact>,
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
}

/// Materialize artifacts from typed, independently valid scene payloads.
///
/// The payload vector is plan-ordered and contains exactly one scene per plan
/// entry. Every payload is validated before it participates in a fresh
/// assembled scene. The assembled scene must be encoding-equivalent to the
/// authoritative scene, so no artifact can escape from a partially validated
/// or merely count-compatible stream.
pub(in crate::gui_runtime::native_vello) fn materialize_native_paint_segment_artifacts(
    scene: &Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: NativePaintSegmentEligibilityPlan,
    payloads: Vec<Scene>,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentArtifactMaterialization {
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
