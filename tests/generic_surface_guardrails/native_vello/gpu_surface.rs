use super::*;

#[path = "gpu_surface/composited_base.rs"]
mod composited_base;
#[path = "gpu_surface/post_gpu_overlay.rs"]
mod post_gpu_overlay;

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
    let custom_shader_pipeline = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline.rs",
    ))
    .expect("GPU surface custom shader pipeline module should be readable");
    let custom_shader_binding =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/binding.rs",
        ))
        .expect("GPU surface custom shader binding module should be readable");
    let custom_shader_diagnostics = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/diagnostics.rs",
    ))
    .expect("GPU surface custom shader diagnostics module should be readable");
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
            && custom_shader.contains("#[path = \"custom_shader/pipeline.rs\"]")
            && custom_shader.contains("#[path = \"custom_shader/binding.rs\"]")
            && custom_shader.contains("#[path = \"custom_shader/diagnostics.rs\"]")
            && custom_shader.contains("self.ensure_custom_shader_pipeline")
            && custom_shader.contains("self.ensure_custom_shader_binding")
            && custom_shader.contains("record_unsupported_custom_shader")
            && custom_shader.contains("custom_shader_surfaces_rendered += 1")
            && !custom_shader.contains("fn ensure_custom_shader_pipeline")
            && !custom_shader.contains("fn ensure_custom_shader_binding")
            && !custom_shader.contains("fn custom_shader_layout_entries")
            && !custom_shader.contains("fn custom_shader_buffer")
            && !custom_shader.contains("device.create_shader_module")
            && !custom_shader.contains("device.create_bind_group"),
        "native custom shader draw orchestration should stay focused while delegating WGPU setup"
    );
    assert!(
        custom_shader_pipeline.contains("fn ensure_custom_shader_pipeline")
            && custom_shader_pipeline.contains("custom_shader_layout_entries")
            && custom_shader_pipeline.contains("binding: 1")
            && custom_shader_pipeline.contains("binding: 2")
            && custom_shader_pipeline.contains("BufferBindingType::Storage { read_only: true }")
            && custom_shader_pipeline.contains("device.create_shader_module")
            && custom_shader_pipeline.contains("device.create_render_pipeline")
            && custom_shader_pipeline.contains("custom_shader_pipeline_rebuilds += 1")
            && custom_shader_pipeline.contains("custom_shader_shader_module_failures += 1")
            && custom_shader_pipeline.contains("custom_shader_pipeline_failures += 1")
            && custom_shader_pipeline
                .contains("device.push_error_scope(wgpu::ErrorFilter::Validation)")
            && custom_shader_pipeline
                .contains("fn custom_shader_pipeline_key_tracks_payload_bindings"),
        "native custom shader pipeline setup and validation diagnostics should stay in the pipeline module"
    );
    assert!(
        custom_shader_binding.contains("fn ensure_custom_shader_binding")
            && custom_shader_binding.contains("fn custom_shader_buffer")
            && custom_shader_binding.contains("binding: 1")
            && custom_shader_binding.contains("binding: 2")
            && custom_shader_binding.contains("device.create_bind_group")
            && custom_shader_binding.contains("custom_shader_binding_rebuilds += 1")
            && custom_shader_binding.contains("custom_shader_binding_cache_hits += 1")
            && custom_shader_binding.contains("custom_shader_binding_failures += 1")
            && custom_shader_binding
                .contains("device.push_error_scope(wgpu::ErrorFilter::Validation)"),
        "native custom shader bind-group and payload-buffer setup should stay in the binding module"
    );
    assert!(
        custom_shader_diagnostics.contains("fn custom_shader_validation_error")
            && custom_shader_diagnostics.contains("fn record_unsupported_custom_shader")
            && custom_shader_diagnostics.contains("custom_shader_surfaces_failed += 1")
            && custom_shader_diagnostics.contains("device.pop_error_scope()")
            && custom_shader_diagnostics.contains("unsupported_custom_shader_surfaces += 1")
            && custom_shader_diagnostics.contains("unsupported_custom_shader_vertices")
            && custom_shader_diagnostics.contains("unsupported_custom_shader_source_bytes")
            && custom_shader_diagnostics.contains("unsupported_custom_shader_uniform_bytes")
            && custom_shader_diagnostics.contains("unsupported_custom_shader_storage_bytes")
            && custom_shader_diagnostics
                .contains("fn custom_shader_unsupported_diagnostics_count_payload_bytes"),
        "native custom shader validation and fallback diagnostics should stay in the diagnostics module"
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
            && stats.contains("custom_shader_surfaces_rendered")
            && stats.contains("custom_shader_pipeline_rebuilds")
            && stats.contains("custom_shader_binding_rebuilds")
            && stats.contains("custom_shader_binding_cache_hits")
            && stats.contains("custom_shader_surfaces_failed")
            && stats.contains("custom_shader_shader_module_failures")
            && stats.contains("custom_shader_pipeline_failures")
            && stats.contains("custom_shader_binding_failures")
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
