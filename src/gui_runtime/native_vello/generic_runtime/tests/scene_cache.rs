use super::*;
use crate::{
    gui::types::ImageRgba,
    runtime::{
        GpuSurfaceRuntimeOverlays, PaintClipEnd, PaintClipStart, PaintFillRect, PaintSegment,
        PaintSegmentObservation, PaintTextAlign, PaintTextRun, RetainedSurfaceCachePolicy,
        SurfacePaintPlan,
    },
    theme::ThemeTokens,
    widgets::{TextWrap, WidgetRevision},
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

fn paint_for_plan(
    plan: super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
) -> PaintSegmentObservation {
    let mut paint = PaintSegmentObservation::empty();
    paint.segment_count = plan.entry_count;
    for index in 0..usize::from(plan.entry_count) {
        let Some(entry) = plan.entries[index] else {
            continue;
        };
        let revision = match entry.disposition {
            super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(
                fingerprint,
            ) => fingerprint.revision,
            super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                _,
            ) => 1,
        };
        paint.segments[index] = Some(PaintSegment {
            identity: entry.span.identity,
            owner: None,
            revision,
            implicated: false,
        });
    }
    paint
}

fn test_scene_validity() -> super::super::frame_state::NativeSceneValidityFingerprint {
    let runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 180.0),
    );
    runner.frame.native_scene_validity_fingerprint(
        runner.core.base_paint_plan_context(),
        runner.core.resolved_appearance(),
        runner.window.dpi_scale,
    )
}

fn test_scene_validity_with_dpi(
    dpi_scale: f64,
) -> super::super::frame_state::NativeSceneValidityFingerprint {
    let runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 180.0),
    );
    runner.frame.native_scene_validity_fingerprint(
        runner.core.base_paint_plan_context(),
        runner.core.resolved_appearance(),
        crate::theme::DpiScale::new(dpi_scale),
    )
}

#[derive(Clone, Debug)]
struct RetainedSegmentWidget {
    common: WidgetCommon,
}

impl RetainedSegmentWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(201, WidgetSizing::fixed(Vector2::new(320.0, 80.0)));
        common.paint.bounds = crate::widgets::PaintBounds::AllowOverflow;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for RetainedSegmentWidget {
    fn revision(&self) -> WidgetRevision {
        WidgetRevision::exact(
            "retained-segment",
            (320_u32, 80_u32),
            "green-fill-signal-bands",
            (),
        )
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8::new(0, 255, 64, 255),
        }));
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: 901,
            revision: 1,
            rect: bounds,
            content: GpuSurfaceContent::SignalBands {
                frames: 1,
                band_count: 1,
                frame_range: [0.0, 1.0],
                samples: Arc::<[f32]>::from(vec![0.0]),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        }));
    }
}

#[derive(Default)]
struct RetainedSegmentBridge;

impl RuntimeBridge<()> for RetainedSegmentBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            RetainedSegmentWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }
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

fn typed_artifact_source_plan(segment_count: usize) -> SurfacePaintPlan {
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
    SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives,
    }
}

fn typed_artifact_fixture(
    segment_count: usize,
) -> (
    Scene,
    super::scene::ArtifactFeasibilityObservation,
    super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
    Vec<super::scene::NativePaintSegmentPayload>,
) {
    let source_plan = typed_artifact_source_plan(segment_count);

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
    let mut feasibility = source_stats.artifact_feasibility;
    let mut paint = PaintSegmentObservation::empty();
    paint.segment_count = segment_count as u8;
    let encoding = source_stats.segment_encoding;

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
        payloads.push(payload);

        feasibility.segments[index]
            .as_mut()
            .expect("source feasibility segment")
            .disposition = super::scene::ArtifactFeasibilityDisposition::ContiguousCandidate;
        paint.segments[index] = Some(PaintSegment {
            identity: segment.identity,
            owner: None,
            revision: 1,
            implicated: false,
        });
    }

    let plan = classify_collector_evidence_parts(paint, encoding, feasibility);
    let empty_store = super::scene::NativePaintSegmentArtifactStore::default();
    let production_payloads = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint,
        plan,
        test_scene_validity(),
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &empty_store,
    )
    .into_parts()
    .0;
    assert_eq!(production_payloads.len(), payloads.len());
    for (production, fixture) in production_payloads.iter().zip(payloads.iter()) {
        assert_encoding_equal(production.scene_for_test(), fixture);
    }
    (source_scene, feasibility, plan, production_payloads)
}

fn materialize_fixture(
    fixture: (
        Scene,
        super::scene::ArtifactFeasibilityObservation,
        super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
        Vec<super::scene::NativePaintSegmentPayload>,
    ),
) -> super::scene::NativePaintSegmentArtifactMaterialization {
    materialize_fixture_with_validity(fixture, test_scene_validity())
}

fn materialize_fixture_with_validity(
    fixture: (
        Scene,
        super::scene::ArtifactFeasibilityObservation,
        super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
        Vec<super::scene::NativePaintSegmentPayload>,
    ),
    scene_validity: super::super::frame_state::NativeSceneValidityFingerprint,
) -> super::scene::NativePaintSegmentArtifactMaterialization {
    let (scene, feasibility, plan, payloads) = fixture;
    super::runner::materialize_native_paint_segment_artifacts_for_test(
        &scene,
        feasibility,
        plan,
        payloads,
        scene_validity,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
    )
}

fn seed_publication_admission_for_test(
    admission: &mut super::super::retained_paint_segments::NativePaintSegmentCacheAdmission,
    materialization: &super::scene::NativePaintSegmentArtifactMaterialization,
) {
    for artifact in materialization.artifacts_for_test() {
        admission.add_warming_for_test(
            artifact.identity_for_test(),
            artifact.span_for_test(),
            artifact.revision_for_test(),
            artifact.target_generation_for_test(),
        );
    }
}

fn seed_frame_publication_admission_for_test(
    frame: &mut NativeVelloFrameState,
    materialization: &super::scene::NativePaintSegmentArtifactMaterialization,
) {
    seed_publication_admission_for_test(
        &mut frame.native_paint_segment_cache_admission,
        materialization,
    );
}

fn artifact_store_for_fixture(
    scene: &Scene,
    feasibility: super::scene::ArtifactFeasibilityObservation,
    plan: super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
    payloads: &[super::scene::NativePaintSegmentPayload],
) -> super::scene::NativePaintSegmentArtifactStore {
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialize_fixture((
        scene.clone(),
        feasibility,
        plan,
        payloads.to_vec(),
    )));
    store
}

fn frame_for_mixed_fixture(
    source_plan: &SurfacePaintPlan,
    previous_scene: &Scene,
    feasibility: super::scene::ArtifactFeasibilityObservation,
    plan: super::super::retained_paint_segments::NativePaintSegmentEligibilityPlan,
    store: super::scene::NativePaintSegmentArtifactStore,
) -> NativeVelloFrameState {
    let mut previous_stats_scene = Scene::new();
    let mut previous_text_renderer = NativeTextRenderer::new();
    let mut previous_bridge = demo_bridge();
    let mut previous_cache = RetainedSurfaceFrameCache::default();
    let mut previous_text_runs = SceneTextRunBuffer::new();
    let mut previous_stats = encode_plan(
        source_plan,
        &mut previous_stats_scene,
        &mut previous_text_renderer,
        &mut previous_bridge,
        Vector2::new(320.0, 180.0),
        &mut previous_cache,
        &mut previous_text_runs,
    );
    assert_encoding_equal(&previous_stats_scene, previous_scene);
    previous_stats.artifact_feasibility = feasibility;

    let scene_validity = test_scene_validity();
    let mut frame = NativeVelloFrameState::new(
        NativeTextRenderer::new(),
        RetainedSurfaceCachePolicy::default(),
    );
    frame.scene = previous_scene.clone();
    frame.last_paint_plan = source_plan.clone();
    frame.last_scene_stats = previous_stats;
    frame.last_native_paint_segment_eligibility = plan;
    frame.native_paint_segment_artifact_store = store;
    frame.record_scene_encode(scene_validity);
    frame
}

fn assert_encoding_equal(actual: &Scene, expected: &Scene) {
    let actual = actual.encoding();
    let expected = expected.encoding();
    assert_eq!(
        actual.path_tags.iter().map(|tag| tag.0).collect::<Vec<_>>(),
        expected
            .path_tags
            .iter()
            .map(|tag| tag.0)
            .collect::<Vec<_>>(),
    );
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
fn retained_scene_assembly_matches_authoritative_encoding_in_plan_order() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let scene_validity = test_scene_validity();
    let store = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    let result = super::scene::assemble_retained_native_paint_segment_scene(
        &authoritative,
        feasibility,
        plan,
        &store,
        scene_validity,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
    );
    let super::scene::NativePaintSegmentAssemblyResult::Assembled(assembled) = result else {
        panic!("exact retained plan should assemble");
    };
    assert_encoding_equal(&assembled, &authoritative);
}

#[test]
fn retained_scene_assembly_vetoes_mixed_and_missing_artifacts() {
    let (authoritative, feasibility, mut mixed_plan, payloads) = typed_artifact_fixture(3);
    mixed_plan.entries[1]
        .as_mut()
        .expect("middle entry")
        .disposition = super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
        super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::RevisionChanged,
    );
    let mixed_store =
        artifact_store_for_fixture(&authoritative, feasibility, mixed_plan, &payloads);
    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &authoritative,
            feasibility,
            mixed_plan,
            &mixed_store,
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(
            super::scene::NativePaintSegmentAssemblyVetoReason::MixedDisposition
        )
    ));

    let (authoritative, feasibility, plan, _) = typed_artifact_fixture(1);
    let empty_store = super::scene::NativePaintSegmentArtifactStore::default();
    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &authoritative,
            feasibility,
            plan,
            &empty_store,
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(
            super::scene::NativePaintSegmentAssemblyVetoReason::MissingArtifact
        )
    ));
}

#[test]
fn retained_scene_assembly_and_lookup_veto_context_provenance_mismatch() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let scene_validity = test_scene_validity();
    let store = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    let source_plan = typed_artifact_source_plan(1);
    let selection = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint_for_plan(plan),
        plan,
        test_scene_validity_with_dpi(2.0),
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &store,
    );
    assert_eq!(selection.reused_count_for_test(), 0);
    assert_eq!(selection.fresh_count_for_test(), 1);

    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &authoritative,
            feasibility,
            plan,
            &store,
            test_scene_validity_with_dpi(2.0),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(
            super::scene::NativePaintSegmentAssemblyVetoReason::ArtifactMetadataMismatch
        )
    ));
    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &authoritative,
            feasibility,
            plan,
            &store,
            scene_validity,
            super::super::runner_state::NativeTargetGeneration::from_test_serial(2),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(_)
    ));
}

#[test]
fn late_retained_assembly_veto_leaves_frame_scene_untouched() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let scene_validity = test_scene_validity();
    let mut frame = NativeVelloFrameState::new(
        NativeTextRenderer::new(),
        RetainedSurfaceCachePolicy::default(),
    );
    frame.scene = authoritative.clone();
    frame.last_scene_stats.artifact_feasibility = feasibility;
    frame.last_native_paint_segment_eligibility = plan;
    let materialization = materialize_fixture_with_validity(
        (authoritative.clone(), feasibility, plan, payloads),
        scene_validity,
    );
    seed_frame_publication_admission_for_test(&mut frame, &materialization);
    frame.reconcile_native_paint_segment_artifacts(materialization);
    frame
        .native_paint_segment_artifact_store
        .artifact_for_test_mut(1)
        .expect("middle artifact")
        .scene_for_test_mut()
        .encoding_mut()
        .path_data[0] ^= 1;
    let previous_scene = frame.scene.clone();

    assert!(matches!(
        frame.assemble_retained_native_scene(
            scene_validity,
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        Err(super::scene::NativePaintSegmentAssemblyVetoReason::InvalidPayload)
    ));
    assert_encoding_equal(&frame.scene, &previous_scene);
}

#[test]
fn runner_warms_artifacts_before_admission_aware_retained_assembly() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RetainedSegmentBridge,
        Vector2::new(320.0, 180.0),
    );
    assert!(runner.window.target_generation.advance());

    // Startup view-delta evidence is conservatively unavailable. Establish a
    // classified unchanged projection before the initial encode so the next
    // rebuild can exercise the retained-plan/artifact warm-up fence.
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_assembly_count, 0);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()
            .iter()
            .all(Option::is_none)
    );

    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_assembly_count, 0);
    assert_eq!(runner.frame.scene_assembly_veto_count, 0);
    assert_eq!(
        runner.frame.scene_build_outcome,
        super::super::frame_state::NativeSceneBuildOutcome::FullEncode
    );
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()
            .iter()
            .any(Option::is_some)
    );
    let resident_index = runner
        .frame
        .native_paint_segment_artifact_store
        .snapshot_identities()
        .iter()
        .position(Option::is_some);
    assert!(
        resident_index.is_some(),
        "a resident slot should be removable"
    );
    if let Some(index) = resident_index {
        assert!(
            runner
                .frame
                .native_paint_segment_artifact_store
                .clear_artifact_for_test(index)
        );
    }
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .resident_count_for_test()
            < runner
                .frame
                .native_paint_segment_artifact_store
                .plan_entry_count_for_test()
    );
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 3);
    assert_eq!(runner.frame.scene_assembly_count, 0);
    assert_eq!(runner.frame.scene_assembly_veto_count, 0);
    assert_eq!(
        runner.frame.scene_build_outcome,
        super::super::frame_state::NativeSceneBuildOutcome::FullEncode
    );

    let segment_identity = runner.frame.last_native_paint_segment_eligibility.entries[0]
        .expect("segment eligibility")
        .span
        .identity;

    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 3);
    assert_eq!(runner.frame.scene_assembly_count, 1);
    assert_eq!(runner.frame.scene_assembly_reused_count, 1);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .resident_count_for_test()
            > 0
    );
    assert_eq!(
        runner.frame.scene_build_outcome,
        super::super::frame_state::NativeSceneBuildOutcome::RetainedAssembly
    );
    assert!(
        !runner
            .frame
            .native_paint_segment_cache_admission
            .admitted_for_test(segment_identity)
    );

    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 3);
    assert_eq!(runner.frame.scene_assembly_count, 2);
    assert_eq!(
        runner.frame.scene_build_outcome,
        super::super::frame_state::NativeSceneBuildOutcome::RetainedAssembly
    );
    assert!(
        runner
            .frame
            .native_paint_segment_cache_admission
            .admitted_for_test(segment_identity)
    );
}

#[test]
fn render_selection_intersects_admission_and_sparse_residency() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let scene_validity = test_scene_validity();
    let target_generation = super::super::runner_state::NativeTargetGeneration::from_test_serial(1);
    let materialization =
        materialize_fixture((authoritative.clone(), feasibility, plan, payloads.clone()));
    let mut admission =
        super::super::retained_paint_segments::NativePaintSegmentCacheAdmission::default();
    seed_publication_admission_for_test(&mut admission, &materialization);
    let mut store = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);

    let dense = super::super::retained_paint_segments::select_native_paint_segment_render_boundary(
        plan,
        &admission,
        &store,
        scene_validity,
        Some(scene_validity),
        target_generation,
    );
    assert!(dense.should_attempt_mixed_assembly());
    assert!(dense
        .full_encode_plan()
        .entries[..3]
        .iter()
        .all(|entry| matches!(
            entry,
            Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
                ..
            })
        )));

    assert!(store.clear_artifact_for_test(1));
    let sparse = super::super::retained_paint_segments::select_native_paint_segment_render_boundary(
        plan,
        &admission,
        &store,
        scene_validity,
        Some(scene_validity),
        target_generation,
    );
    assert!(sparse.should_attempt_mixed_assembly());
    assert!(matches!(
        sparse.full_encode_plan().entries[1],
        Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
            disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::NoResident,
            ),
            ..
        })
    ));
    assert!(matches!(
        sparse.full_encode_plan().entries[0],
        Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
            disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
            ..
        })
    ));
    assert!(matches!(
        sparse.full_encode_plan().entries[2],
        Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
            disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
            ..
        })
    ));

    let mut corrupted = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    let corrupted_before = {
        let artifact = corrupted
            .artifact_for_test_mut(0)
            .expect("first resident artifact");
        artifact.set_revision_for_test(0);
        artifact.revision_for_test()
    };
    let corrupted_selection =
        super::super::retained_paint_segments::select_native_paint_segment_render_boundary(
            plan,
            &admission,
            &corrupted,
            scene_validity,
            Some(scene_validity),
            target_generation,
        );
    assert!(!corrupted_selection.should_attempt_mixed_assembly());
    assert!(corrupted_selection
        .full_encode_plan()
        .entries[..3]
        .iter()
        .all(|entry| matches!(
            entry,
            Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::RenderSelectionFallback,
                ),
                ..
            })
        )));
    assert_eq!(
        corrupted
            .artifact_for_test_mut(0)
            .expect("first resident artifact")
            .revision_for_test(),
        corrupted_before
    );

    let without_admission =
        super::super::retained_paint_segments::select_native_paint_segment_render_boundary(
            plan,
            &super::super::retained_paint_segments::NativePaintSegmentCacheAdmission::default(),
            &artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads),
            scene_validity,
            Some(scene_validity),
            target_generation,
        );
    assert!(!without_admission.should_attempt_mixed_assembly());
    assert!(without_admission
        .full_encode_plan()
        .entries[..3]
        .iter()
        .all(|entry| matches!(
            entry,
            Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::NotAdmitted,
                ),
                ..
            })
        )));

    let mut no_residents = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    for index in 0..crate::runtime::MAX_PAINT_SEGMENTS {
        no_residents.clear_artifact_for_test(index);
    }
    let admission_without_residency =
        super::super::retained_paint_segments::select_native_paint_segment_render_boundary(
            plan,
            &admission,
            &no_residents,
            scene_validity,
            Some(scene_validity),
            target_generation,
        );
    assert!(!admission_without_residency.should_attempt_mixed_assembly());
    assert!(admission_without_residency
        .full_encode_plan()
        .entries[..3]
        .iter()
        .all(|entry| matches!(
            entry,
            Some(super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::NoResident,
                ),
                ..
            })
        )));
}

#[test]
fn mixed_assembly_commits_changed_resource_free_segment_and_matches_current_oracle() {
    let (previous_scene, feasibility, retained_plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let mut previous_stats_scene = Scene::new();
    let mut previous_text_renderer = NativeTextRenderer::new();
    let mut previous_bridge = demo_bridge();
    let mut previous_cache = RetainedSurfaceFrameCache::default();
    let mut previous_text_runs = SceneTextRunBuffer::new();
    let mut previous_stats = encode_plan(
        &source_plan,
        &mut previous_stats_scene,
        &mut previous_text_renderer,
        &mut previous_bridge,
        Vector2::new(320.0, 180.0),
        &mut previous_cache,
        &mut previous_text_runs,
    );
    assert_encoding_equal(&previous_stats_scene, &previous_scene);
    previous_stats.artifact_feasibility = feasibility;

    let mut current_plan = source_plan.clone();
    let PaintPrimitive::FillRect(fill) = &mut current_plan.primitives[2] else {
        panic!("middle segment should be a fill rectangle");
    };
    fill.color = Rgba8::new(255, 32, 128, 255);

    let mut mixed_plan = retained_plan;
    mixed_plan.entries[1]
        .as_mut()
        .expect("middle entry")
        .disposition = super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
        super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::RevisionChanged,
    );
    let mut current_paint = paint_for_plan(mixed_plan);
    current_paint.segments[1]
        .as_mut()
        .expect("middle paint segment")
        .revision = 2;

    let scene_validity = test_scene_validity();
    let mut frame = NativeVelloFrameState::new(
        NativeTextRenderer::new(),
        RetainedSurfaceCachePolicy::default(),
    );
    frame.scene = previous_scene.clone();
    frame.last_paint_plan = current_plan.clone();
    frame.last_scene_stats = previous_stats;
    frame.last_native_paint_segment_eligibility = mixed_plan;
    frame.native_paint_segment_artifact_store =
        artifact_store_for_fixture(&previous_scene, feasibility, retained_plan, &payloads);
    frame.record_scene_encode(scene_validity);

    let target_generation = super::super::runner_state::NativeTargetGeneration::from_test_serial(1);
    let bundle = frame
        .assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            current_paint,
            scene_validity,
            target_generation,
            mixed_plan,
        )
        .expect("supported changed segment should assemble");
    assert_eq!(bundle.fresh_count, 1);
    assert_eq!(bundle.reused_count, 2);
    assert_eq!(bundle.append_count, 3);
    frame
        .commit_native_scene_assembly(bundle, scene_validity)
        .expect("validated mixed bundle should commit");

    let mut expected_scene = Scene::new();
    let mut expected_text_renderer = NativeTextRenderer::new();
    let mut expected_bridge = demo_bridge();
    let mut expected_cache = RetainedSurfaceFrameCache::default();
    let mut expected_text_runs = SceneTextRunBuffer::new();
    encode_plan(
        &current_plan,
        &mut expected_scene,
        &mut expected_text_renderer,
        &mut expected_bridge,
        Vector2::new(320.0, 180.0),
        &mut expected_cache,
        &mut expected_text_runs,
    );
    assert_encoding_equal(&frame.scene, &expected_scene);
    assert_eq!(frame.scene_assembly_count, 1);
    assert_eq!(frame.scene_mixed_assembly_count, 1);
    assert_eq!(frame.scene_assembly_fresh_count, 1);
    assert_eq!(frame.scene_assembly_reused_count, 2);
    assert_eq!(frame.scene_assembly_append_count, 3);
    assert_eq!(
        frame.scene_build_outcome,
        super::super::frame_state::NativeSceneBuildOutcome::MixedRetainedAssembly
    );
}

#[test]
fn mixed_assembly_fresh_encodes_sparse_hole_and_commits_execution_plan() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let mut materialization =
        materialize_fixture((authoritative.clone(), feasibility, plan, payloads));
    assert!(materialization.remove_artifact_for_test(1));
    let store = {
        let mut store = super::scene::NativePaintSegmentArtifactStore::default();
        store.reconcile(materialization);
        store
    };
    let mut frame = frame_for_mixed_fixture(&source_plan, &authoritative, feasibility, plan, store);
    let scene_validity = test_scene_validity();
    let target_generation = super::super::runner_state::NativeTargetGeneration::from_test_serial(1);

    let paint = paint_for_plan(plan);
    let bundle = frame
        .assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            paint,
            scene_validity,
            target_generation,
            plan,
        )
        .expect("sparse hole should be a supported fresh span");
    assert_eq!(bundle.fresh_count, 1);
    assert_eq!(bundle.reused_count, 2);
    assert_eq!(bundle.append_count, 3);
    assert!(matches!(
        bundle.plan.entries[0],
        Some(
            super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
                ..
            }
        )
    ));
    assert!(matches!(
        bundle.plan.entries[1],
        Some(
            super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
                    super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::NoArtifact,
                ),
                ..
            }
        )
    ));
    assert!(matches!(
        bundle.plan.entries[2],
        Some(
            super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
                ..
            }
        )
    ));

    frame
        .commit_native_scene_assembly(bundle, scene_validity)
        .expect("validated sparse mixed bundle should commit");
    assert_encoding_equal(&frame.scene, &authoritative);
    assert!(
        frame
            .native_paint_segment_benefit_ledger
            .available_for_test()
    );
    assert!(
        frame
            .native_paint_segment_cache_admission
            .has_entries_for_test()
    );
    let evidence = frame
        .native_paint_segment_benefit_ledger
        .latest_frame_evidence();
    for (index, expected_beneficial) in [true, false, true].into_iter().enumerate() {
        assert!(
            evidence.segments[index].is_some(),
            "committed benefit evidence missing segment {index}"
        );
        if let Some(sample) = evidence.segments[index] {
            assert_eq!(sample.is_beneficial_non_zero_work(), expected_beneficial);
        }
    }
}

#[test]
fn mixed_assembly_dense_exact_residents_all_reuse() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let store = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    let mut frame = frame_for_mixed_fixture(&source_plan, &authoritative, feasibility, plan, store);
    let bundle = frame
        .assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            paint_for_plan(plan),
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
            plan,
        )
        .expect("dense exact residents should assemble");
    assert_eq!(bundle.fresh_count, 0);
    assert_eq!(bundle.reused_count, 3);
    assert_eq!(bundle.append_count, 3);
    assert!(bundle.plan.entries[..3].iter().all(|entry| matches!(
        entry,
        Some(
            super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::RetainedCandidate(_),
                ..
            }
        )
    )));
    frame
        .commit_native_scene_assembly(bundle, test_scene_validity())
        .expect("dense exact bundle should commit");
    assert_encoding_equal(&frame.scene, &authoritative);
}

#[test]
fn mixed_assembly_zero_resident_store_fresh_encodes_every_entry() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let mut materialization =
        materialize_fixture((authoritative.clone(), feasibility, plan, payloads));
    materialization.clear_artifacts_for_test();
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialization);
    assert_eq!(store.plan_entry_count_for_test(), 3);
    assert_eq!(store.resident_count_for_test(), 0);

    let mut frame = frame_for_mixed_fixture(&source_plan, &authoritative, feasibility, plan, store);
    let bundle = frame
        .assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            paint_for_plan(plan),
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
            plan,
        )
        .expect("zero-resident valid store should fresh-encode");
    assert_eq!(bundle.fresh_count, 3);
    assert_eq!(bundle.reused_count, 0);
    assert_eq!(bundle.append_count, 3);
    assert!(bundle.plan.entries[..3].iter().all(|entry| matches!(
        entry,
        Some(
            super::super::retained_paint_segments::NativePaintSegmentEligibilityEntry {
                disposition: super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(_),
                ..
            }
        )
    )));
    frame
        .commit_native_scene_assembly(bundle, test_scene_validity())
        .expect("zero-resident mixed bundle should commit");
    assert_encoding_equal(&frame.scene, &authoritative);
}

#[test]
fn mixed_assembly_malformed_present_artifact_vetoes_without_mutating_frame() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let mut store = artifact_store_for_fixture(&authoritative, feasibility, plan, &payloads);
    assert!(
        store.snapshot_identities()[1].is_some(),
        "middle artifact should be resident"
    );
    if let Some(artifact) = store.artifact_for_test_mut(1) {
        artifact
            .scene_for_test_mut()
            .encoding_mut()
            .resources
            .normalized_coords
            .push(0);
    }
    let frame = frame_for_mixed_fixture(&source_plan, &authoritative, feasibility, plan, store);
    let previous_scene = frame.scene.clone();

    assert!(matches!(
        frame.assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            paint_for_plan(plan),
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
            plan,
        ),
        Err(super::scene::NativePaintSegmentAssemblyVetoReason::InvalidPayload)
    ));
    assert_encoding_equal(&frame.scene, &previous_scene);
}

#[test]
fn mixed_assembly_unsupported_sparse_hole_vetoes_without_mutating_frame() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let source_plan = typed_artifact_source_plan(3);
    let mut materialization =
        materialize_fixture((authoritative.clone(), feasibility, plan, payloads));
    assert!(materialization.remove_artifact_for_test(1));
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialization);
    let mut frame = frame_for_mixed_fixture(&source_plan, &authoritative, feasibility, plan, store);
    frame.last_paint_plan.primitives[2] = PaintPrimitive::Text(PaintTextRun {
        widget_id: 99,
        text: "unsupported fresh segment".into(),
        rect: Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(20.0, 12.0)),
        font_size: 12.0,
        color: Rgba8::new(255, 255, 255, 255),
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
        baseline: None,
    });
    let previous_scene = frame.scene.clone();

    assert!(matches!(
        frame.assemble_mixed_native_scene(
            Vector2::new(320.0, 180.0),
            paint_for_plan(plan),
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
            plan,
        ),
        Err(super::scene::NativePaintSegmentAssemblyVetoReason::UnsupportedFreshPrimitive)
    ));
    assert_encoding_equal(&frame.scene, &previous_scene);
}

#[test]
fn scene_artifact_materialization_accepts_one_typed_candidate_with_full_equality() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let expected_payload = payloads[0].clone();
    let expected_entry = plan.entries[0].expect("entry");
    let materialized = materialize_fixture((scene.clone(), feasibility, plan, payloads));

    assert_eq!(materialized.len(), 1);
    let artifact = &materialized.artifacts_for_test()[0];
    assert_encoding_equal(artifact.scene_for_test(), &scene);
    assert_encoding_equal(artifact.scene_for_test(), expected_payload.scene_for_test());
    assert_eq!(artifact.identity_for_test(), expected_entry.span.identity);
    assert_eq!(artifact.span_for_test(), expected_entry.span);
    assert_eq!(artifact.revision_for_test(), 1);
    assert_eq!(
        artifact.target_generation_for_test(),
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
    let expected = payloads.clone();
    let materialized = materialize_fixture((scene, feasibility, plan, payloads));

    assert_eq!(materialized.len(), 3);
    assert_eq!(materialized.plan_entry_count_for_test(), 3);
    for (artifact, expected) in materialized
        .artifacts_for_test()
        .iter()
        .zip(expected.iter())
    {
        assert_encoding_equal(artifact.scene_for_test(), expected.scene_for_test());
    }
    for (index, artifact) in materialized.artifacts_for_test().iter().enumerate() {
        assert_eq!(artifact.plan_index_for_test(), index);
    }
}

#[test]
fn scene_artifact_materialization_projection_preserves_original_sparse_slots() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let materialization = materialize_fixture((scene, feasibility, plan, payloads));
    let mut admission =
        super::super::retained_paint_segments::NativePaintSegmentCacheAdmission::default();
    for index in [0, 2] {
        let artifact = &materialization.artifacts_for_test()[index];
        admission.add_warming_for_test(
            artifact.identity_for_test(),
            artifact.span_for_test(),
            artifact.revision_for_test(),
            artifact.target_generation_for_test(),
        );
    }

    let projected = materialization.filter_for_publication(&admission);
    assert_eq!(projected.plan_entry_count_for_test(), 3);
    assert_eq!(projected.len(), 2);
    assert_eq!(
        projected
            .artifacts_for_test()
            .iter()
            .map(|artifact| artifact.plan_index_for_test())
            .collect::<Vec<_>>(),
        vec![0, 2]
    );

    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(projected);
    let mut expected = [None; crate::runtime::MAX_PAINT_SEGMENTS];
    expected[0] = plan.entries[0].map(|entry| entry.span.identity);
    expected[2] = plan.entries[2].map(|entry| entry.span.identity);
    assert_eq!(store.snapshot_identities(), expected);
    assert_eq!(store.plan_entry_count_for_test(), 3);
    assert_eq!(store.resident_count_for_test(), 2);
}

#[test]
fn scene_artifact_materialization_projection_keeps_zero_resident_cardinality() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let materialization = materialize_fixture((scene, feasibility, plan, payloads));
    let admission =
        super::super::retained_paint_segments::NativePaintSegmentCacheAdmission::default();

    let projected = materialization.filter_for_publication(&admission);
    assert_eq!(projected.plan_entry_count_for_test(), 3);
    assert!(projected.is_empty());

    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(projected);
    assert_eq!(store.plan_entry_count_for_test(), 3);
    assert_eq!(store.resident_count_for_test(), 0);
    assert!(store.snapshot_identities().iter().all(Option::is_none));
}

#[test]
fn scene_artifact_materialization_projection_keeps_atomic_store_rejection() {
    type Materialization = super::scene::NativePaintSegmentArtifactMaterialization;
    type Invalidator = fn(&mut Materialization);

    let cases: [(&str, Invalidator); 2] = [
        ("duplicate", |materialization| {
            materialization.duplicate_first_for_test();
        }),
        ("cardinality", |materialization| {
            materialization.set_plan_entry_count_for_test(1);
        }),
    ];

    for (case, invalidate) in cases {
        let (scene, feasibility, plan, payloads) = typed_artifact_fixture(2);
        let valid = materialize_fixture((scene, feasibility, plan, payloads));
        let mut admission =
            super::super::retained_paint_segments::NativePaintSegmentCacheAdmission::default();
        seed_publication_admission_for_test(&mut admission, &valid);

        let (scene, feasibility, plan, payloads) = typed_artifact_fixture(2);
        let mut invalid = materialize_fixture((scene, feasibility, plan, payloads));
        invalidate(&mut invalid);
        let mut store = super::scene::NativePaintSegmentArtifactStore::default();
        store.reconcile(valid.filter_for_publication(&admission));
        assert!(
            store.snapshot_identities()[0].is_some(),
            "{case}: valid publication"
        );

        store.reconcile(invalid.filter_for_publication(&admission));
        assert!(
            store.snapshot_identities().iter().all(Option::is_none),
            "{case}: invalid projection must clear stale artifacts"
        );
    }
}

#[test]
fn scene_artifact_payload_selection_reuses_matching_stored_artifact() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let source_plan = typed_artifact_source_plan(1);
    let store = artifact_store_for_fixture(&scene, feasibility, plan, &payloads);

    let selection = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint_for_plan(plan),
        plan,
        test_scene_validity(),
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &store,
    );

    assert_eq!(selection.reused_count_for_test(), 1);
    assert_eq!(selection.fresh_count_for_test(), 0);
    assert_eq!(selection.payloads_for_test().len(), 1);
    assert_encoding_equal(
        selection.payloads_for_test()[0].scene_for_test(),
        payloads[0].scene_for_test(),
    );
}

#[test]
fn scene_artifact_payload_selection_missing_store_falls_back_to_fresh_encoding() {
    let (_, _, plan, expected_payloads) = typed_artifact_fixture(1);
    let source_plan = typed_artifact_source_plan(1);
    let store = super::scene::NativePaintSegmentArtifactStore::default();

    let selection = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint_for_plan(plan),
        plan,
        test_scene_validity(),
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &store,
    );

    assert_eq!(selection.reused_count_for_test(), 0);
    assert_eq!(selection.fresh_count_for_test(), 1);
    assert_encoding_equal(
        selection.payloads_for_test()[0].scene_for_test(),
        expected_payloads[0].scene_for_test(),
    );
}

#[test]
fn scene_artifact_payload_selection_metadata_mismatch_falls_back_to_fresh_encoding() {
    type Materialization = super::scene::NativePaintSegmentArtifactMaterialization;
    type Invalidator = fn(&mut Materialization);

    let cases: [(&str, Invalidator); 2] = [
        ("revision", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_revision_for_test(2);
        }),
        ("target generation", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_target_generation_for_test(
                super::super::runner_state::NativeTargetGeneration::from_test_serial(2),
            );
        }),
    ];

    for (case, invalidate) in cases {
        let (scene, feasibility, plan, expected_payloads) = typed_artifact_fixture(1);
        let source_plan = typed_artifact_source_plan(1);
        let mut materialization =
            materialize_fixture((scene.clone(), feasibility, plan, expected_payloads.clone()));
        invalidate(&mut materialization);
        let mut store = super::scene::NativePaintSegmentArtifactStore::default();
        store.reconcile(materialization);

        let selection = super::scene::encode_native_paint_segment_payloads(
            &source_plan.primitives,
            Vector2::new(320.0, 180.0),
            paint_for_plan(plan),
            plan,
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
            &store,
        );

        assert_eq!(selection.reused_count_for_test(), 0, "{case}");
        assert_eq!(selection.fresh_count_for_test(), 1, "{case}");
        assert_encoding_equal(
            selection.payloads_for_test()[0].scene_for_test(),
            expected_payloads[0].scene_for_test(),
        );
    }
}

#[test]
fn scene_artifact_payload_selection_corruption_is_rejected_by_authoritative_materialization() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let source_plan = typed_artifact_source_plan(1);
    let mut store = artifact_store_for_fixture(&scene, feasibility, plan, &payloads);
    store
        .artifact_for_test_mut(0)
        .expect("stored artifact")
        .scene_for_test_mut()
        .encoding_mut()
        .path_data[0] ^= 1;

    let selection = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint_for_plan(plan),
        plan,
        test_scene_validity(),
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &store,
    );
    assert_eq!(selection.reused_count_for_test(), 1);
    assert_eq!(selection.fresh_count_for_test(), 0);

    let materialized = materialize_fixture((scene, feasibility, plan, selection.into_parts().0));
    assert!(materialized.is_empty());
}

#[test]
fn scene_artifact_store_installs_valid_candidate() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let expected_identity = plan.entries[0].map(|entry| entry.span.identity);
    let materialized = materialize_fixture((scene, feasibility, plan, payloads));
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();

    store.reconcile(materialized);

    assert_eq!(store.snapshot_identities()[0], expected_identity);
    assert!(store.snapshot_identities()[1..].iter().all(Option::is_none));
}

#[test]
fn scene_artifact_store_keeps_mixed_candidates_in_plan_order() {
    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(3);
    if let Some(entry) = plan.entries[1].as_mut() {
        entry.disposition = super::super::retained_paint_segments::NativePaintSegmentEligibilityDisposition::FreshEncodingRequired(
            super::super::retained_paint_segments::NativePaintSegmentFreshEncodingReason::RevisionChanged,
        );
    }
    let expected = [
        plan.entries[0].map(|entry| entry.span.identity),
        plan.entries[1].map(|entry| entry.span.identity),
        plan.entries[2].map(|entry| entry.span.identity),
    ];
    let materialized = materialize_fixture((scene, feasibility, plan, payloads));
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();

    store.reconcile(materialized);

    let identities = store.snapshot_identities();
    assert_eq!(identities[0], expected[0]);
    assert_eq!(identities[1], expected[1]);
    assert_eq!(identities[2], expected[2]);
    assert!(identities[3..].iter().all(Option::is_none));
}

#[test]
fn scene_artifact_store_preserves_sparse_plan_indices_without_compaction() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let expected = [
        plan.entries[0].map(|entry| entry.span.identity),
        plan.entries[1].map(|entry| entry.span.identity),
        plan.entries[2].map(|entry| entry.span.identity),
    ];
    let mut materialization = materialize_fixture((scene, feasibility, plan, payloads));
    assert_eq!(materialization.plan_entry_count_for_test(), 3);
    assert!(materialization.remove_artifact_for_test(1));
    assert_eq!(materialization.len(), 2);

    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialization);

    let mut expected_slots = [None; crate::runtime::MAX_PAINT_SEGMENTS];
    expected_slots[0] = expected[0];
    expected_slots[2] = expected[2];
    assert_eq!(store.snapshot_identities(), expected_slots);
    assert_eq!(store.plan_entry_count_for_test(), 3);
    assert_eq!(store.resident_count_for_test(), 2);
}

#[test]
fn scene_artifact_sparse_hole_misses_lookup_and_vetoes_retained_assembly() {
    let (authoritative, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let scene_validity = test_scene_validity();
    let source_plan = typed_artifact_source_plan(3);
    let mut materialization =
        materialize_fixture((authoritative.clone(), feasibility, plan, payloads));
    assert!(materialization.remove_artifact_for_test(1));
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialization);

    let selection = super::scene::encode_native_paint_segment_payloads(
        &source_plan.primitives,
        Vector2::new(320.0, 180.0),
        paint_for_plan(plan),
        plan,
        scene_validity,
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        &store,
    );
    assert_eq!(selection.reused_count_for_test(), 2);
    assert_eq!(selection.fresh_count_for_test(), 1);

    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &authoritative,
            feasibility,
            plan,
            &store,
            scene_validity,
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(
            super::scene::NativePaintSegmentAssemblyVetoReason::MissingArtifact
        )
    ));
}

#[test]
fn scene_artifact_store_allows_zero_resident_nonzero_cardinality() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(3);
    let mut materialization = materialize_fixture((scene, feasibility, plan, payloads));
    materialization.clear_artifacts_for_test();
    assert_eq!(materialization.plan_entry_count_for_test(), 3);
    assert_eq!(materialization.len(), 0);

    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialization);

    assert_eq!(store.plan_entry_count_for_test(), 3);
    assert_eq!(store.resident_count_for_test(), 0);
    assert!(store.snapshot_identities().iter().all(Option::is_none));
}

#[test]
fn scene_artifact_store_rejects_zero_cardinality_with_resident_slot() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let mut store = artifact_store_for_fixture(&scene, feasibility, plan, &payloads);
    store.set_plan_entry_count_for_test(0);

    assert!(matches!(
        super::scene::assemble_retained_native_paint_segment_scene(
            &scene,
            feasibility,
            plan,
            &store,
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        ),
        super::scene::NativePaintSegmentAssemblyResult::Veto(
            super::scene::NativePaintSegmentAssemblyVetoReason::InvalidEvidence
        )
    ));
}

#[test]
fn scene_artifact_store_clears_for_empty_and_fallback_materializations() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let mut store = super::scene::NativePaintSegmentArtifactStore::default();
    store.reconcile(materialize_fixture((scene, feasibility, plan, payloads)));
    assert!(store.snapshot_identities()[0].is_some());

    store.reconcile(super::scene::NativePaintSegmentArtifactMaterialization::default());
    assert!(store.snapshot_identities().iter().all(Option::is_none));

    let (scene, feasibility, mut plan, payloads) = typed_artifact_fixture(1);
    plan.outcome = super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::FullSceneFallback(
        super::super::retained_paint_segments::NativePaintSegmentFallbackReason::TrailingEvidence,
    );
    store.reconcile(materialize_fixture((scene, feasibility, plan, payloads)));
    assert!(store.snapshot_identities().iter().all(Option::is_none));
}

#[test]
fn scene_artifact_store_rejects_invalid_input_and_clears_stale_artifacts() {
    type Materialization = super::scene::NativePaintSegmentArtifactMaterialization;
    type Invalidator = fn(&mut Materialization);

    let cases: [(&str, Invalidator); 12] = [
        ("unknown generation", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_target_generation_for_test(
                super::super::runner_state::NativeTargetGeneration::unknown(),
            );
        }),
        ("zero revision", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_revision_for_test(0);
        }),
        ("mismatched identity and span", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_identity_for_test(
                crate::runtime::PaintSegmentIdentity {
                    preceding: None,
                    following: None,
                },
            );
        }),
        ("empty span", |materialization| {
            let end = materialization.artifacts_for_test()[0].span_for_test().end;
            materialization.artifacts_for_test_mut()[0].set_span_start_for_test(end);
        }),
        ("overlapping or out-of-order span", |materialization| {
            let start = materialization.artifacts_for_test()[0]
                .span_for_test()
                .start;
            materialization.artifacts_for_test_mut()[1].set_span_start_for_test(start);
        }),
        ("duplicate identity", |materialization| {
            let identity = materialization.artifacts_for_test()[0].identity_for_test();
            materialization.artifacts_for_test_mut()[1].set_identity_for_test(identity);
            materialization.artifacts_for_test_mut()[1].set_span_identity_for_test(identity);
        }),
        ("mixed generation", |materialization| {
            materialization.artifacts_for_test_mut()[1].set_target_generation_for_test(
                super::super::runner_state::NativeTargetGeneration::from_test_serial(2),
            );
        }),
        ("duplicate plan index", |materialization| {
            materialization.artifacts_for_test_mut()[1].set_plan_index_for_test(0);
        }),
        ("out-of-range plan index", |materialization| {
            materialization.artifacts_for_test_mut()[0].set_plan_index_for_test(2);
        }),
        (
            "inconsistent cardinality and trailing resident",
            |materialization| {
                materialization.set_plan_entry_count_for_test(1);
            },
        ),
        ("stale scene validity", |materialization| {
            materialization.artifacts_for_test_mut()[0]
                .set_scene_validity_for_test(test_scene_validity_with_dpi(2.0));
        }),
        ("oversize input", |materialization| {
            while materialization.artifacts_for_test().len() <= crate::runtime::MAX_PAINT_SEGMENTS {
                materialization.duplicate_first_for_test();
            }
        }),
    ];

    for (case, invalidate) in cases {
        let (scene, feasibility, plan, payloads) = typed_artifact_fixture(2);
        let mut store = super::scene::NativePaintSegmentArtifactStore::default();
        store.reconcile(materialize_fixture((scene, feasibility, plan, payloads)));
        assert!(
            store.snapshot_identities()[0].is_some(),
            "{case}: valid install"
        );

        let (scene, feasibility, plan, payloads) = typed_artifact_fixture(2);
        let mut invalid = materialize_fixture((scene, feasibility, plan, payloads));
        invalidate(&mut invalid);
        store.reconcile(invalid);

        assert!(
            store.snapshot_identities().iter().all(Option::is_none),
            "{case}: invalid materialization must clear the store"
        );
    }
}

#[test]
fn scene_artifact_store_clears_after_dpi_change_without_unbound_target_promotion() {
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 180.0),
    );
    runner.window.target_generation =
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1);
    let previous_target_generation = runner.window.target_generation;
    let materialized = super::runner::materialize_native_paint_segment_artifacts_for_test(
        &scene,
        feasibility,
        plan,
        payloads,
        test_scene_validity(),
        previous_target_generation,
    );
    assert_eq!(
        materialized.artifacts_for_test()[0].target_generation_for_test(),
        previous_target_generation
    );
    seed_frame_publication_admission_for_test(&mut runner.frame, &materialized);
    runner
        .frame
        .reconcile_native_paint_segment_artifacts(materialized);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()[0]
            .is_some()
    );

    runner.update_native_dpi_scale(2.0);

    assert_eq!(runner.window.target_generation, previous_target_generation);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn other_surface_fence_clears_native_target_state_without_consuming_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 180.0),
    );
    runner.window.target_generation =
        super::super::runner_state::NativeTargetGeneration::from_test_serial(1);
    let scene_validity = runner.frame.native_scene_validity_fingerprint(
        runner.core.base_paint_plan_context(),
        runner.core.resolved_appearance(),
        runner.window.dpi_scale,
    );
    runner.frame.record_scene_encode(scene_validity);
    let (scene, feasibility, plan, payloads) = typed_artifact_fixture(1);
    let materialization =
        materialize_fixture_with_validity((scene, feasibility, plan, payloads), scene_validity);
    seed_frame_publication_admission_for_test(&mut runner.frame, &materialization);
    runner
        .frame
        .reconcile_native_paint_segment_artifacts(materialization);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()
            .iter()
            .any(Option::is_some)
    );
    assert!(runner.frame.can_reuse_native_scene(scene_validity));
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;
    let application_count = runner.core.runtime.bridge().state.count;
    let application_name = runner.core.runtime.bridge().state.name.clone();
    let pending = super::super::FrameWork::RebuildScene {
        reason: super::super::FrameWorkReason::RuntimeSurfaceRepaint,
        mode: super::super::SceneRebuildMode::Immediate,
    };
    runner.timing.pending_frame_work = pending;

    runner.handle_other_surface_acquire_failure(winit::dpi::PhysicalSize::new(640, 360));

    assert!(!runner.window.target_generation.is_known());
    assert!(!runner.frame.can_reuse_native_scene(scene_validity));
    assert!(runner.frame.scene_texture_dirty);
    assert!(runner.frame.composited_base_dirty);
    assert!(
        runner
            .frame
            .native_paint_segment_artifact_store
            .snapshot_identities()
            .iter()
            .all(Option::is_none)
    );
    assert_eq!(runner.core.runtime.bridge().state.count, application_count);
    assert_eq!(runner.core.runtime.bridge().state.name, application_name);
    assert_eq!(runner.timing.pending_frame_work, pending);
}

#[test]
fn scene_artifact_materialization_rejects_missing_and_extra_payloads_atomically() {
    let (_, feasibility, plan, mut payloads) = typed_artifact_fixture(2);
    payloads.pop();
    assert!(
        super::runner::materialize_native_paint_segment_artifacts_for_test(
            &Scene::new(),
            feasibility,
            plan,
            payloads,
            test_scene_validity(),
            super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        )
        .is_empty()
    );

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(2);
    payloads.push(payloads[0].clone());
    assert!(
        super::runner::materialize_native_paint_segment_artifacts_for_test(
            &scene,
            feasibility,
            plan,
            payloads,
            test_scene_validity(),
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
    payloads[0].scene_for_test_mut().encoding_mut().path_data[0] ^= 1;
    assert!(materialize_fixture((scene, feasibility, plan, payloads)).is_empty());
}

#[test]
fn scene_artifact_materialization_rejects_nonzero_flags_open_clips_and_resources() {
    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0].scene_for_test_mut().encoding_mut().flags = 1;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0].scene_for_test_mut().encoding_mut().n_open_clips = 1;
    assert!(materialize_fixture((scene.clone(), feasibility, plan, payloads)).is_empty());

    let (scene, feasibility, plan, mut payloads) = typed_artifact_fixture(1);
    payloads[0]
        .scene_for_test_mut()
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
