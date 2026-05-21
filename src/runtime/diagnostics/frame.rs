use super::{
    NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics, NativeRetainedSurfaceDiagnostics,
    NativeSceneDiagnostics, NativeTextDiagnostics,
};

/// Structured diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameDiagnostics {
    /// Scene and retained-surface encoding counters.
    pub scene: NativeSceneDiagnostics,
    /// Native text layout cache activity.
    pub text: NativeTextDiagnostics,
    /// Retained custom-surface cache state and activity.
    pub retained_surfaces: NativeRetainedSurfaceDiagnostics,
    /// GPU-surface cache and render activity.
    pub gpu_surfaces: NativeGpuSurfaceDiagnostics,
    /// Coarse timing buckets for presentation work.
    pub timings: NativeFrameTimingDiagnostics,
}
