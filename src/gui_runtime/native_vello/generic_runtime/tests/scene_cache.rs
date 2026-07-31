use super::*;
use crate::{
    gui::types::ImageRgba,
    runtime::{
        GpuSurfaceRuntimeOverlays, PaintClipEnd, PaintClipStart, PaintFillRect, PaintSegment,
        PaintSegmentObservation, PaintTextAlign, PaintTextRun, SurfacePaintPlan,
    },
    theme::ThemeTokens,
    widgets::TextWrap,
};

#[path = "scene_cache/retained.rs"]
mod retained;

#[test]
fn generic_paint_plan_encodes_to_vello_scene() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();

    let plan = core.paint_plan();
    let expected_text_primitives = plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::Text(_)))
        .count();
    let expected_text_inputs = plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::TextInput(_)))
        .count();
    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(stats.paint_plan_primitives, plan.primitives.len());
    assert_eq!(stats.text_primitive_count, expected_text_primitives);
    assert_eq!(stats.text_input_count, expected_text_inputs);
    assert!(stats.text_run_count >= expected_text_primitives);
    assert!(text_runs.is_empty());
    assert_eq!(text_runs.overflow_capacity(), 0);
}

fn classify_collector_evidence(
    stats: &RetainedSurfaceEncodeStats,
) -> super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan {
    let mut paint = PaintSegmentObservation::empty();
    paint.segment_count = stats.segment_encoding.segment_count;
    for index in 0..usize::from(paint.segment_count) {
        let encoded = stats.segment_encoding.segments[index].expect("encoded segment");
        paint.segments[index] = Some(PaintSegment {
            identity: encoded.identity,
            owner: None,
            revision: 1,
            implicated: false,
        });
    }
    classify_collector_evidence_parts(paint, stats.segment_encoding, stats.artifact_feasibility)
}

fn classify_collector_evidence_parts(
    paint: PaintSegmentObservation,
    encoding: super::scene::PaintSegmentEncodingObservation,
    feasibility: super::scene::ArtifactFeasibilityObservation,
) -> super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan {
    let target_generation = super::super::runner_state::NativeTargetGeneration::from_test_serial(1);
    let mut retained =
        super::super::retained_paint_segments::NativeRetainedPaintSegmentStore::default();
    retained.reconcile(
        super::super::retained_paint_segments::assemble_native_paint_segment_fingerprints(
            paint,
            encoding,
            target_generation,
        ),
    );
    super::super::retained_paint_segments::classify_native_paint_segment_eligibility(
        paint,
        &retained,
        feasibility,
        target_generation,
    )
}

#[test]
fn collector_fresh_evidence_reaches_classifier_without_aggregate_fallback() {
    let mut bridge = demo_bridge();
    let scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let clip = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let no_artifact = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 1,
                rect: clip,
            }),
            PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 1 }),
        ],
    };
    let identity = crate::runtime::PaintSegmentIdentity {
        preceding: None,
        following: None,
    };
    let mut encoding_segments = [None; crate::runtime::MAX_PAINT_SEGMENTS];
    encoding_segments[0] = Some(super::scene::PaintSegmentEncoding {
        identity,
        primitive_start: 0,
        primitive_end: 2,
        safe_enclosure: super::scene::SafeEnclosure::Empty,
        isolation: super::scene::EncodingIsolation::SelfContained,
        conservative: false,
        reason: super::scene::EncodingConservativeReason::None,
    });
    let encoding = super::scene::PaintSegmentEncodingObservation {
        segments: encoding_segments,
        segment_count: 1,
        conservative: false,
    };
    let feasibility = super::super::scene::ArtifactFeasibilityCollector::new(
        &no_artifact.primitives,
    )
    .finish(&scene, encoding, &no_artifact.primitives);
    let mut paint_segments = [None; crate::runtime::MAX_PAINT_SEGMENTS];
    paint_segments[0] = Some(PaintSegment {
        identity,
        owner: None,
        revision: 1,
        implicated: false,
    });
    let paint = PaintSegmentObservation {
        segments: paint_segments,
        segment_count: 1,
        conservative: false,
        all_implicated: false,
    };
    assert_eq!(
        feasibility.segments[0]
            .expect("NoArtifact evidence")
            .disposition,
        super::scene::ArtifactFeasibilityDisposition::NoArtifact
    );
    assert!(!feasibility.conservative);
    assert_eq!(
        classify_collector_evidence_parts(paint, encoding, feasibility).outcome,
        super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::Plan
    );

    let rect = Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0));
    let fill = || {
        PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 7,
            rect,
            color: Rgba8::new(255, 255, 255, 255),
        })
    };
    let fresh_encoding = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            fill(),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 42,
                key: 42,
                revision: 1,
                rect,
                content: GpuSurfaceContent::CustomShader {
                    descriptor: Arc::new(crate::runtime::GpuShaderSurfaceDescriptor::new("test")),
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            fill(),
        ],
    };
    let mut stats_scene = Scene::new();
    let stats = encode_plan(
        &fresh_encoding,
        &mut stats_scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );
    assert_eq!(
        stats.artifact_feasibility.segments[1]
            .expect("fresh-encoding evidence")
            .disposition,
        super::scene::ArtifactFeasibilityDisposition::RequiresFreshEncoding(
            super::scene::ArtifactFeasibilityReason::CrossSegmentTransformOrStyle,
        )
    );
    assert!(!stats.artifact_feasibility.conservative);
    assert_eq!(
        classify_collector_evidence(&stats).outcome,
        super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::Plan
    );
}

#[test]
fn scene_encoding_elides_containing_nested_clip_but_keeps_draw_content() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let ancestor = Rect::from_min_size(Point::new(10.0, 10.0), Vector2::new(10.0, 10.0));
    let containing = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 30.0));
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 1,
                rect: ancestor,
            }),
            PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 2,
                rect: containing,
            }),
            PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 3,
                rect: ancestor,
                color: Rgba8 {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }),
            PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 2 }),
            PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 1 }),
        ],
    };

    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(stats.clip_layer_count, 1);
    assert_eq!(scene.encoding().n_clips, 2);
    assert!(!scene.encoding().is_empty());
}

#[test]
fn scene_text_run_buffer_presizes_overflow_storage() {
    let text_runs = SceneTextRunBuffer::with_overflow_capacity(32);

    assert!(text_runs.overflow_capacity() >= 32);
}

#[test]
fn scene_encoding_flushes_text_before_later_non_text_primitives() {
    let text = PaintPrimitive::Text(PaintTextRun {
        widget_id: 1,
        text: "base label".into(),
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 18.0)),
        font_size: 12.0,
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
        baseline: None,
    });
    let overlay_fill = PaintPrimitive::FillRect(PaintFillRect {
        widget_id: 2,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 18.0)),
        color: Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
    });

    assert!(!super::scene::flushes_pending_text_before_encoding(&text));
    assert!(super::scene::flushes_pending_text_before_encoding(
        &overlay_fill
    ));
}

#[test]
fn scene_encoding_counts_gpu_surfaces_without_projecting_interactions() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let rect = Rect::from_min_size(Point::new(8.0, 12.0), Vector2::new(64.0, 32.0));
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 42,
            key: 42,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(64.0, 32.0)),
                atlas: Arc::new(
                    ImageRgba::new(64, 32, vec![255; 64 * 32 * 4]).expect("valid test image"),
                ),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: false,
                coalesce_horizontal_wheel: false,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            overlays: Vec::new(),
        })],
    };

    let stats = encode_surface_paint_plan_to_scene(
        &plan,
        SurfaceSceneEncodeContext {
            scene: &mut scene,
            text_renderer: &mut text_renderer,
            bridge: &mut bridge,
            retained_surface: None,
            viewport: Vector2::new(320.0, 180.0),
            retained_cache: &mut retained_cache,
            text_runs: &mut text_runs,
            animation_time: Duration::ZERO,
        },
    );

    assert_eq!(stats.gpu_surface_count, 1);
}

#[test]
fn scene_encoding_observes_finite_self_contained_segment_bounds() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let rect = Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0));
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 7,
            rect,
            color: Rgba8::new(255, 255, 255, 255),
        })],
    };

    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );
    let segment = stats.segment_encoding.segments[0].expect("one segment");
    assert_eq!(segment.primitive_start, 0);
    assert_eq!(segment.primitive_end, 1);
    assert_eq!(
        segment.safe_enclosure,
        super::scene::SafeEnclosure::Bounded(rect)
    );
    assert_eq!(
        segment.isolation,
        super::scene::EncodingIsolation::SelfContained
    );
    assert!(!segment.conservative);
    let artifact = stats.artifact_feasibility.segments[0].expect("artifact evidence");
    assert_eq!(stats.artifact_feasibility.checkpoint_count, 1);
    assert_eq!(
        stats.artifact_feasibility.checkpoints[0]
            .expect("final artifact checkpoint")
            .primitive_end,
        1
    );
    assert_eq!(
        artifact.disposition,
        super::scene::ArtifactFeasibilityDisposition::ContiguousCandidate
    );
}

fn artifact_fill(widget_id: u64, color: Rgba8) -> PaintPrimitive {
    PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0)),
        color,
    })
}

fn artifact_anchor(key: u64) -> PaintPrimitive {
    PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: key,
        key,
        revision: 1,
        rect: Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0)),
        content: GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(crate::runtime::GpuShaderSurfaceDescriptor::new("test")),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    })
}

fn artifact_counts(scene: &Scene) -> super::scene::ArtifactFeasibilityCounts {
    let encoding = scene.encoding();
    super::scene::ArtifactFeasibilityCounts {
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

fn typed_artifact_fixture(
    segment_count: usize,
) -> (
    Scene,
    super::scene::ArtifactFeasibilityObservation,
    super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
    Vec<Scene>,
) {
    assert!((1..=crate::runtime::MAX_PAINT_SEGMENTS).contains(&segment_count));
    let mut primitives = Vec::with_capacity(segment_count * 2 - 1);
    for index in 0..segment_count {
        primitives.push(artifact_fill(
            index as u64 + 1,
            Rgba8::new((index * 53) as u8, (255 - index * 37) as u8, 64, 255),
        ));
        if index + 1 < segment_count {
            primitives.push(artifact_anchor(index as u64 + 10));
        }
    }
    let source_plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives,
    };

    let mut source_scene = Scene::new();
    let mut source_text_renderer = NativeTextRenderer::new();
    let mut source_bridge = demo_bridge();
    let mut source_cache = RetainedSurfaceFrameCache::default();
    let mut source_text_runs = SceneTextRunBuffer::new();
    let source_stats = encode_plan(
        &source_plan,
        &mut source_scene,
        &mut source_text_renderer,
        &mut source_bridge,
        Vector2::new(320.0, 180.0),
        &mut source_cache,
        &mut source_text_runs,
    );
    assert_eq!(
        usize::from(source_stats.segment_encoding.segment_count),
        segment_count
    );

    let mut payloads = Vec::with_capacity(segment_count);
    let mut authoritative = Scene::new();
    let mut feasibility = super::scene::ArtifactFeasibilityObservation {
        segments: [None; crate::runtime::MAX_PAINT_SEGMENTS],
        checkpoints: [None; crate::runtime::MAX_PAINT_SEGMENTS],
        segment_count: segment_count as u8,
        checkpoint_count: segment_count as u8,
        conservative: false,
    };
    let mut paint = PaintSegmentObservation::empty();
    paint.segment_count = segment_count as u8;
    let mut encoding = super::scene::PaintSegmentEncodingObservation {
        segment_count: segment_count as u8,
        ..Default::default()
    };

    for index in 0..segment_count {
        let segment = source_stats.segment_encoding.segments[index].expect("segment");
        let segment_plan = SurfacePaintPlan {
            clear_color: source_plan.clear_color,
            primitives: source_plan.primitives
                [segment.primitive_start as usize..segment.primitive_end as usize]
                .to_vec(),
        };
        let mut payload = Scene::new();
        let mut text_renderer = NativeTextRenderer::new();
        let mut bridge = demo_bridge();
        let mut cache = RetainedSurfaceFrameCache::default();
        let mut text_runs = SceneTextRunBuffer::new();
        encode_plan(
            &segment_plan,
            &mut payload,
            &mut text_renderer,
            &mut bridge,
            Vector2::new(320.0, 180.0),
            &mut cache,
            &mut text_runs,
        );
        assert!(
            payload.encoding().flags == 0
                && payload.encoding().n_open_clips == 0
                && payload.encoding().resources.patches.is_empty()
                && payload.encoding().resources.color_stops.is_empty()
                && payload.encoding().resources.glyphs.is_empty()
                && payload.encoding().resources.glyph_runs.is_empty()
                && payload.encoding().resources.normalized_coords.is_empty()
        );
        authoritative.append(&payload, None);
        payloads.push(payload);

        feasibility.segments[index] = Some(super::scene::ArtifactFeasibilitySegment {
            identity: segment.identity,
            primitive_start: segment.primitive_start,
            primitive_end: segment.primitive_end,
            disposition: super::scene::ArtifactFeasibilityDisposition::ContiguousCandidate,
        });
        feasibility.checkpoints[index] = Some(super::scene::ArtifactFeasibilityCheckpoint {
            primitive_end: segment.primitive_end,
            counts: artifact_counts(&authoritative),
        });
        paint.segments[index] = Some(PaintSegment {
            identity: segment.identity,
            owner: None,
            revision: 1,
            implicated: false,
        });
        encoding.segments[index] = Some(super::scene::PaintSegmentEncoding {
            identity: segment.identity,
            primitive_start: segment.primitive_start,
            primitive_end: segment.primitive_end,
            safe_enclosure: super::scene::SafeEnclosure::Empty,
            isolation: super::scene::EncodingIsolation::SelfContained,
            conservative: false,
            reason: super::scene::EncodingConservativeReason::None,
        });
    }

    let plan = classify_collector_evidence_parts(paint, encoding, feasibility);
    (authoritative, feasibility, plan, payloads)
}

fn materialize_fixture(
    fixture: (
        Scene,
        super::scene::ArtifactFeasibilityObservation,
        super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
        Vec<Scene>,
    ),
) -> super::scene::NativePaintSegmentArtifactMaterialization {
    let (scene, feasibility, plan, payloads) = fixture;
    super::scene::materialize_native_paint_segment_artifacts(
        &scene,
        feasibility,
        plan,
        payloads,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
    )
}

fn assert_encoding_equal(actual: &Scene, expected: &Scene) {
    let actual = actual.encoding();
    let expected = expected.encoding();
    assert!(actual.path_tags == expected.path_tags);
    assert_eq!(actual.path_data, expected.path_data);
    assert!(actual.draw_tags == expected.draw_tags);
    assert_eq!(actual.draw_data, expected.draw_data);
    assert_eq!(actual.transforms, expected.transforms);
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.n_paths, expected.n_paths);
    assert_eq!(actual.n_path_segments, expected.n_path_segments);
    assert_eq!(actual.n_clips, expected.n_clips);
    assert_eq!(actual.n_open_clips, expected.n_open_clips);
    assert_eq!(actual.flags, expected.flags);
    assert!(actual.resources.patches.is_empty());
    assert!(actual.resources.color_stops.is_empty());
    assert!(actual.resources.glyphs.is_empty());
    assert!(actual.resources.glyph_runs.is_empty());
    assert!(actual.resources.normalized_coords.is_empty());
    assert!(expected.resources.patches.is_empty());
    assert!(expected.resources.color_stops.is_empty());
    assert!(expected.resources.glyphs.is_empty());
    assert!(expected.resources.glyph_runs.is_empty());
    assert!(expected.resources.normalized_coords.is_empty());
}

#[test]
fn scene_artifact_materialization_accepts_one_typed_candidate_with_full_equality() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let expected_payload = payloads[0].clone();
    let expected_entry = plan.entries[0].expect("entry");
    let materialized = super::scene::materialize_native_paint_segment_artifacts(
        &scene,
        feasibility,
        plan,
        payloads,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
    );

    assert_eq!(materialized.len(), 1);
    assert_encoding_equal(&materialized.artifacts[0].scene, &scene);
    assert_encoding_equal(&materialized.artifacts[0].scene, &expected_payload);
    assert_eq!(
        materialized.artifacts[0].identity,
        expected_entry.span.identity
    );
    assert_eq!(materialized.artifacts[0].span, expected_entry.span);
    assert_eq!(materialized.artifacts[0].revision, 1);
    assert_eq!(
        materialized.artifacts[0].target_generation,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1)
    );
}

#[test]
fn scene_artifact_materialization_accepts_mixed_retained_and_fresh_order() {
    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(3);
    plan.entries[1]
        .as_mut()
        .expect("middle entry")
        .disposition = super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
        super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::RevisionChanged,
    );
    let expected = [payloads[0].clone(), payloads[2].clone()];
    let materialized = super::scene::materialize_native_paint_segment_artifacts(
        &scene,
        feasibility,
        plan,
        payloads,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
    );

    assert_eq!(materialized.len(), 2);
    for (artifact, expected) in materialized.artifacts.iter().zip(expected.iter()) {
        assert_encoding_equal(&artifact.scene, expected);
    }
}

#[test]
fn scene_artifact_materialization_rejects_missing_and_extra_payloads_atomically() {
    let (_, feasibility, plan, mut payloads) = typed_artifact_fixture(2);
    payloads.pop();
    assert!(
        super::scene::materialize_native_paint_segment_artifacts(
            &Scene::new(),
            feasibility,
            plan,
            payloads,
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        )
        .is_empty()
    );

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(2);
    payloads.push(Scene::new());
    assert!(
        super::scene::materialize_native_paint_segment_artifacts(
            &scene,
            feasibility,
            plan,
            payloads,
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        )
        .is_empty()
    );
}

#[test]
fn scene_artifact_materialization_rejects_identity_span_generation_and_revision_mismatch() {
    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(2);
    let second_identity = plan.entries[1].expect("second entry").span.identity;
    plan.entries[0].as_mut().expect("first entry").span.identity = second_identity;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, mut feasibility, plan, payloads) = typed_artifact_fixture(2);
    feasibility.segments[0]
        .as_mut()
        .expect("first evidence")
        .identity = second_identity;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(2);
    plan.entries[0].as_mut().expect("first entry").span.start += 1;
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(1);
    if let super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(
        fingerprint,
    ) = &mut plan.entries[0].as_mut().expect("entry").disposition
    {
        fingerprint.target_generation =
            super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
        assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());
    } else {
        panic!("fixture must provide a retained candidate");
    }

    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(1);
    if let super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(
        fingerprint,
    ) = &mut plan.entries[0].as_mut().expect("entry").disposition
    {
        fingerprint.revision = 0;
        assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
    } else {
        panic!("fixture must provide a retained candidate");
    }
}

#[test]
fn scene_artifact_materialization_rejects_unsafe_and_fallback_evidence() {
    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(1);
    if let super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(
        fingerprint,
    ) = &mut plan.entries[0].as_mut().expect("entry").disposition
    {
        fingerprint.isolation = super::scene::EncodingIsolation::InheritedClip;
        assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());
    } else {
        panic!("fixture must provide a retained candidate");
    }

    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(1);
    plan.outcome = super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::FullSceneFallback(
        super::super::retained_paint_segments::NativePaintSegmentFallbackReason::TrailingEvidence,
    );
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn scene_artifact_materialization_rejects_duplicate_identity() {
    let (scene, mut feasibility, mut plan, payloads) = typed_artifact_fixture(3);
    let first_identity = plan.entries[0].expect("first entry").span.identity;
    plan.entries[2].as_mut().expect("third entry").span.identity = first_identity;
    feasibility.segments[2]
        .as_mut()
        .expect("third evidence")
        .identity = first_identity;
    let third_span = plan.entries[2].expect("third entry").span;
    if let super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(
        fingerprint,
    ) = &mut plan.entries[2].as_mut().expect("third entry").disposition
    {
        fingerprint.identity = first_identity;
        fingerprint.primitive_start = third_span.start;
        fingerprint.primitive_end = third_span.end;
    }
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn scene_artifact_materialization_rejects_payload_mutation_and_encoding_mismatch() {
    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0].encoding_mut().path_data[0] ^= 1;
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn scene_artifact_materialization_rejects_nonzero_flags_open_clips_and_resources() {
    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0].encoding_mut().flags = 1;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0].encoding_mut().n_open_clips = 1;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0]
        .encoding_mut()
        .resources
        .normalized_coords
        .push(0);
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn scene_artifact_materialization_rejects_corrupt_scalar_checkpoint_without_slicing() {
    let (scene, mut feasibility, plan, payloads) = typed_artifact_fixture(1);
    feasibility.checkpoints[0]
        .as_mut()
        .expect("checkpoint")
        .counts
        .draw_data += 1;
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn artifact_feasibility_rejects_later_fill_that_inherits_style_or_transform() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let rect = Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0));
    let fill = || {
        PaintPrimitive::FillRect(PaintFillRect {
            widget_id: 7,
            rect,
            color: Rgba8::new(255, 255, 255, 255),
        })
    };
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            fill(),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 42,
                key: 42,
                revision: 1,
                rect,
                content: GpuSurfaceContent::CustomShader {
                    descriptor: Arc::new(crate::runtime::GpuShaderSurfaceDescriptor::new("test")),
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            fill(),
        ],
    };
    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );
    assert_eq!(stats.artifact_feasibility.segment_count, 2);
    assert_eq!(
        stats.artifact_feasibility.segments[0]
            .expect("first artifact evidence")
            .disposition,
        super::scene::ArtifactFeasibilityDisposition::ContiguousCandidate
    );
    assert_eq!(
        stats.artifact_feasibility.segments[1]
            .expect("second artifact evidence")
            .disposition,
        super::scene::ArtifactFeasibilityDisposition::RequiresFreshEncoding(
            super::scene::ArtifactFeasibilityReason::CrossSegmentTransformOrStyle
        )
    );
}

#[test]
fn scene_encoding_widens_unclipped_text_and_tracks_clip_isolation() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let clip = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let text = PaintPrimitive::Text(PaintTextRun {
        widget_id: 9,
        text: "evidence".into(),
        rect: Rect::from_min_size(Point::new(2.0, 2.0), Vector2::new(20.0, 12.0)),
        font_size: 12.0,
        color: Rgba8::new(255, 255, 255, 255),
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
        baseline: None,
    });
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            text.clone(),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 42,
                key: 42,
                revision: 1,
                rect: clip,
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: clip,
                    atlas: Arc::new(
                        ImageRgba::new(40, 20, vec![255; 40 * 20 * 4]).expect("valid image"),
                    ),
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 1,
                rect: clip,
            }),
            text,
            PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 1 }),
        ],
    };

    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );
    let first = stats.segment_encoding.segments[0].expect("first segment");
    assert_eq!(
        first.safe_enclosure,
        super::scene::SafeEnclosure::ViewportFallback
    );
    assert_eq!(
        first.reason,
        super::scene::EncodingConservativeReason::UncertainPrimitive
    );
    let second = stats.segment_encoding.segments[1].expect("second segment");
    assert_eq!(
        second.safe_enclosure,
        super::scene::SafeEnclosure::Bounded(clip)
    );
    assert_eq!(
        second.isolation,
        super::scene::EncodingIsolation::SelfContained
    );
    let artifact = stats.artifact_feasibility.segments[0].expect("artifact evidence");
    assert!(matches!(
        artifact.disposition,
        super::scene::ArtifactFeasibilityDisposition::RequiresFreshEncoding(_)
    ));
    assert!(stats.artifact_feasibility.conservative);
}

#[test]
fn scene_encoding_marks_clip_closed_after_gpu_anchor_as_inherited() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let clip = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![
            PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 1,
                rect: clip,
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 42,
                key: 42,
                revision: 1,
                rect: clip,
                content: GpuSurfaceContent::CustomShader {
                    descriptor: Arc::new(crate::runtime::GpuShaderSurfaceDescriptor::new("test")),
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 1 }),
            PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 7,
                rect: Rect::from_min_size(Point::new(2.0, 2.0), Vector2::new(8.0, 8.0)),
                color: Rgba8::new(255, 255, 255, 255),
            }),
        ],
    };

    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        &mut bridge,
        Vector2::new(320.0, 180.0),
        &mut retained_cache,
        &mut text_runs,
    );
    let first = stats.segment_encoding.segments[0].expect("first segment");
    assert_eq!(first.isolation, super::scene::EncodingIsolation::OpenClip);
    let second = stats.segment_encoding.segments[1].expect("second segment");
    assert_eq!(
        second.isolation,
        super::scene::EncodingIsolation::InheritedClip
    );
    assert!(second.conservative);
}

fn encode_plan<Bridge, Message>(
    plan: &crate::runtime::SurfacePaintPlan,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
    retained_cache: &mut RetainedSurfaceFrameCache,
    text_runs: &mut SceneTextRunBuffer,
) -> RetainedSurfaceEncodeStats
where
    Bridge: RuntimeBridge<Message>,
{
    let retained_surface = bridge.host_capabilities().retained_surface;
    encode_surface_paint_plan_to_scene(
        plan,
        SurfaceSceneEncodeContext {
            scene,
            text_renderer,
            bridge,
            retained_surface,
            viewport,
            retained_cache,
            text_runs,
            animation_time: Duration::ZERO,
        },
    )
}
