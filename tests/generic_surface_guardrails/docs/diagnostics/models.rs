use super::read_project_file;

#[test]
fn runtime_diagnostics_models_stay_in_focused_modules() {
    let root = read_project_file("src/runtime/diagnostics.rs");
    let frame = read_project_file("src/runtime/diagnostics/frame.rs");
    let text = read_project_file("src/runtime/diagnostics/text.rs");
    let timing = read_project_file("src/runtime/diagnostics/timing.rs");
    let gpu_surface = read_project_file("src/runtime/diagnostics/gpu_surface.rs");
    let retained_surface = read_project_file("src/runtime/diagnostics/retained_surface.rs");
    let scene = read_project_file("src/runtime/diagnostics/scene.rs");
    let cache_policy = read_project_file("src/runtime/diagnostics/cache_policy.rs");

    assert!(
        root.contains("mod frame;")
            && root.contains("mod text;")
            && root.contains("mod timing;")
            && root.contains("mod gpu_surface;")
            && root.contains("mod retained_surface;")
            && root.contains("mod scene;")
            && root.contains("mod cache_policy;")
            && root.contains("pub use frame::NativeFrameDiagnostics")
            && root.contains("NativeTextDiagnostics")
            && root.contains("NativeTextQualityStatus")
            && root.contains("NativeGpuSurfaceCustomShaderDiagnostics")
            && !root.contains("pub struct NativeTextDiagnostics")
            && !root.contains("pub struct NativeGpuSurfaceDiagnostics"),
        "runtime diagnostics root should index focused public diagnostics modules"
    );
    assert!(
        frame.contains("pub struct NativeFrameDiagnostics")
            && text.contains("pub struct NativeTextDiagnostics")
            && text.contains("pub struct NativeTextCacheDiagnostics")
            && text.contains("pub struct NativeTextCacheCounters")
            && text.contains("pub struct NativeTextQualityDiagnostics")
            && text.contains("pub enum NativeTextQualityStatus")
            && timing.contains("pub struct NativeFrameTimingDiagnostics")
            && timing.contains("pub struct NativeFrameWorkTimings")
            && timing.contains("pub struct NativeCompositedBaseTiming")
            && timing.contains("pub struct NativeTransientOverlayTiming")
            && timing.contains("pub enum NativeGpuTimingStatus")
            && gpu_surface.contains("pub struct NativeGpuSurfaceDiagnostics")
            && retained_surface.contains("pub struct NativeRetainedSurfaceDiagnostics")
            && scene.contains("pub struct NativeSceneDiagnostics")
            && scene.contains("pub struct NativeSceneTraversalDiagnostics")
            && scene.contains("pub struct NativeSceneTextDiagnostics")
            && scene.contains("pub struct NativeSceneMediaDiagnostics")
            && scene.contains("pub struct NativeSceneSurfaceDiagnostics")
            && cache_policy.contains("pub struct RetainedSurfaceCachePolicy"),
        "each runtime diagnostics concern should live with its focused model and policy"
    );
}
