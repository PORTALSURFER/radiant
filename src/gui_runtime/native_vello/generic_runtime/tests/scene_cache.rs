use super::*;
use crate::{
    gui::types::ImageRgba,
    runtime::{
        GpuSurfaceRuntimeOverlays, PaintFillRect, PaintTextAlign, PaintTextRun, SurfacePaintPlan,
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
