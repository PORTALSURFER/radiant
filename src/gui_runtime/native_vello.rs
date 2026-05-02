//! Native `winit + vello` runtime preview used for backend selection rollout.

#![cfg_attr(not(feature = "legacy-shell"), allow(dead_code))]

use super::{NativeRunOptions, WindowIconRgba};
#[cfg(all(test, feature = "legacy-shell"))]
use crate::compat::legacy_shell::PreviewBridge;
#[cfg(feature = "legacy-shell")]
use crate::compat::legacy_shell::{
    AppModel, DirtySegments, FrameBuildResult, KeyPress, NativeAppBridge, NativeMotionModel,
    NativeRunReport, NativeRuntimeArtifacts, SegmentRevisions, UiAction,
};
#[cfg(feature = "legacy-shell")]
use crate::gui::input::KeyCode;
use crate::gui::{
    input::key_code_from_winit,
    paint::{TextAlign, TextRun},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
#[cfg(feature = "legacy-shell")]
use crate::gui::{
    native_shell::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, ShellLayout,
        ShellLayoutRuntime, ShellNodeKind, StaticFrameSegment, StaticFrameSegments, StyleTokens,
        TextFieldVisualState, WaveformMotionOverlayFingerprint,
    },
    paint::{PaintFrame as NativeViewFrame, Primitive},
};
use crate::runtime::{PaintPrimitive, PaintTextAlign, RuntimeBridge, SurfaceRuntime};
use crate::theme::ThemeTokens;
use crate::widgets::{PointerButton, WidgetId, WidgetInput, WidgetKey};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
#[cfg(feature = "legacy-shell")]
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData},
    wgpu,
};
#[cfg(feature = "legacy-shell")]
use vello::{
    kurbo::{Circle, Point as KurboPoint},
    peniko::{Gradient, ImageAlphaType, ImageData, ImageFormat},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};
#[cfg(feature = "legacy-shell")]
use winit::{
    event::MouseScrollDelta, keyboard::ModifiersState, window::CursorIcon,
};

mod generic_runtime;
#[cfg(feature = "legacy-shell")]
mod input;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_runtime;
#[cfg(feature = "legacy-shell")]
mod profiling;
#[cfg(feature = "legacy-shell")]
mod runtime_actions;
#[cfg(feature = "legacy-shell")]
mod runtime_events;
#[cfg(feature = "legacy-shell")]
mod runtime_input;
#[cfg(feature = "legacy-shell")]
mod runtime_render;
#[cfg(feature = "legacy-shell")]
mod runtime_startup;
#[cfg(feature = "legacy-shell")]
mod runtime_state;
#[cfg(feature = "legacy-shell")]
mod scene_cache;
#[cfg(feature = "legacy-shell")]
mod scene_rebuild;
mod startup;
#[cfg(feature = "legacy-shell")]
#[path = "../../../../src/app_core/native_shell/composition/runtime/text_entry/mod.rs"]
mod text_bpm;
#[cfg(feature = "legacy-shell")]
mod text_edit;
mod text_renderer;
#[cfg(feature = "legacy-shell")]
mod text_runtime;

#[cfg(feature = "legacy-shell")]
use self::{
    input::*, profiling::*, runtime_state::*, scene_cache::*, scene_rebuild::*, startup::*,
    text_bpm::*, text_edit::*, text_renderer::*,
};
#[cfg(not(feature = "legacy-shell"))]
use self::{startup::*, text_renderer::*};
#[cfg(feature = "legacy-shell")]
pub(crate) use legacy_shell_runtime::run_legacy_shell_vello_app_with_artifacts;

pub use self::{
    generic_runtime::{
        NativeGenericRunReport, NativeGenericRuntimeArtifacts, run_native_vello_runtime,
        run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};

#[cfg(feature = "legacy-shell")]
const FOCUS_PULSE_HZ: u64 = 60;
#[cfg(feature = "legacy-shell")]
const IDLE_STATUS_REFRESH_HZ: u64 = 4;
/// Short-lived redraw cadence used immediately after cursor movement.
#[cfg(feature = "legacy-shell")]
const CURSOR_ACTIVITY_REDRAW_HZ: u64 = 120;
/// Duration to keep the high-frequency cursor redraw cadence active.
#[cfg(feature = "legacy-shell")]
const CURSOR_ACTIVITY_REDRAW_WINDOW: Duration = Duration::from_millis(100);
/// High-refresh surface present-mode preference order for animation-heavy playback UI.
const HIGH_REFRESH_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 3] = [
    wgpu::PresentMode::Mailbox,
    wgpu::PresentMode::Immediate,
    wgpu::PresentMode::AutoVsync,
];
/// Standard present-mode preference order for non-high-refresh UI.
const STANDARD_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 1] = [wgpu::PresentMode::AutoVsync];
/// Maximum retained image-upload blobs before cache reset.
#[cfg(feature = "legacy-shell")]
const IMAGE_UPLOAD_BLOB_CACHE_LIMIT: usize = 32;
#[cfg(feature = "legacy-shell")]
const INCREMENTAL_FRAME_PIPELINE_ENV: &str = "RADIANT_NATIVE_INCREMENTAL_FRAME_PIPELINE";
/// Maximum time to wait for a deferred startup refresh before revealing anyway.
#[cfg(feature = "legacy-shell")]
const STARTUP_REVEAL_STALL_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeUserEvent {
    RepaintRequested,
}

#[cfg(feature = "legacy-shell")]
fn try_mark_repaint_event_pending(pending: &AtomicBool) -> bool {
    pending
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

/// Return the ordered present-mode fallback chain for the configured frame target.
fn present_mode_candidates(target_fps: u32) -> &'static [wgpu::PresentMode] {
    if target_fps >= 120 {
        &HIGH_REFRESH_PRESENT_MODE_CANDIDATES
    } else {
        &STANDARD_PRESENT_MODE_CANDIDATES
    }
}

fn select_present_mode(
    target_fps: u32,
    supported_present_modes: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    present_mode_candidates(target_fps)
        .iter()
        .copied()
        .find(|mode| present_mode_is_supported(*mode, supported_present_modes))
        .or_else(|| supported_present_modes.first().copied())
        .unwrap_or(wgpu::PresentMode::Fifo)
}

fn present_mode_is_supported(
    present_mode: wgpu::PresentMode,
    supported_present_modes: &[wgpu::PresentMode],
) -> bool {
    matches!(
        present_mode,
        wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync
    ) || supported_present_modes.contains(&present_mode)
}

/// Build renderer startup options for the native shell's fixed AA strategy.
///
/// The native runtime currently renders every frame with [`AaConfig::Area`], so
/// startup should avoid compiling MSAA shader variants that will never be used.
fn startup_renderer_options() -> RendererOptions {
    RendererOptions {
        antialiasing_support: AaSupport::area_only(),
        ..RendererOptions::default()
    }
}

/// Convert one logical pointer point into lossless-enough action coordinates.
#[cfg(feature = "legacy-shell")]
fn ui_action_pointer_coords(point: Point) -> (u16, u16) {
    (
        point.x.clamp(0.0, f32::from(u16::MAX)).round() as u16,
        point.y.clamp(0.0, f32::from(u16::MAX)).round() as u16,
    )
}

#[cfg(feature = "legacy-shell")]
struct NativeVelloRunner<B: NativeAppBridge> {
    options: NativeRunOptions,
    bridge: B,
    repaint_event_pending: Arc<AtomicBool>,
    /// Enable bridge-driven static segment rebuild gating.
    incremental_frame_pipeline: bool,
    model: Arc<AppModel>,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    redraw_requested: bool,
    /// Retained static scene primitives (layout and stable content).
    frame_cache: NativeViewFrame,
    /// Retained per-segment static frame fragments.
    static_segment_frame_cache: StaticFrameSegments,
    /// Retained immutable static segment nodes for diff-based rebuild planning.
    static_segment_graph: StaticSegmentStateGraph,
    /// Retained per-segment static encoded scenes.
    static_segment_scene_cache: StaticSegmentSceneCache,
    /// Retained hover/editor overlay primitives.
    hover_overlay_frame_cache: NativeViewFrame,
    /// Retained focus-emphasis overlay primitives.
    focus_overlay_frame_cache: NativeViewFrame,
    /// Retained modal/popover overlay primitives.
    modal_overlay_frame_cache: NativeViewFrame,
    /// Retained waveform-motion overlay primitives (cursor/playhead/hover marker).
    waveform_motion_overlay_frame_cache: NativeViewFrame,
    /// Retained chrome-motion overlay primitives (toolbar/tabs/status/lamp pulse).
    chrome_motion_overlay_frame_cache: NativeViewFrame,
    /// Full scene sent to Vello after combining static + overlay scenes.
    scene: Scene,
    /// Cached encoded static scene.
    static_scene: Scene,
    /// Cached encoded hover/editor overlay scene.
    hover_overlay_scene: Scene,
    /// Cached encoded focus-emphasis overlay scene.
    focus_overlay_scene: Scene,
    /// Cached encoded modal/popover overlay scene.
    modal_overlay_scene: Scene,
    /// Cached encoded composite for hover/editor and focus-emphasis overlays.
    state_overlay_scene: Scene,
    /// Cached encoded waveform-motion overlay scene.
    waveform_motion_overlay_scene: Scene,
    /// Cached encoded chrome-motion overlay scene.
    chrome_motion_overlay_scene: Scene,
    /// Cached encoded composite for waveform/chrome motion overlays.
    motion_overlay_scene: Scene,
    /// Retained blobs for repeated image draw payload uploads.
    image_upload_blob_cache: HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
    /// Recency queue for bounded retained image-upload blob eviction.
    image_upload_blob_cache_order: VecDeque<ImageUploadBlobCacheKey>,
    /// Last hover-overlay fingerprint used for cache-skip checks.
    hover_overlay_fingerprint: Option<HoverOverlayCacheFingerprint>,
    /// Last focus-overlay fingerprint used for cache-skip checks.
    focus_overlay_fingerprint: Option<FocusOverlayCacheFingerprint>,
    /// Last modal-overlay fingerprint used for cache-skip checks.
    modal_overlay_fingerprint: Option<ModalOverlayCacheFingerprint>,
    /// Last waveform-motion fingerprint used for cache-skip checks.
    waveform_motion_overlay_fingerprint: Option<WaveformMotionOverlayCacheFingerprint>,
    /// Last chrome-motion fingerprint used for cache-skip checks.
    chrome_motion_overlay_fingerprint: Option<ChromeMotionOverlayCacheFingerprint>,
    /// Cached latest motion-only model for lightweight overlay rebuilds.
    motion_model: Option<NativeMotionModel>,
    /// Whether the active bridge supports `project_motion_model`.
    motion_model_supported: bool,
    /// Latest bridge-provided static segment revision snapshot.
    segment_revisions: SegmentRevisions,
    /// Whether the bridge reports non-zero static segment revisions.
    segment_revisions_supported: bool,
    /// Whether we already forced one rebuild for zero-revision bridge fallbacks.
    missing_segment_revision_fallback_applied: bool,
    text_renderer: NativeTextRenderer,
    style_cache: Option<StyleTokens>,
    frame_state: NativeVelloFrameState,
    layout_runtime: ShellLayoutRuntime,
    shell_layout: Option<Arc<ShellLayout>>,
    shell_state: NativeShellState,
    clear_color: Rgba8,
    cursor_icon: CursorIcon,
    last_cursor: Option<Point>,
    pending_cursor: Option<Point>,
    /// Pending first keypress for a multi-step hotkey chord.
    pending_hotkey_chord: Option<KeyPress>,
    /// Latest queued top-bar volume update in normalized milli space.
    pending_volume_milli: Option<u16>,
    /// Active waveform drag mode while primary pointer is held on waveform.
    waveform_drag_mode: Option<WaveformPointerDragMode>,
    /// Whether the next waveform view-based interaction must refresh local bounds.
    waveform_view_refresh_pending: bool,
    /// Exact press snapshot used for plain waveform click-to-seek release handling.
    waveform_click_seek_press: Option<WaveformClickSeekPress>,
    /// Deferred browser-row press captured until click-vs-drag resolution.
    pending_browser_row_press: Option<PendingBrowserRowPress>,
    /// Active browser content-item drag state for primary pointer movement.
    content_item_drag: Option<ContentItemDragState>,
    /// Whether a waveform-selection export drag is currently active.
    selection_drag_active: bool,
    /// Last waveform drag action emitted for pointer-move dedupe.
    last_emitted_waveform_drag_action: Option<UiAction>,
    /// Whether map content focus drag is active for primary pointer movement.
    map_focus_drag_active: bool,
    /// Last map content id emitted during active map focus drag.
    last_emitted_map_drag_content_id: Option<String>,
    /// Active folder-scrollbar thumb drag state for primary pointer movement.
    folder_scrollbar_drag: Option<FolderScrollbarDragState>,
    /// Active browser-scrollbar thumb drag state for primary pointer movement.
    browser_scrollbar_drag: Option<BrowserScrollbarDragState>,
    /// Last emitted browser viewport start during an active scrollbar drag.
    last_emitted_browser_view_start: Option<usize>,
    /// Active waveform-scrollbar thumb drag state for primary pointer movement.
    waveform_scrollbar_drag: Option<WaveformScrollbarDragState>,
    /// Active middle-button waveform pan drag state.
    waveform_pan_drag: Option<WaveformPanDragState>,
    /// Last emitted waveform viewport center during active drag gestures.
    last_emitted_waveform_view_center: Option<u32>,
    volume_drag_active: bool,
    last_emitted_volume_milli: Option<u16>,
    modifiers: ModifiersState,
    text_input_target: TextInputTarget,
    text_input_buffer: Option<String>,
    text_editor_state: Option<SingleLineTextEditorState>,
    active_text_field_visual_cache: Option<ActiveTextFieldVisualCacheEntry>,
    text_input_drag_active: bool,
    waveform_bpm_input_buffer: Option<String>,
    clipboard: Option<arboard::Clipboard>,
    clipboard_fallback_text: String,
    last_redraw: Instant,
    resumed_count: u32,
    window_event_count: u32,
    redraw_count: u32,
    /// Whether at least one frame has been presented to the native surface.
    first_frame_presented: bool,
    /// Whether the window has been revealed after startup frame sequencing.
    startup_window_visible: bool,
    /// Whether the first startup full-model pull is deferred until first present.
    startup_model_pull_pending: bool,
    /// Whether deferred startup full-model refresh is pending completion.
    startup_deferred_model_refresh_pending: bool,
    /// Deadline used to prevent startup reveal from stalling indefinitely.
    startup_reveal_deadline: Option<Instant>,
    /// Startup first-paint timing profile.
    startup_timing: StartupTimingProfile,
    target_frame_interval: Duration,
    focus_animation_interval: Duration,
    idle_status_refresh_interval: Duration,
    next_idle_status_refresh: Instant,
    cursor_activity_redraw_interval: Duration,
    cursor_activity_redraw_until: Option<Instant>,
    model_refresh_count: u32,
    profiler: NativeVelloProfiler,
}

#[cfg(all(test, feature = "legacy-shell"))]
mod tests;
