//! Renderer-frame state owned by the generic native Vello runner.

use super::runner_state::NativeTargetGeneration;
use super::{
    GpuSurfaceInteractionRegion, PaintSegmentEncodingObservation, PostGpuOverlayRenderer,
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, SceneTextRunBuffer,
    gpu_surface::{
        SurfaceVisibleSuffixScratch, gpu_surface_visible_suffix_regions_into_with_scratch,
    },
    post_gpu_overlay,
    retained_paint_segments::{
        NativePaintSegmentBenefitAssemblyInput, NativePaintSegmentBenefitLedger,
        NativePaintSegmentCacheAdmission, NativePaintSegmentEligibilityPlan,
        NativePaintSegmentRenderSelection, NativeRetainedPaintSegmentStore,
        assemble_native_paint_segment_fingerprints,
        classify_native_paint_segment_eligibility_with_spans,
        select_native_paint_segment_render_boundary,
    },
    runtime_helpers::{
        GpuSurfaceInteractionScratch, SurfaceOcclusionPlan,
        collect_gpu_surface_interaction_regions_with_scratch,
    },
    scene::{
        NativePaintSegmentArtifactMaterialization, NativePaintSegmentArtifactResidency,
        NativePaintSegmentArtifactStore, NativePaintSegmentAssemblyBundle,
        NativePaintSegmentAssemblyInput, NativePaintSegmentAssemblyVetoReason,
        focused_text_input_caret_area_from_snapshot, text_input_pointer_target_from_snapshot,
    },
};
use crate::gui::types::Point;
use crate::runtime::BasePaintPlanContext;
use crate::runtime::MAX_PAINT_SEGMENTS;
use crate::runtime::{PaintSegmentObservation, collect_segment_spans};
use crate::theme::DpiScale;
use crate::theme::ResolvedAppearance;
use crate::theme::ThemeTokens;
use crate::{
    gui::types::Rect as UiRect,
    gui_runtime::native_vello::{
        NativeTextInputSnapshotFence, NativeTextInputSnapshotFenceAllocator, NativeTextRenderer,
    },
    runtime::{PaintPrimitive, RetainedSurfaceCachePolicy, SurfacePaintPlan},
    widgets::WidgetId,
};
use vello::Scene;
use vello::kurbo::Affine;

#[cfg(test)]
use super::scene::NativePaintSegmentAssemblyResult;
#[cfg(test)]
use super::scene::PaintSegmentEncoding;
#[cfg(test)]
use std::rc::Rc;

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
    pub(super) last_paint_plan: SurfacePaintPlan,
    pub(super) current_text_input_snapshot_fence: Option<NativeTextInputSnapshotFence>,
    text_input_snapshot_fence_allocator: NativeTextInputSnapshotFenceAllocator,
    pub(super) transient_overlay_primitives: Vec<PaintPrimitive>,
    pub(super) composited_base_dirty: bool,
    pub(super) retained_surface_cache: RetainedSurfaceFrameCache,
    pub(super) last_scene_stats: RetainedSurfaceEncodeStats,
    pub(super) native_retained_paint_segment_store: NativeRetainedPaintSegmentStore,
    pub(super) native_paint_segment_artifact_store: NativePaintSegmentArtifactStore,
    pub(super) native_paint_segment_benefit_ledger: NativePaintSegmentBenefitLedger,
    pub(super) native_paint_segment_cache_admission: NativePaintSegmentCacheAdmission,
    pub(super) last_native_paint_segment_eligibility: NativePaintSegmentEligibilityPlan,
    last_native_paint_segment_render_selection: NativePaintSegmentRenderSelection,
    #[cfg(test)]
    test_phase_trace: NativeVelloTestPhaseTrace,
    #[cfg(test)]
    test_scene_encode_observer: Option<Rc<dyn Fn()>>,
    #[cfg(test)]
    test_scene_admission_observer: Option<Rc<dyn Fn()>>,
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

/// Exact CPU-side evidence carried from native scene selection to publication.
/// The witness keeps the prepared plan's context, observations, revisions,
/// target, and selected resident slots together so a detached scene cannot be
/// published with independently reread admission state.
#[derive(Clone, Copy, Debug)]
pub(super) struct NativeSceneAdmissionWitness {
    pub(super) scene_validity: NativeSceneValidityFingerprint,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) paint: PaintSegmentObservation,
    pub(super) eligibility: NativePaintSegmentEligibilityPlan,
    pub(super) render_selection: NativePaintSegmentRenderSelection,
    pub(super) artifact_residency: [NativePaintSegmentArtifactResidency; MAX_PAINT_SEGMENTS],
}

pub(super) enum NativeSceneAdmissionKind {
    FullEncode { assembly_vetoed: bool },
}

/// Complete detached CPU-side successor state. Nothing in this bundle is
/// installed into the last-complete frame until `commit_native_scene_admission`
/// has all scene, text-run, artifact, and observation values ready.
pub(super) struct NativeSceneAdmissionCandidate {
    pub(super) scene: Scene,
    pub(super) stats: RetainedSurfaceEncodeStats,
    pub(super) text_runs: Option<SceneTextRunBuffer>,
    pub(super) retained_surface_cache: Option<RetainedSurfaceFrameCache>,
    pub(super) materialization: NativePaintSegmentArtifactMaterialization,
    pub(super) witness: NativeSceneAdmissionWitness,
    pub(super) kind: NativeSceneAdmissionKind,
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
            last_paint_plan: SurfacePaintPlan::empty(&ThemeTokens::default()),
            current_text_input_snapshot_fence: None,
            text_input_snapshot_fence_allocator: NativeTextInputSnapshotFenceAllocator::default(),
            transient_overlay_primitives: Vec::new(),
            composited_base_dirty: true,
            retained_surface_cache: RetainedSurfaceFrameCache::with_policy(retained_surface_cache),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
            native_retained_paint_segment_store: NativeRetainedPaintSegmentStore::default(),
            native_paint_segment_artifact_store: NativePaintSegmentArtifactStore::default(),
            native_paint_segment_benefit_ledger: NativePaintSegmentBenefitLedger::default(),
            native_paint_segment_cache_admission: NativePaintSegmentCacheAdmission::default(),
            last_native_paint_segment_eligibility: NativePaintSegmentEligibilityPlan::default(),
            last_native_paint_segment_render_selection: NativePaintSegmentRenderSelection::default(
            ),
            #[cfg(test)]
            test_phase_trace: NativeVelloTestPhaseTrace::default(),
            #[cfg(test)]
            test_scene_encode_observer: None,
            #[cfg(test)]
            test_scene_admission_observer: None,
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
        #[cfg(test)]
        if let Some(observer) = self.test_scene_admission_observer.as_ref() {
            observer();
        }
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
        let materialization =
            materialization.filter_for_publication(&self.native_paint_segment_cache_admission);
        self.native_paint_segment_artifact_store
            .reconcile(materialization);
    }

    pub(super) fn clear_native_paint_segment_artifacts(&mut self) {
        self.native_paint_segment_artifact_store.clear();
        self.native_paint_segment_benefit_ledger.clear();
        self.native_paint_segment_cache_admission.clear();
        self.last_native_paint_segment_render_selection =
            NativePaintSegmentRenderSelection::default();
    }

    pub(super) fn invalidate_native_resources_for_recovery(&mut self) {
        self.clear_native_paint_segment_artifacts();
        self.native_retained_paint_segment_store.clear();
        self.invalidate_native_scene_context();
        self.mark_scene_texture_dirty();
        self.mark_composited_base_dirty();
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
        plan: NativePaintSegmentEligibilityPlan,
    ) -> Result<NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyVetoReason> {
        super::scene::assemble_mixed_native_paint_segment_scene(NativePaintSegmentAssemblyInput {
            previous_scene: &self.scene,
            primitives: &self.last_paint_plan.primitives,
            viewport,
            paint,
            previous_stats: self.last_scene_stats,
            plan,
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
        let benefit_paint = bundle.paint;
        let benefit_encoding = bundle.stats.segment_encoding;
        let benefit_feasibility = bundle.stats.artifact_feasibility;
        let benefit_plan = bundle.plan;
        let benefit_target_generation = bundle.target_generation;
        let benefit_fresh_count = bundle.fresh_count;
        let benefit_reused_count = bundle.reused_count;
        let benefit_append_count = bundle.append_count;
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
        self.reconcile_native_paint_segment_artifacts(bundle.materialization);
        self.native_retained_paint_segment_store
            .reconcile(fingerprint_observation);
        self.record_scene_assembly(
            scene_validity,
            bundle.fresh_count,
            bundle.reused_count,
            bundle.append_count,
        );
        self.native_paint_segment_benefit_ledger
            .record_successful_assembly(NativePaintSegmentBenefitAssemblyInput {
                paint: benefit_paint,
                encoding: benefit_encoding,
                feasibility: benefit_feasibility,
                plan: benefit_plan,
                target_generation: benefit_target_generation,
                fresh_count: benefit_fresh_count,
                reused_count: benefit_reused_count,
                append_count: benefit_append_count,
            });
        self.native_paint_segment_cache_admission.reconcile(
            self.native_paint_segment_benefit_ledger
                .latest_frame_evidence(),
        );
        Ok(())
    }

    pub(super) fn commit_native_scene_admission(
        &mut self,
        candidate: NativeSceneAdmissionCandidate,
    ) {
        let NativeSceneAdmissionCandidate {
            scene,
            stats,
            text_runs,
            retained_surface_cache,
            materialization,
            witness,
            kind,
        } = candidate;
        let paint = witness.paint;
        let target_generation = witness.target_generation;
        let scene_validity = witness.scene_validity;
        let encoding = stats.segment_encoding;
        let feasibility = stats.artifact_feasibility;
        debug_assert_eq!(
            witness.artifact_residency,
            witness.render_selection.selected_artifact_residency()
        );
        self.scene = scene;
        self.last_scene_stats = stats;
        if let Some(text_runs) = text_runs {
            self.scene_text_runs = text_runs;
        }
        if let Some(retained_surface_cache) = retained_surface_cache {
            self.retained_surface_cache = retained_surface_cache;
        }
        self.last_native_paint_segment_eligibility = witness.eligibility;
        self.last_native_paint_segment_render_selection = witness.render_selection;
        self.reconcile_native_paint_segment_artifacts(materialization);
        self.reconcile_native_paint_segments(paint, encoding, target_generation);

        match kind {
            NativeSceneAdmissionKind::FullEncode { assembly_vetoed } => {
                if assembly_vetoed {
                    self.record_scene_encode_after_assembly_veto(scene_validity);
                } else {
                    self.record_scene_encode(scene_validity);
                }
                self.record_native_paint_segment_full_encode(
                    paint,
                    encoding,
                    feasibility,
                    target_generation,
                    assembly_vetoed,
                );
            }
        }
    }

    pub(super) fn record_native_paint_segment_full_encode(
        &mut self,
        paint: PaintSegmentObservation,
        encoding: PaintSegmentEncodingObservation,
        feasibility: super::scene::ArtifactFeasibilityObservation,
        target_generation: NativeTargetGeneration,
        assembly_vetoed: bool,
    ) {
        self.native_paint_segment_benefit_ledger.record_full_encode(
            paint,
            encoding,
            feasibility,
            target_generation,
            assembly_vetoed,
        );
        self.native_paint_segment_cache_admission.reconcile(
            self.native_paint_segment_benefit_ledger
                .latest_frame_evidence(),
        );
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

    pub(super) fn derive_native_paint_segment_render_selection(
        &mut self,
        scene_validity: NativeSceneValidityFingerprint,
        target_generation: NativeTargetGeneration,
    ) {
        self.last_native_paint_segment_render_selection =
            select_native_paint_segment_render_boundary(
                self.last_native_paint_segment_eligibility,
                &self.native_paint_segment_cache_admission,
                &self.native_paint_segment_artifact_store,
                scene_validity,
                self.last_scene_validity,
                target_generation,
            );
    }

    pub(super) fn native_paint_segment_render_selection(
        &self,
    ) -> NativePaintSegmentRenderSelection {
        self.last_native_paint_segment_render_selection
    }

    #[cfg(test)]
    pub(super) fn record_scene_encode_boundary(&mut self) {
        self.test_phase_trace
            .record(NativeVelloTestPhase::SceneEncode);
        if let Some(observer) = self.test_scene_encode_observer.as_ref() {
            observer();
        }
    }

    #[cfg(test)]
    pub(super) fn set_test_scene_encode_observer(&mut self, observer: Rc<dyn Fn()>) {
        self.test_scene_encode_observer = Some(observer);
    }

    #[cfg(test)]
    pub(super) fn set_test_scene_admission_observer(&mut self, observer: Rc<dyn Fn()>) {
        self.test_scene_admission_observer = Some(observer);
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

    #[cfg(test)]
    pub(super) const fn native_scene_context_generation_for_test(&self) -> u64 {
        self.native_scene_context_generation
    }

    pub(super) fn mark_scene_texture_dirty(&mut self) {
        self.scene_texture_dirty = true;
        self.composited_base_dirty = true;
    }

    pub(super) fn mark_scene_content_dirty(&mut self) {
        self.scaled_scene_dirty = true;
        self.mark_scene_texture_dirty();
    }

    pub(super) fn seed_text_input_snapshots_for_current_plan(&mut self, force_new_fence: bool) {
        if !force_new_fence && self.current_text_input_snapshot_fence.is_some() {
            return;
        }
        let Some(fence) = self.text_input_snapshot_fence_allocator.allocate() else {
            self.current_text_input_snapshot_fence = None;
            self.text_renderer.invalidate_text_input_snapshots();
            return;
        };
        self.current_text_input_snapshot_fence = Some(fence);
        super::scene::seed_text_input_snapshots_for_plan(
            &self.last_paint_plan,
            &mut self.text_renderer,
            fence,
        );
    }

    pub(super) fn native_ime_cursor_area(&mut self) -> Option<UiRect> {
        let fence = self.current_text_input_snapshot_fence?;
        focused_text_input_caret_area_from_snapshot(
            &self.last_paint_plan,
            &mut self.text_renderer,
            fence,
        )
    }

    /// Resolve a native pointer through the retained paragraph that paints the
    /// current text-input plan. The source travels with the result so runtime
    /// dispatch can reject a stale plan atomically.
    pub(super) fn native_text_pointer_target(
        &mut self,
        position: Point,
        captured_widget_id: Option<WidgetId>,
    ) -> Option<(
        WidgetId,
        String,
        usize,
        crate::gui_runtime::native_vello::CaretAffinity,
    )> {
        let fence = self.current_text_input_snapshot_fence?;
        let input = self
            .last_paint_plan
            .primitives
            .iter()
            .rev()
            .find_map(|primitive| {
                let PaintPrimitive::TextInput(input) = primitive else {
                    return None;
                };
                let hit = captured_widget_id.map_or_else(
                    || input.rect.contains(position),
                    |widget_id| input.widget_id == widget_id,
                );
                hit.then_some(input)
            })?;
        if !input.rect.has_finite_positive_area() {
            return None;
        }

        let snapshot = self.text_renderer.text_input_snapshot_for_input(
            input.widget_id,
            input.state.value.as_str(),
            input.font_size,
            input.rect,
            fence,
        )?;
        let (scalar, affinity) =
            text_input_pointer_target_from_snapshot(input, position, snapshot)?;
        Some((input.widget_id, input.state.value.clone(), scalar, affinity))
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
        renderer: &mut PostGpuOverlayRenderer,
        target: &mut post_gpu_overlay::PostGpuOverlayRenderTarget<'_>,
    ) {
        let Self {
            last_paint_plan,
            transient_overlay_primitives,
            post_gpu_overlay_gpu_regions,
            post_gpu_overlay_suffix_start,
            ..
        } = self;
        let suffix =
            post_gpu_overlay_suffix_start.and_then(|start| last_paint_plan.primitives.get(start..));
        renderer.render_cached_layers(
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
    use crate::{
        gui::types::{Rect, Rgba8},
        runtime::{
            PaintPrimitive, PaintSegment, PaintSegmentAnchor, PaintSegmentIdentity, PaintTextInput,
        },
        widgets::TextInputState,
    };
    use std::sync::Arc;

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

    #[test]
    fn current_text_input_fence_is_reused_until_plan_replacement() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        let input = text_input(7, true, "candidate");
        frame.last_paint_plan = SurfacePaintPlan {
            clear_color: Rgba8::default(),
            primitives: vec![PaintPrimitive::TextInput(input.clone())],
        };

        frame.seed_text_input_snapshots_for_current_plan(false);
        let first_fence = frame
            .current_text_input_snapshot_fence
            .expect("current plan should have a fence");
        let first_snapshot = frame
            .text_renderer
            .text_input_snapshot_for_input(
                input.widget_id,
                input.state.value.as_str(),
                input.font_size,
                input.rect,
                first_fence,
            )
            .expect("current plan snapshot should be available");
        assert!(frame.native_ime_cursor_area().is_some());
        let (widget_id, value, scalar, _) = frame
            .native_text_pointer_target(
                Point::new(input.rect.min.x + 1.0, input.rect.min.y + 1.0),
                None,
            )
            .expect("pointer should use the current plan snapshot");
        assert_eq!(widget_id, input.widget_id);
        assert_eq!(value, input.state.value);
        assert!(scalar <= input.state.value.chars().count());

        frame.seed_text_input_snapshots_for_current_plan(false);
        assert_eq!(frame.current_text_input_snapshot_fence, Some(first_fence));
        let repeated_snapshot = frame
            .text_renderer
            .text_input_snapshot_for_input(
                input.widget_id,
                input.state.value.as_str(),
                input.font_size,
                input.rect,
                first_fence,
            )
            .expect("same-fence snapshot should remain available");
        assert!(Arc::ptr_eq(&first_snapshot, &repeated_snapshot));

        let replacement = text_input(7, true, "replacement");
        frame.last_paint_plan = SurfacePaintPlan {
            clear_color: Rgba8::default(),
            primitives: vec![PaintPrimitive::TextInput(replacement.clone())],
        };
        frame.seed_text_input_snapshots_for_current_plan(true);
        let replacement_fence = frame
            .current_text_input_snapshot_fence
            .expect("replacement plan should have a new fence");
        assert_ne!(replacement_fence, first_fence);
        assert!(
            frame
                .text_renderer
                .text_input_snapshot_for_input(
                    input.widget_id,
                    input.state.value.as_str(),
                    input.font_size,
                    input.rect,
                    first_fence,
                )
                .is_none()
        );
        assert!(
            frame
                .text_renderer
                .text_input_snapshot_for_input(
                    replacement.widget_id,
                    replacement.state.value.as_str(),
                    replacement.font_size,
                    replacement.rect,
                    replacement_fence,
                )
                .is_some()
        );
    }

    fn text_input(widget_id: u64, focused: bool, value: &str) -> PaintTextInput {
        PaintTextInput {
            widget_id,
            rect: Rect::from_min_max(Point::new(8.0, 10.0), Point::new(160.0, 38.0)),
            placeholder: None,
            completion_suffix: None,
            state: TextInputState::from_value(value.to_owned()),
            font_size: 14.0,
            baseline: None,
            color: Rgba8::default(),
            placeholder_color: Rgba8::default(),
            completion_color: Rgba8::default(),
            selection_color: Rgba8::default(),
            caret_color: Rgba8::default(),
            focused,
        }
    }
}
