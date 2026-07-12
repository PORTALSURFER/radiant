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

mod automation;
mod bridge;
mod command;
mod controller;
mod devtools;
mod diagnostics;
mod drag;
mod external_drag;
mod file_drop;
mod file_open;
mod gpu_surface;
mod paint;
mod platform;
mod resource;
mod surface;
mod update_snapshot;

pub use crate::application::runtime::{BusinessEventSink, BusinessWorkContext};
pub use crate::application::{
    GpuSurfaceConfiguredParts, GpuSurfaceInputParts, RetainedCanvasBuilder, canvas, gpu_surface,
    gpu_surface_configured_from_parts, gpu_surface_from_parts, gpu_surface_input,
    gpu_surface_input_from_parts, gpu_surface_with_capabilities, retained_canvas,
    retained_canvas_with,
};
pub use crate::gui::automation::{
    AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS, AUTOMATION_ACTION_SELECT,
    AUTOMATION_ACTION_SET_TEXT, AUTOMATION_ACTION_SET_VALUE, AUTOMATION_ACTION_TOGGLE,
    AutomationBounds, AutomationFocusHints, AutomationLiveRegion, AutomationNodeId,
    AutomationNodeSemantics, AutomationNodeSnapshot, AutomationPoint, AutomationRole,
    AutomationTarget, GuiAutomationSnapshot, GuiAutomationTargetSnapshot,
};
pub use crate::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS,
    EmbeddedVelloError, EmbeddedVelloRenderer, EmbeddedVelloSurfaceHandle,
    EmbeddedVelloUnsupportedPrimitive,
    NativeFrameOptions, NativeGenericRunError, NativeGenericRunReport,
    NativeGenericRuntimeArtifacts, NativeGpuBackend, NativeGpuOptions, NativePopupOptions,
    NativeRunOptions, NativeRunOptionsError, NativeStartupTimingArtifact, NativeTextOptions,
    NativeWindowBehavior, NativeWindowGeometry, NativeWindowMode, NativeWindowOptions,
    RuntimeRunReport, WindowIconRgba, WindowManifest, WindowManifestError, WindowSpec,
    WindowSpecError, WindowSpecParts, run_native_vello_runtime,
    run_native_vello_runtime_with_artifacts,
};
pub use crate::widgets::GpuSurfaceParts;
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
    BusinessMessageSink, Command, RepaintScope, ScrollFixedRowIntoViewParts, ScrollIntoViewParts,
    TaskPriority,
};
pub(crate) use controller::WheelOrScrollRoute;
pub use controller::{
    CommandOutcome, DeclarativeOwnedSurfaceRuntime, DeclarativeSurfaceRuntime, Event,
    FocusTraversal, PointerClickOutcome, PointerMoveOutcome, RuntimeContext, RuntimeSurfaceFrame,
    RuntimeSurfaceFrameRef, ScrollUpdate, SurfaceRuntime,
};
pub use devtools::{
    DevtoolsLayoutDiagnostic, DevtoolsNodeKind, DevtoolsNodeSnapshot, DevtoolsOverlayOptions,
    DevtoolsSnapshot, DevtoolsWidgetSnapshot,
};
pub use diagnostics::{
    BusinessRuntimeDiagnostics, BusinessTaskDiagnostic, BusinessTaskDiagnosticState,
    DEFAULT_SLOW_UPDATE_HANDLER_THRESHOLD, NativeCompositedBaseTiming, NativeFrameDiagnostics,
    NativeFramePresentationDiagnostics, NativeFrameTimingDiagnostics, NativeFrameWorkTimings,
    NativeGpuSurfaceAtlasDiagnostics, NativeGpuSurfaceCompositeDiagnostics,
    NativeGpuSurfaceCustomShaderDiagnostics, NativeGpuSurfaceCustomShaderFailureDiagnostics,
    NativeGpuSurfaceDiagnostics, NativeGpuSurfaceSignalDiagnostics,
    NativeGpuSurfaceUnsupportedCustomShaderDiagnostics, NativeGpuTimingStatus,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, NativeSceneMediaDiagnostics,
    NativeSceneSurfaceDiagnostics, NativeSceneTextDiagnostics, NativeSceneTraversalDiagnostics,
    NativeTextCacheCounters, NativeTextCacheDiagnostics, NativeTextDiagnostics,
    NativeTextQualityDiagnostics, NativeTextQualityStatus, NativeTransientOverlayTiming,
    RetainedSurfaceCachePolicy, RuntimeDiagnostics, RuntimeMessageQueueDiagnostics,
    SLOW_UPDATE_HANDLER_GUIDANCE, UiRuntimeDiagnostics, UiUpdateHandlerDiagnostic,
    UiUpdateHandlerDiagnosticsMode, UiUpdateHandlerDiagnosticsPolicy,
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
pub use file_open::NativeFileOpen;
pub use gpu_surface::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview,
    GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
    GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceContentError, GpuSurfaceLineStyle,
    GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays,
};
pub use paint::{
    PaintBrush, PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintFillRectBatch, PaintFillRule, PaintGpuSurface, PaintImage,
    PaintLinearGradient, PaintOverlayPanel, PaintPath, PaintPathCommand, PaintPointList,
    PaintPrimitive, PaintRectList, PaintStrokePolygon, PaintStrokePolyline, PaintStrokeRect,
    PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintText, PaintTextAlign, PaintTextInput,
    PaintTextMetrics, PaintTextRun, PaintTransform, Renderer, SurfacePaintPlan, SurfacePaintStats,
    SvgParseError, TransientOverlayContext, WidgetPaint, push_fill_polygon, push_fill_rect,
    push_fill_rect_batch, push_stroke_polyline, push_stroke_rect, push_stroke_rect_batch,
    push_text, push_text_run_with_metrics, push_visible_fill_rect,
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
    ClipAncestors, SurfaceRuntimeProjection, SurfaceTraversalIndex, WheelHitTarget,
    WidgetDispatchResult, WidgetPath, empty_paint_plan_for_layout,
};
pub use surface::{
    Element, LayerKind, MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper,
    SurfaceChild, SurfaceContainer, SurfaceFrame, SurfaceLayer, SurfaceNode, SurfaceOverlay,
    SurfaceScene, SurfaceWidget, UiSurface, View, WidgetMessageMapper,
};
pub use update_snapshot::RuntimeUpdateSnapshot;
