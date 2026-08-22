use crate::runtime::{
    FrameGpuTimingSample, FrameProfile, NativeFrameDiagnostics, RuntimeDiagnostics,
};

/// Optional host capability for application-runtime diagnostics snapshots.
pub trait RuntimeDiagnosticsHost {
    /// Return application-runtime diagnostics contributed by this host.
    fn runtime_diagnostics(&self) -> RuntimeDiagnostics;
}

/// Optional host capability for native per-frame diagnostics.
pub trait RuntimeFrameDiagnosticsHost {
    /// Observe structured diagnostics for one successfully presented native
    /// frame from the primary or an auxiliary window.
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics);
}

/// Optional host capability for backend-neutral fixed-cost frame profiles.
pub trait RuntimeFrameProfileHost {
    /// Observe one profile for a successfully presented frame.
    fn observe_frame_profile(&mut self, profile: FrameProfile);
}

/// Optional host capability for correlated asynchronous aggregate GPU timing.
///
/// The generic native primary and auxiliary runners can emit per-window
/// terminal samples when that window's frame profiling and the observer are
/// enabled. Auxiliary samples use the existing parent handoff, and lifecycle
/// fencing prevents stale or invalid completions from being delivered.
pub trait RuntimeFrameGpuTimingHost {
    /// Observe one terminal GPU timing result for a successfully presented
    /// frame.
    fn observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample);
}

pub(crate) struct RuntimeDiagnosticsCapability<Bridge> {
    pub runtime_diagnostics: fn(&Bridge) -> RuntimeDiagnostics,
}

impl<Bridge> RuntimeDiagnosticsCapability<Bridge>
where
    Bridge: RuntimeDiagnosticsHost,
{
    pub const fn new() -> Self {
        Self {
            runtime_diagnostics: Bridge::runtime_diagnostics,
        }
    }
}

pub(crate) struct RuntimeFrameDiagnosticsCapability<Bridge> {
    pub observe_frame_diagnostics: fn(&mut Bridge, NativeFrameDiagnostics),
}

impl<Bridge> RuntimeFrameDiagnosticsCapability<Bridge>
where
    Bridge: RuntimeFrameDiagnosticsHost,
{
    pub const fn new() -> Self {
        Self {
            observe_frame_diagnostics: Bridge::observe_frame_diagnostics,
        }
    }
}

pub(crate) struct RuntimeFrameProfileCapability<Bridge> {
    pub observe_frame_profile: fn(&mut Bridge, FrameProfile),
}

pub(crate) struct RuntimeFrameGpuTimingCapability<Bridge> {
    pub observe_frame_gpu_timing: fn(&mut Bridge, FrameGpuTimingSample),
}

impl<Bridge> RuntimeFrameProfileCapability<Bridge>
where
    Bridge: RuntimeFrameProfileHost,
{
    pub const fn new() -> Self {
        Self {
            observe_frame_profile: Bridge::observe_frame_profile,
        }
    }
}

impl<Bridge> RuntimeFrameGpuTimingCapability<Bridge>
where
    Bridge: RuntimeFrameGpuTimingHost,
{
    pub const fn new() -> Self {
        Self {
            observe_frame_gpu_timing: Bridge::observe_frame_gpu_timing,
        }
    }
}
