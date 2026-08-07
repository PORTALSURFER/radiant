//! Public native runtime diagnostics models.

mod business;
mod cache_policy;
mod frame;
mod gpu_surface;
mod lifecycle;
mod profile;
mod retained_surface;
mod scene;
mod text;
mod timing;

pub use business::{
    BusinessRuntimeDiagnostics, BusinessTaskDiagnostic, BusinessTaskDiagnosticState,
    DEFAULT_SLOW_UPDATE_HANDLER_THRESHOLD, RuntimeDiagnostics, RuntimeMessageQueueDiagnostics,
    SLOW_UPDATE_HANDLER_GUIDANCE, UiRuntimeDiagnostics, UiUpdateHandlerDiagnostic,
    UiUpdateHandlerDiagnosticsMode, UiUpdateHandlerDiagnosticsPolicy,
};
pub(crate) use business::{RuntimeDiagnosticsRecorder, elapsed_since};
pub use cache_policy::RetainedSurfaceCachePolicy;
pub use frame::NativeCpuFrameCompletionOutcome;
pub use frame::NativeCpuFrameFairnessDiagnostics;
pub use frame::NativeCpuFrameFairnessDisposition;
pub use frame::NativeCpuFrameObservationDiagnostics;
pub use frame::NativeFrameDiagnostics;
pub use frame::NativeFramePresentationDiagnostics;
pub use frame::NativeSurfaceRecoveryDiagnostics;
pub use frame::NativeWindowDiagnosticIdentity;
pub use gpu_surface::{
    GpuSurfaceOcclusionPlanningDiagnostics, NativeGpuSurfaceAtlasDiagnostics,
    NativeGpuSurfaceCompositeDiagnostics, NativeGpuSurfaceCustomShaderDiagnostics,
    NativeGpuSurfaceCustomShaderFailureDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeGpuSurfaceSignalDiagnostics, NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
};
pub(crate) use lifecycle::RuntimeLifecycleController;
pub use lifecycle::{
    RUNTIME_LIFECYCLE_HISTORY_CAPACITY, RuntimeLifecycleDiagnostics, RuntimeLifecyclePhase,
    RuntimeLifecycleTransition,
};
pub use profile::{
    FrameProfile, FrameProfileCacheCounters, FrameProfileCompositedBaseTiming,
    FrameProfileCounters, FrameProfileCpuCompletionOutcome, FrameProfileCpuFairnessCounters,
    FrameProfileCpuFairnessDisposition, FrameProfileCpuObservationCounters,
    FrameProfileGpuSurfaceAtlasCounters, FrameProfileGpuSurfaceCompositeCounters,
    FrameProfileGpuSurfaceCounters, FrameProfileGpuSurfaceCustomShaderCounters,
    FrameProfileGpuSurfaceCustomShaderFailureCounters, FrameProfileGpuSurfaceSignalCounters,
    FrameProfileGpuSurfaceUnsupportedCustomShaderCounters, FrameProfileGpuTimingStatus,
    FrameProfileRetainedSurfaceCounters, FrameProfileSceneCounters, FrameProfileSceneMediaCounters,
    FrameProfileSceneSurfaceCounters, FrameProfileSceneTextCounters,
    FrameProfileSceneTraversalCounters, FrameProfileSurfaceRecoveryCounters,
    FrameProfileTextCacheCounters, FrameProfileTextCounters, FrameProfileTextQualityCounters,
    FrameProfileTimings, FrameProfileTransientOverlayTiming, FrameProfileWorkTimings,
    ProfilingMode, ProfilingOptions,
};
pub use retained_surface::NativeRetainedSurfaceDiagnostics;
pub use scene::{
    NativeSceneDiagnostics, NativeSceneMediaDiagnostics, NativeSceneSurfaceDiagnostics,
    NativeSceneTextDiagnostics, NativeSceneTraversalDiagnostics,
};
pub use text::{
    NativeTextCacheCounters, NativeTextCacheDiagnostics, NativeTextDiagnostics,
    NativeTextQualityDiagnostics, NativeTextQualityStatus,
};
pub use timing::{
    NativeCompositedBaseTiming, NativeFrameTimingDiagnostics, NativeFrameWorkTimings,
    NativeGpuTimingStatus, NativeTransientOverlayTiming,
};
