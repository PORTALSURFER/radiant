use super::{super::*, fixtures::*};

#[test]
fn retained_custom_surface_miss_is_counted_as_fallback() {
    let mut core =
        GenericNativeRuntimeCore::new(MissingRetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let stats = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(stats.custom_surface_count, 1);
    assert_eq!(stats.bridge_calls, 1);
    assert_eq!(stats.cache_hits, 0);
    assert_eq!(stats.retained_surface_miss_count, 1);
    assert_eq!(stats.custom_surface_fallback_count, 1);
    assert_eq!(stats.retained_frame_primitive_count, 0);
    assert_eq!(core.runtime.bridge().render_count, 1);
}
