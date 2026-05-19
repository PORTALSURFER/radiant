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
mod external_drag;
mod gpu_surface;
mod paint;
mod platform;
mod resource;
mod surface;

pub use crate::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS,
    NativeGenericRunError, NativeGenericRunReport, NativeGenericRuntimeArtifacts, NativeGpuBackend,
    NativeGpuOptions, NativePopupOptions, NativeRunOptions, NativeRunOptionsError,
    NativeStartupTimingArtifact, NativeTextOptions, NativeWindowMode, RuntimeRunReport,
    WindowIconRgba, WindowManifest, WindowManifestError, WindowSpec, WindowSpecError,
    run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
};
pub use bridge::{
    App, DeclarativeCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridge,
    DeclarativeOwnedRuntimeBridge, DeclarativeRuntimeBridge, RuntimeAnimationActivity,
    RuntimeBridge, declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
    declarative_owned_runtime_bridge, declarative_runtime_bridge,
};
pub use command::{Command, RepaintScope};
pub use controller::{
    CommandOutcome, Event, FocusTraversal, PointerMoveOutcome, RuntimeContext, RuntimeSurfaceFrame,
    RuntimeSurfaceFrameRef, ScrollUpdate, SurfaceRuntime,
};
pub use diagnostics::{
    NativeFrameDiagnostics, NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, RetainedSurfaceCachePolicy,
};
pub(crate) use external_drag::ExternalDragSession;
pub use external_drag::{
    ExternalDragEffect, ExternalDragOutcome, ExternalDragPayload, ExternalDragPreview,
    ExternalDragRequest,
};
pub use gpu_surface::{
    GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket,
    GpuSignalSummaryLevel, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceContentError,
    GpuSurfaceLineStyle, GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays,
};
pub use paint::{
    PaintCustomSurface, PaintFillPath, PaintFillPolygon, PaintFillRect, PaintFillRule,
    PaintGpuSurface, PaintImage, PaintOverlayPanel, PaintPath, PaintPathCommand, PaintPointList,
    PaintPrimitive, PaintStrokePolygon, PaintStrokePolyline, PaintStrokeRect, PaintSvg,
    PaintSvgDocument, PaintText, PaintTextAlign, PaintTextInput, PaintTextRun, PaintTransform,
    Renderer, SurfacePaintPlan, SurfacePaintStats, SvgParseError, TransientOverlayContext,
};
pub(crate) use paint::{
    blend_color, button_font_size, diagonal_cut_rect_points, input_font_size, inset_rect,
    optical_centered_baseline, push_axis_stroke, push_text_run, text_font_size,
};
pub use platform::{
    ConfirmDialogParts, ConfirmDialogRequest, ConfirmationButtons, ConfirmationLevel,
    ConfirmationResponse, FileDialogFilter, FileDialogRequest, PlatformCompletion, PlatformRequest,
    PlatformResponse, PlatformServiceFallback,
};
pub use resource::{
    ResourceCompletion, ResourceKey, ResourceLoad, ResourceLoadState, ResourceRequest, ResourceSlot,
};
pub(in crate::runtime) use surface::{
    ClipAncestors, SurfaceRuntimeProjection, SurfaceTraversalIndex, WidgetDispatchResult,
    WidgetPath, empty_paint_plan_for_layout,
};
pub use surface::{
    Element, MessageMapper, SurfaceChild, SurfaceContainer, SurfaceFrame, SurfaceNode,
    SurfaceOverlay, SurfaceWidget, UiSurface, View, WidgetMessageMapper,
};
