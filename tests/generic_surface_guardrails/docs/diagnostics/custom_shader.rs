use super::{normalized, read_project_file};

#[test]
fn api_docs_describe_custom_shader_frame_diagnostics() {
    let docs = read_project_file("docs/API.md");
    let runtime_diagnostics = read_project_file("src/runtime/diagnostics/gpu_surface.rs");
    let native_diagnostics =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs");
    let render_profile =
        read_project_file("src/gui_runtime/native_vello/generic_runtime/render_profile.rs");

    let normalized_docs = normalized(&docs);
    for required in [
        "custom shader pipeline rebuilds",
        "`NativeGpuSurfaceDiagnostics::custom_shader_surfaces_rendered`",
        "`custom_shader_pipeline_rebuilds`",
        "`custom_shader_binding_rebuilds`",
        "`custom_shader_binding_cache_hits`",
        "`custom_shader_surfaces_failed`",
        "`custom_shader_shader_module_failures`",
        "`custom_shader_pipeline_failures`",
        "`custom_shader_binding_failures`",
        "the native renderer also logs the backend validation error through tracing",
    ] {
        assert!(
            normalized_docs.contains(required),
            "API docs should describe custom shader frame diagnostics with `{required}`"
        );
    }
    for required in [
        "custom_shader_surfaces_rendered",
        "custom_shader_pipeline_rebuilds",
        "custom_shader_binding_rebuilds",
        "custom_shader_binding_cache_hits",
        "custom_shader_surfaces_failed",
        "custom_shader_shader_module_failures",
        "custom_shader_pipeline_failures",
        "custom_shader_binding_failures",
    ] {
        assert!(
            runtime_diagnostics.contains(required)
                && native_diagnostics.contains(required)
                && render_profile.contains(required),
            "custom shader diagnostic field `{required}` should flow through public diagnostics and the render profile"
        );
    }
}
