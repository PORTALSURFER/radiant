//! Generic declarative runtime surfaces for new Radiant host applications.
//!
//! This module exposes a message-driven top-level UI tree built from public
//! layout containers and widget primitives. Hosts project immutable
//! [`UiSurface`](crate::runtime::UiSurface) snapshots and reduce host-defined messages while compatibility
//! adapters continue to live outside this generic surface.
//!
//! [`SurfaceRuntime`](crate::runtime::SurfaceRuntime) closes the generic declarative flow by running public
//! layout, routing backend-neutral widget input, mapping widget outputs into
//! host-defined messages, reducing those messages, reprojecting the next
//! immutable surface snapshot, and exposing deterministic backend-neutral paint
//! plans for generic renderers.
//! [`Command`](crate::runtime::Command) is the domain-neutral follow-up value for host-side reducers
//! that need to queue messages, batch runtime-visible work, or request repaint
//! without moving side-effect ownership into Radiant.
//!
//! Native window adapters can compose against this controller without coupling
//! the public runtime API to any host application's top-level contracts.

mod bridge;
mod command;
mod controller;
mod diagnostics;
mod drag;
mod external_drag;
mod file_drop;
mod gpu_surface;
mod paint;
mod platform;
mod resource;
mod surface;

pub use crate::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS,
    NativeFrameOptions, NativeGenericRunError, NativeGenericRunReport,
    NativeGenericRuntimeArtifacts, NativeGpuBackend, NativeGpuOptions, NativePopupOptions,
    NativeRunOptions, NativeRunOptionsError, NativeStartupTimingArtifact, NativeTextOptions,
    NativeWindowBehavior, NativeWindowGeometry, NativeWindowMode, NativeWindowOptions,
    RuntimeRunReport, WindowIconRgba, WindowManifest, WindowManifestError, WindowSpec,
    WindowSpecError, WindowSpecParts, run_native_vello_runtime,
    run_native_vello_runtime_with_artifacts,
};
pub use bridge::{
    App, AuxiliaryWindow, AuxiliaryWindowClosePolicy, DeclarativeCommandRuntimeBridge,
    DeclarativeCommandRuntimeBridgeParts, DeclarativeOwnedCommandRuntimeBridge,
    DeclarativeOwnedCommandRuntimeBridgeParts, DeclarativeOwnedRuntimeBridge,
    DeclarativeOwnedRuntimeBridgeParts, DeclarativeRuntimeBridge, DeclarativeRuntimeBridgeParts,
    RuntimeAnimationActivity, RuntimeAnimationDemand, RuntimeBridge,
    declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
    declarative_owned_runtime_bridge, declarative_runtime_bridge,
};
pub use command::{
    Command, RepaintScope, ScrollFixedRowIntoViewParts, ScrollIntoViewParts, TaskPriority,
};
pub use controller::{
    CommandOutcome, DeclarativeOwnedSurfaceRuntime, DeclarativeSurfaceRuntime, Event,
    FocusTraversal, PointerClickOutcome, PointerMoveOutcome, RuntimeContext, RuntimeSurfaceFrame,
    RuntimeSurfaceFrameRef, ScrollUpdate, SurfaceRuntime,
};
pub use diagnostics::{
    BusinessRuntimeDiagnostics, BusinessTaskDiagnostic, BusinessTaskDiagnosticState,
    NativeCompositedBaseTiming, NativeFrameDiagnostics, NativeFrameTimingDiagnostics,
    NativeFrameWorkTimings, NativeGpuSurfaceAtlasDiagnostics, NativeGpuSurfaceCompositeDiagnostics,
    NativeGpuSurfaceCustomShaderDiagnostics, NativeGpuSurfaceCustomShaderFailureDiagnostics,
    NativeGpuSurfaceDiagnostics, NativeGpuSurfaceSignalDiagnostics,
    NativeGpuSurfaceUnsupportedCustomShaderDiagnostics, NativeGpuTimingStatus,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, NativeSceneMediaDiagnostics,
    NativeSceneSurfaceDiagnostics, NativeSceneTextDiagnostics, NativeSceneTraversalDiagnostics,
    NativeTextCacheCounters, NativeTextCacheDiagnostics, NativeTextDiagnostics,
    NativeTextQualityDiagnostics, NativeTextQualityStatus, NativeTransientOverlayTiming,
    RetainedSurfaceCachePolicy, RuntimeDiagnostics, UiRuntimeDiagnostics,
    UiUpdateHandlerDiagnostic,
};
pub(crate) use diagnostics::{RuntimeDiagnosticsRecorder, elapsed_since};
pub(crate) use drag::DragSession;
pub use drag::{DragPreview, DragPreviewTextSizing, DragRequest};
pub(crate) use external_drag::ExternalDragSession;
pub use external_drag::{
    ExternalDragEffect, ExternalDragOutcome, ExternalDragPayload, ExternalDragPreview,
    ExternalDragRequest,
};
pub use file_drop::{NativeFileDrop, NativeFileDropPhase};
pub use gpu_surface::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview,
    GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
    GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceContentError, GpuSurfaceLineStyle,
    GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays,
};
pub use paint::{
    PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintFillRectBatch, PaintFillRule, PaintGpuSurface, PaintImage,
    PaintOverlayPanel, PaintPath, PaintPathCommand, PaintPointList, PaintPrimitive, PaintRectList,
    PaintStrokePolygon, PaintStrokePolyline, PaintStrokeRect, PaintStrokeRectBatch, PaintSvg,
    PaintSvgDocument, PaintText, PaintTextAlign, PaintTextInput, PaintTextMetrics, PaintTextRun,
    PaintTransform, Renderer, SurfacePaintPlan, SurfacePaintStats, SvgParseError,
    TransientOverlayContext, WidgetPaint, push_fill_polygon, push_fill_rect, push_fill_rect_batch,
    push_stroke_polyline, push_stroke_rect, push_stroke_rect_batch, push_text,
    push_text_run_with_metrics, push_visible_fill_rect,
};
pub(crate) use paint::{
    blend_color, button_font_size, diagonal_cut_rect_points, input_font_size, inset_rect,
    optical_centered_baseline, push_axis_stroke, push_text_run, text_font_size,
};
pub use platform::{
    ConfirmDialogParts, ConfirmDialogRequest, ConfirmationButtons, ConfirmationLevel,
    ConfirmationResponse, FileDialogFilter, FileDialogRequest, PlatformCompletion, PlatformRequest,
    PlatformResponse, PlatformResult, PlatformResultExt, PlatformServiceFallback,
};
pub use resource::{
    ResourceCompletion, ResourceCompletionParts, ResourceKey, ResourceLoad, ResourceLoadState,
    ResourceRequest, ResourceSlot,
};
pub(in crate::runtime) use surface::{
    ClipAncestors, SurfaceRuntimeProjection, SurfaceTraversalIndex, WidgetDispatchResult,
    WidgetPath, empty_paint_plan_for_layout,
};
pub use surface::{
    Element, LayerKind, MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper,
    SurfaceChild, SurfaceContainer, SurfaceFrame, SurfaceLayer, SurfaceNode, SurfaceOverlay,
    SurfaceScene, SurfaceWidget, UiSurface, View, WidgetMessageMapper,
};
