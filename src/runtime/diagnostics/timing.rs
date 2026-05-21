use std::time::Duration;

/// Coarse timing diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameTimingDiagnostics {
    /// Current source and precision of GPU timing information for this frame.
    pub gpu_timing_status: NativeGpuTimingStatus,
    /// Time spent routing a coalesced GPU-surface wheel event.
    pub coalesced_wheel_route: Duration,
    /// Time spent refreshing the runtime surface snapshot.
    pub refresh_surface: Duration,
    /// Time spent building the backend-neutral paint plan.
    pub paint_plan: Duration,
    /// Time spent rendering the scene to the cached texture.
    pub render_to_texture: Duration,
    /// Time spent encoding the full-screen blit/composite pass.
    pub full_screen_blit: Duration,
    /// Time spent refreshing the composited base frame.
    pub composited_base_refresh: Duration,
    /// Whether the composited base frame was reused from cache.
    pub composited_base_cache_hit: bool,
    /// Time spent collecting transient overlay primitives.
    pub transient_overlay_paint: Duration,
    /// Transient overlay primitive count.
    pub transient_overlay_primitives: usize,
    /// Time spent submitting GPU work and presenting the surface.
    pub submit_present: Duration,
    /// Time since the previous successful present.
    pub since_last_present: Duration,
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
        self.coalesced_wheel_route
            + self.refresh_surface
            + self.paint_plan
            + self.render_to_texture
            + self.full_screen_blit
            + self.composited_base_refresh
            + self.transient_overlay_paint
            + self.submit_present
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
