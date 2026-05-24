use std::{fs, path::PathBuf};

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
            && stats.contains("pub(crate) atlas: GpuSurfaceAtlasRenderStats")
            && stats.contains("pub(crate) signal: GpuSurfaceSignalRenderStats")
            && stats.contains("pub(crate) composite: GpuSurfaceCompositeRenderStats")
            && stats.contains("pub(crate) custom_shader: GpuSurfaceCustomShaderRenderStats"),
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
            && custom_shader.contains("custom_shader.surfaces_rendered += 1")
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
            && custom_shader_pipeline.contains("custom_shader.pipeline_rebuilds += 1")
            && custom_shader_pipeline
                .contains("custom_shader.failures.shader_module_failures += 1")
            && custom_shader_pipeline.contains("custom_shader.failures.pipeline_failures += 1")
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
            && custom_shader_binding.contains("custom_shader.binding_rebuilds += 1")
            && custom_shader_binding.contains("custom_shader.binding_cache_hits += 1")
            && custom_shader_binding.contains("custom_shader.failures.binding_failures += 1")
            && custom_shader_binding
                .contains("device.push_error_scope(wgpu::ErrorFilter::Validation)"),
        "native custom shader bind-group and payload-buffer setup should stay in the binding module"
    );
    assert!(
        custom_shader_diagnostics.contains("fn custom_shader_validation_error")
            && custom_shader_diagnostics.contains("fn record_unsupported_custom_shader")
            && custom_shader_diagnostics.contains("custom_shader.failures.surfaces_failed += 1")
            && custom_shader_diagnostics.contains("device.pop_error_scope()")
            && custom_shader_diagnostics.contains("custom_shader.unsupported.surfaces += 1")
            && custom_shader_diagnostics.contains("custom_shader.unsupported.vertices")
            && custom_shader_diagnostics.contains("custom_shader.unsupported.source_bytes")
            && custom_shader_diagnostics.contains("custom_shader.unsupported.uniform_bytes")
            && custom_shader_diagnostics.contains("custom_shader.unsupported.storage_bytes")
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
        stats.contains("pub(crate) surfaces_rendered")
            && stats.contains("pub(crate) pipeline_rebuilds")
            && stats.contains("pub(crate) binding_rebuilds")
            && stats.contains("pub(crate) binding_cache_hits")
            && stats.contains("pub(crate) surfaces_failed")
            && stats.contains("pub(crate) shader_module_failures")
            && stats.contains("pub(crate) pipeline_failures")
            && stats.contains("pub(crate) binding_failures")
            && stats.contains("pub(crate) source_bytes")
            && stats.contains("pub(crate) uniform_bytes")
            && stats.contains("pub(crate) storage_bytes"),
        "GPU surface render stats should keep skipped custom-shader draw and payload counters"
    );
}
