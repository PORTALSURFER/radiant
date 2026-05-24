//! Public native runtime diagnostics models.

mod cache_policy;
mod frame;
mod gpu_surface;
mod retained_surface;
mod scene;
mod text;
mod timing;

pub use cache_policy::RetainedSurfaceCachePolicy;
pub use frame::NativeFrameDiagnostics;
pub use gpu_surface::{
    NativeGpuSurfaceAtlasDiagnostics, NativeGpuSurfaceCompositeDiagnostics,
    NativeGpuSurfaceCustomShaderDiagnostics, NativeGpuSurfaceCustomShaderFailureDiagnostics,
    NativeGpuSurfaceDiagnostics, NativeGpuSurfaceSignalDiagnostics,
    NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
};
pub use retained_surface::NativeRetainedSurfaceDiagnostics;
pub use scene::{
    NativeSceneDiagnostics, NativeSceneMediaDiagnostics, NativeSceneSurfaceDiagnostics,
    NativeSceneTextDiagnostics, NativeSceneTraversalDiagnostics,
};
pub use text::{
    NativeTextCacheCounters, NativeTextCacheDiagnostics, NativeTextDiagnostics,
    NativeTextQualityDiagnostics, NativeTextQualityStatus,
};
pub use timing::{
    NativeCompositedBaseTiming, NativeFrameTimingDiagnostics, NativeFrameWorkTimings,
    NativeGpuTimingStatus, NativeTransientOverlayTiming,
};
