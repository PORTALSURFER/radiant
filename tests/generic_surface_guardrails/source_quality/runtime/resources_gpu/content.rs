use super::radiant_source;

#[test]
fn gpu_surface_content_models_stay_focused() {
    let content = radiant_source("src/runtime/gpu_surface/content.rs");
    let model = radiant_source("src/runtime/gpu_surface/content/model.rs");
    let validation = radiant_source("src/runtime/gpu_surface/content/validation.rs");
    let error = radiant_source("src/runtime/gpu_surface/content/error.rs");
    let error_display = radiant_source("src/runtime/gpu_surface/content/error/display.rs");
    let tests = radiant_source("src/runtime/gpu_surface/content/tests.rs");
    let atlas_tests = radiant_source("src/runtime/gpu_surface/content/tests/atlas.rs");
    let signal_tests = radiant_source("src/runtime/gpu_surface/content/tests/signal_shape.rs");
    let validation_tests = radiant_source("src/runtime/gpu_surface/content/tests/validation.rs");
    let gpu_surface = radiant_source("src/runtime/gpu_surface.rs");

    assert!(
        content.contains("mod model;")
            && content.contains("GpuShaderSurfaceDescriptor")
            && content.contains("GpuShaderSurfaceDescriptorParts")
            && content.contains("GpuSignalGainPreview")
            && content.contains("GpuSignalRenderShape")
            && content.contains("pub enum GpuSurfaceContent")
            && !content.contains("pub struct GpuSignalGainPreview")
            && !content.contains("pub struct GpuSignalRenderShape"),
        "GPU surface content root should expose the retained content enum while re-exporting focused signal content models"
    );
    assert!(
        model.contains("pub struct GpuShaderSurfaceDescriptor")
            && model.contains("pub struct GpuShaderSurfaceDescriptorParts")
            && model.contains("pub shader_key: String")
            && model.contains("pub wgsl_source: Option<Arc<str>>")
            && model.contains("pub fragment_entry_point: Option<String>")
            && model.contains("pub fn wgsl_source")
            && model.contains("pub fn fragment_entry_point")
            && model.contains("pub struct GpuSignalGainPreview")
            && model.contains("pub fade_in_length: f32")
            && model.contains("pub struct GpuSignalRenderShape")
            && model.contains("pub sample_count: usize"),
        "GPU shader descriptor, signal gain-preview, and render-shape DTOs should live in content/model.rs"
    );
    assert!(
        validation.contains("validate_signal_gain_preview")
            && validation.contains("validate_shader_descriptor")
            && validation.contains("validate_signal_render_shape"),
        "GPU surface content validation should stay in the validation module"
    );
    assert!(
        error.contains("mod display;")
            && error.contains("pub enum GpuSurfaceContentError")
            && !error.contains("impl fmt::Display for GpuSurfaceContentError")
            && error_display.contains("impl fmt::Display for GpuSurfaceContentError")
            && error_display.contains("impl std::error::Error for GpuSurfaceContentError"),
        "GPU surface validation error models should delegate formatting to error/display.rs"
    );
    assert!(
        tests.contains("mod atlas;")
            && tests.contains("mod signal_shape;")
            && tests.contains("mod validation;")
            && !tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && !tests.contains("fn signal_render_shape_rejects_invalid_payload_contracts"),
        "GPU surface content test root should index focused content behavior groups instead of owning all cases"
    );
    assert!(
        atlas_tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && signal_tests.contains("fn signal_render_shape_uses_effective_available_frame_count")
            && validation_tests
                .contains("fn gpu_surface_content_validation_rejects_non_finite_gain_preview"),
        "GPU surface content behavior tests should stay grouped by atlas, signal shape, and validation concerns"
    );
    assert!(
        gpu_surface.contains("GpuShaderSurfaceDescriptor")
            && gpu_surface.contains("GpuShaderSurfaceDescriptorParts")
            && gpu_surface.contains("GpuSignalGainPreview")
            && gpu_surface.contains("GpuSignalRenderShape")
            && gpu_surface.contains("GpuSurfaceContent")
            && gpu_surface.contains("GpuSurfaceContentError"),
        "GPU surface content models and diagnostics should remain available through the runtime facade"
    );
}
