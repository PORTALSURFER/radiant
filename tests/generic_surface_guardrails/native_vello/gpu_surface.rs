use super::*;

#[test]
fn composited_base_frame_cache_avoids_post_mutation_expect() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base.rs"),
    )
    .expect("composited base presenter should be readable");
    let source = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base/frame.rs"),
    )
    .expect("composited base frame cache should be readable");
    let ensure_body = source
        .split("pub(super) fn ensure")
        .nth(1)
        .and_then(|tail| tail.split("fn new").next())
        .expect("CompositedBaseFrame::ensure should be present");

    assert!(
        module.contains("mod frame;")
            && module.contains("pub(super) use frame::CompositedBaseFrame;"),
        "composited base presentation should delegate cached texture ownership to the frame module"
    );
    assert!(
        !module.contains("struct CompositedBaseFrame")
            && source.contains("struct CompositedBaseFrame"),
        "cached composited base texture state should stay out of the presenter module"
    );
    assert!(
        ensure_body.contains(".is_some_and(|frame| frame.matches(device, width, height, format))")
            && ensure_body.contains("frame.insert(Self::new(device, width, height, format))"),
        "CompositedBaseFrame::ensure should reuse device-matching frames and install replacements directly"
    );
    assert!(
        !ensure_body.contains(".expect(") && !ensure_body.contains(".unwrap("),
        "CompositedBaseFrame::ensure should not assert the Option state after mutating it"
    );
}

#[test]
fn post_gpu_overlay_vertex_buffer_upload_is_non_panicking() {
    let source = fs::read_to_string(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/buffer.rs",
    )
    .expect("post GPU overlay vertex buffer should be readable");

    assert!(
        !source.contains(".expect(") && !source.contains(".unwrap("),
        "post GPU overlay vertex buffer upload should handle missing cached buffers without panicking"
    );
}

#[test]
fn post_gpu_overlay_geometry_tests_stay_grouped_by_replay_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests.rs",
        ))
        .expect("post GPU overlay geometry test root should be readable");
    let suffix = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/suffix.rs",
    ))
    .expect("post GPU overlay suffix tests should be readable");
    let vertices = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/vertices.rs",
    ))
    .expect("post GPU overlay vertex tests should be readable");
    let regions = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/regions.rs",
    ))
    .expect("post GPU overlay region tests should be readable");
    let fixtures = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/geometry/tests/fixtures.rs",
    ))
    .expect("post GPU overlay geometry fixtures should be readable");

    assert!(
        root.contains("mod fixtures;")
            && root.contains("mod suffix;")
            && root.contains("mod vertices;")
            && root.contains("mod regions;")
            && !root.contains("fn replayable_vertices_batch_fill_and_stroke_rectangles"),
        "post GPU overlay geometry test root should index focused replay groups instead of owning all cases"
    );
    assert!(
        suffix.contains("fn replayable_suffix_starts_after_last_gpu_surface")
            && vertices.contains("fn replayable_vertices_batch_fill_and_stroke_rectangles")
            && regions.contains(
                "fn replayable_vertices_in_regions_clip_translucent_fills_to_gpu_regions"
            )
            && fixtures.contains("fn translucent_white"),
        "post GPU overlay geometry tests should stay grouped by suffix, full-target vertices, region clipping, and fixtures"
    );
}

#[test]
fn gpu_surface_render_stats_stay_in_focused_diagnostics_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let types = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types.rs"),
    )
    .expect("GPU surface type bucket should be readable");
    let stats = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs"),
    )
    .expect("GPU surface stats module should be readable");
    let custom_shader = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader.rs"),
    )
    .expect("GPU surface custom shader module should be readable");
    let custom_shader_types = fs::read_to_string(
        manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/custom_shader.rs",
        ),
    )
    .expect("GPU surface custom shader type module should be readable");

    assert!(
        module.contains("mod stats;")
            && module.contains("mod custom_shader;")
            && module.contains("pub(super) use stats::GpuSurfaceRenderStats;"),
        "GPU surface renderer should delegate diagnostics and re-export render stats from focused modules"
    );
    assert!(
        !types.contains("struct GpuSurfaceRenderStats")
            && stats.contains("struct GpuSurfaceRenderStats")
            && stats.contains("atlas_texture_uploads")
            && stats.contains("signal_body_encode_elapsed")
            && stats.contains("composite_binding_rebuilds"),
        "render profiling counters should stay out of resource/cache-key type definitions"
    );
    assert!(
        custom_shader.contains("fn render_custom_shader")
            && custom_shader.contains("fn ensure_custom_shader_pipeline")
            && custom_shader.contains("fn ensure_custom_shader_binding")
            && custom_shader.contains("custom_shader_layout_entries")
            && custom_shader.contains("binding: 1")
            && custom_shader.contains("binding: 2")
            && custom_shader.contains("BufferBindingType::Storage { read_only: true }")
            && custom_shader.contains("device.create_shader_module")
            && custom_shader.contains("device.create_render_pipeline")
            && custom_shader.contains("record_unsupported_custom_shader")
            && custom_shader.contains("unsupported_custom_shader_surfaces += 1")
            && custom_shader.contains("unsupported_custom_shader_vertices")
            && custom_shader.contains("unsupported_custom_shader_source_bytes")
            && custom_shader.contains("unsupported_custom_shader_uniform_bytes")
            && custom_shader.contains("unsupported_custom_shader_storage_bytes")
            && custom_shader.contains("fn custom_shader_pipeline_key_tracks_payload_bindings"),
        "native custom shader GPU-surface execution and fallback diagnostics should stay focused"
    );
    assert!(
        types.contains("mod custom_shader;")
            && types.contains("CustomShaderPipeline")
            && custom_shader_types.contains("struct CustomShaderPipelineKey")
            && custom_shader_types.contains("has_uniform_payload")
            && custom_shader_types.contains("has_storage_payload")
            && custom_shader_types.contains("struct CustomShaderBindingKey")
            && custom_shader_types
                .contains("fn custom_shader_pipeline_key_tracks_shader_stage_contract"),
        "native custom shader pipeline identity should stay in a focused type module"
    );
    assert!(
        stats.contains("unsupported_custom_shader_vertices")
            && stats.contains("unsupported_custom_shader_source_bytes")
            && stats.contains("unsupported_custom_shader_uniform_bytes")
            && stats.contains("unsupported_custom_shader_storage_bytes"),
        "GPU surface render stats should keep skipped custom-shader draw and payload counters"
    );
}

#[test]
fn gpu_signal_surface_cache_keys_stay_in_focused_identity_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let signal = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal.rs",
    ))
    .expect("GPU signal type module should be readable");
    let cache_key = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal/cache_key.rs",
    ))
    .expect("GPU signal cache-key module should be readable");

    assert!(
        signal.contains("mod cache_key;")
            && signal.contains("SignalBodyTexture")
            && signal.contains("signal_body_matches_key(")
            && !signal.contains("struct SignalGainPreviewKey")
            && !signal.contains("const GPU_SIGNAL_STYLE_REVISION"),
        "GPU signal resource DTOs should delegate cache identity details"
    );
    assert!(
        cache_key.contains("struct SignalBufferCacheKey")
            && cache_key.contains("struct SignalBodyCacheKey")
            && cache_key.contains("struct SignalGainPreviewKey")
            && cache_key.contains("const GPU_SIGNAL_STYLE_REVISION")
            && cache_key.contains("fn signal_body_matches_key"),
        "GPU signal cache-key identity rules should live in signal/cache_key.rs"
    );
}

#[test]
fn native_gpu_surface_resource_lifecycle_stays_with_resource_cache() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let resources = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources.rs"),
    )
    .expect("GPU surface resources module should be readable");
    let cache = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs"),
    )
    .expect("GPU surface resource cache module should be readable");

    assert!(
        resources.contains("mod cache;")
            && resources.contains("pub(super) use cache::GpuSurfaceResourceCache;")
            && renderer.contains("resources: GpuSurfaceResourceCache"),
        "GPU surface renderer should store retained WGPU resources through a focused resource cache"
    );
    assert!(
        cache.contains("struct GpuSurfaceResourceCache")
            && cache.contains("fn prune_inactive")
            && cache.contains("fn clear")
            && !renderer.contains("textures: HashMap")
            && !renderer.contains("signal_summaries: HashMap"),
        "resource-map lifecycle should live with the resource cache, not top-level renderer fields"
    );
}

#[test]
fn native_gpu_signal_summary_cache_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let signal = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs"),
    )
    .expect("GPU signal resource module should be readable");
    let summary = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal/summary.rs",
    ))
    .expect("GPU signal summary cache module should be readable");

    assert!(
        signal.contains("mod summary;")
            && signal.contains("fn ensure_signal_body_texture")
            && signal.contains("fn ensure_signal_buffer")
            && !signal.contains("fn cached_signal_summary")
            && !signal.contains("signal_summary_cache_hits"),
        "GPU signal resource upload/rendering should delegate CPU summary caching"
    );
    assert!(
        summary.contains("fn cached_signal_summary")
            && summary.contains("signal_summary_cache_hits")
            && summary.contains("signal_summary_builds")
            && summary.contains("GpuSignalSummary::from_interleaved_samples"),
        "GPU signal summary memoization should live in resources/signal/summary.rs"
    );
}

#[test]
fn native_gpu_surface_visibility_occlusion_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let visibility = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility.rs"),
    )
    .expect("GPU surface visibility module should be readable");
    let occlusion =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility/occlusion.rs",
        ))
        .expect("GPU surface visibility occlusion module should be readable");

    assert!(
        visibility.contains("mod occlusion;")
            && visibility.contains("visible_rects_after_occlusion")
            && visibility.contains("gpu_surface_opaque_suffix_regions(surface.rect, suffix)")
            && !visibility.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && !visibility.contains("PaintPrimitive::FillRect(fill)"),
        "GPU surface visibility should delegate opaque suffix collection"
    );
    assert!(
        occlusion.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && occlusion.contains("fn gpu_surface_opaque_suffix_regions")
            && occlusion.contains("PaintPrimitive::FillRect(fill)")
            && occlusion.contains("intersect_rect(surface_rect, fill.rect)"),
        "opaque suffix occlusion filtering should live in visibility/occlusion.rs"
    );
}

#[test]
fn native_gpu_surface_overlay_uniforms_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let passes = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs"),
    )
    .expect("GPU surface pass module should be readable");
    let overlays = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs"),
    )
    .expect("GPU surface overlay uniform module should be readable");

    assert!(
        renderer.contains("mod overlays;") && renderer.contains("use overlays::*;"),
        "GPU surface renderer should route overlay uniform packing through a focused module"
    );
    assert!(
        !passes.contains("fn vertical_overlays")
            && !passes.contains("fn normalized_ratio")
            && overlays.contains("fn vertical_overlays")
            && overlays.contains("fn normalized_ratio")
            && overlays.contains("fn rgba_to_float"),
        "overlay uniform packing should not live with WGPU render-pass and scissor setup"
    );
}
