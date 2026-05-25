use super::*;

#[test]
fn native_gpu_surface_custom_shader_pipeline_uses_named_requests() {
    let custom_shader = gpu_surface_source(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader.rs",
    );
    let custom_shader_pipeline = gpu_surface_source(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline.rs",
    );

    assert!(
        custom_shader.contains("CustomShaderPipelineRequest")
            && custom_shader_pipeline.contains("pub(super) struct CustomShaderPipelineRequest")
            && custom_shader_pipeline.contains("struct CreatedCustomShaderPipeline")
            && custom_shader_pipeline.contains("fn create_custom_shader_module")
            && custom_shader_pipeline.contains("fn create_custom_shader_pipeline")
            && custom_shader_pipeline.contains("fn custom_shader_pipeline_needs_rebuild")
            && !custom_shader.contains("surface.key,\r\n            target.device")
            && !custom_shader_pipeline.contains("Option<(wgpu::BindGroupLayout"),
        "custom shader rendering should pass named pipeline requests and keep validation/build/cache steps explicit"
    );
}
