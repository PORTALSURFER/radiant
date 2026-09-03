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
mod effect;
mod environment;
mod external_drag;
mod file_drop;
mod file_open;
mod gpu_surface;
mod paint;
mod platform;
mod resource;
mod surface;
pub mod testing;
mod update_snapshot;
pub mod virtual_layout;

#[cfg(test)]
pub(crate) fn test_arc_surface<Message>(
    surface: UiSurface<Message>,
) -> std::sync::Arc<UiSurface<Message>> {
    std::sync::Arc::new(surface)
}

pub use crate::application::runtime::{BusinessEventSink, BusinessWorkContext};
pub use crate::application::{
    GpuSurfaceConfiguredParts, GpuSurfaceInputParts, RenderCanvasConfiguredParts,
    RenderCanvasInputParts, RetainedCanvasBuilder, canvas, gpu_surface,
    gpu_surface_configured_from_parts, gpu_surface_from_parts, gpu_surface_input,
    gpu_surface_input_from_parts, gpu_surface_with_capabilities, render_canvas,
    render_canvas_configured_from_parts, render_canvas_from_parts, render_canvas_input,
    render_canvas_input_from_parts, render_canvas_with_capabilities, retained_canvas,
    retained_canvas_with,
};
pub use crate::gui::automation::{
    AUTOMATION_ACTION_DECREMENT, AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_INCREMENT,
    AUTOMATION_ACTION_PRESS, AUTOMATION_ACTION_SELECT, AUTOMATION_ACTION_SET_TEXT,
    AUTOMATION_ACTION_SET_VALUE, AUTOMATION_ACTION_TOGGLE, AutomationBounds, AutomationFocusHints,
    AutomationLiveRegion, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
    AutomationPoint, AutomationRole, AutomationTarget, AutomationTargetAuthority,
    GuiAutomationSnapshot, GuiAutomationTargetSnapshot,
};
pub use crate::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, EmbeddedVelloError, EmbeddedVelloRenderer,
    EmbeddedVelloSurfaceHandle, EmbeddedVelloUnsupportedPrimitive, FrameRate,
    MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS, NativeFrameOptions, NativeGenericRunError,
    NativeGenericRunReport, NativeGenericRuntimeArtifacts, NativeGpuBackend, NativeGpuOptions,
    NativeInitializationStage, NativePopupOptions, NativeRenderDeviceErrorKind, NativeRunOptions,
    NativeRunOptionsError, NativeStartupTimingArtifact, NativeTextOptions, NativeWindowBehavior,
    NativeWindowGeometry, NativeWindowMode, NativeWindowOptions, RuntimeRunReport, WindowIconRgba,
    WindowManifest, WindowManifestError, WindowSpec, WindowSpecError, WindowSpecParts,
    run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
};
pub use crate::theme::{AppearancePolicy, ResolvedAppearance};
pub use crate::widgets::{GpuSurfaceParts, RenderCanvasParts};
#[cfg(target_os = "macos")]
pub(crate) use automation::NativeSemanticContainerSnapshot;
pub(crate) use automation::NativeSemanticCoordinateAuthority;
pub use automation::{
    NumericAccessibilityDispatchResult, NumericAccessibilityRequest,
    NumericAccessibilityUnavailableReason, SemanticAutomationContainerHandle,
    SemanticAutomationDemand, SemanticAutomationDemandError, SemanticAutomationFallbackReason,
    SemanticAutomationRefresh, SemanticAutomationRefreshStatus, SemanticAutomationSelectedSnapshot,
    SemanticAutomationSessionError, SemanticAutomationSessionHandle,
};
#[allow(deprecated)]
pub use bridge::{
    App, AuxiliaryWindow, AuxiliaryWindowClosePolicy, DeclarativeCommandRuntimeBridge,
    DeclarativeCommandRuntimeBridgeParts, DeclarativeOwnedCommandRuntimeBridge,
    DeclarativeOwnedCommandRuntimeBridgeParts, DeclarativeOwnedRuntimeBridge,
    DeclarativeOwnedRuntimeBridgeParts, DeclarativeRuntimeBridge, DeclarativeRuntimeBridgeParts,
    RuntimeAnimationActivity, RuntimeAnimationDemand, RuntimeAnimationHost, RuntimeBridge,
    RuntimeDiagnosticsHost, RuntimeFrameDiagnosticsHost, RuntimeFrameGpuTimingHost,
    RuntimeFrameProfileHost, RuntimeHostCapabilities, RuntimeInputHost, RuntimeLifecycleHost,
    RuntimePlatformHost, RuntimePlatformResultHost, RuntimeQueueDelivery, RuntimeQueueHost,
    RuntimeQueueItem, RuntimeRetainedSurfaceHost, RuntimeTaskHost, RuntimeTimerOwner,
    RuntimeTimerWake, RuntimeTransientOverlayHost, RuntimeWindowHost,
    declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
    declarative_owned_runtime_bridge, declarative_runtime_bridge,
};
pub(crate) use bridge::{RuntimeQueueCapability, RuntimeRetainedSurfaceCapability};
pub(crate) use command::WorkerStreamOptions;
pub use command::{
    Command, PlatformEffect, RepaintScope, ScrollFixedRowIntoViewParts, ScrollIntoViewParts,
    SurfaceInvalidation, SurfaceRevisions, TaskPriority,
};
pub(crate) use command::{EffectId, WorkerEffectSink};
#[cfg(test)]
pub(crate) use controller::AuxiliaryFocusCommand;
pub(crate) use controller::BasePaintPlanContext;
pub(crate) use controller::PreparedSurfaceRefresh;
pub(crate) use controller::SequentialFocusTraversalDisposition;
pub(crate) use controller::WheelOrScrollRoute;
pub(crate) use controller::{AuxiliaryFocusRequest, AuxiliaryWindowOwner};
pub use controller::{
    CommandOutcome, DeclarativeOwnedSurfaceRuntime, DeclarativeSurfaceRuntime, Event,
    FocusTraversal, IdentityAudit, PointerClickOutcome, PointerMoveOutcome, RuntimeContext,
    RuntimeSurfaceFrame, RuntimeSurfaceFrameRef, ScrollUpdate, ScrollUpdateMetadata,
    SurfaceIdentityDiagnostics, SurfaceIdentityOwnership, SurfaceIdentityPath,
    SurfaceIdentityReplacement, SurfaceLayoutStateDiagnostics, SurfaceLayoutStateReplacement,
    SurfaceRefreshCounters, SurfaceRefreshDiagnostics, SurfaceRefreshTimings, SurfaceRuntime,
};
#[cfg(target_os = "macos")]
pub(crate) use controller::{
    NormalizedSemanticPublicationFenceSet, VirtualLayoutAutomationComposition,
    VirtualLayoutNormalizedSemanticSidecar, VirtualLayoutNormalizedSemanticSidecarEntry,
};
pub use devtools::{
    DevtoolsLayoutDiagnostic, DevtoolsNodeKind, DevtoolsNodeSnapshot, DevtoolsOverlayOptions,
    DevtoolsSnapshot, DevtoolsWidgetSnapshot,
};
pub use diagnostics::{
    BusinessRuntimeDiagnostics, BusinessTaskDiagnostic, BusinessTaskDiagnosticState,
    DEFAULT_SLOW_UPDATE_HANDLER_THRESHOLD, FrameGpuTimingOutcome, FrameGpuTimingSample,
    FrameGpuTimingUnavailableReason, FrameProfile, FrameProfileCacheCounters,
    FrameProfileCompositedBaseTiming, FrameProfileCounters, FrameProfileCpuCompletionOutcome,
    FrameProfileCpuFairnessCounters, FrameProfileCpuFairnessDisposition,
    FrameProfileCpuObservationCounters, FrameProfileGpuSurfaceAtlasCounters,
    FrameProfileGpuSurfaceCompositeCounters, FrameProfileGpuSurfaceCounters,
    FrameProfileGpuSurfaceCustomShaderCounters, FrameProfileGpuSurfaceCustomShaderFailureCounters,
    FrameProfileGpuSurfaceSignalCounters, FrameProfileGpuSurfaceUnsupportedCustomShaderCounters,
    FrameProfileGpuTimingStatus, FrameProfileRetainedSurfaceCounters, FrameProfileSceneCounters,
    FrameProfileSceneMediaCounters, FrameProfileSceneSurfaceCounters,
    FrameProfileSceneTextCounters, FrameProfileSceneTraversalCounters,
    FrameProfileSurfaceRecoveryCounters, FrameProfileTextCacheCounters, FrameProfileTextCounters,
    FrameProfileTextQualityCounters, FrameProfileTimings, FrameProfileTransientOverlayTiming,
    FrameProfileWorkTimings, GpuSurfaceOcclusionPlanningDiagnostics, NativeCompositedBaseTiming,
    NativeCpuFrameCompletionOutcome, NativeCpuFrameFairnessDiagnostics,
    NativeCpuFrameFairnessDisposition, NativeCpuFrameObservationDiagnostics,
    NativeFrameDiagnostics, NativeFramePresentationDiagnostics, NativeFrameTimingDiagnostics,
    NativeFrameWorkTimings, NativeGpuSurfaceAtlasDiagnostics, NativeGpuSurfaceCompositeDiagnostics,
    NativeGpuSurfaceCustomShaderDiagnostics, NativeGpuSurfaceCustomShaderFailureDiagnostics,
    NativeGpuSurfaceDiagnostics, NativeGpuSurfaceSignalDiagnostics,
    NativeGpuSurfaceUnsupportedCustomShaderDiagnostics, NativeGpuTimingStatus,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, NativeSceneMediaDiagnostics,
    NativeSceneSurfaceDiagnostics, NativeSceneTextDiagnostics, NativeSceneTraversalDiagnostics,
    NativeSurfaceRecoveryDiagnostics, NativeTextCacheCounters, NativeTextCacheDiagnostics,
    NativeTextDiagnostics, NativeTextQualityDiagnostics, NativeTextQualityStatus,
    NativeTransientOverlayTiming, NativeWindowDiagnosticIdentity, PlatformOwnerKind, ProfilingMode,
    ProfilingOptions, RUNTIME_LIFECYCLE_HISTORY_CAPACITY, RetainedSurfaceCachePolicy,
    RuntimeDiagnostics, RuntimeLifecycleDiagnostics, RuntimeLifecyclePhase,
    RuntimeLifecycleTransition, RuntimeMessageQueueDiagnostics, SLOW_UPDATE_HANDLER_GUIDANCE,
    UiRuntimeDiagnostics, UiUpdateHandlerDiagnostic, UiUpdateHandlerDiagnosticsMode,
    UiUpdateHandlerDiagnosticsPolicy,
};
pub(crate) use diagnostics::{
    RuntimeDiagnosticsRecorder, RuntimeLifecycleController, elapsed_since,
};
pub(crate) use drag::DragSession;
pub use drag::{DragPreview, DragPreviewTextSizing, DragRequest};
pub use effect::{Effect, EffectOwner};
pub use environment::{
    ResolvedEnvironment, WindowColorScheme, WindowEnvironment, WindowEnvironmentChange,
};
pub(crate) use external_drag::{
    ExternalDragCompletion, ExternalDragIdentity, ExternalDragLaunch, ExternalDragSession,
    PendingExternalDragCompletion,
};
pub use external_drag::{
    ExternalDragEffect, ExternalDragOutcome, ExternalDragPayload, ExternalDragPreview,
    ExternalDragRequest,
};
pub use file_drop::{NativeFileDrop, NativeFileDropPhase};
pub use file_open::NativeFileOpen;
pub use gpu_surface::{
    CanvasKey, GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT, GpuShaderPresentationUniformUpdate,
    GpuShaderPresentationUniformUpdateError, GpuShaderSurfaceDescriptor,
    GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary,
    GpuSignalSummaryBucket, GpuSignalSummaryLevel, GpuSurfaceCapabilities, GpuSurfaceContent,
    GpuSurfaceContentError, GpuSurfaceLineStyle, GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays,
    MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES, RenderCanvasCapabilities, RenderCanvasContent,
    RenderCanvasContentError, RenderCanvasLineStyle, RenderCanvasOverlay,
    RenderCanvasRuntimeOverlays, RenderCanvasShaderSurfaceDescriptor,
    RenderCanvasShaderSurfaceDescriptorParts,
};
pub(crate) use paint::{
    MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentObservation, PaintSegmentObserver,
    PaintSegmentSpan, collect_segment_spans,
};
pub use paint::{
    PaintBrush, PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintFillRectBatch, PaintFillRule, PaintGpuSurface, PaintImage,
    PaintLinearGradient, PaintOverlayPanel, PaintPath, PaintPathCommand, PaintPointList,
    PaintPrimitive, PaintRectList, PaintRenderCanvas, PaintStrokePolygon, PaintStrokePolyline,
    PaintStrokeRect, PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintText, PaintTextAlign,
    PaintTextInput, PaintTextMetrics, PaintTextRun, PaintTransform, Renderer, SurfacePaintPlan,
    SurfacePaintStats, SvgParseError, TransientOverlayContext, WidgetPaint, push_fill_polygon,
    push_fill_rect, push_fill_rect_batch, push_stroke_polyline, push_stroke_rect,
    push_stroke_rect_batch, push_text, push_text_run_with_metrics, push_visible_fill_rect,
};
#[cfg(test)]
pub(crate) use paint::{PaintSegment, PaintSegmentAnchor};
pub(crate) use paint::{
    blend_color, button_font_size, diagonal_cut_rect_points, input_font_size, inset_rect,
    optical_centered_baseline, push_axis_stroke, push_text_run, text_font_size,
};
pub use platform::{
    ClipboardContent, ClipboardContentFormat, ClipboardFormat, ClipboardIdentity, ClipboardValue,
    ClipboardValueError, ConfirmDialogParts, ConfirmDialogRequest, ConfirmationButtons,
    ConfirmationLevel, ConfirmationResponse, FileDialogFilter, FileDialogRequest,
    InProcessClipboardIdentity, MAX_CLIPBOARD_TEXT_BYTES, MAX_NOTIFICATION_BODY_BYTES,
    MAX_NOTIFICATION_TITLE_BYTES, MAX_PLATFORM_PATH_BYTES, MAX_PLATFORM_PATH_COUNT,
    MAX_PLATFORM_TEXT_BYTES, NotificationLevel, NotificationRequest, PlatformCompletion,
    PlatformError, PlatformFailure, PlatformNotificationRequest, PlatformRequest, PlatformResponse,
    PlatformResult, PlatformResultExt, PlatformResultServiceFallback, PlatformService,
    PlatformServiceFallback, RuntimePlatformResultSink,
};
pub(crate) use platform::{PlatformCompletionIdentity, PlatformResultDelivery};
pub use resource::{
    ResourceCompletion, ResourceCompletionParts, ResourceKey, ResourceLoad, ResourceLoadState,
    ResourceRequest, ResourceSlot,
};
pub(crate) use surface::lower_public_virtual_layout;
pub(in crate::runtime) use surface::{
    ClipAncestors, SurfaceLayoutInteractionRecord, SurfaceRuntimeProjection,
    SurfaceSplitPaneFocusOrderCandidate, SurfaceTraversalIndex, WheelHitTarget,
    WidgetDispatchResult, WidgetPath, empty_paint_plan_for_layout,
};
pub use surface::{
    Element, EventMapper, LayerKind, MessageMapper, NativeFileDropMessageMapper,
    ScrollMessageMapper, SurfaceChild, SurfaceContainer, SurfaceFrame, SurfaceLayer, SurfaceNode,
    SurfaceOverlay, SurfaceScene, SurfaceWidget, UiSurface, View, WidgetMessageMapper,
};
pub(crate) use surface::{
    KeyedNodeEvidence, SourceCompatibility, SourceIdentity, SourceMetadata, SourceTopology,
};
pub use update_snapshot::RuntimeUpdateSnapshot;
pub use virtual_layout::{
    VirtualLayoutRevisions, VirtualLayoutSemanticDeferredReason, VirtualLayoutSemanticEntry,
    VirtualLayoutSemanticProvider, VirtualLayoutSemanticProviderOutcome,
    VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeRequest,
    VirtualLayoutSemanticRequest, VirtualLayoutSemanticUnavailableReason,
};
pub(crate) use virtual_layout::{
    adapt_coordinate_transform, adapt_item_provider, adapt_range_provider, provider_identity,
};
