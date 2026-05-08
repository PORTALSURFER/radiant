use super::*;

#[test]
fn generic_paint_plan_encodes_to_vello_scene() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();

    encode_surface_paint_plan_to_scene(
        &core.paint_plan(),
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
}

#[test]
fn retained_custom_surface_cache_skips_unchanged_bridge_render() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    let second = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 1);
    assert_eq!(core.runtime.bridge().render_count, 1);
}

#[test]
fn retained_custom_surface_cache_rejects_current_dirty_descriptor() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let second = encode_surface_paint_plan_to_scene(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
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
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    let second = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}
