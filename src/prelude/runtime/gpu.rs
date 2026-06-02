//! GPU-surface payload and diagnostics prelude exports.

pub use crate::runtime::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview,
    GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
    GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceContentError, GpuSurfaceLineStyle,
    GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, NativeGpuSurfaceAtlasDiagnostics,
    NativeGpuSurfaceCompositeDiagnostics, NativeGpuSurfaceCustomShaderDiagnostics,
    NativeGpuSurfaceCustomShaderFailureDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeGpuSurfaceSignalDiagnostics, NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
};
