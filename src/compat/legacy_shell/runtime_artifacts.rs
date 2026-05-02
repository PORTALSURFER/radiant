use crate::gui_runtime::NativeStartupTimingArtifact;

/// Structured runtime artifacts exported after one native compatibility-shell run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Result plus structured artifacts returned by one native compatibility-shell runtime execution.
#[derive(Debug)]
pub struct NativeRunReport {
    /// Structured artifacts captured during the run.
    pub artifacts: NativeRuntimeArtifacts,
    /// Native runtime success or error outcome.
    pub result: Result<(), String>,
}
