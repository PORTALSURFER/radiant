use crate::runtime::{NativeFrameDiagnostics, RuntimeDiagnostics};

/// Optional host capability for application-runtime diagnostics snapshots.
pub trait RuntimeDiagnosticsHost {
    /// Return application-runtime diagnostics contributed by this host.
    fn runtime_diagnostics(&self) -> RuntimeDiagnostics;
}

/// Optional host capability for native per-frame diagnostics.
pub trait RuntimeFrameDiagnosticsHost {
    /// Observe structured diagnostics for one native presentation frame.
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics);
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
