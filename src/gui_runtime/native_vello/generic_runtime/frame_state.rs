//! Renderer-frame state owned by the generic native Vello runner.

use super::runner_state::NativeTargetGeneration;
use super::{
    CompositedBaseFrame, GpuSurfaceInteractionRegion, GpuSurfaceRenderer,
    PaintSegmentEncodingObservation, PostGpuOverlayRenderer, RetainedSurfaceEncodeStats,
    RetainedSurfaceFrameCache, SceneTextRunBuffer,
    gpu_surface::{
        SurfaceVisibleSuffixScratch, gpu_surface_visible_suffix_regions_into_with_scratch,
    },
    post_gpu_overlay,
    retained_paint_segments::{
        NativePaintSegmentEligibilityPlan, NativeRetainedPaintSegmentStore,
        assemble_native_paint_segment_fingerprints,
        classify_native_paint_segment_eligibility_with_spans,
    },
    runtime_helpers::{
        GpuSurfaceInteractionScratch, SurfaceOcclusionPlan,
        collect_gpu_surface_interaction_regions_with_scratch,
    },
    scene::{
        NativePaintSegmentArtifactMaterialization, NativePaintSegmentArtifactStore,
        NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyInput,
        NativePaintSegmentAssemblyVetoReason,
    },
};
use crate::runtime::BasePaintPlanContext;
use crate::runtime::{PaintSegmentObservation, collect_segment_spans};
use crate::theme::DpiScale;
use crate::theme::ResolvedAppearance;
use crate::theme::ThemeTokens;
use crate::{
    gui::types::Rect as UiRect,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{PaintPrimitive, RetainedSurfaceCachePolicy, SurfacePaintPlan},
};
use vello::Scene;
use vello::kurbo::Affine;

#[cfg(test)]
use super::scene::NativePaintSegmentAssemblyResult;
#[cfg(test)]
use super::scene::PaintSegmentEncoding;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeVelloTestPhase {
    EligibilityObserved,
    SceneEncode,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct NativeVelloTestPhaseTrace {
    phases: [Option<NativeVelloTestPhase>; 2],
    len: u8,
}

#[cfg(test)]
impl NativeVelloTestPhaseTrace {
    fn reset(&mut self) {
        self.phases = [None; 2];
        self.len = 0;
    }

    fn record(&mut self, phase: NativeVelloTestPhase) {
        if let Some(slot) = self.phases.get_mut(usize::from(self.len)) {
            *slot = Some(phase);
            self.len = self.len.saturating_add(1);
        }
    }
}

pub(super) struct NativeVelloFrameState {
    pub(super) text_renderer: NativeTextRenderer,
    pub(super) scene: Scene,
    scaled_scene: Scene,
    scaled_scene_dpi_scale: DpiScale,
    scaled_scene_dirty: bool,
    pub(super) gpu_surface_renderer: GpuSurfaceRenderer,
    pub(super) post_gpu_overlay_renderer: PostGpuOverlayRenderer,
    pub(super) last_paint_plan: SurfacePaintPlan,
    pub(super) transient_overlay_primitives: Vec<PaintPrimitive>,
    pub(super) composited_base_frame: Option<CompositedBaseFrame>,
    pub(super) composited_base_dirty: bool,
    pub(super) retained_surface_cache: RetainedSurfaceFrameCache,
    pub(super) last_scene_stats: RetainedSurfaceEncodeStats,
    pub(super) native_retained_paint_segment_store: NativeRetainedPaintSegmentStore,
    pub(super) native_paint_segment_artifact_store: NativePaintSegmentArtifactStore,
    pub(super) last_native_paint_segment_eligibility: NativePaintSegmentEligibilityPlan,
    #[cfg(test)]
    test_phase_trace: NativeVelloTestPhaseTrace,
    pub(super) scene_text_runs: SceneTextRunBuffer,
    pub(super) gpu_surface_interaction_regions: Vec<GpuSurfaceInteractionRegion>,
    pub(super) surface_occlusion_plan: SurfaceOcclusionPlan,
    gpu_surface_interaction_scratch: GpuSurfaceInteractionScratch,
    pub(super) post_gpu_overlay_gpu_regions: Vec<UiRect>,
    post_gpu_overlay_gpu_regions_scratch: SurfaceVisibleSuffixScratch,
    pub(super) post_gpu_overlay_suffix_start: Option<usize>,
    pub(super) post_gpu_overlay_has_replayable_suffix: bool,
    pub(super) scene_texture_dirty: bool,
    pub(super) scene_encode_count: u64,
    pub(super) scene_reuse_count: u64,
    pub(super) scene_assembly_count: u64,
    pub(super) scene_assembly_veto_count: u64,
    pub(super) scene_mixed_assembly_count: u64,
    pub(super) scene_assembly_fresh_count: u64,
    pub(super) scene_assembly_reused_count: u64,
    pub(super) scene_assembly_append_count: u64,
    pub(super) scene_build_outcome: NativeSceneBuildOutcome,
    native_scene_context_generation: u64,
    native_scene_invalidated: bool,
    last_scene_validity: Option<NativeSceneValidityFingerprint>,
}

/// Native-only context required for reusing an already encoded Vello scene.
///
/// This intentionally includes the backend-neutral paint context as well as
/// native target/cache generations. A plan cache hit alone is insufficient:
/// device loss, resize, DPI, appearance, layout-debug, text, retained-surface,
/// or GPU target changes must all take the conservative encode path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct NativeSceneValidityFingerprint {
    pub(super) base_paint_plan_context: BasePaintPlanContext,
    pub(super) appearance: ResolvedAppearance,
    pub(super) dpi_scale: DpiScale,
    pub(super) native_scene_context_generation: u64,
    pub(super) retained_cache_policy: RetainedSurfaceCachePolicy,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum NativeSceneBuildOutcome {
    #[default]
    NotRebuilt,
    WholeSceneReuse,
    RetainedAssembly,
    MixedRetainedAssembly,
    FullEncode,
    RetainedAssemblyVetoFallback,
}

impl NativeSceneBuildOutcome {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::NotRebuilt => "not_rebuilt",
            Self::WholeSceneReuse => "whole_scene_reuse",
            Self::RetainedAssembly => "retained_assembly",
            Self::MixedRetainedAssembly => "mixed_retained_assembly",
            Self::FullEncode => "full_encode",
            Self::RetainedAssemblyVetoFallback => "retained_assembly_veto_fallback",
        }
    }
}

impl NativeVelloFrameState {
    pub(super) fn new(
        text_renderer: NativeTextRenderer,
        retained_surface_cache: RetainedSurfaceCachePolicy,
    ) -> Self {
        Self {
            text_renderer,
            scene: Scene::new(),
            scaled_scene: Scene::new(),
            scaled_scene_dpi_scale: DpiScale::ONE,
            scaled_scene_dirty: true,
            gpu_surface_renderer: GpuSurfaceRenderer::default(),
            post_gpu_overlay_renderer: PostGpuOverlayRenderer::default(),
            last_paint_plan: SurfacePaintPlan::empty(&ThemeTokens::default()),
            transient_overlay_primitives: Vec::new(),
            composited_base_frame: None,
            composited_base_dirty: true,
            retained_surface_cache: RetainedSurfaceFrameCache::with_policy(retained_surface_cache),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
            native_retained_paint_segment_store: NativeRetainedPaintSegmentStore::default(),
            native_paint_segment_artifact_store: NativePaintSegmentArtifactStore::default(),
            last_native_paint_segment_eligibility: NativePaintSegmentEligibilityPlan::default(),
            #[cfg(test)]
            test_phase_trace: NativeVelloTestPhaseTrace::default(),
            scene_text_runs: SceneTextRunBuffer::new(),
            gpu_surface_interaction_regions: Vec::new(),
            surface_occlusion_plan: SurfaceOcclusionPlan::default(),
            gpu_surface_interaction_scratch: GpuSurfaceInteractionScratch::default(),
            post_gpu_overlay_gpu_regions: Vec::new(),
            post_gpu_overlay_gpu_regions_scratch: SurfaceVisibleSuffixScratch::default(),
            post_gpu_overlay_suffix_start: None,
            post_gpu_overlay_has_replayable_suffix: false,
            scene_texture_dirty: true,
            scene_encode_count: 0,
            scene_reuse_count: 0,
            scene_assembly_count: 0,
            scene_assembly_veto_count: 0,
            scene_mixed_assembly_count: 0,
            scene_assembly_fresh_count: 0,
            scene_assembly_reused_count: 0,
            scene_assembly_append_count: 0,
            scene_build_outcome: NativeSceneBuildOutcome::NotRebuilt,
            native_scene_context_generation: 0,
            native_scene_invalidated: true,
            last_scene_validity: None,
        }
    }

    pub(super) fn native_scene_validity_fingerprint(
        &self,
        base_paint_plan_context: BasePaintPlanContext,
        appearance: ResolvedAppearance,
        dpi_scale: DpiScale,
    ) -> NativeSceneValidityFingerprint {
        NativeSceneValidityFingerprint {
            base_paint_plan_context,
            appearance,
            dpi_scale,
            native_scene_context_generation: self.native_scene_context_generation,
            retained_cache_policy: self.retained_surface_cache.policy(),
        }
    }

    pub(super) fn can_reuse_native_scene(
        &self,
        fingerprint: NativeSceneValidityFingerprint,
    ) -> bool {
        !self.native_scene_invalidated && self.last_scene_validity == Some(fingerprint)
    }

    pub(super) fn record_scene_encode(&mut self, fingerprint: NativeSceneValidityFingerprint) {
        self.scene_encode_count = self.scene_encode_count.saturating_add(1);
        self.native_scene_invalidated = false;
        self.last_scene_validity = Some(fingerprint);
        self.scene_build_outcome = NativeSceneBuildOutcome::FullEncode;
    }

    pub(super) fn record_scene_encode_after_assembly_veto(
        &mut self,
        fingerprint: NativeSceneValidityFingerprint,
    ) {
        self.record_scene_encode(fingerprint);
        self.scene_assembly_veto_count = self.scene_assembly_veto_count.saturating_add(1);
        self.scene_build_outcome = NativeSceneBuildOutcome::RetainedAssemblyVetoFallback;
    }

    pub(super) fn record_scene_assembly(
        &mut self,
        fingerprint: NativeSceneValidityFingerprint,
        fresh_count: usize,
        reused_count: usize,
        append_count: usize,
    ) {
        self.scene_assembly_count = self.scene_assembly_count.saturating_add(1);
        if fresh_count > 0 {
            self.scene_mixed_assembly_count = self.scene_mixed_assembly_count.saturating_add(1);
        }
        self.scene_assembly_fresh_count = self
            .scene_assembly_fresh_count
            .saturating_add(fresh_count as u64);
        self.scene_assembly_reused_count = self
            .scene_assembly_reused_count
            .saturating_add(reused_count as u64);
        self.scene_assembly_append_count = self
            .scene_assembly_append_count
            .saturating_add(append_count as u64);
        self.native_scene_invalidated = false;
        self.last_scene_validity = Some(fingerprint);
        self.scene_build_outcome = if fresh_count == 0 {
            NativeSceneBuildOutcome::RetainedAssembly
        } else {
            NativeSceneBuildOutcome::MixedRetainedAssembly
        };
    }

    pub(super) fn record_scene_reuse(&mut self) {
        self.scene_reuse_count = self.scene_reuse_count.saturating_add(1);
        self.scene_build_outcome = NativeSceneBuildOutcome::WholeSceneReuse;
    }

    pub(super) fn reset_scene_build_outcome(&mut self) {
        self.scene_build_outcome = NativeSceneBuildOutcome::NotRebuilt;
    }

    pub(super) fn reconcile_native_paint_segments(
        &mut self,
        paint: PaintSegmentObservation,
        encoding: PaintSegmentEncodingObservation,
        target_generation: NativeTargetGeneration,
    ) {
        let observation =
            assemble_native_paint_segment_fingerprints(paint, encoding, target_generation);
        self.native_retained_paint_segment_store
            .reconcile(observation);
    }

    pub(super) fn reconcile_native_paint_segment_artifacts(
        &mut self,
        materialization: NativePaintSegmentArtifactMaterialization,
    ) {
        self.native_paint_segment_artifact_store
            .reconcile(materialization);
    }

    pub(super) fn clear_native_paint_segment_artifacts(&mut self) {
        self.native_paint_segment_artifact_store.clear();
    }

    #[cfg(test)]
    pub(super) fn assemble_retained_native_scene(
        &self,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> Result<Box<Scene>, NativePaintSegmentAssemblyVetoReason> {
        match super::scene::assemble_retained_native_paint_segment_scene(
            &self.scene,
            self.last_scene_stats.artifact_feasibility,
            self.last_native_paint_segment_eligibility,
            &self.native_paint_segment_artifact_store,
            scene_validity,
            target_generation,
        ) {
            NativePaintSegmentAssemblyResult::Assembled(scene) => Ok(scene),
            NativePaintSegmentAssemblyResult::Veto(reason) => Err(reason),
        }
    }

    pub(super) fn assemble_mixed_native_scene(
        &self,
        viewport: crate::gui::types::Vector2,
        paint: PaintSegmentObservation,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) -> Result<NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyVetoReason> {
        super::scene::assemble_mixed_native_paint_segment_scene(NativePaintSegmentAssemblyInput {
            previous_scene: &self.scene,
            primitives: &self.last_paint_plan.primitives,
            viewport,
            paint,
            previous_stats: self.last_scene_stats,
            plan: self.last_native_paint_segment_eligibility,
            artifacts: &self.native_paint_segment_artifact_store,
            scene_validity,
            previous_scene_validity: self.last_scene_validity,
            target_generation,
        })
    }

    pub(super) fn commit_native_scene_assembly(
        &mut self,
        bundle: NativePaintSegmentAssemblyBundle,
        scene_validity: NativeSceneValidityFingerprint,
    ) -> Result<(), NativePaintSegmentAssemblyVetoReason> {
        let fingerprint_observation = assemble_native_paint_segment_fingerprints(
            bundle.paint,
            bundle.stats.segment_encoding,
            bundle.target_generation,
        );
        if fingerprint_observation.conservative {
            return Err(NativePaintSegmentAssemblyVetoReason::InvalidEvidence);
        }

        self.scene = bundle.scene;
        self.last_scene_stats = bundle.stats;
        self.native_paint_segment_artifact_store
            .reconcile(bundle.materialization);
        self.native_retained_paint_segment_store
            .reconcile(fingerprint_observation);
        self.record_scene_assembly(
            scene_validity,
            bundle.fresh_count,
            bundle.reused_count,
            bundle.append_count,
        );
        Ok(())
    }

    pub(super) fn observe_native_paint_segment_eligibility(
        &mut self,
        paint: PaintSegmentObservation,
        feasibility: super::scene::ArtifactFeasibilityObservation,
        target_generation: NativeTargetGeneration,
    ) {
        let mut current_spans = [None; crate::runtime::MAX_PAINT_SEGMENTS];
        let (current_span_count, current_spans_malformed) =
            collect_segment_spans(&self.last_paint_plan.primitives, &mut current_spans);
        self.last_native_paint_segment_eligibility =
            classify_native_paint_segment_eligibility_with_spans(
                paint,
                &self.native_retained_paint_segment_store,
                feasibility,
                current_spans,
                current_span_count,
                current_spans_malformed,
                target_generation,
            );
        #[cfg(test)]
        self.test_phase_trace
            .record(NativeVelloTestPhase::EligibilityObserved);
    }

    #[cfg(test)]
    pub(super) fn record_scene_encode_boundary(&mut self) {
        self.test_phase_trace
            .record(NativeVelloTestPhase::SceneEncode);
    }

    #[cfg(test)]
    pub(super) fn begin_test_phase_trace(&mut self) {
        self.test_phase_trace.reset();
    }

    #[cfg(test)]
    pub(super) fn test_phase_trace(&self) -> [Option<NativeVelloTestPhase>; 2] {
        self.test_phase_trace.phases
    }

    pub(super) fn invalidate_native_scene_context(&mut self) {
        self.native_scene_context_generation =
            self.native_scene_context_generation.saturating_add(1);
        self.native_scene_invalidated = true;
        self.last_scene_validity = None;
    }

    pub(super) fn mark_scene_texture_dirty(&mut self) {
        self.scene_texture_dirty = true;
        self.composited_base_dirty = true;
    }

    pub(super) fn mark_scene_content_dirty(&mut self) {
        self.scaled_scene_dirty = true;
        self.mark_scene_texture_dirty();
    }

    pub(super) fn mark_composited_base_dirty(&mut self) {
        self.composited_base_dirty = true;
    }

    pub(super) fn scene_for_dpi_scale(&mut self, dpi_scale: DpiScale) -> &Scene {
        if dpi_scale == DpiScale::ONE {
            return &self.scene;
        }
        if self.scaled_scene_dirty || self.scaled_scene_dpi_scale != dpi_scale {
            self.scaled_scene.reset();
            self.scaled_scene
                .append(&self.scene, Some(Affine::scale(dpi_scale.factor() as f64)));
            self.scaled_scene_dpi_scale = dpi_scale;
            self.scaled_scene_dirty = false;
        }
        &self.scaled_scene
    }

    pub(super) fn refresh_gpu_surface_interaction_regions(&mut self) {
        self.surface_occlusion_plan
            .preprocess(&self.last_paint_plan.primitives);
        collect_gpu_surface_interaction_regions_with_scratch(
            &self.last_paint_plan.primitives,
            &self.surface_occlusion_plan,
            &mut self.gpu_surface_interaction_regions,
            &mut self.gpu_surface_interaction_scratch,
        );
    }

    pub(super) fn refresh_post_gpu_overlay_cache(&mut self) {
        self.post_gpu_overlay_suffix_start = self
            .last_paint_plan
            .primitives
            .iter()
            .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
            .and_then(|index| index.checked_add(1));
        self.post_gpu_overlay_has_replayable_suffix = self
            .post_gpu_overlay_suffix_start
            .and_then(|start| self.last_paint_plan.primitives.get(start..))
            .is_some_and(|suffix| {
                suffix
                    .iter()
                    .any(post_gpu_overlay::geometry::primitive_is_replayable)
            });
        gpu_surface_visible_suffix_regions_into_with_scratch(
            &self.last_paint_plan.primitives,
            &self.surface_occlusion_plan,
            &mut self.post_gpu_overlay_gpu_regions,
            &mut self.post_gpu_overlay_gpu_regions_scratch,
        );
    }

    pub(super) fn has_post_gpu_overlay_work(&self) -> bool {
        !self.transient_overlay_primitives.is_empty()
            || (self.post_gpu_overlay_has_replayable_suffix
                && !self.post_gpu_overlay_gpu_regions.is_empty())
    }

    pub(super) fn render_post_gpu_overlay(
        &mut self,
        target: &mut post_gpu_overlay::PostGpuOverlayRenderTarget<'_>,
    ) {
        let Self {
            post_gpu_overlay_renderer,
            last_paint_plan,
            transient_overlay_primitives,
            post_gpu_overlay_gpu_regions,
            post_gpu_overlay_suffix_start,
            ..
        } = self;
        let suffix =
            post_gpu_overlay_suffix_start.and_then(|start| last_paint_plan.primitives.get(start..));
        post_gpu_overlay_renderer.render_cached_layers(
            target,
            suffix,
            post_gpu_overlay_gpu_regions,
            transient_overlay_primitives,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{PaintSegment, PaintSegmentAnchor, PaintSegmentIdentity};

    fn identity(key: u64) -> PaintSegmentIdentity {
        PaintSegmentIdentity {
            preceding: None,
            following: Some(PaintSegmentAnchor {
                widget_id: key,
                key,
            }),
        }
    }

    fn observations(
        identity: PaintSegmentIdentity,
        revision: u64,
    ) -> (PaintSegmentObservation, PaintSegmentEncodingObservation) {
        let mut paint = PaintSegmentObservation::empty();
        paint.segment_count = 1;
        paint.segments[0] = Some(PaintSegment {
            identity,
            owner: None,
            revision,
            implicated: false,
        });
        let mut encoding = PaintSegmentEncodingObservation {
            segment_count: 1,
            ..PaintSegmentEncodingObservation::default()
        };
        encoding.segments[0] = Some(PaintSegmentEncoding {
            identity,
            primitive_start: 2,
            primitive_end: 4,
            safe_enclosure: super::super::scene::SafeEnclosure::Empty,
            isolation: super::super::scene::EncodingIsolation::SelfContained,
            conservative: false,
            reason: super::super::scene::EncodingConservativeReason::None,
        });
        (paint, encoding)
    }

    #[test]
    fn scaled_scene_cache_reuses_fractional_dpi_scene_until_content_changes() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        let dpi_scale = DpiScale::new(1.25);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);

        frame.mark_scene_content_dirty();
        assert!(frame.scaled_scene_dirty);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);
    }

    #[test]
    fn native_fingerprint_changes_for_revision_identity_span_and_target() {
        let id = identity(1);
        let (paint, encoding) = observations(id, 1);
        let first = assemble_native_paint_segment_fingerprints(
            paint,
            encoding,
            NativeTargetGeneration::from_test_serial(1),
        );
        assert!(!first.conservative);
        assert_eq!(first.segment_count, 1);

        let (changed_revision, same_encoding) = observations(id, 2);
        assert_ne!(
            first,
            assemble_native_paint_segment_fingerprints(
                changed_revision,
                same_encoding,
                NativeTargetGeneration::from_test_serial(1),
            )
        );
        let (changed_identity, changed_identity_encoding) = observations(identity(2), 1);
        let (_, changed_encoding) = observations(id, 1);
        let mut span_changed = changed_encoding;
        span_changed.segments[0].as_mut().unwrap().primitive_end = 5;
        assert_ne!(
            first,
            assemble_native_paint_segment_fingerprints(
                changed_identity,
                changed_identity_encoding,
                NativeTargetGeneration::from_test_serial(1),
            )
        );
        assert_ne!(
            first,
            assemble_native_paint_segment_fingerprints(
                paint,
                encoding,
                NativeTargetGeneration::from_test_serial(2),
            )
        );
        assert_ne!(
            first,
            assemble_native_paint_segment_fingerprints(
                paint,
                span_changed,
                NativeTargetGeneration::from_test_serial(1),
            )
        );
    }

    #[test]
    fn native_fingerprint_rejects_mismatch_and_conservative_encoding() {
        let (paint, encoding) = observations(identity(1), 1);
        let (_, mismatched_encoding) = observations(identity(2), 1);
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                mismatched_encoding,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        let mut count_mismatch = encoding;
        count_mismatch.segment_count = 0;
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                count_mismatch,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        let mut inherited = encoding;
        inherited.segments[0].as_mut().unwrap().isolation =
            super::super::scene::EncodingIsolation::InheritedClip;
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                inherited,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        let mut fallback = encoding;
        fallback.segments[0].as_mut().unwrap().safe_enclosure =
            super::super::scene::SafeEnclosure::ViewportFallback;
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                fallback,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        let mut malformed = encoding;
        malformed.conservative = true;
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                malformed,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        assert!(
            assemble_native_paint_segment_fingerprints(
                paint,
                encoding,
                NativeTargetGeneration::unknown(),
            )
            .conservative
        );
        let mut saturated = paint;
        saturated.conservative = true;
        assert!(
            assemble_native_paint_segment_fingerprints(
                saturated,
                encoding,
                NativeTargetGeneration::from_test_serial(1),
            )
            .conservative
        );
        let mut exhausted = NativeTargetGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted.advance());
        assert!(
            assemble_native_paint_segment_fingerprints(paint, encoding, exhausted).conservative
        );
    }

    #[test]
    fn full_encode_reconcile_records_store_and_scene_reuse_preserves_it() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        let (paint, encoding) = observations(identity(7), 3);
        frame.reconcile_native_paint_segments(
            paint,
            encoding,
            NativeTargetGeneration::from_test_serial(1),
        );
        let retained = frame.native_retained_paint_segment_store.snapshot();
        assert!(retained[0].is_some());
        assert_eq!(frame.scene_encode_count, 0);

        frame.record_scene_reuse();
        assert_eq!(frame.scene_encode_count, 0);
        assert_eq!(frame.scene_reuse_count, 1);
        assert_eq!(
            frame.native_retained_paint_segment_store.snapshot(),
            retained
        );
    }
}
