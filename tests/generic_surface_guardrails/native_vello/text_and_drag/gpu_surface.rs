use super::*;

#[test]
fn native_gpu_surface_wheel_coalescing_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let interaction = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface_interaction.rs"),
    )
    .expect("GPU surface interaction module should be readable");
    let wheel = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface_wheel.rs"),
    )
    .expect("GPU surface wheel module should be readable");

    assert!(
        module.contains("mod gpu_surface_wheel;")
            && module.contains("use gpu_surface_wheel::PendingGpuSurfaceWheel;"),
        "generic runtime should keep pending wheel state owned by the wheel module"
    );
    assert!(
        !interaction.contains("struct PendingGpuSurfaceWheel")
            && !interaction.contains("fn flush_pending_gpu_surface_wheel")
            && wheel.contains("struct PendingGpuSurfaceWheel")
            && wheel.contains("fn flush_pending_gpu_surface_wheel")
            && wheel.contains("coalesced_wheel"),
        "wheel coalescing should stay separate from pointer hover overlay interaction"
    );
}

#[test]
fn native_gpu_surface_interaction_region_model_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let collector = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions.rs",
    ))
    .expect("GPU surface interaction region collector should be readable");
    let region = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions/region.rs",
    ))
    .expect("GPU surface interaction region model should be readable");
    let tests = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions/tests.rs",
    ))
    .expect("GPU surface interaction region tests should be readable");

    assert!(
        collector.contains("mod region;")
            && collector.contains("#[cfg(test)]")
            && collector.contains("mod tests;")
            && collector.contains(
                "pub(in crate::gui_runtime::native_vello) use region::GpuSurfaceInteractionRegion;"
            ),
        "GPU surface interaction collection should delegate region state and capability filtering"
    );
    assert!(
        !collector.contains("struct GpuSurfaceInteractionRegion")
            && !collector.contains("fn from_gpu_surface")
            && region.contains("struct GpuSurfaceInteractionRegion")
            && region.contains("fn from_gpu_surface")
            && region.contains("fn contains"),
        "GPU surface interaction region model and renderability checks should live in runtime_helpers/gpu_surface_regions/region.rs"
    );
    assert!(
        !collector.contains("fn gpu_surface_interaction_region_collection_reuses_existing_buffer")
            && tests
                .contains("fn gpu_surface_interaction_region_collection_reuses_existing_buffer")
            && tests.contains("fn gpu_surface_interaction_regions_skip_opaque_later_panels")
            && tests.contains("fn gpu_surface_interaction_regions_reject_nonfinite_geometry"),
        "GPU surface interaction collector regression tests should stay in runtime_helpers/gpu_surface_regions/tests.rs"
    );
}
