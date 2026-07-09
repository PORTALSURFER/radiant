use super::{
    NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics, NativeRetainedSurfaceDiagnostics,
    NativeSceneDiagnostics, NativeTextDiagnostics,
};

/// Structured diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameDiagnostics {
    /// Redraw routing metadata for the presented native frame.
    pub presentation: NativeFramePresentationDiagnostics,
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

/// Native redraw routing metadata for one presented frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeFramePresentationDiagnostics {
    /// Coarse frame-work kind selected by native event/runtime routing.
    pub frame_work_kind: &'static str,
    /// Stable reason label for the frame-work request.
    pub frame_work_reason: &'static str,
    /// Whether the frame-work path stayed on paint-only redraw.
    pub paint_only: bool,
    /// Whether the frame-work path required a scene rebuild.
    pub scene_rebuild: bool,
}

impl Default for NativeFramePresentationDiagnostics {
    fn default() -> Self {
        Self {
            frame_work_kind: "none",
            frame_work_reason: "none",
            paint_only: false,
            scene_rebuild: false,
        }
    }
}
