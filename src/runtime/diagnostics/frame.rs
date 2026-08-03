use super::{
    NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics, NativeRetainedSurfaceDiagnostics,
    NativeSceneDiagnostics, NativeTextDiagnostics,
};

/// Opaque identity for one native window runner within one native runtime run.
///
/// The value is allocated by the native runtime and can only be inspected
/// through [`Self::get`]. Pair it with a frame sequence when correlating
/// diagnostics across primary and auxiliary windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeWindowDiagnosticIdentity(u64);

impl NativeWindowDiagnosticIdentity {
    /// Return the numeric identity for host diagnostics or export.
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_runtime_value(value: u64) -> Self {
        Self(value)
    }
}

/// Cumulative, bounded observations of native surface recovery for one window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSurfaceRecoveryDiagnostics {
    /// Number of surface-acquisition failures reported as lost.
    pub lost: u64,
    /// Number of surface-acquisition failures reported as outdated.
    pub outdated: u64,
    /// Number of surface-acquisition failures reported as timeouts.
    pub timeouts: u64,
    /// Number of surface-acquisition failures reported as other errors.
    pub others: u64,
    /// Number of forced surface reconfigurations that completed.
    pub completed_reconfigures: u64,
    /// Number of lost or outdated acquisitions deferred while the window had
    /// a zero width or height.
    pub zero_size_deferrals: u64,
    /// Number of redraw retries requested after a completed reconfiguration.
    pub retry_requests: u64,
    /// Number of one-shot redraw retries requested after a timeout.
    pub timeout_retry_requests: u64,
    /// Number of one-shot redraw retries requested after an other error.
    pub other_retry_requests: u64,
}

/// Structured diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameDiagnostics {
    /// Opaque identity of the native window runner that presented this frame.
    /// Pair it with [`Self::frame_sequence`] to correlate frames across
    /// primary and auxiliary windows.
    pub window_identity: Option<NativeWindowDiagnosticIdentity>,
    /// Monotonic sequence for this native window's successfully presented
    /// frames. It starts at one and remains scoped to the window across
    /// recovery; `None` means no presentation has occurred yet or the `u64`
    /// counter is exhausted without wrapping or reusing a value.
    pub frame_sequence: Option<u64>,
    /// Redraw routing metadata for the presented native frame.
    pub presentation: NativeFramePresentationDiagnostics,
    /// Cumulative native surface recovery observations for the window.
    pub surface_recovery: NativeSurfaceRecoveryDiagnostics,
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
    /// Typed surface invalidation stage selected by the runtime.
    pub surface_invalidation: &'static str,
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
            surface_invalidation: "none",
            paint_only: false,
            scene_rebuild: false,
        }
    }
}
