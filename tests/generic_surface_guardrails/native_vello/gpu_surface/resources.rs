use std::{fs, path::PathBuf};

#[test]
fn native_gpu_surface_resource_lifecycle_stays_with_resource_cache() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let resources = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources.rs"),
    )
    .expect("GPU surface resources module should be readable");
    let cache = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs"),
    )
    .expect("GPU surface resource cache module should be readable");

    assert!(
        resources.contains("mod cache;")
            && resources.contains("pub(super) use cache::GpuSurfaceResourceCache;")
            && renderer.contains("resources: GpuSurfaceResourceCache"),
        "GPU surface renderer should store retained WGPU resources through a focused resource cache"
    );
    assert!(
        cache.contains("struct GpuSurfaceResourceCache")
            && cache.contains("fn prune_inactive")
            && cache.contains("fn clear")
            && !renderer.contains("textures: HashMap")
            && !renderer.contains("signal_summaries: HashMap"),
        "resource-map lifecycle should live with the resource cache, not top-level renderer fields"
    );
}

#[test]
fn native_gpu_signal_summary_cache_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let signal = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs"),
    )
    .expect("GPU signal resource module should be readable");
    let summary = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal/summary.rs",
    ))
    .expect("GPU signal summary cache module should be readable");

    assert!(
        signal.contains("mod summary;")
            && signal.contains("fn ensure_signal_body_texture")
            && signal.contains("fn ensure_signal_buffer")
            && !signal.contains("fn cached_signal_summary")
            && !signal.contains("signal_summary_cache_hits"),
        "GPU signal resource upload/rendering should delegate CPU summary caching"
    );
    assert!(
        summary.contains("fn cached_signal_summary")
            && summary.contains("signal_summary_cache_hits")
            && summary.contains("signal_summary_builds")
            && summary.contains("GpuSignalSummary::from_interleaved_samples"),
        "GPU signal summary memoization should live in resources/signal/summary.rs"
    );
}
