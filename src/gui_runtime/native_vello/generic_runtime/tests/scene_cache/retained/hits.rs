use super::{super::*, fixtures::*};

#[test]
fn retained_custom_surface_cache_skips_unchanged_bridge_render() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
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
    let mut text_runs = SceneTextRunBuffer::new();
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
