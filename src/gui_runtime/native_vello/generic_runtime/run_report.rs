use crate::gui_runtime::{NativeRunOptionsError, NativeStartupTimingArtifact, RuntimeRunReport};

/// Structured runtime artifacts exported after one generic native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeGenericRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Typed failure reported by the generic native Vello runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeGenericRunError {
    /// Native launch options failed validation before platform startup.
    InvalidWindowOptions(NativeRunOptionsError),
    /// Creating the native event loop failed before runtime startup.
    EventLoopBuild(String),
    /// The native event loop returned an error while running.
    EventLoopRun(String),
}

impl std::fmt::Display for NativeGenericRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWindowOptions(err) => {
                write!(formatter, "invalid native window options: {err}")
            }
            Self::EventLoopBuild(message) => {
                write!(formatter, "failed to create native event loop: {message}")
            }
            Self::EventLoopRun(message) => {
                write!(formatter, "native event loop failed: {message}")
            }
        }
    }
}

impl std::error::Error for NativeGenericRunError {}

/// Result plus structured artifacts returned by one generic native runtime execution.
pub type NativeGenericRunReport =
    RuntimeRunReport<NativeGenericRuntimeArtifacts, NativeGenericRunError>;
