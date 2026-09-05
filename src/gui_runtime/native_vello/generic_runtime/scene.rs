//! Scene encoding for generic runtime paint plans.

use crate::{
    gui::types::{Rect, Rgba8, Vector2},
    gui_runtime::native_vello::{
        CaretAffinity, NativeTextInputSnapshotFence, NativeTextRenderer, ParagraphSnapshot,
        to_kurbo_rect,
    },
    runtime::{
        MAX_PAINT_SEGMENTS, PaintPrimitive, PaintSegmentObservation, PaintSegmentSpan,
        PaintTextInput, RuntimeBridge, RuntimeRetainedSurfaceCapability, SurfacePaintPlan,
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
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::ArtifactFeasibilityCounts;
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::ArtifactFeasibilityObservation;
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::segment_local_count_delta;
#[cfg(test)]
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::{
    ArtifactFeasibilityCheckpoint, ArtifactFeasibilityCollector, ArtifactFeasibilitySegment,
};
pub(in crate::gui_runtime::native_vello) use artifact_feasibility::{
    ArtifactFeasibilityDisposition, ArtifactFeasibilityReason,
};
pub(in crate::gui_runtime::native_vello::generic_runtime) use artifact_materialization::NativePaintSegmentArtifactResidency;
pub(in crate::gui_runtime::native_vello) use artifact_materialization::{
    NativePaintSegmentArtifactMaterialization, NativePaintSegmentArtifactStore,
    NativePaintSegmentPayload, NativePaintSegmentPayloadEvidence,
};
pub(super) use artifact_materialization::{
    NativePaintSegmentAssemblyBundle, NativePaintSegmentAssemblyInput,
    NativePaintSegmentAssemblyVetoReason, assemble_mixed_native_paint_segment_scene,
    materialize_native_paint_segment_artifacts,
};
#[cfg(test)]
pub(super) use artifact_materialization::{
    NativePaintSegmentAssemblyResult, assemble_retained_native_paint_segment_scene,
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

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "legacy renderer-backed caret projection remains covered by focused text-input tests"
    )
)]
pub(super) fn focused_text_input_caret_area(
    plan: &SurfacePaintPlan,
    text_renderer: &mut NativeTextRenderer,
) -> Option<Rect> {
    let mut focused_input = None;
    for primitive in &plan.primitives {
        let PaintPrimitive::TextInput(input) = primitive else {
            continue;
        };
        if !input.focused {
            continue;
        }
        if focused_input.replace(input).is_some() {
            return None;
        }
    }
    focused_input.and_then(|input| text_input::focused_text_input_caret_rect(input, text_renderer))
}

pub(super) fn focused_text_input_caret_area_from_snapshot(
    plan: &SurfacePaintPlan,
    text_renderer: &mut NativeTextRenderer,
    fence: NativeTextInputSnapshotFence,
) -> Option<Rect> {
    let mut focused_input = None;
    for primitive in &plan.primitives {
        let PaintPrimitive::TextInput(input) = primitive else {
            continue;
        };
        if !input.focused {
            continue;
        }
        if focused_input.replace(input).is_some() {
            return None;
        }
    }
    let input = focused_input?;
    let snapshot = text_renderer.text_input_snapshot_for_input_aligned(
        input.widget_id,
        input.state.value.as_str(),
        input.font_size,
        native_text_alignment(input.align),
        input.rect,
        fence,
    )?;
    text_input::focused_text_input_caret_rect_from_snapshot(input, text_renderer, snapshot)
}

fn native_text_alignment(align: crate::runtime::PaintTextAlign) -> crate::gui::paint::TextAlign {
    match align {
        crate::runtime::PaintTextAlign::Left => crate::gui::paint::TextAlign::Left,
        crate::runtime::PaintTextAlign::Center => crate::gui::paint::TextAlign::Center,
        crate::runtime::PaintTextAlign::Right => crate::gui::paint::TextAlign::Right,
    }
}

pub(in crate::gui_runtime::native_vello) fn seed_text_input_snapshots_for_plan(
    plan: &SurfacePaintPlan,
    text_renderer: &mut NativeTextRenderer,
    fence: NativeTextInputSnapshotFence,
) {
    text_renderer.begin_text_input_snapshot_fence(fence);
    for primitive in &plan.primitives {
        let PaintPrimitive::TextInput(input) = primitive else {
            continue;
        };
        text_input::seed_text_input_snapshot(text_renderer, input, fence);
    }
}

pub(super) fn text_input_pointer_target_from_snapshot(
    input: &PaintTextInput,
    position: crate::gui::types::Point,
    snapshot: std::sync::Arc<ParagraphSnapshot>,
) -> Option<(usize, CaretAffinity)> {
    text_input::text_input_pointer_target_from_snapshot(input, position, snapshot)
}

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
        text_input_snapshot_fence,
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
                let snapshot = text_input_snapshot_fence.and_then(|fence| {
                    text_renderer.text_input_snapshot_for_input_aligned(
                        input.widget_id,
                        input.state.value.as_str(),
                        input.font_size,
                        native_text_alignment(input.align),
                        input.rect,
                        fence,
                    )
                });
                encode_text_input(scene, text_renderer, input, animation_time, snapshot);
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
    payloads: Vec<NativePaintSegmentPayload>,
    reused_count: usize,
    fresh_count: usize,
}

impl NativePaintSegmentPayloadSelection {
    fn empty() -> Self {
        Self {
            payloads: Vec::new(),
            reused_count: 0,
            fresh_count: 0,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn into_parts(
        self,
    ) -> (Vec<NativePaintSegmentPayload>, usize, usize) {
        (self.payloads, self.fresh_count, self.reused_count)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn payloads_for_test(
        &self,
    ) -> &[NativePaintSegmentPayload] {
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

/// Select one typed, resource-free payload and current local evidence for every
/// entry in an eligibility plan. Matching retained artifacts are cloned; fresh
/// entries are encoded only after the caller has preflighted the complete plan.
pub(in crate::gui_runtime::native_vello) fn encode_native_paint_segment_payloads(
    primitives: &[PaintPrimitive],
    viewport: Vector2,
    paint: PaintSegmentObservation,
    plan: NativePaintSegmentEligibilityPlan,
    scene_validity: super::frame_state::NativeSceneValidityFingerprint,
    target_generation: super::runner_state::NativeTargetGeneration,
    artifacts: &NativePaintSegmentArtifactStore,
) -> NativePaintSegmentPayloadSelection {
    let count = usize::from(plan.entry_count);
    if count == 0
        || count > MAX_PAINT_SEGMENTS
        || usize::from(paint.segment_count) != count
        || paint.conservative
        || paint.all_implicated
    {
        return NativePaintSegmentPayloadSelection::empty();
    }

    let mut selection = NativePaintSegmentPayloadSelection {
        payloads: Vec::with_capacity(count),
        reused_count: 0,
        fresh_count: 0,
    };
    for index in 0..count {
        let (Some(entry), Some(current)) = (plan.entries[index], paint.segments[index]) else {
            return NativePaintSegmentPayloadSelection::empty();
        };
        if let Some(payload) =
            artifacts.reusable_payload(index, entry, scene_validity, target_generation)
        {
            selection.payloads.push(payload);
            selection.reused_count += 1;
            continue;
        };

        let Some(payload) = encode_resource_free_segment(
            primitives,
            viewport,
            entry.span,
            current.revision,
            scene_validity,
            target_generation,
        ) else {
            return NativePaintSegmentPayloadSelection::empty();
        };
        selection.payloads.push(payload);
        selection.fresh_count += 1;
    }
    selection
}

fn encode_resource_free_segment(
    primitives: &[PaintPrimitive],
    viewport: Vector2,
    span: PaintSegmentSpan,
    revision: u64,
    scene_validity: super::frame_state::NativeSceneValidityFingerprint,
    target_generation: super::runner_state::NativeTargetGeneration,
) -> Option<NativePaintSegmentPayload> {
    if revision == 0 || !target_generation.is_known() {
        return None;
    }
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    let primitives = primitives.get(start..end)?;
    let mut scene = Scene::new();
    let mut clip_state = SceneClipState::default();
    let mut evidence = segment_evidence::PaintSegmentEvidenceCollector::new(
        primitives,
        Rect::from_size(viewport.x, viewport.y),
    );
    let mut clip_layer_count: usize = 0;

    for (index, primitive) in primitives.iter().enumerate() {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                let depth_before = clip_state.depth();
                let begin = clip_state.begin(clip.rect);
                evidence.observe_clip_start(index, depth_before, begin, clip.rect);
                if begin.pushes_layer() {
                    clip_layer_count = clip_layer_count.saturating_add(1);
                    scene.push_clip_layer(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &to_kurbo_rect(clip.rect),
                    );
                }
            }
            PaintPrimitive::ClipEnd(_) => {
                let depth_before = clip_state.depth();
                let end = clip_state.end();
                evidence.observe_clip_end(index, depth_before, clip_state.depth(), end);
                if end.pops_layer() {
                    scene.pop_layer();
                }
            }
            _ if clip_state.is_suppressed() => {
                evidence.observe_suppressed(index, clip_state.depth());
            }
            PaintPrimitive::FillRect(fill) => {
                evidence.observe_paint(index, primitive, &clip_state);
                encode_rect(&mut scene, fill.color, fill.rect);
            }
            PaintPrimitive::FillRectBatch(fill) => {
                evidence.observe_paint(index, primitive, &clip_state);
                encode_rect_batch(&mut scene, fill.color, &fill.rects)
            }
            PaintPrimitive::OverlayPanel(panel) => {
                evidence.observe_paint(index, primitive, &clip_state);
                encode_rect(
                    &mut scene,
                    Rgba8 {
                        r: 48,
                        g: 48,
                        b: 48,
                        a: 255,
                    },
                    panel.rect,
                );
            }
            _ => return None,
        }
    }

    let local_encoding = evidence.finish(clip_state.depth());
    let local_encoding = local_encoding.segments[0]?;
    if local_encoding.conservative
        || !matches!(local_encoding.isolation, EncodingIsolation::SelfContained)
        || matches!(
            local_encoding.safe_enclosure,
            SafeEnclosure::ViewportFallback
        )
        || !matches!(local_encoding.reason, EncodingConservativeReason::None)
        || clip_state.depth() != 0
    {
        return None;
    }

    let counts = artifact_counts_from_scene(&scene);
    Some(NativePaintSegmentPayload {
        scene,
        evidence: NativePaintSegmentPayloadEvidence {
            identity: span.identity,
            span,
            revision,
            target_generation,
            scene_validity,
            encoding: PaintSegmentEncoding {
                identity: span.identity,
                primitive_start: span.start,
                primitive_end: span.end,
                safe_enclosure: local_encoding.safe_enclosure,
                isolation: local_encoding.isolation,
                conservative: local_encoding.conservative,
                reason: local_encoding.reason,
            },
            counts,
            clip_layer_count,
        },
    })
}

fn artifact_counts_from_scene(scene: &Scene) -> ArtifactFeasibilityCounts {
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
    pub text_input_snapshot_fence: Option<NativeTextInputSnapshotFence>,
}

#[cfg(test)]
mod focused_text_input_tests {
    use super::{
        focused_text_input_caret_area, focused_text_input_caret_area_from_snapshot,
        seed_text_input_snapshots_for_plan, text_input_pointer_target_from_snapshot,
    };
    use crate::{
        gui::types::{Point, Rect, Rgba8},
        gui_runtime::native_vello::{
            CaretAffinity, NativeTextInputSnapshotFenceAllocator, NativeTextRenderer,
        },
        runtime::{PaintPrimitive, PaintTextInput, SurfacePaintPlan},
        widgets::TextInputState,
    };
    use std::sync::Arc;

    #[test]
    fn focused_text_input_caret_area_requires_exactly_one_focused_input() {
        let mut text_renderer = NativeTextRenderer::new();
        let unfocused = text_input(1, false);
        let focused = text_input(2, true);

        assert_eq!(caret_area(&mut text_renderer, []), None);
        assert_eq!(caret_area(&mut text_renderer, [unfocused.clone()]), None);
        assert!(caret_area(&mut text_renderer, [focused.clone()]).is_some());
        assert!(caret_area(&mut text_renderer, [unfocused, focused.clone()]).is_some());
        assert_eq!(
            caret_area(&mut text_renderer, [focused.clone(), focused]),
            None
        );
    }

    #[test]
    fn seeded_text_inputs_feed_ime_and_pointer_from_one_current_fence() {
        let mut text_renderer = NativeTextRenderer::new();
        let mut fences = NativeTextInputSnapshotFenceAllocator::default();
        let fence = fences.allocate().expect("current plan fence");
        let mut empty = text_input(1, false);
        empty.state = TextInputState::from_value(String::new());
        let focused = text_input(2, true);
        let plan = SurfacePaintPlan {
            clear_color: Rgba8::default(),
            primitives: vec![
                PaintPrimitive::TextInput(empty.clone()),
                PaintPrimitive::TextInput(focused.clone()),
            ],
        };

        seed_text_input_snapshots_for_plan(&plan, &mut text_renderer, fence);
        let focused_snapshot = text_renderer
            .text_input_snapshot_for_input(
                focused.widget_id,
                focused.state.value.as_str(),
                focused.font_size,
                focused.rect,
                fence,
            )
            .expect("focused input snapshot should be seeded");
        let empty_snapshot = text_renderer
            .text_input_snapshot_for_input(
                empty.widget_id,
                empty.state.value.as_str(),
                empty.font_size,
                empty.rect,
                fence,
            )
            .expect("empty input snapshot should be seeded");

        assert!(!Arc::ptr_eq(&focused_snapshot, &empty_snapshot));
        assert!(
            focused_text_input_caret_area_from_snapshot(&plan, &mut text_renderer, fence,)
                .is_some()
        );
        let (scalar, affinity) = text_input_pointer_target_from_snapshot(
            &focused,
            Point::new(focused.rect.min.x + 1.0, focused.rect.min.y + 1.0),
            focused_snapshot.clone(),
        )
        .expect("pointer target should use the seeded snapshot");
        assert!(scalar <= focused.state.value.chars().count());
        assert!(matches!(
            affinity,
            CaretAffinity::Upstream | CaretAffinity::Downstream
        ));

        seed_text_input_snapshots_for_plan(&plan, &mut text_renderer, fence);
        let repeated_snapshot = text_renderer
            .text_input_snapshot_for_input(
                focused.widget_id,
                focused.state.value.as_str(),
                focused.font_size,
                focused.rect,
                fence,
            )
            .expect("same-fence reseed should remain available");
        assert!(Arc::ptr_eq(&focused_snapshot, &repeated_snapshot));

        let next_fence = fences.allocate().expect("next plan fence");
        assert!(
            focused_text_input_caret_area_from_snapshot(&plan, &mut text_renderer, next_fence,)
                .is_none()
        );
    }

    fn caret_area<const N: usize>(
        text_renderer: &mut NativeTextRenderer,
        inputs: [PaintTextInput; N],
    ) -> Option<Rect> {
        focused_text_input_caret_area(
            &SurfacePaintPlan {
                clear_color: Rgba8::default(),
                primitives: inputs.into_iter().map(PaintPrimitive::TextInput).collect(),
            },
            text_renderer,
        )
    }

    fn text_input(widget_id: u64, focused: bool) -> PaintTextInput {
        PaintTextInput {
            widget_id,
            rect: Rect::from_min_max(Point::new(8.0, 10.0), Point::new(160.0, 38.0)),
            placeholder: None,
            completion_suffix: None,
            state: TextInputState::from_value(String::from("candidate")),
            font_size: 14.0,
            align: crate::runtime::PaintTextAlign::Left,
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
