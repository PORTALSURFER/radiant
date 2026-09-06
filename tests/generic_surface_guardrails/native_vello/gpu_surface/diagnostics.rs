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
    let custom_shader_pipeline_tests = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline/tests.rs",
    ))
    .expect("GPU surface custom shader pipeline tests should be readable");
    let custom_shader_pipeline_layout = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline/layout.rs",
    ))
    .expect("GPU surface custom shader pipeline layout module should be readable");
    let custom_shader_binding =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/binding.rs",
        ))
        .expect("GPU surface custom shader binding module should be readable");
    let custom_shader_diagnostics = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/diagnostics.rs",
    ))
    .expect("GPU surface custom shader diagnostics module should be readable");
    let custom_shader_draw = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/draw.rs"),
    )
    .expect("GPU surface custom shader draw module should be readable");
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
            && custom_shader.contains("#[path = \"custom_shader/draw.rs\"]")
            && custom_shader.contains("self.ensure_custom_shader_pipeline")
            && custom_shader.contains("self.ensure_custom_shader_binding")
            && custom_shader.contains("draw::upload_custom_shader_buffers")
            && custom_shader.contains("draw::encode_custom_shader_draw")
            && custom_shader.contains("record_unsupported_custom_shader")
            && custom_shader.contains("custom_shader.surfaces_rendered += 1")
            && !custom_shader.contains("fn ensure_custom_shader_pipeline")
            && !custom_shader.contains("fn ensure_custom_shader_binding")
            && !custom_shader.contains("fn upload_custom_shader_buffers")
            && !custom_shader.contains("fn encode_custom_shader_draw")
            && !custom_shader.contains("fn custom_shader_layout_entries")
            && !custom_shader.contains("fn custom_shader_buffer")
            && !custom_shader.contains("device.create_shader_module")
            && !custom_shader.contains("device.create_bind_group"),
        "native custom shader draw orchestration should stay focused while delegating WGPU setup"
    );
    assert!(
        custom_shader_pipeline.contains("fn ensure_custom_shader_pipeline")
            && custom_shader_pipeline.contains("struct CustomShaderPipelineRequest")
            && custom_shader_pipeline.contains("struct OwnedCustomShaderPipelineRequest")
            && custom_shader_pipeline.contains("fn custom_shader_pipeline_needs_rebuild")
            && custom_shader_pipeline.contains("#[path = \"pipeline/layout.rs\"]")
            && custom_shader_pipeline.contains("fn create_custom_shader_module")
            && custom_shader_pipeline.contains("fn prepare_custom_shader_pipeline")
            && custom_shader_pipeline.contains("create_custom_shader_bind_group_layout")
            && !custom_shader_pipeline.contains("fn custom_shader_layout_entries")
            && !custom_shader_pipeline.contains("fn custom_shader_buffer_layout_entry")
            && custom_shader_pipeline.contains(".create_shader_module")
            && custom_shader_pipeline.contains(".create_render_pipeline")
            && !custom_shader_pipeline.contains("custom_shader.pipeline_rebuilds += 1")
            && custom_shader_pipeline.contains("self.prepared_custom_shader_pipeline(&identity)")
            && custom_shader_pipeline
                .contains("custom_shader.failures.shader_module_failures += 1")
            && custom_shader_pipeline.contains("custom_shader.failures.pipeline_failures += 1")
            && custom_shader_pipeline.contains(".push_error_scope(wgpu::ErrorFilter::Validation)")
            && custom_shader_pipeline.contains("#[path = \"pipeline/tests.rs\"]")
            && custom_shader_pipeline_tests
                .contains("fn custom_shader_pipeline_key_tracks_payload_bindings"),
        "native custom shader pipeline setup and validation diagnostics should stay in the pipeline module"
    );
    assert!(
        custom_shader_pipeline_layout.contains("fn create_custom_shader_bind_group_layout")
            && custom_shader_pipeline_layout.contains("fn create_custom_shader_pipeline_layout")
            && custom_shader_pipeline_layout.contains("fn custom_shader_layout_entries")
            && custom_shader_pipeline_layout.contains("fn custom_shader_buffer_layout_entry")
            && custom_shader_pipeline_layout.contains("binding: 1")
            && custom_shader_pipeline_layout.contains("binding: 2")
            && custom_shader_pipeline_layout
                .contains("BufferBindingType::Storage { read_only: true }"),
        "native custom shader pipeline layout construction should stay in the layout module"
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
        custom_shader_draw.contains("struct CustomShaderBufferUploadRequest")
            && custom_shader_draw.contains("struct CustomShaderDrawRequest")
            && custom_shader_draw.contains("fn upload_custom_shader_buffers")
            && custom_shader_draw.contains("fn encode_custom_shader_draw")
            && custom_shader_draw.contains("uniforms_as_bytes")
            && custom_shader_draw.contains("gpu_surface_render_pass")
            && custom_shader_draw.contains("visible_surface_regions")
            && custom_shader_draw.contains("surface_dest")
            && custom_shader_draw.contains("pass.draw(0..request.descriptor.vertex_count"),
        "native custom shader draw encoding and per-frame uploads should stay in the draw module"
    );
    assert!(
        custom_shader_diagnostics.contains("fn custom_shader_validation_error")
            && custom_shader_diagnostics.contains("fn record_unsupported_custom_shader")
            && custom_shader_diagnostics.contains("custom_shader.failures.surfaces_failed += 1")
            && custom_shader_diagnostics.contains("error_scope.pop()")
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
            && stats.contains("pub(crate) static_writes")
            && stats.contains("pub(crate) static_write_bytes")
            && stats.contains("pub(crate) presentation_writes")
            && stats.contains("pub(crate) presentation_write_bytes")
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

#[test]
fn active_composited_base_profile_record_includes_retired_residency() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let render_profile = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/render_profile.rs"),
    )
    .expect("native render profile should be readable");
    let active_record = render_profile
        .split("gpu_surface_composited_base_active_generation_known =")
        .nth(1)
        .and_then(|tail| {
            tail.split("gpu_surface_composited_base_q0_generation_known =")
                .next()
        })
        .expect("active composited-base profile record should be present");

    assert!(
        active_record.contains(
            "gpu_surface_composited_base_active_retired_object_count =\n            active_composited_base.retired_object_count"
        ) && active_record.contains(
            "gpu_surface_composited_base_active_retired_requested_backing_bytes =\n            active_composited_base.retired_requested_backing_bytes"
        ),
        "active composited-base profile record should emit projected retired residency fields"
    );
}

#[test]
fn render_canvas_upload_evidence_stays_private_and_follows_actual_write_sites() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stats = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs"),
    )
    .expect("GPU surface stats module should be readable");
    let atlas_resources = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/atlas.rs"),
    )
    .expect("GPU surface atlas resources should be readable");
    let signal_resources = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs"),
    )
    .expect("GPU surface signal resources should be readable");
    let atlas = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/atlas.rs"),
    )
    .expect("GPU surface atlas renderer should be readable");
    let custom_shader_draw = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/draw.rs"),
    )
    .expect("GPU surface custom shader draw should be readable");
    let render_profile = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/render_profile.rs"),
    )
    .expect("native render profile should be readable");

    assert!(
        stats.contains("pub(crate) render_canvas_uploads")
            && stats.contains("struct GpuSurfaceRenderCanvasUploadStats")
            && stats.contains("operations: Option<usize>")
            && stats.contains("logical_bytes: Option<u64>")
            && stats.contains("operations.checked_add(1)")
            && stats.contains("logical_bytes.checked_add(byte_len)")
            && stats.contains("fn mark_unavailable"),
        "render-canvas upload evidence should be fixed-size, private, and checked"
    );
    assert!(
        atlas_resources.find("queue.write_texture").unwrap()
            < atlas_resources.find("record_immutable_payload").unwrap()
            && signal_resources.matches("record_immutable_payload").count() == 1
            && signal_resources
                .matches("record_renderer_parameter")
                .count()
                == 2
            && atlas.matches("record_renderer_parameter").count() == 1
            && custom_shader_draw
                .matches("record_renderer_parameter")
                .count()
                == 1
            && custom_shader_draw
                .matches("record_immutable_payload")
                .count()
                == 2
            && custom_shader_draw
                .matches("record_volatile_payload")
                .count()
                == 2,
        "each scoped native transfer family should record only after its existing write path"
    );
    for required in [
        "gpu_surface_render_canvas_upload_immutable_payload_operations",
        "gpu_surface_render_canvas_upload_immutable_payload_bytes",
        "gpu_surface_render_canvas_upload_volatile_payload_operations",
        "gpu_surface_render_canvas_upload_volatile_payload_bytes",
        "gpu_surface_render_canvas_upload_renderer_parameter_operations",
        "gpu_surface_render_canvas_upload_renderer_parameter_bytes",
    ] {
        assert!(
            render_profile.contains(required),
            "private render profile should expose upload evidence field `{required}`"
        );
    }
    for required in [
        "gpu_surface_render_canvas_upload_observed_candidate_plan_count",
        "gpu_surface_render_canvas_upload_observed_candidate_plan_window_count",
        "gpu_surface_render_canvas_upload_observed_candidate_no_work_count",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_count",
        "gpu_surface_render_canvas_upload_observed_candidate_invalid_count",
        "gpu_surface_render_canvas_upload_observed_candidate_unsupported_count",
        "gpu_surface_render_canvas_upload_observed_candidate_incomplete_count",
        "gpu_surface_render_canvas_upload_observed_candidate_overflow_count",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_operations",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_bytes",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_operations",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_bytes",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_operations",
        "gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_bytes",
    ] {
        assert!(
            render_profile.contains(required),
            "private render profile should expose observed candidate-plan field `{required}`"
        );
    }
}

#[test]
fn application_render_canvas_upload_aggregate_stays_private_to_native_profiling() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native runtime module should be readable");
    let adapter = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/adapter.rs"),
    )
    .expect("native adapter module should be readable");
    let public_diagnostics =
        fs::read_to_string(manifest_dir.join("src/runtime/diagnostics/gpu_surface.rs"))
            .expect("public GPU surface diagnostics should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("native runner should be readable");
    let gpu_surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer should be readable");

    assert!(
        module.contains("struct NativeAdapterRenderCanvasUploadAccountToken")
            && module.contains("struct NativeAdapterRenderCanvasUploadProfile")
            && adapter.contains("pub(super) struct NativeAdapterRenderCanvasUploadLedger")
            && adapter.contains("render_canvas_upload_ledger")
            && adapter.contains("NativeAdapterRenderCanvasUploadCandidateAggregate")
            && adapter.contains("Option<GpuSurfaceRenderCanvasUploadPlan>")
            && !adapter.contains("pub struct NativeAdapterRenderCanvasUploadLedger")
            && !public_diagnostics.contains("RenderCanvasUpload"),
        "application upload aggregation should remain crate-private and outside public diagnostics"
    );
    for field in [
        "pub(super) immutable_payload_operations",
        "pub(super) immutable_payload_logical_bytes",
        "pub(super) volatile_payload_operations",
        "pub(super) volatile_payload_logical_bytes",
        "pub(super) renderer_parameter_operations",
        "pub(super) renderer_parameter_logical_bytes",
    ] {
        assert!(
            module.contains(field),
            "private aggregate profile should retain `{field}`"
        );
    }
    for field in [
        "pub(super) observed_candidate_plan_count",
        "pub(super) observed_candidate_plan_window_count",
        "pub(super) observed_candidate_no_work_count",
        "pub(super) observed_candidate_exact_count",
        "pub(super) observed_candidate_invalid_count",
        "pub(super) observed_candidate_unsupported_count",
        "pub(super) observed_candidate_incomplete_count",
        "pub(super) observed_candidate_overflow_count",
        "pub(super) observed_candidate_exact_immutable_payload_operations",
        "pub(super) observed_candidate_exact_immutable_payload_logical_bytes",
        "pub(super) observed_candidate_exact_volatile_payload_operations",
        "pub(super) observed_candidate_exact_volatile_payload_logical_bytes",
        "pub(super) observed_candidate_exact_renderer_parameter_operations",
        "pub(super) observed_candidate_exact_renderer_parameter_logical_bytes",
    ] {
        assert!(
            module.contains(field),
            "private aggregate profile should retain `{field}`"
        );
    }
    assert!(
        !runner.contains("observed_candidate_")
            && !gpu_surface.contains("NativeAdapterRenderCanvasUploadProfile"),
        "candidate-plan aggregate evidence must not become a scheduler or renderer consumer"
    );
}

#[test]
fn render_canvas_upload_transaction_has_renderer_wide_preflight_scaffold() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let upload_plan = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/upload_plan.rs"),
    )
    .expect("render-canvas upload plan should be readable");
    let gpu_surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer should be readable");
    let composited_base = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base.rs"),
    )
    .expect("composited-base renderer should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present orchestration should be readable");

    let plan_marker = "struct GpuSurfaceRenderCanvasUploadPlan {";
    let plan_start = upload_plan
        .find(plan_marker)
        .expect("the private transaction plan should remain named and focused");
    let plan_prefix = &upload_plan[..plan_start];
    let derive_start = plan_prefix
        .rfind("#[derive(")
        .expect("the transaction plan should carry an explicit derive list");
    let plan_derive = &plan_prefix[derive_start..];
    assert!(
        plan_derive.starts_with("#[derive(Debug)]"),
        "the one-shot transaction plan must not become Clone or Copy"
    );
    for required in [
        "actions: Vec<GpuSurfaceRenderCanvasUploadAction>",
        "consumed: bool",
        "fn preflight(",
        "fn begin_execution(",
        "fn consume_action(",
        "fn finish_execution(",
        "EnsurePipeline",
        "SignalValidation",
        "CustomPresentationState",
    ] {
        assert!(
            upload_plan.contains(required),
            "one-shot transaction plan should retain `{required}`"
        );
    }

    let preflight = gpu_surface
        .find("fn preflight_render_canvas_upload_plan")
        .expect("renderer-wide preflight should be present");
    let render = gpu_surface
        .find("pub(super) fn render(")
        .expect("renderer execution should remain the single render entry point");
    assert!(
        preflight < render,
        "preflight should be declared before execution"
    );
    assert!(
        gpu_surface.contains("GpuSurfaceRenderCanvasUploadPlan::preflight")
            && gpu_surface
                .contains("for (surface_index, primitive) in primitives.iter().enumerate()")
            && gpu_surface.contains("target.upload_plan.take()")
            && gpu_surface.contains("plan.begin_execution(")
            && gpu_surface.contains("plan.finish_execution()")
            && gpu_surface.contains(".collect_upload_plan")
            && gpu_surface.contains("then_some(target.upload_plan_context)")
            && !upload_plan.contains("primitives.to_vec()")
            && !upload_plan.contains("clone_from(primitives"),
        "the scaffold should preflight the complete borrowed stream and consume one plan"
    );

    let base_preflight = composited_base
        .find("preflight_render_canvas_upload_plan(")
        .expect("composited-base presentation should invoke renderer preflight");
    let base_render = composited_base
        .find(".render(")
        .expect("composited-base presentation should retain renderer execution");
    assert!(
        base_preflight < base_render,
        "composited-base orchestration should preflight before renderer execution"
    );
    assert!(
        composited_base.contains("collect_upload_plan: request.collect_upload_plan")
            && present.contains("let upload_plan_context = self.window.native_resources")
            && !present.contains("profile_enabled.then_some(upload_plan_context)"),
        "the transaction context should be available to execution even when profiling is off"
    );
}
