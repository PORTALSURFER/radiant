use super::{prelude_source, radiant_source};

#[test]
fn gpu_surface_input_builder_uses_named_parts_for_message_mapping() {
    let application_builder = radiant_source("src/application/builders/leaf/gpu.rs");
    let application_facade = radiant_source("src/application/facade/surfaces.rs");
    let runtime = radiant_source("src/runtime/mod.rs");
    let prelude_surfaces = radiant_source("src/prelude/application/surfaces.rs");
    let prelude = prelude_source();
    let app_test = radiant_source("tests/app_runtime_api/render_canvas.rs");

    assert!(
        application_builder.contains("pub struct GpuSurfaceInputParts<Map>")
            && application_builder.contains("pub fn gpu_surface_input_from_parts<Message, Map>(")
            && application_builder.contains("GpuSurfaceInputParts {")
            && application_builder.contains("key: parts.key")
            && application_builder.contains("revision: parts.revision")
            && application_builder.contains("content: parts.content")
            && application_builder.contains("(parts.map)(input)"),
        "input-emitting GPU surface builders should use named parts for resource identity and message mapping"
    );
    assert!(
        application_facade.contains("GpuSurfaceInputParts")
            && application_facade.contains("gpu_surface_input_from_parts")
            && runtime.contains("GpuSurfaceInputParts")
            && runtime.contains("gpu_surface_input_from_parts")
            && prelude_surfaces.contains("render_canvas")
            && !prelude_surfaces.contains("gpu_surface")
            && !prelude.contains("GpuSurfaceInputParts")
            && !prelude.contains("gpu_surface_input_from_parts"),
        "GPU surface input parts should remain public through application and runtime without entering the common prelude"
    );
    assert!(
        app_test.contains("render_canvas_input_from_parts(RenderCanvasInputParts {")
            && app_test
                .contains("fn app_render_canvas_input_helper_routes_through_normal_message_path"),
        "tests should cover both named-parts construction and the canonical helper"
    );
}
