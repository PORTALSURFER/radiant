use super::*;

#[test]
fn native_vello_scene_encoder_keeps_custom_surfaces_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scene = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene.rs"),
    )
    .expect("native Vello scene encoder should be readable");
    let custom_surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/custom_surface.rs"),
    )
    .expect("custom surface scene encoder should be readable");
    let cache = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/cache.rs"),
    )
    .expect("retained surface scene cache should be readable");
    let cache_tests = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/cache/tests.rs"),
    )
    .expect("retained surface scene cache tests should be readable");

    assert!(
        scene.contains("mod custom_surface;")
            && scene.contains("mod cache;")
            && scene.contains(
                "use custom_surface::{CustomSurfaceEncodeContext, encode_custom_surface};"
            ),
        "central scene encoder should delegate retained custom-surface rendering"
    );
    assert!(
        !scene.contains("capability.render(")
            && custom_surface.contains("RuntimeRetainedSurfaceCapability")
            && custom_surface.contains("capability.render(")
            && custom_surface.contains(".cached_frame(retained, custom.rect, context.viewport)")
            && custom_surface.contains("encode_custom_surface_fallback"),
        "retained custom-surface cache/bridge/fallback logic should stay in the focused custom-surface encoder"
    );
    assert!(
        custom_surface.contains("gui::types::{Rgba8, Vector2}")
            && custom_surface.contains("gui_runtime::native_vello::NativeTextRenderer")
            && custom_surface.contains(
                "runtime::{PaintCustomSurface, RuntimeBridge, RuntimeRetainedSurfaceCapability}"
            )
            && custom_surface.contains("use vello::Scene;")
            && !custom_surface.contains("gui_runtime::native_vello::*"),
        "custom-surface scene encoding should name native text, geometry, runtime bridge, custom-surface, color, and Vello scene dependencies explicitly"
    );
    assert!(
        cache.contains("struct RetainedSurfaceFrameCache")
            && cache.contains("struct RetainedSurfaceFrameCacheEntry")
            && cache.contains("#[cfg(test)]")
            && cache.contains("mod tests;")
            && !cache
                .contains("fn retained_frame_cache_evicts_oldest_entry_without_shifting_storage")
            && cache_tests
                .contains("fn retained_frame_cache_evicts_oldest_entry_without_shifting_storage")
            && cache_tests.contains("fn retained_frame_cache_policy_can_disable_storage"),
        "retained scene cache data structures should stay in cache.rs while regression tests live in scene/cache/tests.rs"
    );
}

#[test]
fn native_retained_scene_cache_tests_stay_grouped_by_cache_behavior() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let retained = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained.rs"),
    )
    .expect("retained scene-cache test root should be readable");
    let fixtures = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained/fixtures.rs",
    ))
    .expect("retained scene-cache fixtures should be readable");
    let hits =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained/hits.rs",
        ))
        .expect("retained scene-cache hit tests should be readable");
    let miss =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained/miss.rs",
        ))
        .expect("retained scene-cache miss tests should be readable");
    let invalidation = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained/invalidation.rs",
    ))
    .expect("retained scene-cache invalidation tests should be readable");
    let volatile = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/scene_cache/retained/volatile.rs",
    ))
    .expect("retained scene-cache volatile tests should be readable");

    assert!(
        retained.contains("mod fixtures;")
            && retained.contains("mod hits;")
            && retained.contains("mod miss;")
            && retained.contains("mod invalidation;")
            && retained.contains("mod volatile;")
            && !retained.contains("struct RetainedBridge")
            && !retained.contains("fn retained_custom_surface_cache_skips_unchanged_bridge_render"),
        "retained scene-cache test root should index focused cache behavior groups instead of owning fixtures and all cases"
    );
    assert!(
        fixtures.contains("struct RetainedBridge")
            && fixtures.contains("struct MultiRetainedBridge")
            && hits.contains("fn retained_custom_surface_cache_keeps_multiple_stable_surfaces")
            && miss.contains("fn retained_custom_surface_miss_is_counted_as_fallback")
            && invalidation
                .contains("fn retained_custom_surface_cache_invalidates_dirty_descriptor_key")
            && volatile.contains("fn retained_custom_surface_cache_rejects_volatile_descriptor"),
        "retained scene-cache tests should stay grouped by fixtures, cache hits, misses, invalidation, and volatility"
    );
}
