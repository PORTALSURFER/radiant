use super::radiant_source;

#[test]
fn gpu_surface_primitive_keeps_surface_builders_focused() {
    let root = radiant_source("src/widgets/primitives/gpu_surface.rs");
    let builders = radiant_source("src/widgets/primitives/gpu_surface/builders.rs");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct GpuSurfaceWidget")
            && root.contains("impl Widget for GpuSurfaceWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "gpu-surface primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn gpu_surface("),
        "gpu-surface runtime builder helper should live in gpu_surface/builders.rs"
    );
}
