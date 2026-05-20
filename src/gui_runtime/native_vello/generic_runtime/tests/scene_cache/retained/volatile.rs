use super::{super::*, fixtures::*};

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
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}
