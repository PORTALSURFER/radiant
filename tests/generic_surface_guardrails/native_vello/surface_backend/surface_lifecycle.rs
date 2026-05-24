use super::read_runtime_source;

#[test]
fn native_surface_texture_acquire_stays_with_surface_lifecycle() {
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let surface = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface.rs");

    assert!(
        present.contains("self.acquire_present_surface_texture(event_loop, &window)")
            && !present.contains("get_current_texture()")
            && !present.contains("SurfaceError::OutOfMemory"),
        "present driver should delegate WGPU surface texture acquisition and recovery"
    );
    assert!(
        surface.contains("fn acquire_present_surface_texture")
            && surface.contains("get_current_texture()")
            && surface.contains("SurfaceError::Lost | wgpu::SurfaceError::Outdated")
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
            && surface.contains("render_context_for_options(&self.options)")
            && !surface.contains("fn wgpu_backends")
            && !surface.contains("InstanceDescriptor"),
        "surface lifecycle should delegate explicit WGPU backend policy"
    );
    assert!(
        backend.contains("fn render_context_for_options")
            && backend.contains("fn wgpu_backends")
            && backend.contains("NativeGpuBackend::Auto")
            && backend.contains("wgpu::InstanceDescriptor"),
        "WGPU backend selection and render-context construction should live in surface/backend.rs"
    );
}
