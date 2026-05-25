use super::{prelude_source, radiant_source};

#[test]
fn gpu_surface_input_builder_uses_named_parts_for_message_mapping() {
    let application_builder = radiant_source("src/application/builders/leaf.rs");
    let application = radiant_source("src/application.rs");
    let prelude = prelude_source();
    let app_test = radiant_source("tests/app_runtime_api/gpu_surface.rs");

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
        application.contains("GpuSurfaceInputParts")
            && application.contains("gpu_surface_input_from_parts")
            && prelude.contains("GpuSurfaceInputParts")
            && prelude.contains("gpu_surface_input_from_parts"),
        "GPU surface input parts should remain exported through the application facade and prelude"
    );
    assert!(
        app_test.contains("gpu_surface_input_from_parts(GpuSurfaceInputParts {")
            && app_test
                .contains("fn app_gpu_surface_input_helper_routes_through_normal_message_path"),
        "tests should cover both named-parts construction and the compatibility helper"
    );
}
