//! Scene encoding for generic runtime paint plans.

use crate::{
    gui::types::{Rgba8, Vector2},
    gui_runtime::native_vello::{NativeTextRenderer, to_kurbo_rect},
    runtime::{
        MAX_PAINT_SEGMENTS, PaintPrimitive, PaintSegmentSpan, RuntimeBridge,
        RuntimeRetainedSurfaceCapability,
    },
};
use std::{sync::Arc, time::Duration};
use vello::{Scene, kurbo::Affine, peniko::Fill};

mod artifact_feasibility;
mod artifact_materialization;
mod cache;
mod clip;
mod custom_surface;
mod frame;
mod image;
mod segment_evidence;
mod shape;
mod svg;
mod text;
mod text_input;
mod text_input_selection;
mod text_runs;
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::ArtifactFeasibilityObservation;
#[cfg(test)]
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::{
    ArtifactFeasibilityCheckpoint, ArtifactFeasibilityCollector, ArtifactFeasibilityCounts,
    ArtifactFeasibilitySegment,
};
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::{
    ArtifactFeasibilityDisposition, ArtifactFeasibilityReason,
};
pub(in crate::gui_runtime::native_vello) use artifact_materialization::{
    NativePaintSegmentArtifactMaterialization, NativePaintSegmentArtifactStore,
};
pub(super) use artifact_materialization::{
    NativePaintSegmentAssemblyResult, NativePaintSegmentAssemblyVetoReason,
    assemble_retained_native_paint_segment_scene, materialize_native_paint_segment_artifacts,
};
pub(in crate::gui_runtime::native_vello) use cache::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache,
};
pub(in crate::gui_runtime::native_vello) use clip::{SceneClipBegin, SceneClipEnd, SceneClipState};
use custom_surface::{CustomSurfaceEncodeContext, encode_custom_surface};
use image::encode_image;
pub(in crate::gui_runtime::native_vello) use segment_evidence::PaintSegmentEncoding;
pub(in crate::gui_runtime::native_vello) use segment_evidence::PaintSegmentEncodingObservation;
pub(in crate::gui_runtime::native_vello) use segment_evidence::{
    EncodingConservativeReason, EncodingIsolation, SafeEnclosure,
};
use shape::{
    encode_path_fill, encode_polygon_fill, encode_polygon_stroke, encode_polyline_stroke,
    encode_rect, encode_rect_batch, encode_rect_stroke, encode_rect_stroke_batch,
};
use svg::encode_svg;
use text::encode_text;
use text_input::encode_text_input;
pub(in crate::gui_runtime::native_vello) use text_runs::SceneTextRunBuffer;
use text_runs::flush_text_runs;

use super::retained_paint_segments::NativePaintSegmentEligibilityPlan;

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene<Bridge, Message>(
    plan: &crate::runtime::SurfacePaintPlan,
    context: SurfaceSceneEncodeContext<'_, Bridge>,
) -> RetainedSurfaceEncodeStats
where
    Bridge: RuntimeBridge<Message>,
{
    let SurfaceSceneEncodeContext {
        scene,
        text_renderer,
        bridge,
        retained_surface,
        viewport,
        retained_cache,
        text_runs,
        animation_time,
    } = context;
    scene.reset();
    text_runs.clear();
    let mut stats = RetainedSurfaceEncodeStats {
        paint_plan_primitives: plan.primitives.len(),
        ..RetainedSurfaceEncodeStats::default()
    };
    let mut clip_state = SceneClipState::default();
    let mut segment_evidence = segment_evidence::PaintSegmentEvidenceCollector::new(
        &plan.primitives,
        crate::gui::types::Rect::from_size(viewport.x, viewport.y),
    );
    let mut artifact_feasibility =
        artifact_feasibility::ArtifactFeasibilityCollector::new(&plan.primitives);
    for (index, primitive) in plan.primitives.iter().enumerate() {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                let depth_before = clip_state.depth();
                flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                let begin = clip_state.begin(clip.rect);
                segment_evidence.observe_clip_start(index, depth_before, begin, clip.rect);
                if begin.pushes_layer() {
                    stats.clip_layer_count = stats.clip_layer_count.saturating_add(1);
                    scene.push_clip_layer(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &to_kurbo_rect(clip.rect),
                    );
                }
                continue;
            }
            PaintPrimitive::ClipEnd(_) => {
                let depth_before = clip_state.depth();
                let end = clip_state.end();
                segment_evidence.observe_clip_end(index, depth_before, clip_state.depth(), end);
                if end.pops_layer() {
                    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                    scene.pop_layer();
                }
                continue;
            }
            _ if clip_state.is_suppressed() => {
                segment_evidence.observe_suppressed(index, clip_state.depth());
                if primitive.gpu_surface().is_some() {
                    segment_evidence.observe_anchor(index, clip_state.depth());
                    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                    artifact_feasibility.checkpoint(index as u32, scene);
                }
                continue;
            }
            _ => {}
        }
        if primitive.gpu_surface().is_some() {
            segment_evidence.observe_anchor(index, clip_state.depth());
            flush_text_runs(scene, text_renderer, text_runs, &mut stats);
            artifact_feasibility.checkpoint(index as u32, scene);
        } else {
            segment_evidence.observe_paint(index, primitive, &clip_state);
        }
        if flushes_pending_text_before_encoding(primitive) {
            flush_text_runs(scene, text_renderer, text_runs, &mut stats);
        }
        match primitive {
            PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => {}
            PaintPrimitive::FillRect(fill) => encode_rect(scene, fill.color, fill.rect),
            PaintPrimitive::FillRectBatch(fill) => {
                encode_rect_batch(scene, fill.color, &fill.rects);
            }
            PaintPrimitive::FillPath(fill) => {
                encode_path_fill(
                    scene,
                    &fill.brush,
                    fill.transform,
                    fill.fill_rule,
                    &fill.path,
                );
            }
            PaintPrimitive::Svg(svg) => {
                stats.svg_document_count = stats.svg_document_count.saturating_add(1);
                encode_svg(scene, svg);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                encode_rect_stroke(scene, stroke.color, stroke.width, stroke.rect);
            }
            PaintPrimitive::StrokeRectBatch(stroke) => {
                encode_rect_stroke_batch(scene, stroke.color, stroke.width, &stroke.rects);
            }
            PaintPrimitive::FillPolygon(fill) => {
                encode_polygon_fill(scene, fill.color, &fill.points);
            }
            PaintPrimitive::StrokePolygon(stroke) => {
                encode_polygon_stroke(scene, stroke.color, stroke.width, &stroke.points);
            }
            PaintPrimitive::StrokePolyline(stroke) => {
                encode_polyline_stroke(scene, stroke.color, stroke.width, &stroke.points);
            }
            PaintPrimitive::Text(text) => {
                stats.text_primitive_count = stats.text_primitive_count.saturating_add(1);
                encode_text(text_runs, text);
            }
            PaintPrimitive::OverlayPanel(panel) => {
                encode_rect(
                    scene,
                    Rgba8 {
                        r: 48,
                        g: 48,
                        b: 48,
                        a: 255,
                    },
                    panel.rect,
                );
            }
            PaintPrimitive::TextInput(input) => {
                stats.text_input_count = stats.text_input_count.saturating_add(1);
                encode_text_input(scene, text_renderer, input, animation_time);
                stats.record_text_runs(1);
            }
            PaintPrimitive::Image(draw) => {
                stats.image_count = stats.image_count.saturating_add(1);
                encode_image(
                    scene,
                    Arc::clone(draw.image.shared_pixels()),
                    draw.image.width(),
                    draw.image.height(),
                    draw.source_rect,
                    draw.rect,
                );
            }
            PaintPrimitive::GpuSurface(_) => {
                stats.gpu_surface_count = stats.gpu_surface_count.saturating_add(1);
            }
            PaintPrimitive::CustomSurface(custom) => {
                encode_custom_surface(
                    CustomSurfaceEncodeContext {
                        scene,
                        text_renderer,
                        bridge,
                        retained_surface,
                        viewport,
                        retained_cache,
                        stats: &mut stats,
                    },
                    custom,
                );
            }
        }
    }
    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
    stats.segment_encoding = segment_evidence.finish(clip_state.depth());
    stats.artifact_feasibility =
        artifact_feasibility.finish(scene, stats.segment_encoding, &plan.primitives);
    stats
}

/// The auxiliary payloads selected for one eligibility plan.
pub(in crate::gui_runtime::native_vello) struct NativePaintSegmentPayloadSelection {
    payloads: Vec<Scene>,
    #[cfg(test)]
    reused_count: usize,
    #[cfg(test)]
    fresh_count: usize,
}

impl NativePaintSegmentPayloadSelection {
    fn empty() -> Self {
        Self {
            payloads: Vec::new(),
            #[cfg(test)]
            reused_count: 0,
            #[cfg(test)]
            fresh_count: 0,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn into_payloads(self) -> Vec<Scene> {
        self.payloads
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn payloads_for_test(&self) -> &[Scene] {
        &self.payloads
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn reused_count_for_test(&self) -> usize {
        self.reused_count
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn fresh_count_for_test(&self) -> usize {
        self.fresh_count
    }
}

/// Select one typed, resource-free payload for every entry in an eligibility
/// plan. Matching retained artifacts are cloned; all other entries are freshly
/// encoded from the current primitives. This is auxiliary preparation only: the
/// authoritative scene above remains the render source and is never assembled
/// from these payloads.
pub(in crate::gui_runtime::native_vello) fn encode_native_paint_segment_payloads(
    primitives: &[PaintPrimitive],
    plan: NativePaintSegmentEligibilityPlan,
    scene_validity: super::frame_state::NativeSceneValidityFingerprint,
    artifacts: &NativePaintSegmentArtifactStore,
) -> NativePaintSegmentPayloadSelection {
    let count = usize::from(plan.entry_count);
    if count == 0 || count > MAX_PAINT_SEGMENTS {
        return NativePaintSegmentPayloadSelection::empty();
    }

    let mut selection = NativePaintSegmentPayloadSelection {
        payloads: Vec::with_capacity(count),
        #[cfg(test)]
        reused_count: 0,
        #[cfg(test)]
        fresh_count: 0,
    };
    for index in 0..count {
        let Some(entry) = plan.entries[index] else {
            return NativePaintSegmentPayloadSelection::empty();
        };
        if let Some(payload) = artifacts.reusable_payload(entry, scene_validity) {
            selection.payloads.push(payload);
            #[cfg(test)]
            {
                selection.reused_count += 1;
            }
            continue;
        };

        let Some(payload) = encode_resource_free_segment(primitives, entry.span) else {
            return NativePaintSegmentPayloadSelection::empty();
        };
        selection.payloads.push(payload);
        #[cfg(test)]
        {
            selection.fresh_count += 1;
        }
    }
    selection
}

fn encode_resource_free_segment(
    primitives: &[PaintPrimitive],
    span: PaintSegmentSpan,
) -> Option<Scene> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    let primitives = primitives.get(start..end)?;
    let mut scene = Scene::new();
    let mut clip_state = SceneClipState::default();

    for primitive in primitives {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                if clip_state.begin(clip.rect).pushes_layer() {
                    scene.push_clip_layer(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &to_kurbo_rect(clip.rect),
                    );
                }
            }
            PaintPrimitive::ClipEnd(_) => {
                if clip_state.end().pops_layer() {
                    scene.pop_layer();
                }
            }
            _ if clip_state.is_suppressed() => {}
            PaintPrimitive::FillRect(fill) => encode_rect(&mut scene, fill.color, fill.rect),
            PaintPrimitive::FillRectBatch(fill) => {
                encode_rect_batch(&mut scene, fill.color, &fill.rects)
            }
            PaintPrimitive::OverlayPanel(panel) => encode_rect(
                &mut scene,
                Rgba8 {
                    r: 48,
                    g: 48,
                    b: 48,
                    a: 255,
                },
                panel.rect,
            ),
            _ => return None,
        }
    }

    (clip_state.depth() == 0).then_some(scene)
}

pub(super) fn flushes_pending_text_before_encoding(primitive: &PaintPrimitive) -> bool {
    !matches!(
        primitive,
        PaintPrimitive::Text(_) | PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_)
    )
}

pub(in crate::gui_runtime::native_vello) struct SurfaceSceneEncodeContext<'a, Bridge> {
    pub scene: &'a mut Scene,
    pub text_renderer: &'a mut NativeTextRenderer,
    pub bridge: &'a mut Bridge,
    pub retained_surface: Option<RuntimeRetainedSurfaceCapability<Bridge>>,
    pub viewport: Vector2,
    pub retained_cache: &'a mut RetainedSurfaceFrameCache,
    pub text_runs: &'a mut SceneTextRunBuffer,
    pub animation_time: Duration,
}
