//! Admission-aware native Vello render-boundary selection.
//!
//! This is a CPU-side policy boundary only. It selects exact, already resident
//! payload boundaries for the existing transactional mixed assembler; it does
//! not allocate GPU resources, form paint segments, or change admission.

use super::super::frame_state::NativeSceneValidityFingerprint;
use super::super::runner_state::NativeTargetGeneration;
use super::super::scene::{NativePaintSegmentArtifactResidency, NativePaintSegmentArtifactStore};
use super::{
    NativePaintSegmentCacheAdmission, NativePaintSegmentEligibilityDisposition,
    NativePaintSegmentEligibilityOutcome, NativePaintSegmentEligibilityPlan,
    NativePaintSegmentFallbackReason, NativePaintSegmentFreshEncodingReason,
    NativePaintSegmentRenderAdmission, NativePaintSegmentRenderAdmissionQuery,
};
use crate::runtime::{MAX_PAINT_SEGMENTS, PaintSegmentSpan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativePaintSegmentRenderSelectionFallbackReason
{
    Eligibility(NativePaintSegmentFallbackReason),
    SceneFenceMismatch,
    UnknownOrExhaustedTargetGeneration,
    InvalidAdmissionEvidence,
    InvalidResidentEvidence,
    NoExactResident,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativePaintSegmentRenderSelectionOutcome
{
    FullSceneFallback(NativePaintSegmentRenderSelectionFallbackReason),
    Mixed {
        warming_probe_count: u8,
        admitted_retained_count: u8,
    },
}

/// One immutable, bounded decision consumed by the production scene rebuild.
///
/// The plan is fresh-only for a full-scene fallback. For `Mixed`, retained
/// dispositions are present only at exact current admission and residency
/// matches; every other valid entry remains ordered fresh work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativePaintSegmentRenderSelection {
    outcome: NativePaintSegmentRenderSelectionOutcome,
    plan: NativePaintSegmentEligibilityPlan,
    admissions: [Option<NativePaintSegmentRenderAdmission>; MAX_PAINT_SEGMENTS],
}

impl Default for NativePaintSegmentRenderSelection {
    fn default() -> Self {
        Self {
            outcome: NativePaintSegmentRenderSelectionOutcome::FullSceneFallback(
                NativePaintSegmentRenderSelectionFallbackReason::UnknownOrExhaustedTargetGeneration,
            ),
            plan: NativePaintSegmentEligibilityPlan::default(),
            admissions: [None; MAX_PAINT_SEGMENTS],
        }
    }
}

impl NativePaintSegmentRenderSelection {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn full_encode_plan(
        self,
    ) -> NativePaintSegmentEligibilityPlan {
        self.plan
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn should_attempt_mixed_assembly(
        self,
    ) -> bool {
        matches!(
            self.outcome,
            NativePaintSegmentRenderSelectionOutcome::Mixed {
                warming_probe_count,
                admitted_retained_count,
            } if warming_probe_count.saturating_add(admitted_retained_count) > 0
        )
    }

    /// Carry the residence selected by this immutable boundary into the
    /// detached scene publication witness. A slot is exact only when the
    /// selection retained its admission; all fresh slots are intentionally
    /// represented as absent from the selected resident set.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn selected_artifact_residency(
        self,
    ) -> [NativePaintSegmentArtifactResidency; MAX_PAINT_SEGMENTS] {
        let mut residency = [NativePaintSegmentArtifactResidency::Absent; MAX_PAINT_SEGMENTS];
        for (slot, admission) in residency.iter_mut().zip(self.admissions) {
            if admission.is_some() {
                *slot = NativePaintSegmentArtifactResidency::Exact;
            }
        }
        residency
    }

    #[cfg(test)]
    fn outcome(self) -> NativePaintSegmentRenderSelectionOutcome {
        self.outcome
    }

    #[cfg(test)]
    fn plan_for_test(self) -> NativePaintSegmentEligibilityPlan {
        self.plan
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn select_native_paint_segment_render_boundary(
    eligibility: NativePaintSegmentEligibilityPlan,
    admission: &NativePaintSegmentCacheAdmission,
    artifacts: &NativePaintSegmentArtifactStore,
    scene_validity: NativeSceneValidityFingerprint,
    previous_scene_validity: Option<NativeSceneValidityFingerprint>,
    target_generation: NativeTargetGeneration,
) -> NativePaintSegmentRenderSelection {
    let NativePaintSegmentEligibilityOutcome::Plan = eligibility.outcome else {
        let reason = match eligibility.outcome {
            NativePaintSegmentEligibilityOutcome::FullSceneFallback(reason) => {
                NativePaintSegmentRenderSelectionFallbackReason::Eligibility(reason)
            }
            NativePaintSegmentEligibilityOutcome::Plan => {
                NativePaintSegmentRenderSelectionFallbackReason::Eligibility(
                    NativePaintSegmentFallbackReason::MalformedSpans,
                )
            }
        };
        return full_scene_selection(eligibility, reason);
    };

    if previous_scene_validity != Some(scene_validity) {
        return full_scene_selection(
            eligibility,
            NativePaintSegmentRenderSelectionFallbackReason::SceneFenceMismatch,
        );
    }
    if !target_generation.is_known() {
        return full_scene_selection(
            eligibility,
            NativePaintSegmentRenderSelectionFallbackReason::UnknownOrExhaustedTargetGeneration,
        );
    }
    if !valid_eligibility_plan(eligibility, target_generation) {
        return full_scene_selection(
            eligibility,
            NativePaintSegmentRenderSelectionFallbackReason::Eligibility(
                NativePaintSegmentFallbackReason::MalformedSpans,
            ),
        );
    }

    let count = usize::from(eligibility.entry_count);
    let mut plan = eligibility;
    let mut admissions = [None; MAX_PAINT_SEGMENTS];
    let mut warming_probe_count: u8 = 0;
    let mut admitted_retained_count: u8 = 0;

    for (index, admission_slot) in admissions.iter_mut().enumerate().take(count) {
        let Some(entry) = eligibility.entries[index] else {
            return full_scene_selection(
                eligibility,
                NativePaintSegmentRenderSelectionFallbackReason::Eligibility(
                    NativePaintSegmentFallbackReason::MissingSegment,
                ),
            );
        };
        let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
        else {
            continue;
        };

        let admission_match = admission.render_admission(
            fingerprint.identity,
            entry.span,
            fingerprint.revision,
            target_generation,
        );
        let render_admission = match admission_match {
            NativePaintSegmentRenderAdmissionQuery::NoMatch => {
                replace_with_fresh(
                    &mut plan,
                    index,
                    NativePaintSegmentFreshEncodingReason::NotAdmitted,
                );
                continue;
            }
            NativePaintSegmentRenderAdmissionQuery::InvalidEvidence => {
                return full_scene_selection(
                    eligibility,
                    NativePaintSegmentRenderSelectionFallbackReason::InvalidAdmissionEvidence,
                );
            }
            NativePaintSegmentRenderAdmissionQuery::WarmingProbe => {
                NativePaintSegmentRenderAdmission::WarmingProbe
            }
            NativePaintSegmentRenderAdmissionQuery::AdmittedRetained => {
                NativePaintSegmentRenderAdmission::AdmittedRetained
            }
        };

        match artifacts.residency_for_selection(
            index,
            count,
            entry,
            scene_validity,
            target_generation,
        ) {
            NativePaintSegmentArtifactResidency::Exact => {
                *admission_slot = Some(render_admission);
                match render_admission {
                    NativePaintSegmentRenderAdmission::WarmingProbe => {
                        warming_probe_count = warming_probe_count.saturating_add(1);
                    }
                    NativePaintSegmentRenderAdmission::AdmittedRetained => {
                        admitted_retained_count = admitted_retained_count.saturating_add(1);
                    }
                }
            }
            NativePaintSegmentArtifactResidency::Absent => {
                replace_with_fresh(
                    &mut plan,
                    index,
                    NativePaintSegmentFreshEncodingReason::NoResident,
                );
            }
            NativePaintSegmentArtifactResidency::Invalid => {
                return full_scene_selection(
                    eligibility,
                    NativePaintSegmentRenderSelectionFallbackReason::InvalidResidentEvidence,
                );
            }
        }
    }

    if warming_probe_count == 0 && admitted_retained_count == 0 {
        return NativePaintSegmentRenderSelection {
            outcome: NativePaintSegmentRenderSelectionOutcome::FullSceneFallback(
                NativePaintSegmentRenderSelectionFallbackReason::NoExactResident,
            ),
            plan,
            admissions,
        };
    }

    NativePaintSegmentRenderSelection {
        outcome: NativePaintSegmentRenderSelectionOutcome::Mixed {
            warming_probe_count,
            admitted_retained_count,
        },
        plan,
        admissions,
    }
}

fn full_scene_selection(
    eligibility: NativePaintSegmentEligibilityPlan,
    reason: NativePaintSegmentRenderSelectionFallbackReason,
) -> NativePaintSegmentRenderSelection {
    NativePaintSegmentRenderSelection {
        outcome: NativePaintSegmentRenderSelectionOutcome::FullSceneFallback(reason),
        plan: eligibility
            .force_fresh_candidates(NativePaintSegmentFreshEncodingReason::RenderSelectionFallback),
        admissions: [None; MAX_PAINT_SEGMENTS],
    }
}

fn replace_with_fresh(
    plan: &mut NativePaintSegmentEligibilityPlan,
    index: usize,
    reason: NativePaintSegmentFreshEncodingReason,
) {
    if let Some(entry) = plan.entries.get_mut(index).and_then(Option::as_mut) {
        entry.disposition = NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(reason);
    }
}

fn valid_eligibility_plan(
    plan: NativePaintSegmentEligibilityPlan,
    target_generation: NativeTargetGeneration,
) -> bool {
    let count = usize::from(plan.entry_count);
    if count == 0 || count > MAX_PAINT_SEGMENTS || plan.entries[count..].iter().any(Option::is_some)
    {
        return false;
    }

    let mut previous_end = 0;
    let mut identities = [None; MAX_PAINT_SEGMENTS];
    for index in 0..count {
        let Some(entry) = plan.entries[index] else {
            return false;
        };
        if !valid_span(entry.span, previous_end)
            || identities[..index].contains(&Some(entry.span.identity))
        {
            return false;
        }
        if let NativePaintSegmentEligibilityDisposition::RetainedCandidate(fingerprint) =
            entry.disposition
            && !valid_fingerprint(fingerprint, entry.span, target_generation)
        {
            return false;
        }
        identities[index] = Some(entry.span.identity);
        previous_end = entry.span.end;
    }
    true
}

fn valid_span(span: PaintSegmentSpan, previous_end: u32) -> bool {
    span.start < span.end && span.start >= previous_end
}

fn valid_fingerprint(
    fingerprint: super::NativePaintSegmentFingerprint,
    span: PaintSegmentSpan,
    target_generation: NativeTargetGeneration,
) -> bool {
    fingerprint.identity == span.identity
        && fingerprint.revision != 0
        && fingerprint.target_generation == target_generation
        && fingerprint.primitive_start == span.start
        && fingerprint.primitive_end == span.end
        && !matches!(
            fingerprint.safe_enclosure,
            super::super::scene::SafeEnclosure::ViewportFallback
        )
        && matches!(
            fingerprint.isolation,
            super::super::scene::EncodingIsolation::SelfContained
        )
        && matches!(
            fingerprint.conservative_reason,
            super::super::scene::EncodingConservativeReason::None
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rect, Vector2};
    use crate::gui_runtime::native_vello::NativeTextRenderer;
    use crate::runtime::{
        BasePaintPlanContext, PaintSegmentAnchor, PaintSegmentIdentity, RetainedSurfaceCachePolicy,
    };
    use crate::theme::{DpiScale, ResolvedAppearance, ThemeTokens};

    fn identity(key: u64) -> PaintSegmentIdentity {
        PaintSegmentIdentity {
            preceding: None,
            following: Some(PaintSegmentAnchor {
                widget_id: key,
                key,
            }),
        }
    }

    fn scene_validity() -> NativeSceneValidityFingerprint {
        let frame = super::super::super::frame_state::NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        frame.native_scene_validity_fingerprint(
            BasePaintPlanContext {
                viewport: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                window_environment: Default::default(),
                layout_state_generation: 0,
                layout_debug_options: Default::default(),
                hovered_container: None,
                hovered_widget: None,
                hovered_scroll_affordance: None,
                focused_widget: None,
                pointer_capture: None,
                pointer_capture_state: None,
                scrollbar_drag: None,
            },
            ResolvedAppearance::fixed(ThemeTokens::default()),
            DpiScale::ONE,
        )
    }

    fn plan() -> NativePaintSegmentEligibilityPlan {
        let id = identity(1);
        let span = PaintSegmentSpan {
            identity: id,
            start: 0,
            end: 1,
        };
        let mut entries = [None; MAX_PAINT_SEGMENTS];
        entries[0] = Some(super::super::NativePaintSegmentEligibilityEntry {
            span,
            disposition:
                super::super::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    NativePaintSegmentFreshEncodingReason::NoArtifact,
                ),
        });
        NativePaintSegmentEligibilityPlan::plan(entries, 1)
    }

    #[test]
    fn cold_selection_is_full_scene_without_assembly() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let validity = scene_validity();
        let selection = select_native_paint_segment_render_boundary(
            plan(),
            &NativePaintSegmentCacheAdmission::default(),
            &NativePaintSegmentArtifactStore::default(),
            validity,
            Some(validity),
            generation,
        );
        assert!(!selection.should_attempt_mixed_assembly());
        assert!(matches!(
            selection.outcome(),
            NativePaintSegmentRenderSelectionOutcome::FullSceneFallback(
                NativePaintSegmentRenderSelectionFallbackReason::NoExactResident
            )
        ));
    }

    #[test]
    fn scene_fence_mismatch_forces_fresh_fallback() {
        let generation = NativeTargetGeneration::from_test_serial(1);
        let validity = scene_validity();
        let selection = select_native_paint_segment_render_boundary(
            plan(),
            &NativePaintSegmentCacheAdmission::default(),
            &NativePaintSegmentArtifactStore::default(),
            validity,
            None,
            generation,
        );
        assert!(matches!(
            selection.outcome(),
            NativePaintSegmentRenderSelectionOutcome::FullSceneFallback(
                NativePaintSegmentRenderSelectionFallbackReason::SceneFenceMismatch
            )
        ));
        assert!(selection.plan_for_test().entries[0].is_some());
    }
}
