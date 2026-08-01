use crate::gui_runtime::{NativeRunOptionsError, NativeStartupTimingArtifact, RuntimeRunReport};
use std::fmt;

/// Structured runtime artifacts exported after one generic native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeGenericRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Typed failure reported by the generic native Vello runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeInitializationStage {
    /// Creation of the native window.
    WindowCreation,
    /// Creation of the WGPU surface associated with the native window.
    WgpuSurfaceCreation,
    /// Acquisition of a compatible WGPU device.
    DeviceAcquisition,
    /// Creation and configuration of the Vello render surface.
    RenderSurfaceCreation,
    /// Creation of the Vello renderer.
    RendererCreation,
}

impl fmt::Display for NativeInitializationStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::WindowCreation => "native window creation",
            Self::WgpuSurfaceCreation => "WGPU surface creation",
            Self::DeviceAcquisition => "WGPU device acquisition",
            Self::RenderSurfaceCreation => "render-surface creation",
            Self::RendererCreation => "renderer creation",
        };
        formatter.write_str(label)
    }
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
    /// Native surface texture acquisition failed because the GPU ran out of memory.
    SurfaceAcquireOutOfMemory,
    /// Rendering a native Vello scene into its target texture failed.
    FrameRender(String),
    /// Native window or renderer setup failed before the runtime became usable.
    NativeInitialization {
        /// Initialization stage that reported the failure.
        stage: NativeInitializationStage,
        /// Backend-provided diagnostic converted to owned text at the adapter boundary.
        message: String,
    },
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
            Self::SurfaceAcquireOutOfMemory => {
                write!(
                    formatter,
                    "native surface acquisition failed: out of memory"
                )
            }
            Self::FrameRender(message) => {
                write!(formatter, "native frame rendering failed: {message}")
            }
            Self::NativeInitialization { stage, message } => write!(
                formatter,
                "native initialization failed during {stage}: {message}"
            ),
        }
    }
}

impl std::error::Error for NativeGenericRunError {}

/// Result plus structured artifacts returned by one generic native runtime execution.
pub type NativeGenericRunReport =
    RuntimeRunReport<NativeGenericRuntimeArtifacts, NativeGenericRunError>;
