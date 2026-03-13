//! Native `winit + vello` runtime preview used for backend selection rollout.

use super::{NativeRunOptions, WindowIconRgba};
use crate::app::{
    AppModel, DirtySegments, FrameBuildResult, NativeAppBridge, NativeMotionModel,
    SegmentRevisions, UiAction,
};
use crate::gui::{
    input::{KeyCode, key_code_from_winit},
    native_shell::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, NativeViewFrame,
        Primitive, ShellLayout, ShellLayoutDirtyKind, ShellLayoutRuntime, ShellNodeKind,
        StateOverlayFingerprint, StaticFrameSegment, StaticFrameSegments, StyleTokens, TextAlign,
        TextFieldVisualState, TextRun, WaveformMotionOverlayFingerprint,
    },
    repaint::RepaintSignal,
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::panic::AssertUnwindSafe;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Circle, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData, ImageAlphaType, ImageData, ImageFormat},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
    window::{CursorIcon, Icon, Window, WindowAttributes, WindowId},
};

mod input;
mod profiling;
mod runtime_actions;
mod runtime_events;
mod runtime_input;
mod runtime_render;
mod runtime_startup;
mod runtime_state;
mod scene_cache;
mod scene_rebuild;
mod startup;
mod text_bpm;
mod text_edit;
mod text_renderer;
mod text_runtime;

use self::{
    input::*, profiling::*, runtime_state::*, scene_cache::*, scene_rebuild::*, startup::*,
    text_bpm::*, text_edit::*, text_renderer::*,
};
const FOCUS_PULSE_HZ: u64 = 60;
const IDLE_STATUS_REFRESH_HZ: u64 = 4;
/// Short-lived redraw cadence used immediately after cursor movement.
const CURSOR_ACTIVITY_REDRAW_HZ: u64 = 120;
/// Duration to keep the high-frequency cursor redraw cadence active.
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
const IMAGE_UPLOAD_BLOB_CACHE_LIMIT: usize = 32;
const INCREMENTAL_FRAME_PIPELINE_ENV: &str = "SEMPAL_NATIVE_INCREMENTAL_FRAME_PIPELINE";
/// Maximum time to wait for a deferred startup refresh before revealing anyway.
const STARTUP_REVEAL_STALL_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeUserEvent {
    RepaintRequested,
}

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

/// Convert one logical pointer point into lossless-enough action coordinates.
fn ui_action_pointer_coords(point: Point) -> (u16, u16) {
    (
        point.x.clamp(0.0, f32::from(u16::MAX)).round() as u16,
        point.y.clamp(0.0, f32::from(u16::MAX)).round() as u16,
    )
}

#[derive(Clone)]
struct EventLoopProxyRepaintSignal {
    proxy: EventLoopProxy<RuntimeUserEvent>,
    pending: Arc<AtomicBool>,
}

impl EventLoopProxyRepaintSignal {
    fn new(proxy: EventLoopProxy<RuntimeUserEvent>, pending: Arc<AtomicBool>) -> Self {
        Self { proxy, pending }
    }
}

impl RepaintSignal for EventLoopProxyRepaintSignal {
    fn request_repaint(&self) {
        if !try_mark_repaint_event_pending(self.pending.as_ref()) {
            return;
        }
        if self
            .proxy
            .send_event(RuntimeUserEvent::RepaintRequested)
            .is_err()
        {
            self.pending.store(false, Ordering::Release);
        }
    }
}

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
    /// Retained state-driven overlay primitives (focus/hover and dialog state).
    state_overlay_frame_cache: NativeViewFrame,
    /// Retained waveform-motion overlay primitives (cursor/playhead/hover marker).
    waveform_motion_overlay_frame_cache: NativeViewFrame,
    /// Retained chrome-motion overlay primitives (toolbar/tabs/status/lamp pulse).
    chrome_motion_overlay_frame_cache: NativeViewFrame,
    /// Full scene sent to Vello after combining static + overlay scenes.
    scene: Scene,
    /// Cached encoded static scene.
    static_scene: Scene,
    /// Cached encoded state-driven overlay scene.
    state_overlay_scene: Scene,
    /// Cached encoded waveform-motion overlay scene.
    waveform_motion_overlay_scene: Scene,
    /// Cached encoded chrome-motion overlay scene.
    chrome_motion_overlay_scene: Scene,
    /// Retained blobs for repeated image draw payload uploads.
    image_upload_blob_cache: HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
    /// Recency queue for bounded retained image-upload blob eviction.
    image_upload_blob_cache_order: VecDeque<ImageUploadBlobCacheKey>,
    /// Last state-overlay fingerprint used for cache-skip checks.
    state_overlay_fingerprint: Option<StateOverlayCacheFingerprint>,
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
    /// Latest queued top-bar volume update in normalized milli space.
    pending_volume_milli: Option<u16>,
    /// Active waveform drag mode while primary pointer is held on waveform.
    waveform_drag_mode: Option<WaveformPointerDragMode>,
    /// Whether a no-drag waveform release should clear the old playback selection first.
    clear_playback_selection_on_click_release: bool,
    /// Whether a waveform-selection export drag is currently active.
    selection_drag_active: bool,
    /// Last waveform drag action emitted for pointer-move dedupe.
    last_emitted_waveform_drag_action: Option<UiAction>,
    /// Whether map sample focus drag is active for primary pointer movement.
    map_focus_drag_active: bool,
    /// Last map sample id emitted during active map focus drag.
    last_emitted_map_drag_sample_id: Option<String>,
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

#[derive(Default)]
struct PreviewBridge;

impl NativeAppBridge for PreviewBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(AppModel::default())
    }
}

/// Run the native Vello backend window with a host-provided app bridge.
///
/// The runtime loop is owned by winit and blocks until the native window
/// closes. The host receives user input each frame through the bridge-driven
/// action path, and this function returns the host result from the event loop
/// invocation.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    info!("radiant native vello: creating event loop");
    let run_started = Instant::now();
    let event_loop = EventLoop::<RuntimeUserEvent>::with_user_event()
        .build()
        .map_err(|err| err.to_string())?;
    info!(
        "radiant native vello: event loop created with window_size={:?} min_window_size={:?} target_fps={}",
        options.inner_size, options.min_inner_size, options.target_fps
    );
    let mut runner = NativeVelloRunner::new(options, bridge);
    let repaint_signal: Arc<dyn RepaintSignal> = Arc::new(EventLoopProxyRepaintSignal::new(
        event_loop.create_proxy(),
        Arc::clone(&runner.repaint_event_pending),
    ));
    runner.bridge.install_repaint_signal(repaint_signal);
    info!("radiant native vello: runner initialized");
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    let elapsed = run_started.elapsed();
    match &run_result {
        Ok(_) => info!(
            "radiant native vello: event loop ended in {} ms",
            elapsed.as_millis()
        ),
        Err(err) => warn!(
            "radiant native vello: event loop returned error in {} ms: {}",
            elapsed.as_millis(),
            err
        ),
    }
    info!("radiant native vello: event loop finished");
    runner.bridge.on_runtime_exit();
    run_result
}

/// Run the native Vello backend using a declarative state+reducer bridge.
///
/// This is an API-level alias to [`run_native_vello_app`] that emphasizes
/// one-way declarative host integration (`project_model` + `reduce_action`).
pub fn run_native_vello_app_declarative<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    run_native_vello_app(options, bridge)
}

/// Run the experimental native Vello backend window for backend-selection testing.
///
/// This preview path now renders an interactive backend-neutral shell model with
/// Vello primitives and exercises native input hit-testing without `egui`.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    run_native_vello_app_declarative(options, PreviewBridge)
}

/// Capture a deterministic native-shell automation snapshot without launching a window.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &AppModel,
) -> crate::app::GuiAutomationSnapshot {
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut shell_state = NativeShellState::new();
    shell_state.sync_from_model(model);
    shell_state.automation_snapshot(&layout, model)
}

#[cfg(test)]
mod tests;
