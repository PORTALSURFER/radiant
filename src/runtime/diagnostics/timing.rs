use std::time::Duration;

/// Coarse timing diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameTimingDiagnostics {
    /// Current source and precision of GPU timing information for this frame.
    pub gpu_timing_status: NativeGpuTimingStatus,
    /// CPU-side work buckets performed while preparing the frame.
    pub frame_work: NativeFrameWorkTimings,
    /// Timing and cache state for the composited base frame.
    pub composited_base: NativeCompositedBaseTiming,
    /// Timing and primitive count for host-supplied transient overlays.
    pub transient_overlay: NativeTransientOverlayTiming,
    /// Time spent submitting GPU work and presenting the surface.
    pub submit_present: Duration,
    /// Time since the previous successful present.
    pub since_last_present: Duration,
}

/// CPU-side work buckets performed while preparing one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameWorkTimings {
    /// Time spent routing a coalesced GPU-surface wheel event.
    pub coalesced_wheel_route: Duration,
    /// Time spent refreshing the runtime surface snapshot.
    pub refresh_surface: Duration,
    /// Time spent pulling the host application projection.
    pub application_projection: Duration,
    /// Time spent rebuilding runtime projection and traversal.
    pub runtime_projection: Duration,
    /// Time spent synchronizing stateful widgets.
    pub widget_state_sync: Duration,
    /// Time spent recomputing layout.
    pub layout: Duration,
    /// Time spent building the backend-neutral paint plan.
    pub paint_plan: Duration,
    /// Time spent rendering the scene to the cached texture.
    pub render_to_texture: Duration,
    /// Time spent encoding the full-screen blit/composite pass.
    pub full_screen_blit: Duration,
}

/// Timing and cache state for a composited base frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeCompositedBaseTiming {
    /// Time spent refreshing the composited base frame.
    pub refresh: Duration,
    /// Whether the composited base frame was reused from cache.
    pub cache_hit: bool,
}

/// Timing and primitive count for host-supplied transient overlays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTransientOverlayTiming {
    /// Time spent collecting transient overlay primitives.
    pub paint: Duration,
    /// Transient overlay primitive count.
    pub primitives: usize,
}

impl NativeFrameTimingDiagnostics {
    /// Return the sum of the tracked CPU-side frame timing buckets.
    ///
    /// This intentionally excludes [`Self::since_last_present`], which is a
    /// cadence interval rather than work performed for the current frame. When
    /// [`Self::gpu_timing_status`] is [`NativeGpuTimingStatus::CpuEnvelopeOnly`],
    /// this total remains an encode/submit/present envelope, not a backend GPU
    /// execution duration.
    pub fn cpu_envelope_total(self) -> Duration {
        self.frame_work.total()
            + self.composited_base.refresh
            + self.transient_overlay.paint
            + self.submit_present
    }
}

impl NativeFrameWorkTimings {
    /// Return the sum of tracked CPU-side frame preparation buckets.
    pub fn total(self) -> Duration {
        self.coalesced_wheel_route
            + self.refresh_surface
            + self.paint_plan
            + self.render_to_texture
            + self.full_screen_blit
    }
}

/// GPU timing availability for native frame diagnostics.
///
/// Radiant currently exposes CPU-side encode, submit, and present timing
/// buckets. True GPU timestamp queries are backend- and adapter-dependent, so
/// hosts should inspect this status before treating frame timings as GPU
/// execution duration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeGpuTimingStatus {
    /// No backend GPU timestamp data was collected for this frame.
    #[default]
    CpuEnvelopeOnly,
}
