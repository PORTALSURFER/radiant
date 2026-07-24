use super::radiant_source;

#[test]
fn gpu_surface_widget_uses_named_parts_for_retained_resource_identity() {
    let source = radiant_source("src/widgets/primitives/gpu_surface.rs");
    let widgets = radiant_source("src/widgets/mod.rs");
    let application_builder = radiant_source("src/application/builders/leaf/gpu.rs");

    assert!(
        source.contains("pub struct GpuSurfaceParts")
            && source.contains("pub fn from_parts(parts: GpuSurfaceParts) -> Self"),
        "retained GPU surfaces should expose named parts for resource identity, revision, and content"
    );
    assert!(
        source.contains("Self::from_parts(GpuSurfaceParts {")
            && widgets.contains("GpuSurfaceParts")
            && application_builder.contains("pub fn gpu_surface_from_parts")
            && application_builder.contains("pub fn render_canvas_from_parts"),
        "retained canvas constructors, public exports, and application builders should keep named-parts paths available"
    );
}
