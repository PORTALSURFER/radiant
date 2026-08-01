use super::read_runtime_source;

#[test]
fn native_surface_texture_acquire_stays_with_surface_lifecycle() {
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let surface = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface.rs");

    assert!(
        present.contains("self.acquire_present_surface_texture(event_loop, adapter)")
            && present.contains("self.window.window.is_none()")
            && !present.contains("window.clone()")
            && !present.contains("get_current_texture()")
            && !present.contains("SurfaceError::OutOfMemory"),
        "present driver should delegate WGPU surface texture acquisition and recovery without cloning the window on every frame"
    );
    assert!(
        surface.contains("fn acquire_present_surface_texture")
            && surface.contains("get_current_texture()")
            && surface.contains("SurfaceError::Lost | wgpu::SurfaceError::Outdated")
            && surface.contains("window.inner_size()")
            && surface.contains("SurfaceError::OutOfMemory"),
        "surface texture acquisition and surface-error handling should stay with surface lifecycle"
    );
}

#[test]
fn native_surface_backend_policy_stays_in_focused_module() {
    let surface = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface.rs");
    let backend =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface/backend.rs");

    assert!(
        surface.contains("mod backend;")
            && surface.contains("GenericNativeAdapterOwner")
            && surface.contains("adapter\n            .instance()")
            && surface.contains("adapter\n            .create_render_surface")
            && surface.contains("adapter.resize_surface")
            && !surface.contains("fn wgpu_backends")
            && !surface.contains("InstanceDescriptor")
            && !surface.contains("RenderContext"),
        "surface lifecycle should delegate device and context operations to the adapter owner"
    );
    assert!(
        backend.contains("fn instance_for_options")
            && backend.contains("fn wgpu_backends")
            && backend.contains("NativeGpuBackend::Auto")
            && backend.contains("wgpu::InstanceDescriptor"),
        "WGPU backend selection and instance construction should remain in surface/backend.rs"
    );
}
