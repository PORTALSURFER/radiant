use super::*;
use crate::gui::types::ImageRgba;

#[test]
fn generic_paint_plan_encodes_to_vello_scene() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
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
    assert!(text_runs.capacity() >= expected_text_primitives);
}

#[test]
fn scene_encoding_collects_fast_pointer_gpu_surface_hit_rects() {
    let mut bridge = demo_bridge();
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let mut hit_rects = Vec::new();
    let rect = Rect::from_min_size(Point::new(8.0, 12.0), Vector2::new(64.0, 32.0));
    let plan = SurfacePaintPlan {
        clear_color: ThemeTokens::default().clear_color,
        primitives: vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 42,
            key: 42,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: rect,
                atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid test image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: false,
                native_hover_cursor: None,
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
            viewport: Vector2::new(320.0, 180.0),
            retained_cache: &mut retained_cache,
            text_runs: &mut text_runs,
            fast_pointer_move_gpu_surface_hit_rects: &mut hit_rects,
            animation_time: Duration::ZERO,
        },
    );

    assert_eq!(stats.gpu_surface_count, 1);
    assert_eq!(hit_rects, [rect]);
}

#[test]
fn retained_custom_surface_cache_skips_unchanged_bridge_render() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(first.retained_frame_primitive_count, 1);
    assert_eq!(first.retained_frame_text_run_count, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 1);
    assert_eq!(second.retained_frame_primitive_count, 1);
    assert_eq!(second.retained_frame_text_run_count, 0);
    assert_eq!(core.runtime.bridge().render_count, 1);
}

#[test]
fn retained_custom_surface_cache_keeps_multiple_stable_surfaces() {
    let mut core =
        GenericNativeRuntimeCore::new(MultiRetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 2);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 2);
    assert_eq!(core.runtime.bridge().render_count_for(7), 1);
    assert_eq!(core.runtime.bridge().render_count_for(8), 1);
}

#[test]
fn retained_custom_surface_cache_rejects_current_dirty_descriptor() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let second = encode_plan(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}

#[test]
fn retained_custom_surface_cache_invalidates_dirty_descriptor_key() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let clean = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let dirty = encode_plan(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 0;
    core.refresh_surface();
    let clean_again_plan = core.paint_plan();
    let clean_again = encode_plan(
        &clean_again_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(clean.bridge_calls, 1);
    assert_eq!(dirty.bridge_calls, 1);
    assert_eq!(clean_again.bridge_calls, 1);
    assert_eq!(clean_again.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 3);
}

#[test]
fn retained_custom_surface_cache_rejects_volatile_descriptor() {
    let mut core = GenericNativeRuntimeCore::new(
        RetainedBridge {
            volatile: true,
            ..RetainedBridge::default()
        },
        Vector2::new(320.0, 40.0),
    );
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = Vec::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}

fn encode_plan<'plan, Bridge, Message>(
    plan: &'plan crate::runtime::SurfacePaintPlan,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
    retained_cache: &mut RetainedSurfaceFrameCache,
    text_runs: &mut Vec<SceneTextRun<'plan>>,
) -> RetainedSurfaceEncodeStats
where
    Bridge: RuntimeBridge<Message>,
{
    let mut fast_pointer_move_gpu_surface_hit_rects = Vec::new();
    encode_surface_paint_plan_to_scene(
        plan,
        SurfaceSceneEncodeContext {
            scene,
            text_renderer,
            bridge,
            viewport,
            retained_cache,
            text_runs,
            fast_pointer_move_gpu_surface_hit_rects: &mut fast_pointer_move_gpu_surface_hit_rects,
            animation_time: Duration::ZERO,
        },
    )
}
