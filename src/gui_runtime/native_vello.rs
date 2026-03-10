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
mod scene_rebuild;
mod text_edit;
mod text_renderer;

use self::{input::*, scene_rebuild::*, text_edit::*, text_renderer::*};

#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_INTERVAL_FRAMES: u64 = 240;
#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_ENV: &str = "SEMPAL_NATIVE_RENDER_PROFILE";
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
const STARTUP_PROFILE_ENV: &str = "SEMPAL_NATIVE_STARTUP_PROFILE";
/// Maximum time to wait for a deferred startup refresh before revealing anyway.
const STARTUP_REVEAL_STALL_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeUserEvent {
    RepaintRequested,
}

/// Stable cache key for one retained image-upload blob payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ImageUploadBlobCacheKey {
    pixels_ptr: usize,
    width: u32,
    height: u32,
}

/// Sized wrapper allowing `Arc<[u8]>` payloads to be reused by `Blob<u8>`.
struct SharedPixelBytes(Arc<[u8]>);

impl AsRef<[u8]> for SharedPixelBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
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

/// Active browser-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BrowserScrollbarDragState {
    /// Pointer offset from the top edge of the thumb captured at drag start.
    thumb_pointer_offset_y: f32,
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

/// Interaction classes tracked by runtime performance profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InteractionProfileKind {
    /// Pointer hover and focus-follow updates in list surfaces.
    Hover,
    /// Wheel-driven browser row navigation updates.
    Wheel,
    /// Map-pan proxy updates produced by map-mode pointer motion.
    MapPanProxy,
    /// Waveform interaction actions (seek/cursor/selection/zoom).
    Waveform,
    /// Top-bar volume slider drag/click updates.
    Volume,
}

#[cfg(feature = "gui-performance")]
/// Aggregate timing stats for one interaction profile class.
#[derive(Clone, Copy, Debug, Default)]
struct InteractionProfileStats {
    samples: u64,
    total_ns: u128,
    max_ns: u128,
}

#[cfg(feature = "gui-performance")]
impl InteractionProfileStats {
    /// Record one interaction latency sample.
    fn record(&mut self, duration: Duration) {
        let nanos = duration.as_nanos();
        self.samples = self.samples.saturating_add(1);
        self.total_ns = self.total_ns.saturating_add(nanos);
        self.max_ns = self.max_ns.max(nanos);
    }

    /// Return average latency in milliseconds.
    fn avg_ms(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }
        (self.total_ns as f64 / self.samples as f64) / 1_000_000.0
    }

    /// Return max latency in milliseconds.
    fn max_ms(&self) -> f64 {
        self.max_ns as f64 / 1_000_000.0
    }
}

#[cfg(feature = "gui-performance")]
/// Optional frame-time and overlay rebuild profiler.
///
/// Enabled only when `cfg(feature = "gui-performance")` is active and the
/// `SEMPAL_NATIVE_RENDER_PROFILE` environment variable is set to a truthy value.
/// All instrumentation values are accumulated and periodically emitted on stderr
/// to avoid per-frame logging overhead on hot paths.
#[derive(Debug, Default)]
struct NativeVelloProfiler {
    enabled: bool,
    frames: u64,
    rebuild_ns: u128,
    acquire_ns: u128,
    render_ns: u128,
    blit_ns: u128,
    present_ns: u128,
    total_ns: u128,
    scene_rebuilds: u64,
    state_overlay_rebuilds: u64,
    motion_overlay_rebuilds: u64,
    model_refreshes: u64,
    model_pull_ns: u128,
    motion_pull_ns: u128,
    bridge_model_pull_rebuilds: u64,
    bridge_motion_pull_rebuilds: u64,
    explicit_static_rebuilds: u64,
    dirty_mask_static_rebuilds: u64,
    tick_ns: u128,
    build_static_ns: u128,
    build_state_overlay_ns: u128,
    build_motion_overlay_ns: u128,
    encode_static_ns: u128,
    encode_state_overlay_ns: u128,
    encode_motion_overlay_ns: u128,
    motion_overlay_skips: u64,
    hover_latency: InteractionProfileStats,
    wheel_latency: InteractionProfileStats,
    map_pan_proxy_latency: InteractionProfileStats,
    waveform_latency: InteractionProfileStats,
    volume_latency: InteractionProfileStats,
}

#[cfg(feature = "gui-performance")]
impl NativeVelloProfiler {
    fn new() -> Self {
        let enabled = crate::env_flags::env_var_truthy(REDRAW_PROFILE_ENV);
        Self {
            enabled,
            ..Self::default()
        }
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    fn now_if_enabled(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }

    #[inline]
    fn add_tick(&mut self, duration: Duration) {
        self.tick_ns = self.tick_ns.saturating_add(duration.as_nanos());
    }

    #[inline]
    fn record_scene_rebuilds(&mut self, scene: bool, state_overlay: bool, motion_overlay: bool) {
        if scene {
            self.scene_rebuilds = self.scene_rebuilds.saturating_add(1);
        }
        if state_overlay {
            self.state_overlay_rebuilds = self.state_overlay_rebuilds.saturating_add(1);
        }
        if motion_overlay {
            self.motion_overlay_rebuilds = self.motion_overlay_rebuilds.saturating_add(1);
        }
    }

    #[inline]
    fn add_model_refresh(&mut self) {
        self.model_refreshes = self.model_refreshes.saturating_add(1);
    }

    #[inline]
    fn add_model_pull(&mut self, duration: Duration) {
        self.model_pull_ns = self.model_pull_ns.saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_bridge_model_pull_rebuild(&mut self) {
        self.bridge_model_pull_rebuilds = self.bridge_model_pull_rebuilds.saturating_add(1);
    }

    #[inline]
    fn add_bridge_motion_pull_rebuild(&mut self) {
        self.bridge_motion_pull_rebuilds = self.bridge_motion_pull_rebuilds.saturating_add(1);
    }

    #[inline]
    fn add_explicit_static_rebuild(&mut self) {
        self.explicit_static_rebuilds = self.explicit_static_rebuilds.saturating_add(1);
    }

    #[inline]
    fn add_dirty_mask_static_rebuild(&mut self) {
        self.dirty_mask_static_rebuilds = self.dirty_mask_static_rebuilds.saturating_add(1);
    }

    #[inline]
    fn add_motion_pull(&mut self, duration: Duration) {
        self.motion_pull_ns = self.motion_pull_ns.saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_motion_overlay_skip(&mut self) {
        self.motion_overlay_skips = self.motion_overlay_skips.saturating_add(1);
    }

    #[inline]
    fn add_build_static(&mut self, duration: Duration) {
        self.build_static_ns = self.build_static_ns.saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_build_state_overlay(&mut self, duration: Duration) {
        self.build_state_overlay_ns = self
            .build_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_build_motion_overlay(&mut self, duration: Duration) {
        self.build_motion_overlay_ns = self
            .build_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_encode_static(&mut self, duration: Duration) {
        self.encode_static_ns = self.encode_static_ns.saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_encode_state_overlay(&mut self, duration: Duration) {
        self.encode_state_overlay_ns = self
            .encode_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    #[inline]
    fn add_encode_motion_overlay(&mut self, duration: Duration) {
        self.encode_motion_overlay_ns = self
            .encode_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    /// Add one interaction latency sample for the requested interaction class.
    #[inline]
    fn add_interaction_latency(&mut self, kind: InteractionProfileKind, duration: Duration) {
        match kind {
            InteractionProfileKind::Hover => self.hover_latency.record(duration),
            InteractionProfileKind::Wheel => self.wheel_latency.record(duration),
            InteractionProfileKind::MapPanProxy => self.map_pan_proxy_latency.record(duration),
            InteractionProfileKind::Waveform => self.waveform_latency.record(duration),
            InteractionProfileKind::Volume => self.volume_latency.record(duration),
        }
    }

    fn record_redraw(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
        text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
        if !self.enabled {
            return;
        }
        self.frames = self.frames.saturating_add(1);
        self.rebuild_ns = self.rebuild_ns.saturating_add(rebuild.as_nanos());
        self.acquire_ns = self.acquire_ns.saturating_add(acquire.as_nanos());
        self.render_ns = self.render_ns.saturating_add(render.as_nanos());
        self.blit_ns = self.blit_ns.saturating_add(blit.as_nanos());
        self.present_ns = self.present_ns.saturating_add(present.as_nanos());
        self.total_ns = self.total_ns.saturating_add(total.as_nanos());

        if self.frames < REDRAW_PROFILE_INTERVAL_FRAMES {
            return;
        }

        let frames = self.frames as f64;
        let total_ns = self.total_ns as f64;
        if total_ns <= 0.0 {
            self.reset();
            return;
        }

        let ms = |value_ns: u128| value_ns as f64 / 1_000_000.0;
        let avg_total_ms = ms(self.total_ns) / frames;
        let avg_rebuild_ms = ms(self.rebuild_ns) / frames;
        let avg_acquire_ms = ms(self.acquire_ns) / frames;
        let avg_render_ms = ms(self.render_ns) / frames;
        let avg_blit_ms = ms(self.blit_ns) / frames;
        let avg_present_ms = ms(self.present_ns) / frames;
        let avg_model_pull_ms = ms(self.model_pull_ns) / frames;
        let avg_motion_pull_ms = ms(self.motion_pull_ns) / frames;
        let avg_tick_ms = ms(self.tick_ns) / frames;
        let avg_build_static_ms = ms(self.build_static_ns) / frames;
        let avg_build_state_overlay_ms = ms(self.build_state_overlay_ns) / frames;
        let avg_build_motion_overlay_ms = ms(self.build_motion_overlay_ns) / frames;
        let avg_encode_static_ms = ms(self.encode_static_ns) / frames;
        let avg_encode_state_overlay_ms = ms(self.encode_state_overlay_ns) / frames;
        let avg_encode_motion_overlay_ms = ms(self.encode_motion_overlay_ns) / frames;
        let fps = 1000.0 / avg_total_ms.max(0.001);
        let rebuild_pct = (self.rebuild_ns as f64) * 100.0 / total_ns;
        let acquire_pct = (self.acquire_ns as f64) * 100.0 / total_ns;
        let render_pct = (self.render_ns as f64) * 100.0 / total_ns;
        let blit_pct = (self.blit_ns as f64) * 100.0 / total_ns;
        let present_pct = (self.present_ns as f64) * 100.0 / total_ns;
        let model_refresh_avg = self.model_refreshes as f64 / frames;
        let scene_rebuild_avg = self.scene_rebuilds as f64 / frames;
        let state_overlay_rebuild_avg = self.state_overlay_rebuilds as f64 / frames;
        let motion_overlay_rebuild_avg = self.motion_overlay_rebuilds as f64 / frames;
        let motion_overlay_skip_avg = self.motion_overlay_skips as f64 / frames;
        let bridge_model_pull_rebuild_avg = self.bridge_model_pull_rebuilds as f64 / frames;
        let bridge_motion_pull_rebuild_avg = self.bridge_motion_pull_rebuilds as f64 / frames;
        let explicit_static_rebuild_avg = self.explicit_static_rebuilds as f64 / frames;
        let dirty_mask_static_rebuild_avg = self.dirty_mask_static_rebuilds as f64 / frames;
        let (text_hits, text_misses, text_evictions, atom_hits, atom_misses, atom_evictions) =
            text_profile;
        let text_cache_hit_rate = if text_hits + text_misses == 0 {
            0.0
        } else {
            100.0 * (text_hits as f64) / (text_hits + text_misses) as f64
        };
        let text_cache_miss_rate = if text_hits + text_misses == 0 {
            0.0
        } else {
            100.0 * (text_misses as f64) / (text_hits + text_misses) as f64
        };
        let atom_cache_hit_rate = if atom_hits + atom_misses == 0 {
            0.0
        } else {
            100.0 * (atom_hits as f64) / (atom_hits + atom_misses) as f64
        };
        let atom_cache_miss_rate = if atom_hits + atom_misses == 0 {
            0.0
        } else {
            100.0 * (atom_misses as f64) / (atom_hits + atom_misses) as f64
        };
        let hover_samples = self.hover_latency.samples;
        let wheel_samples = self.wheel_latency.samples;
        let map_samples = self.map_pan_proxy_latency.samples;
        let waveform_samples = self.waveform_latency.samples;
        let volume_samples = self.volume_latency.samples;
        let hover_avg_ms = self.hover_latency.avg_ms();
        let wheel_avg_ms = self.wheel_latency.avg_ms();
        let map_avg_ms = self.map_pan_proxy_latency.avg_ms();
        let waveform_avg_ms = self.waveform_latency.avg_ms();
        let volume_avg_ms = self.volume_latency.avg_ms();
        let hover_max_ms = self.hover_latency.max_ms();
        let wheel_max_ms = self.wheel_latency.max_ms();
        let map_max_ms = self.map_pan_proxy_latency.max_ms();
        let waveform_max_ms = self.waveform_latency.max_ms();
        let volume_max_ms = self.volume_latency.max_ms();
        eprintln!(
            "[native-vello] redraw avg over {REDRAW_PROFILE_INTERVAL_FRAMES} frames: \
             total={avg_total_ms:.2}ms ({fps:.1} fps) rebuild={avg_rebuild_ms:.2}ms ({rebuild_pct:.1}%) \
             acquire={avg_acquire_ms:.2}ms ({acquire_pct:.1}%) render={avg_render_ms:.2}ms ({render_pct:.1}%) \
             blit={avg_blit_ms:.2}ms ({blit_pct:.1}%) present={avg_present_ms:.2}ms ({present_pct:.1}%) \
             model_refresh_avg={model_refresh_avg:.2} scene_rebuild_avg={scene_rebuild_avg:.2} \
             state_overlay_rebuild_avg={state_overlay_rebuild_avg:.2} \
             motion_overlay_rebuild_avg={motion_overlay_rebuild_avg:.2} motion_overlay_skip_avg={motion_overlay_skip_avg:.2} \
             bridge_model_pull_rebuild_avg={bridge_model_pull_rebuild_avg:.2} \
             bridge_motion_pull_rebuild_avg={bridge_motion_pull_rebuild_avg:.2} \
             explicit_static_rebuild_avg={explicit_static_rebuild_avg:.2} \
             dirty_mask_static_rebuild_avg={dirty_mask_static_rebuild_avg:.2} \
             model_pull_ms={avg_model_pull_ms:.3} motion_pull_ms={avg_motion_pull_ms:.3} \
             tick_ms={avg_tick_ms:.3} build_static_ms={avg_build_static_ms:.3} \
             build_state_overlay_ms={avg_build_state_overlay_ms:.3} build_motion_overlay_ms={avg_build_motion_overlay_ms:.3} \
             encode_static_ms={avg_encode_static_ms:.3} encode_state_overlay_ms={avg_encode_state_overlay_ms:.3} \
             encode_motion_overlay_ms={avg_encode_motion_overlay_ms:.3} \
             hover_samples={hover_samples} hover_avg_ms={hover_avg_ms:.3} hover_max_ms={hover_max_ms:.3} \
             wheel_samples={wheel_samples} wheel_avg_ms={wheel_avg_ms:.3} wheel_max_ms={wheel_max_ms:.3} \
             map_proxy_samples={map_samples} map_proxy_avg_ms={map_avg_ms:.3} map_proxy_max_ms={map_max_ms:.3} \
             waveform_samples={waveform_samples} waveform_avg_ms={waveform_avg_ms:.3} waveform_max_ms={waveform_max_ms:.3} \
             volume_samples={volume_samples} volume_avg_ms={volume_avg_ms:.3} volume_max_ms={volume_max_ms:.3} \
             text_layout_hits={text_hits} text_layout_misses={text_misses} text_layout_evictions={text_evictions} \
             text_hit_rate={text_cache_hit_rate:.1}% text_miss_rate={text_cache_miss_rate:.1}% \
             text_atom_hits={atom_hits} text_atom_misses={atom_misses} text_atom_evictions={atom_evictions} \
             text_atom_hit_rate={atom_cache_hit_rate:.1}% text_atom_miss_rate={atom_cache_miss_rate:.1}%"
        );
        self.reset();
    }

    #[inline]
    fn reset(&mut self) {
        self.frames = 0;
        self.rebuild_ns = 0;
        self.acquire_ns = 0;
        self.render_ns = 0;
        self.blit_ns = 0;
        self.present_ns = 0;
        self.total_ns = 0;
        self.scene_rebuilds = 0;
        self.state_overlay_rebuilds = 0;
        self.motion_overlay_rebuilds = 0;
        self.model_refreshes = 0;
        self.model_pull_ns = 0;
        self.motion_pull_ns = 0;
        self.bridge_model_pull_rebuilds = 0;
        self.bridge_motion_pull_rebuilds = 0;
        self.explicit_static_rebuilds = 0;
        self.dirty_mask_static_rebuilds = 0;
        self.tick_ns = 0;
        self.build_static_ns = 0;
        self.build_state_overlay_ns = 0;
        self.build_motion_overlay_ns = 0;
        self.encode_static_ns = 0;
        self.encode_state_overlay_ns = 0;
        self.encode_motion_overlay_ns = 0;
        self.motion_overlay_skips = 0;
        self.hover_latency = InteractionProfileStats::default();
        self.wheel_latency = InteractionProfileStats::default();
        self.map_pan_proxy_latency = InteractionProfileStats::default();
        self.waveform_latency = InteractionProfileStats::default();
        self.volume_latency = InteractionProfileStats::default();
    }
}

#[cfg(not(feature = "gui-performance"))]
#[derive(Debug, Default)]
struct NativeVelloProfiler;

#[cfg(not(feature = "gui-performance"))]
impl NativeVelloProfiler {
    fn new() -> Self {
        Self
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        false
    }

    #[inline]
    fn now_if_enabled(&self) -> Option<Instant> {
        None
    }

    #[inline]
    fn add_tick(&mut self, _duration: Duration) {}
    #[inline]
    fn record_scene_rebuilds(&mut self, _scene: bool, _state_overlay: bool, _motion_overlay: bool) {
    }
    #[inline]
    fn add_model_refresh(&mut self) {}
    #[inline]
    fn add_model_pull(&mut self, _duration: Duration) {}
    #[inline]
    fn add_bridge_model_pull_rebuild(&mut self) {}
    #[inline]
    fn add_bridge_motion_pull_rebuild(&mut self) {}
    #[inline]
    fn add_explicit_static_rebuild(&mut self) {}
    #[inline]
    fn add_dirty_mask_static_rebuild(&mut self) {}
    #[inline]
    fn add_motion_pull(&mut self, _duration: Duration) {}
    #[inline]
    fn add_motion_overlay_skip(&mut self) {}
    #[inline]
    fn add_build_static(&mut self, _duration: Duration) {}
    #[inline]
    fn add_build_state_overlay(&mut self, _duration: Duration) {}
    #[inline]
    fn add_build_motion_overlay(&mut self, _duration: Duration) {}
    #[inline]
    fn add_encode_static(&mut self, _duration: Duration) {}
    #[inline]
    fn add_encode_state_overlay(&mut self, _duration: Duration) {}
    #[inline]
    fn add_encode_motion_overlay(&mut self, _duration: Duration) {}
    /// No-op interaction latency recorder for non-profile builds.
    #[inline]
    fn add_interaction_latency(&mut self, _kind: InteractionProfileKind, _duration: Duration) {}
    fn record_redraw(
        &mut self,
        _rebuild: Duration,
        _acquire: Duration,
        _render: Duration,
        _blit: Duration,
        _present: Duration,
        _total: Duration,
        _text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    FolderSearch,
    PromptInput,
    WaveformBpm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeInvalidationScope {
    OverlayStateOnly,
    OverlayMotionOnly,
    ModelAndOverlays,
    StaticAndOverlays,
    LayoutAndAll,
}

/// Cache key for state-overlay rebuild skipping.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StateOverlayCacheFingerprint {
    /// Layout-width bits used for geometry-sensitive invalidation.
    layout_width_bits: u32,
    /// Layout-height bits used for geometry-sensitive invalidation.
    layout_height_bits: u32,
    /// Layout UI-scale bits used for token-sensitive invalidation.
    layout_scale_bits: u32,
    /// Shell interaction-state fingerprint.
    shell: StateOverlayFingerprint,
    /// Compact deterministic signature for state-overlay model inputs.
    model_signature: u64,
}

/// Cache key for waveform-motion overlay rebuild skipping.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WaveformMotionOverlayCacheFingerprint {
    /// Layout-width bits used for geometry-sensitive invalidation.
    layout_width_bits: u32,
    /// Layout-height bits used for geometry-sensitive invalidation.
    layout_height_bits: u32,
    /// Layout UI-scale bits used for token-sensitive invalidation.
    layout_scale_bits: u32,
    /// Shell waveform-motion-state fingerprint.
    shell: WaveformMotionOverlayFingerprint,
    /// Compact deterministic signature for waveform-motion model inputs.
    motion_signature: u64,
}

/// Cache key for chrome-motion overlay rebuild skipping.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ChromeMotionOverlayCacheFingerprint {
    /// Layout-width bits used for geometry-sensitive invalidation.
    layout_width_bits: u32,
    /// Layout-height bits used for geometry-sensitive invalidation.
    layout_height_bits: u32,
    /// Layout UI-scale bits used for token-sensitive invalidation.
    layout_scale_bits: u32,
    /// Shell chrome-motion-state fingerprint.
    shell: ChromeMotionOverlayFingerprint,
    /// Compact deterministic signature for chrome-motion model inputs.
    motion_signature: u64,
}

/// Cache key for one retained static-scene segment.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StaticSegmentCacheFingerprint {
    /// Segment identifier for the cache entry.
    segment: StaticFrameSegment,
    /// Layout-width bits used for geometry-sensitive invalidation.
    layout_width_bits: u32,
    /// Layout-height bits used for geometry-sensitive invalidation.
    layout_height_bits: u32,
    /// Layout UI-scale bits used for token-sensitive invalidation.
    layout_scale_bits: u32,
    /// Compact deterministic signature for style token changes.
    style_signature: u64,
    /// Monotonic bridge-provided revision for this static segment.
    segment_revision: u64,
}

/// Immutable retained state node for one static segment in the view graph.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StaticSegmentStateNode {
    /// Segment represented by this node.
    segment: StaticFrameSegment,
    /// Last projected fingerprint associated with this node.
    fingerprint: StaticSegmentCacheFingerprint,
}

/// Diff plan produced from retained segment-node state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StaticSegmentDiffPlan {
    /// Candidate fingerprints for this frame keyed by segment index.
    fingerprints: [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT],
    /// Bitset for segments that must rebuild this frame.
    rebuild_bits: u8,
}

impl StaticSegmentDiffPlan {
    /// Return whether one segment should rebuild this frame.
    fn should_rebuild(&self, segment: StaticFrameSegment) -> bool {
        (self.rebuild_bits & (1 << segment.index())) != 0
    }

    /// Return the candidate fingerprint for one segment.
    fn fingerprint(&self, segment: StaticFrameSegment) -> &StaticSegmentCacheFingerprint {
        &self.fingerprints[segment.index()]
    }
}

/// Retained graph of immutable static segment state nodes.
struct StaticSegmentStateGraph {
    nodes: [Option<StaticSegmentStateNode>; StaticFrameSegment::COUNT],
}

impl Default for StaticSegmentStateGraph {
    /// Create an empty retained static segment state graph.
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|_| None),
        }
    }
}

impl StaticSegmentStateGraph {
    /// Discard all retained nodes so the next diff pass rebuilds every segment.
    fn clear(&mut self) {
        for node in &mut self.nodes {
            *node = None;
        }
    }

    /// Diff candidate segment fingerprints against retained immutable nodes.
    fn diff(
        &self,
        dirty_segments: DirtySegments,
        force_rebuild: bool,
        fingerprints: [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT],
    ) -> StaticSegmentDiffPlan {
        let mut rebuild_bits = 0u8;
        for segment in StaticFrameSegment::ALL {
            let idx = segment.index();
            let explicit_dirty = (dirty_segments.bits() & segment.dirty_mask()) != 0;
            let fingerprint_changed =
                self.nodes[idx].as_ref().map(|node| &node.fingerprint) != Some(&fingerprints[idx]);
            let needs_rebuild = force_rebuild || explicit_dirty || fingerprint_changed;
            if needs_rebuild {
                rebuild_bits |= 1 << idx;
            }
        }
        StaticSegmentDiffPlan {
            fingerprints,
            rebuild_bits,
        }
    }

    /// Commit one rebuilt segment fingerprint into the retained graph.
    fn commit_segment(
        &mut self,
        segment: StaticFrameSegment,
        fingerprint: &StaticSegmentCacheFingerprint,
    ) {
        self.nodes[segment.index()] = Some(StaticSegmentStateNode {
            segment,
            fingerprint: fingerprint.clone(),
        });
    }
}

/// Retained scene cache entry for one static segment.
struct StaticSegmentSceneCacheEntry {
    /// Encoded scene for this segment.
    scene: Scene,
}

impl Default for StaticSegmentSceneCacheEntry {
    /// Create an empty static segment scene-cache entry.
    fn default() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

/// Retained static segment scenes keyed by deterministic fingerprints.
struct StaticSegmentSceneCache {
    entries: [StaticSegmentSceneCacheEntry; StaticFrameSegment::COUNT],
}

impl Default for StaticSegmentSceneCache {
    /// Create empty scene entries for all static segments.
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| StaticSegmentSceneCacheEntry::default()),
        }
    }
}

impl StaticSegmentSceneCache {
    /// Return an immutable scene for one static segment.
    fn scene(&self, segment: StaticFrameSegment) -> &Scene {
        &self.entries[segment.index()].scene
    }

    /// Return a mutable cache entry for one static segment.
    fn entry_mut(&mut self, segment: StaticFrameSegment) -> &mut StaticSegmentSceneCacheEntry {
        &mut self.entries[segment.index()]
    }
}

const FINGERPRINT_FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FINGERPRINT_FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fingerprint_mix_u8(state: &mut u64, value: u8) {
    *state ^= u64::from(value);
    *state = state.wrapping_mul(FINGERPRINT_FNV_PRIME);
}

fn fingerprint_mix_u16(state: &mut u64, value: u16) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

/// Mix one `u32` into a fingerprint accumulator.
fn fingerprint_mix_u32(state: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

fn fingerprint_mix_u64(state: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        fingerprint_mix_u8(state, byte);
    }
}

fn fingerprint_mix_usize(state: &mut u64, value: usize) {
    fingerprint_mix_u64(state, value as u64);
}

fn fingerprint_mix_bool(state: &mut u64, value: bool) {
    fingerprint_mix_u8(state, u8::from(value));
}

/// Mix one `f32` by its deterministic bit representation.
fn fingerprint_mix_f32(state: &mut u64, value: f32) {
    fingerprint_mix_u32(state, value.to_bits());
}

fn fingerprint_mix_string(state: &mut u64, value: &str) {
    fingerprint_mix_usize(state, value.len());
    for byte in value.as_bytes() {
        fingerprint_mix_u8(state, *byte);
    }
}

fn fingerprint_mix_option_string(state: &mut u64, value: Option<&str>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_string(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_usize(state: &mut u64, value: Option<usize>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_usize(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

fn fingerprint_mix_option_u16(state: &mut u64, value: Option<u16>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_u16(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

/// Mix one optional `u32` into a fingerprint accumulator.
fn fingerprint_mix_option_u32(state: &mut u64, value: Option<u32>) {
    if let Some(value) = value {
        fingerprint_mix_bool(state, true);
        fingerprint_mix_u32(state, value);
        return;
    }
    fingerprint_mix_bool(state, false);
}

/// Build a compact, deterministic signature for state-overlay model inputs.
fn state_overlay_model_signature(model: &AppModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_usize(&mut state, model.selected_column);
    fingerprint_mix_option_usize(&mut state, model.browser.selected_visible_row);
    fingerprint_mix_option_usize(&mut state, model.browser.anchor_visible_row);
    fingerprint_mix_option_usize(&mut state, model.sources.selected_row);
    fingerprint_mix_option_usize(&mut state, model.sources.focused_folder_row);
    fingerprint_mix_bool(&mut state, model.confirm_prompt.visible);
    fingerprint_mix_u8(
        &mut state,
        match model.confirm_prompt.kind {
            None => 0,
            Some(crate::app::ConfirmPromptKind::DestructiveEdit) => 1,
            Some(crate::app::ConfirmPromptKind::BrowserRename) => 2,
            Some(crate::app::ConfirmPromptKind::FolderRename) => 3,
            Some(crate::app::ConfirmPromptKind::FolderCreate) => 4,
        },
    );
    fingerprint_mix_string(&mut state, &model.confirm_prompt.title);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.message);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.confirm_label);
    fingerprint_mix_string(&mut state, &model.confirm_prompt.cancel_label);
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.target_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.input_value.as_deref());
    fingerprint_mix_option_string(
        &mut state,
        model.confirm_prompt.input_placeholder.as_deref(),
    );
    fingerprint_mix_option_string(&mut state, model.confirm_prompt.input_error.as_deref());
    fingerprint_mix_bool(&mut state, model.progress_overlay.visible);
    fingerprint_mix_bool(&mut state, model.progress_overlay.modal);
    fingerprint_mix_string(&mut state, &model.progress_overlay.title);
    fingerprint_mix_option_string(&mut state, model.progress_overlay.detail.as_deref());
    fingerprint_mix_usize(&mut state, model.progress_overlay.completed);
    fingerprint_mix_usize(&mut state, model.progress_overlay.total);
    fingerprint_mix_bool(&mut state, model.progress_overlay.cancelable);
    fingerprint_mix_bool(&mut state, model.progress_overlay.cancel_requested);
    fingerprint_mix_bool(&mut state, model.drag_overlay.active);
    fingerprint_mix_string(&mut state, &model.drag_overlay.label);
    fingerprint_mix_string(&mut state, &model.drag_overlay.target_label);
    fingerprint_mix_bool(&mut state, model.drag_overlay.valid_target);
    fingerprint_mix_u8(
        &mut state,
        match model.update.status {
            crate::app::UpdateStatusModel::Idle => 0,
            crate::app::UpdateStatusModel::Checking => 1,
            crate::app::UpdateStatusModel::Available => 2,
            crate::app::UpdateStatusModel::Error => 3,
        },
    );
    fingerprint_mix_bool(&mut state, model.map.active);
    state
}

/// Build a compact, deterministic signature for waveform-motion overlay model inputs.
fn waveform_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.transport_running);
    if let Some(selection) = model.waveform_selection_milli {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u16(&mut state, selection.start_milli);
        fingerprint_mix_u16(&mut state, selection.end_milli);
        fingerprint_mix_u32(&mut state, selection.start_micros);
        fingerprint_mix_u32(&mut state, selection.end_micros);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    if let Some(edit_selection) = model.waveform_edit_selection_milli {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u16(&mut state, edit_selection.start_milli);
        fingerprint_mix_u16(&mut state, edit_selection.end_milli);
        fingerprint_mix_u32(&mut state, edit_selection.start_micros);
        fingerprint_mix_u32(&mut state, edit_selection.end_micros);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_end_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_in_end_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_mute_start_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_in_mute_start_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_in_curve_milli);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_start_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_out_start_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_mute_end_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_edit_fade_out_mute_end_micros);
    fingerprint_mix_option_u16(&mut state, model.waveform_edit_fade_out_curve_milli);
    fingerprint_mix_bool(&mut state, model.waveform_loop_enabled);
    fingerprint_mix_option_u16(&mut state, model.waveform_cursor_milli);
    fingerprint_mix_option_u16(&mut state, model.waveform_playhead_milli);
    fingerprint_mix_option_u32(&mut state, model.waveform_playhead_micros);
    fingerprint_mix_u16(&mut state, model.waveform_view_start_milli);
    fingerprint_mix_u16(&mut state, model.waveform_view_end_milli);
    fingerprint_mix_u32(&mut state, model.waveform_view_start_micros);
    fingerprint_mix_u32(&mut state, model.waveform_view_end_micros);
    if let Some(signature) = model.waveform_image_signature {
        fingerprint_mix_bool(&mut state, true);
        fingerprint_mix_u64(&mut state, signature);
    } else {
        fingerprint_mix_bool(&mut state, false);
    }
    state
}

/// Build a compact, deterministic signature for chrome-motion overlay model inputs.
fn chrome_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_bool(&mut state, model.transport_running);
    fingerprint_mix_bool(&mut state, model.map_active);
    fingerprint_mix_option_string(&mut state, model.waveform_tempo_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.waveform_zoom_label.as_deref());
    fingerprint_mix_option_string(&mut state, model.waveform_loaded_label.as_deref());
    fingerprint_mix_u8(
        &mut state,
        match model.waveform_channel_view {
            crate::app::WaveformChannelViewModel::Mono => 0,
            crate::app::WaveformChannelViewModel::Stereo => 1,
        },
    );
    fingerprint_mix_bool(&mut state, model.waveform_normalized_audition_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_bpm_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_snap_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_transient_markers_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_slice_mode_enabled);
    fingerprint_mix_bool(&mut state, model.waveform_loop_enabled);
    fingerprint_mix_string(&mut state, &model.waveform_transport_hint);
    fingerprint_mix_string(&mut state, &model.status_right);
    state
}

/// Mix a compact RGBA8 color into a fingerprint accumulator.
fn fingerprint_mix_rgba8(state: &mut u64, color: Rgba8) {
    fingerprint_mix_u8(state, color.r);
    fingerprint_mix_u8(state, color.g);
    fingerprint_mix_u8(state, color.b);
    fingerprint_mix_u8(state, color.a);
}

/// Build a deterministic signature for style values that affect static segments.
fn static_segment_style_signature(style: &StyleTokens) -> u64 {
    let mut state = FINGERPRINT_FNV_OFFSET_BASIS;
    fingerprint_mix_rgba8(&mut state, style.clear_color);
    fingerprint_mix_rgba8(&mut state, style.surface_base);
    fingerprint_mix_rgba8(&mut state, style.surface_raised);
    fingerprint_mix_rgba8(&mut state, style.surface_overlay);
    fingerprint_mix_rgba8(&mut state, style.border);
    fingerprint_mix_rgba8(&mut state, style.border_emphasis);
    fingerprint_mix_f32(&mut state, style.sizing.border_width);
    fingerprint_mix_f32(&mut state, style.sizing.focus_stroke_width);
    fingerprint_mix_f32(&mut state, style.sizing.font_header);
    fingerprint_mix_f32(&mut state, style.sizing.font_body);
    fingerprint_mix_f32(&mut state, style.sizing.font_meta);
    fingerprint_mix_f32(&mut state, style.sizing.font_status);
    state
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeVelloFrameState {
    /// Layout-only changes invalidate both static frame and cached overlays.
    layout_dirty: bool,
    /// Full static scene cache needs a rebuild.
    scene_dirty: bool,
    /// State-driven overlay cache needs a rebuild.
    state_overlay_dirty: bool,
    /// Motion/timer-driven overlay cache needs a rebuild.
    motion_overlay_dirty: bool,
    /// Input/model changes require a model pull before the next redraw.
    model_dirty: bool,
}

impl NativeVelloFrameState {
    /// Mark layout as stale, requiring full scene and overlay refresh.
    fn mark_layout_dirty(&mut self) {
        self.layout_dirty = true;
        self.scene_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    /// Mark only the state overlay cache dirty.
    fn mark_state_overlay_dirty(&mut self) {
        self.state_overlay_dirty = true;
    }

    /// Mark only the motion overlay cache dirty.
    fn mark_motion_overlay_dirty(&mut self) {
        self.motion_overlay_dirty = true;
    }

    /// Clear one-off layout flags after layout rebuild.
    fn clear_layout_dirty(&mut self) {
        self.layout_dirty = false;
    }

    /// Mark model-backed state as dirty.
    fn mark_model_dirty(&mut self) {
        self.model_dirty = true;
        self.scene_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    /// Mark model-backed state dirty without forcing static-scene invalidation.
    ///
    /// Static segment rebuilds are decided later from bridge dirty-segment masks.
    fn mark_model_overlay_dirty(&mut self) {
        self.model_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    /// Take and clear the static scene dirty bit.
    fn take_scene(&mut self) -> bool {
        let dirty = self.scene_dirty;
        self.scene_dirty = false;
        dirty
    }

    /// Take and clear the state overlay dirty bit.
    fn take_state_overlay(&mut self) -> bool {
        let dirty = self.state_overlay_dirty;
        self.state_overlay_dirty = false;
        dirty
    }

    /// Take and clear the motion overlay dirty bit.
    fn take_motion_overlay(&mut self) -> bool {
        let dirty = self.motion_overlay_dirty;
        self.motion_overlay_dirty = false;
        dirty
    }

    /// Take and clear model pull flag.
    fn take_model(&mut self) -> bool {
        let dirty = self.model_dirty;
        self.model_dirty = false;
        dirty
    }

    fn has_pending_rebuild(&self) -> bool {
        self.layout_dirty
            || self.scene_dirty
            || self.state_overlay_dirty
            || self.motion_overlay_dirty
            || self.model_dirty
    }
}

/// Startup lifecycle timing breakdown for first paint and deferred refresh.
#[derive(Debug, Default)]
struct StartupTimingProfile {
    /// Whether stderr startup summary output is enabled.
    enabled: bool,
    /// Runtime initialization start instant.
    init_started_at: Option<Instant>,
    /// Native window creation complete instant.
    window_created_at: Option<Instant>,
    /// Render surface creation complete instant.
    surface_ready_at: Option<Instant>,
    /// Renderer creation complete instant.
    renderer_ready_at: Option<Instant>,
    /// First scene build/encode completion instant.
    first_scene_ready_at: Option<Instant>,
    /// First successful present completion instant.
    first_presented_at: Option<Instant>,
    /// Deferred full-model refresh completion instant.
    deferred_model_refresh_done_at: Option<Instant>,
    /// Whether summary output already emitted.
    summary_emitted: bool,
}

impl StartupTimingProfile {
    /// Create startup timing profile configuration from environment.
    fn new() -> Self {
        let enabled = crate::env_flags::env_var_truthy(STARTUP_PROFILE_ENV);
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Record startup initialization begin timestamp.
    fn mark_init_started(&mut self) {
        self.init_started_at = Some(Instant::now());
    }

    /// Record native window creation completion timestamp.
    fn mark_window_created(&mut self) {
        self.window_created_at = Some(Instant::now());
    }

    /// Record render-surface creation completion timestamp.
    fn mark_surface_ready(&mut self) {
        self.surface_ready_at = Some(Instant::now());
    }

    /// Record renderer creation completion timestamp.
    fn mark_renderer_ready(&mut self) {
        self.renderer_ready_at = Some(Instant::now());
    }

    /// Record first scene build/encode completion timestamp.
    fn mark_first_scene_ready(&mut self) {
        self.first_scene_ready_at = Some(Instant::now());
    }

    /// Record first successful present completion timestamp.
    fn mark_first_presented(&mut self) {
        self.first_presented_at = Some(Instant::now());
    }

    /// Record deferred full-model refresh completion timestamp.
    fn mark_deferred_model_refresh_done(&mut self) {
        self.deferred_model_refresh_done_at = Some(Instant::now());
    }

    /// Emit one startup timing summary once all required milestones are available.
    fn maybe_emit_summary(&mut self) {
        if self.summary_emitted {
            return;
        }
        let (
            Some(init_started_at),
            Some(window_created_at),
            Some(surface_ready_at),
            Some(renderer_ready_at),
            Some(first_scene_ready_at),
            Some(first_presented_at),
            Some(deferred_model_refresh_done_at),
        ) = (
            self.init_started_at,
            self.window_created_at,
            self.surface_ready_at,
            self.renderer_ready_at,
            self.first_scene_ready_at,
            self.first_presented_at,
            self.deferred_model_refresh_done_at,
        )
        else {
            return;
        };
        let ms = |start: Instant, end: Instant| (end - start).as_secs_f64() * 1000.0;
        let window_create_ms = ms(init_started_at, window_created_at);
        let surface_ready_ms = ms(init_started_at, surface_ready_at);
        let renderer_ready_ms = ms(init_started_at, renderer_ready_at);
        let first_scene_ready_ms = ms(init_started_at, first_scene_ready_at);
        let first_present_ms = ms(init_started_at, first_presented_at);
        let deferred_model_refresh_ms = ms(first_presented_at, deferred_model_refresh_done_at);
        let deferred_model_refresh_total_ms = ms(init_started_at, deferred_model_refresh_done_at);
        info!(
            window_create_ms,
            surface_ready_ms,
            renderer_ready_ms,
            first_scene_ready_ms,
            first_present_ms,
            deferred_model_refresh_ms,
            deferred_model_refresh_total_ms,
            "native vello startup timing summary"
        );
        if self.enabled {
            eprintln!(
                "[native-vello-startup] window_create_ms={window_create_ms:.3} \
surface_ready_ms={surface_ready_ms:.3} renderer_ready_ms={renderer_ready_ms:.3} \
first_scene_ready_ms={first_scene_ready_ms:.3} first_present_ms={first_present_ms:.3} \
deferred_model_refresh_ms={deferred_model_refresh_ms:.3} \
deferred_model_refresh_total_ms={deferred_model_refresh_total_ms:.3}"
            );
        }
        self.summary_emitted = true;
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

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Keep the native window hidden until startup sequencing decides reveal timing.
    fn startup_should_launch_hidden() -> bool {
        true
    }

    /// Use eager full-scene startup by default and reserve deferred placeholder
    /// startup for explicit fallback paths only.
    fn startup_should_defer_first_model_pull() -> bool {
        false
    }

    /// Resolve a deterministic startup clear color used before style/layout are ready.
    fn startup_placeholder_clear_color() -> Rgba8 {
        StyleTokens::for_viewport_width(1280.0).clear_color
    }

    fn new(options: NativeRunOptions, bridge: B) -> Self {
        let target_fps = options.target_fps.max(1);
        let frame_interval_ns = (1_000_000_000u64 / target_fps as u64).max(1);
        let target_frame_interval = Duration::from_nanos(frame_interval_ns);
        let focus_animation_interval =
            Duration::from_nanos((1_000_000_000u64 / FOCUS_PULSE_HZ).max(1));
        let idle_status_refresh_interval =
            Duration::from_nanos(1_000_000_000u64 / IDLE_STATUS_REFRESH_HZ.max(1));
        let cursor_activity_redraw_interval =
            Duration::from_nanos(1_000_000_000u64 / CURSOR_ACTIVITY_REDRAW_HZ.max(1));
        let startup_clear_color = Self::startup_placeholder_clear_color();
        let incremental_frame_pipeline =
            crate::env_flags::env_var_truthy(INCREMENTAL_FRAME_PIPELINE_ENV);
        info!(
            "radiant native vello runner created: title={} target_fps={} maximized={} has_icon={}",
            options.title,
            options.target_fps,
            options.maximized,
            options.icon.is_some()
        );
        Self {
            options,
            bridge,
            repaint_event_pending: Arc::new(AtomicBool::new(false)),
            incremental_frame_pipeline,
            model: Arc::new(AppModel::default()),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            redraw_requested: false,
            frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            static_segment_frame_cache: StaticFrameSegments::default(),
            static_segment_graph: StaticSegmentStateGraph::default(),
            static_segment_scene_cache: StaticSegmentSceneCache::default(),
            state_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            waveform_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            chrome_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            scene: Scene::new(),
            static_scene: Scene::new(),
            state_overlay_scene: Scene::new(),
            waveform_motion_overlay_scene: Scene::new(),
            chrome_motion_overlay_scene: Scene::new(),
            image_upload_blob_cache: HashMap::new(),
            image_upload_blob_cache_order: VecDeque::new(),
            state_overlay_fingerprint: None,
            waveform_motion_overlay_fingerprint: None,
            chrome_motion_overlay_fingerprint: None,
            motion_model: None,
            motion_model_supported: true,
            segment_revisions: SegmentRevisions::default(),
            segment_revisions_supported: false,
            missing_segment_revision_fallback_applied: false,
            text_renderer: NativeTextRenderer::new(),
            style_cache: None,
            frame_state: NativeVelloFrameState {
                model_dirty: true,
                ..NativeVelloFrameState::default()
            },
            layout_runtime: ShellLayoutRuntime::default(),
            shell_layout: None,
            shell_state: NativeShellState::new(),
            clear_color: startup_clear_color,
            cursor_icon: CursorIcon::Default,
            last_cursor: None,
            pending_cursor: None,
            pending_volume_milli: None,
            waveform_drag_mode: None,
            selection_drag_active: false,
            last_emitted_waveform_drag_action: None,
            map_focus_drag_active: false,
            last_emitted_map_drag_sample_id: None,
            browser_scrollbar_drag: None,
            last_emitted_browser_view_start: None,
            volume_drag_active: false,
            last_emitted_volume_milli: None,
            modifiers: ModifiersState::default(),
            text_input_target: TextInputTarget::None,
            text_input_buffer: None,
            text_editor_state: None,
            text_input_drag_active: false,
            waveform_bpm_input_buffer: None,
            clipboard: None,
            clipboard_fallback_text: String::new(),
            last_redraw: Instant::now(),
            resumed_count: 0,
            window_event_count: 0,
            redraw_count: 0,
            first_frame_presented: false,
            startup_window_visible: false,
            startup_model_pull_pending: Self::startup_should_defer_first_model_pull(),
            startup_deferred_model_refresh_pending: false,
            startup_reveal_deadline: None,
            startup_timing: StartupTimingProfile::new(),
            target_frame_interval,
            focus_animation_interval,
            idle_status_refresh_interval,
            next_idle_status_refresh: Instant::now() + idle_status_refresh_interval,
            cursor_activity_redraw_interval,
            cursor_activity_redraw_until: None,
            model_refresh_count: 0,
            profiler: NativeVelloProfiler::new(),
        }
    }

    fn ui_scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| {
                let scale = window.scale_factor() as f32;
                scale.clamp(1.0, 3.0)
            })
            .unwrap_or(1.0)
    }

    fn build_window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_maximized(self.options.maximized)
            .with_visible(!Self::startup_should_launch_hidden());
        if let Some([w, h]) = self.options.inner_size {
            attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some([w, h]) = self.options.min_inner_size {
            attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some(icon) = self.options.icon.as_ref().and_then(icon_from_rgba) {
            attrs = attrs.with_window_icon(Some(icon));
        }
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_drag_and_drop(true);
        }
        attrs
    }

    fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        info!("radiant native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!("radiant native vello: failed to create window: {:?}", err);
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_window_created();
        info!("radiant native vello: window created");
        self.arm_startup_reveal_deadline(Instant::now());
        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        info!(
            "radiant native vello: creating render surface with {}x{}",
            width, height
        );
        let present_mode_candidates = present_mode_candidates(self.options.target_fps);
        let mut create_surface_with_mode = |present_mode| {
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                pollster::block_on(render_ctx.create_surface(
                    window.clone(),
                    width,
                    height,
                    present_mode,
                ))
            }))
        };
        let mut render_surface = None;
        for (index, present_mode) in present_mode_candidates.iter().copied().enumerate() {
            let last_attempt = index + 1 == present_mode_candidates.len();
            match create_surface_with_mode(present_mode) {
                Ok(Ok(surface)) => {
                    if index == 0 {
                        info!(
                            "radiant native vello: render surface created using {:?}",
                            present_mode
                        );
                    } else {
                        info!(
                            "radiant native vello: render surface created using {:?} fallback",
                            present_mode
                        );
                    }
                    render_surface = Some(surface);
                    break;
                }
                Ok(Err(err)) => {
                    if last_attempt {
                        error!(
                            "radiant native vello: failed to create {:?} surface: {:?}",
                            present_mode, err
                        );
                        event_loop.exit();
                        return;
                    }
                    let next_mode = present_mode_candidates[index + 1];
                    warn!(
                        "radiant native vello: {:?} surface creation failed (error): {:?}; retrying {:?}",
                        present_mode, err, next_mode
                    );
                }
                Err(_) => {
                    if last_attempt {
                        error!(
                            "radiant native vello: {:?} surface creation panicked during startup",
                            present_mode
                        );
                        event_loop.exit();
                        return;
                    }
                    let next_mode = present_mode_candidates[index + 1];
                    warn!(
                        "radiant native vello: {:?} surface creation panicked; retrying {:?}",
                        present_mode, next_mode
                    );
                }
            }
        }
        let Some(render_surface) = render_surface else {
            event_loop.exit();
            return;
        };
        self.startup_timing.mark_surface_ready();
        info!("radiant native vello: render surface created");
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        info!("radiant native vello: creating renderer");
        let renderer = match Renderer::new(&dev_handle.device, RendererOptions::default()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!("radiant native vello: failed to create renderer: {:?}", err);
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_renderer_ready();
        info!("radiant native vello: renderer created");

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.frame_state.mark_layout_dirty();
        if self.startup_model_pull_pending {
            self.prepare_startup_first_frame_scene();
        } else {
            self.frame_state.mark_model_dirty();
        }
        self.rebuild_scene_if_needed();
        self.startup_timing.mark_first_scene_ready();
        self.maybe_reveal_startup_window_after_first_scene_ready();
        self.last_redraw = Instant::now();
    }

    /// Keep startup first-frame work minimal when the deferred fallback path is armed.
    ///
    /// This preserves static scene rebuild work (for deterministic first paint)
    /// while skipping model and overlay pulls until first present completes.
    fn prepare_startup_first_frame_scene(&mut self) {
        let _ = self.frame_state.take_model();
        let _ = self.frame_state.take_state_overlay();
        let _ = self.frame_state.take_motion_overlay();
    }

    fn rebuild_layout(&mut self) {
        let Some(surface) = self.render_surface.as_ref() else {
            return;
        };

        let viewport = Vector2::new(surface.config.width as f32, surface.config.height as f32);
        let style = StyleTokens::for_viewport_with_scale(viewport.x, self.ui_scale_factor());
        self.style_cache = Some(style);
        self.shell_layout = Some(Arc::new(ShellLayout::build_with_style_and_runtime(
            viewport,
            &style,
            &mut self.layout_runtime,
        )));
        self.static_segment_graph.clear();
        self.frame_state.clear_layout_dirty();
        if let Some(point) = self.pending_cursor.take() {
            let _ = self.process_cursor_move_immediately(point);
        }
    }

    /// Borrow the retained shell layout while mutating runtime state without
    /// cloning the full layout payload.
    fn with_shell_layout<T>(
        &mut self,
        work: impl FnOnce(&mut Self, &ShellLayout) -> T,
    ) -> Option<T> {
        let layout = self.shell_layout.take()?;
        let result = work(self, layout.as_ref());
        self.shell_layout = Some(layout);
        Some(result)
    }

    fn request_redraw_if_needed(&mut self) {
        if self.redraw_requested {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.redraw_requested = true;
        }
    }

    fn build_style_for_layout(layout: &ShellLayout) -> StyleTokens {
        StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
    }

    fn cached_style_for_layout(&self, layout: &ShellLayout) -> StyleTokens {
        self.style_cache
            .unwrap_or_else(|| Self::build_style_for_layout(layout))
    }

    /// Arm the hidden-startup reveal timeout so redraw stalls cannot deadlock launch.
    fn arm_startup_reveal_deadline(&mut self, now: Instant) {
        if Self::startup_should_launch_hidden() && !self.startup_window_visible {
            self.startup_reveal_deadline = Some(now + STARTUP_REVEAL_STALL_TIMEOUT);
        }
    }

    /// Build one minimal branded startup scene for deferred-startup fallback.
    fn build_startup_placeholder_scene(&mut self, layout: &ShellLayout, style: &StyleTokens) {
        let root = layout.root.rect;
        let panel_width = (root.width() * 0.36).clamp(220.0, 420.0);
        let panel_height = (style.sizing.font_header * 2.8).clamp(58.0, 86.0);
        let panel_min = Point::new(
            root.min.x + (root.width() - panel_width) * 0.5,
            root.min.y + (root.height() - panel_height) * 0.5,
        );
        let panel = UiRect::from_min_size(panel_min, Vector2::new(panel_width, panel_height));
        let accent_height = (panel_height * 0.08).clamp(3.0, 6.0);
        let accent = UiRect::from_min_max(
            panel.min,
            Point::new(panel.max.x, panel.min.y + accent_height),
        );
        let title = TextRun {
            text: String::from("Sempal"),
            position: Point::new(panel.min.x + 12.0, panel.min.y + 10.0),
            font_size: style.sizing.font_header.max(12.0),
            color: style.text_primary,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };
        let subtitle = TextRun {
            text: String::from("Starting audio engine..."),
            position: Point::new(panel.min.x + 12.0, panel.min.y + panel_height * 0.48),
            font_size: style.sizing.font_meta.max(10.0),
            color: style.text_muted,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };

        self.frame_cache.clear_color = style.clear_color;
        self.frame_cache.primitives.clear();
        self.frame_cache.text_runs.clear();
        self.frame_cache.text_runs.push(title.clone());
        self.frame_cache.text_runs.push(subtitle.clone());
        self.state_overlay_frame_cache.clear_color = style.clear_color;
        self.state_overlay_frame_cache.primitives.clear();
        self.state_overlay_frame_cache.text_runs.clear();
        self.waveform_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.waveform_motion_overlay_frame_cache.primitives.clear();
        self.waveform_motion_overlay_frame_cache.text_runs.clear();
        self.chrome_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.chrome_motion_overlay_frame_cache.primitives.clear();
        self.chrome_motion_overlay_frame_cache.text_runs.clear();
        self.clear_color = style.clear_color;

        self.static_scene.reset();
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_base),
            None,
            &to_kurbo_rect(root),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_raised),
            None,
            &to_kurbo_rect(panel),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.accent_mint),
            None,
            &to_kurbo_rect(accent),
        );
        self.text_renderer
            .draw_text_runs(&mut self.static_scene, &[title, subtitle]);
        self.state_overlay_scene.reset();
        self.waveform_motion_overlay_scene.reset();
        self.chrome_motion_overlay_scene.reset();
        self.scene.reset();
        self.scene.append(&self.static_scene, None);
    }

    fn rebuild_scene_if_needed(&mut self) {
        if self.shell_layout.is_none() || self.frame_state.layout_dirty {
            self.rebuild_layout();
        }
        let model_refresh_requested = self.frame_state.take_model();
        let static_rebuild_requested = self.frame_state.take_scene();
        let state_overlay_requested = self.frame_state.take_state_overlay();
        let motion_overlay_requested = self.frame_state.take_motion_overlay();
        if self.startup_model_pull_pending
            && !self.first_frame_presented
            && !model_refresh_requested
            && static_rebuild_requested
        {
            let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
                return;
            };
            let style = self.cached_style_for_layout(layout.as_ref());
            self.build_startup_placeholder_scene(layout.as_ref(), &style);
            return;
        }
        if static_rebuild_requested {
            self.profiler.add_explicit_static_rebuild();
        }
        let rebuild_static = static_rebuild_requested || model_refresh_requested;
        let rebuild_state_overlay = state_overlay_requested || rebuild_static;
        let rebuild_motion_overlay = motion_overlay_requested || rebuild_static;
        if !rebuild_static && !rebuild_state_overlay && !rebuild_motion_overlay {
            return;
        }
        self.rebuild_scene(
            model_refresh_requested,
            static_rebuild_requested,
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
        );
    }

    fn apply_invalidation_scope(&mut self, scope: RuntimeInvalidationScope) {
        match scope {
            RuntimeInvalidationScope::OverlayStateOnly => {
                self.frame_state.mark_state_overlay_dirty();
            }
            RuntimeInvalidationScope::OverlayMotionOnly => {
                self.frame_state.mark_motion_overlay_dirty();
            }
            RuntimeInvalidationScope::ModelAndOverlays => {
                self.frame_state.mark_model_overlay_dirty();
            }
            RuntimeInvalidationScope::StaticAndOverlays => {
                self.frame_state.mark_model_dirty();
            }
            RuntimeInvalidationScope::LayoutAndAll => {
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.layout_runtime.reset();
                self.layout_runtime
                    .mark_all_dirty(ShellLayoutDirtyKind::Measure);
            }
        }
        self.request_redraw_if_needed();
    }

    fn rebuild_overlay_and_request_redraw(&mut self) {
        self.frame_state.mark_state_overlay_dirty();
        self.request_redraw_if_needed();
    }

    fn rebuild_scene_for_tick(&mut self) {
        self.frame_state.mark_motion_overlay_dirty();
        self.rebuild_scene_if_needed();
    }

    fn rebuild_scene_for_redraw(
        &mut self,
        needs_animation: bool,
        delta_seconds: f32,
    ) -> (bool, FrameBuildResult) {
        if !needs_animation {
            if self.frame_state.has_pending_rebuild() {
                self.rebuild_scene_if_needed();
                return (true, self.frame_result_base());
            }
            return (false, self.frame_result_base());
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, self.frame_result_base());
        };
        let tick_start = self.profiler.now_if_enabled();
        let style = self.cached_style_for_layout(layout);
        self.shell_state.tick_with_style(delta_seconds, &style);
        self.rebuild_scene_for_tick();
        let tick_duration = tick_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_tick(tick_duration);
        (true, self.frame_result_base())
    }

    fn maybe_record_redraw_profile(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
    ) {
        let text_profile = if self.profiler.is_enabled() {
            self.text_renderer.take_layout_profile_counters()
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        self.profiler
            .record_redraw(rebuild, acquire, render, blit, present, total, text_profile);
    }

    /// Build per-frame renderer counts shared with bridge-side telemetry.
    fn frame_result_base(&self) -> FrameBuildResult {
        FrameBuildResult {
            primitive_count: self
                .frame_cache
                .primitives
                .len()
                .saturating_add(self.state_overlay_frame_cache.primitives.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.primitives.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.primitives.len()),
            text_run_count: self
                .frame_cache
                .text_runs
                .len()
                .saturating_add(self.state_overlay_frame_cache.text_runs.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.text_runs.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.text_runs.len()),
            needs_animation: self.shell_state.needs_animation(),
            ..FrameBuildResult::default()
        }
    }

    /// Convert one duration to microseconds while saturating at `u32::MAX`.
    fn duration_us_u32(duration: Duration) -> u32 {
        duration.as_micros().min(u128::from(u32::MAX)) as u32
    }

    /// Return the configured redraw frame budget in microseconds.
    fn frame_budget_us(&self) -> u32 {
        Self::duration_us_u32(self.target_frame_interval)
    }

    /// Finalize and emit one frame result payload to the host bridge.
    fn emit_frame_result(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_total: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        let frame_budget_us = self.frame_budget_us();
        let frame_total_us = Self::duration_us_u32(frame_total);
        frame_result.frame_total_us = frame_total_us;
        frame_result.present_us = Self::duration_us_u32(present);
        frame_result.frame_budget_us = frame_budget_us;
        frame_result.presented = presented;
        frame_result.missed_present = present_expected && !presented;
        frame_result.jank = presented && frame_total_us > frame_budget_us;
        self.bridge.observe_frame_result(*frame_result);
    }

    /// Record profiler data (if enabled) and emit one finalized frame result.
    fn finish_redraw_attempt(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_started_at: Instant,
        frame_profile_start: Option<Instant>,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        if let Some(start) = frame_profile_start {
            self.maybe_record_redraw_profile(
                rebuild,
                acquire,
                render,
                blit,
                present,
                start.elapsed(),
            );
        }
        self.emit_frame_result(
            frame_result,
            frame_started_at.elapsed(),
            present,
            presented,
            present_expected,
        );
    }

    /// Resolve a retained image-upload blob for one RGBA payload.
    fn cached_image_upload_blob(
        cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
        pixels: &Arc<[u8]>,
        width: u32,
        height: u32,
    ) -> Blob<u8> {
        let key = ImageUploadBlobCacheKey {
            pixels_ptr: pixels.as_ptr() as usize,
            width,
            height,
        };
        if let Some(blob) = cache.get(&key) {
            touch_image_upload_blob_cache_key(cache_order, key);
            return blob.clone();
        }
        while cache.len() >= IMAGE_UPLOAD_BLOB_CACHE_LIMIT {
            let Some(stale_key) = cache_order.pop_front() else {
                cache.clear();
                break;
            };
            cache.remove(&stale_key);
        }
        let blob = Blob::new(Arc::new(SharedPixelBytes(Arc::clone(pixels))));
        cache.insert(key, blob.clone());
        cache_order.push_back(key);
        blob
    }

    fn encode_frame_to_scene(
        frame: &NativeViewFrame,
        scene: &mut Scene,
        text_renderer: &mut NativeTextRenderer,
        image_upload_blob_cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        image_upload_blob_cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
    ) {
        scene.reset();
        for primitive in frame.primitives.iter() {
            match primitive {
                Primitive::Rect(fill) => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &to_kurbo_rect(fill.rect),
                    );
                }
                Primitive::Circle(fill) => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &Circle::new(
                            (fill.center.x as f64, fill.center.y as f64),
                            fill.radius as f64,
                        ),
                    );
                }
                Primitive::Image(draw) => {
                    let (Ok(width), Ok(height)) = (
                        u32::try_from(draw.image.width),
                        u32::try_from(draw.image.height),
                    ) else {
                        continue;
                    };
                    if width == 0
                        || height == 0
                        || draw.rect.width() <= 0.0
                        || draw.rect.height() <= 0.0
                    {
                        continue;
                    }
                    let blob = Self::cached_image_upload_blob(
                        image_upload_blob_cache,
                        image_upload_blob_cache_order,
                        &draw.image.pixels,
                        width,
                        height,
                    );
                    let image_data = ImageData {
                        data: blob,
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width,
                        height,
                    };
                    let transform =
                        Affine::translate((draw.rect.min.x as f64, draw.rect.min.y as f64))
                            * Affine::scale_non_uniform(
                                draw.rect.width() as f64 / f64::from(width),
                                draw.rect.height() as f64 / f64::from(height),
                            );
                    scene.draw_image(&image_data, transform);
                }
            }
        }
        text_renderer.draw_text_runs(scene, &frame.text_runs);
    }

    fn queue_cursor(&mut self, point: Point) {
        self.pending_cursor = Some(point);
    }

    /// Update the native cursor icon only when it changed.
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        if self.cursor_icon == icon {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(icon);
        }
        self.cursor_icon = icon;
    }

    /// Resolve waveform-resize hover cursor state for the current pointer.
    fn update_waveform_resize_cursor(&mut self, point: Point) {
        let icon = if let Some(layout) = self.shell_layout.as_deref() {
            if waveform_selection_drag_handle_hovered(&layout, &self.model, point) {
                CursorIcon::Grab
            } else if waveform_resize_handle_hovered(&layout, &self.model, point) {
                CursorIcon::EwResize
            } else if self
                .shell_state
                .prompt_input_at_point(&layout, &self.model, point)
                || self
                    .shell_state
                    .waveform_bpm_input_at_point(&layout, &self.model, point)
            {
                CursorIcon::Text
            } else {
                CursorIcon::Default
            }
        } else {
            CursorIcon::Default
        };
        self.set_cursor_icon(icon);
    }

    /// Record recent pointer activity for short-lived high-frequency redraw pacing.
    fn note_cursor_activity(&mut self, now: Instant) {
        self.cursor_activity_redraw_until = Some(now + CURSOR_ACTIVITY_REDRAW_WINDOW);
    }

    /// Return the next redraw deadline while recent cursor activity is active.
    fn next_cursor_activity_redraw_deadline(&mut self, now: Instant) -> Option<Instant> {
        let until = self.cursor_activity_redraw_until?;
        if now >= until {
            self.cursor_activity_redraw_until = None;
            return None;
        }
        let mut next_redraw_at = self.last_redraw + self.cursor_activity_redraw_interval;
        if next_redraw_at < now {
            next_redraw_at = now;
        }
        if next_redraw_at > until {
            next_redraw_at = until;
        }
        Some(next_redraw_at)
    }

    /// Process one cursor move immediately when layout state is available.
    ///
    /// Returns `(processed, handled)` where:
    /// - `processed` indicates whether layout/model state was available now.
    /// - `handled` indicates whether hover state changed and triggered redraw.
    fn process_cursor_move_immediately(&mut self, point: Point) -> (bool, bool) {
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, false);
        };
        let profile_start = self.profiler.now_if_enabled();
        let effect = self
            .shell_state
            .handle_cursor_move_effect(layout, &self.model, point);
        let handled = effect != CursorMoveEffect::None;
        if handled {
            if let Some(start) = profile_start {
                let kind = if self.model.map.active {
                    InteractionProfileKind::MapPanProxy
                } else {
                    InteractionProfileKind::Hover
                };
                self.profiler.add_interaction_latency(kind, start.elapsed());
            }
            match effect {
                CursorMoveEffect::WaveformHoverOnly => {
                    self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayMotionOnly);
                }
                CursorMoveEffect::GeneralOverlay => self.rebuild_overlay_and_request_redraw(),
                CursorMoveEffect::None => {}
            }
        }
        (true, handled)
    }

    /// Emit one wheel-derived browser viewport-scroll action immediately.
    ///
    /// Returns whether an action was emitted.
    fn process_wheel_rows_immediately(&mut self, visible_row: usize) -> bool {
        self.emit_model_action_with_profile(
            UiAction::SetBrowserViewStart { visible_row },
            Some(InteractionProfileKind::Wheel),
        );
        true
    }

    /// Emit one browser-scrollbar drag viewport update immediately.
    fn process_browser_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.browser_scrollbar_drag else {
            return false;
        };
        let Some(visible_row) = self.shell_state.browser_scrollbar_view_start_for_drag(
            layout,
            &self.model,
            point.y,
            drag.thumb_pointer_offset_y,
        ) else {
            return false;
        };
        if self.last_emitted_browser_view_start == Some(visible_row) {
            return true;
        }
        self.last_emitted_browser_view_start = Some(visible_row);
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    /// Emit one browser-scrollbar track-click viewport update immediately.
    fn process_browser_scrollbar_track_click_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(visible_row) =
            self.shell_state
                .browser_scrollbar_view_start_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    /// Return whether one held-key repeat should be processed for navigation.
    ///
    /// Repeats stay intentionally narrow to preserve existing single-fire behavior
    /// for actions like toggles/prompts while allowing rapid sample stepping.
    fn allows_key_repeat(&self, key: KeyCode) -> bool {
        if self.text_input_target == TextInputTarget::WaveformBpm {
            if self.modifiers.control_key()
                || self.modifiers.super_key()
                || self.modifiers.alt_key()
            {
                return false;
            }
            return matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown);
        }
        if self.modifiers.shift_key()
            || self.modifiers.control_key()
            || self.modifiers.super_key()
            || self.modifiers.alt_key()
        {
            return false;
        }
        if self.text_input_target != TextInputTarget::None {
            return false;
        }
        matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown)
    }

    fn queue_volume_milli(&mut self, value_milli: u16) {
        self.pending_volume_milli = Some(value_milli.min(1000));
    }

    /// Emit one waveform action immediately during active pointer drag.
    fn emit_waveform_drag_action_immediately(&mut self, action: UiAction) {
        if self.last_emitted_waveform_drag_action.as_ref() == Some(&action) {
            return;
        }
        self.last_emitted_waveform_drag_action = Some(action.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::Waveform));
    }

    /// Process one waveform drag cursor update when waveform drag mode is active.
    fn process_waveform_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(mode) = self.waveform_drag_mode else {
            return false;
        };
        if self.last_emitted_waveform_drag_action.is_none()
            && !waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
        {
            return false;
        }
        let action = waveform_drag_action_for_mode(layout, &self.model, point, mode);
        self.emit_waveform_drag_action_immediately(action);
        true
    }

    /// Process one waveform-selection export drag cursor update.
    fn process_selection_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let (pointer_x, pointer_y) = ui_action_pointer_coords(point);
        self.emit_model_action_with_profile(
            UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                over_browser_list: !self.model.map.active && layout.browser_rows.contains(point),
                shift_down: self.modifiers.shift_key(),
                alt_down: self.modifiers.alt_key(),
            },
            Some(InteractionProfileKind::Waveform),
        );
        true
    }

    /// Process one map-focus drag cursor update when map drag mode is active.
    fn process_map_focus_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        if !self.model.map.active {
            return false;
        }
        let Some(action) = self
            .shell_state
            .map_sample_action_at_point(layout, &self.model, point)
        else {
            return false;
        };
        let UiAction::FocusMapSample { sample_id } = &action else {
            return false;
        };
        if self.last_emitted_map_drag_sample_id.as_deref() == Some(sample_id.as_str()) {
            return false;
        }
        self.last_emitted_map_drag_sample_id = Some(sample_id.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::MapPanProxy));
        true
    }

    /// Emit one normalized volume update immediately for smooth drag visuals.
    fn emit_volume_milli_immediately(&mut self, value_milli: u16) {
        self.queue_volume_milli(value_milli);
        let _ = self.flush_pending_volume_action();
    }

    fn flush_pending_volume_action(&mut self) -> bool {
        let Some(value_milli) = self.pending_volume_milli.take() else {
            return false;
        };
        self.emit_model_action_with_profile(
            UiAction::SetVolume { value_milli },
            Some(InteractionProfileKind::Volume),
        );
        true
    }

    /// Handle one pointer-press action, deferring drag-only waveform edits until movement.
    fn handle_pointer_press_action(&mut self, action: UiAction, map_drag_start: bool) -> bool {
        let waveform_drag_mode = waveform_drag_mode_for_action(&action);
        let map_drag_sample_id = match &action {
            UiAction::FocusMapSample { sample_id } => Some(sample_id.clone()),
            _ => None,
        };
        self.waveform_drag_mode = waveform_drag_mode;
        self.selection_drag_active = matches!(action, UiAction::StartWaveformSelectionDrag { .. });
        if !waveform_press_action_emits_immediately(&action) {
            return false;
        }
        self.update_text_target_after_action(&action);
        self.emit_model_action(action);
        if map_drag_start {
            self.map_focus_drag_active = true;
            self.last_emitted_map_drag_sample_id = map_drag_sample_id;
        }
        true
    }

    fn finish_volume_drag(&mut self, released_button: Option<MouseButton>) {
        let finish_edit_fade_drag = self
            .waveform_drag_mode
            .is_some_and(waveform_drag_mode_is_edit_fade);
        let finish_selection_drag =
            self.selection_drag_active && matches!(released_button, Some(MouseButton::Left));
        let seek_on_waveform_click_release = if matches!(released_button, Some(MouseButton::Left))
            && self.last_emitted_waveform_drag_action.is_none()
        {
            if let Some(mode @ WaveformPointerDragMode::Selection { .. }) = self.waveform_drag_mode
            {
                if let (Some(layout), Some(point)) = (self.shell_layout.as_ref(), self.last_cursor)
                {
                    !waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        let _ = self.flush_pending_volume_action();
        if self.volume_drag_active {
            self.emit_model_action(UiAction::CommitVolumeSetting);
        }
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        if finish_edit_fade_drag {
            self.emit_model_action(UiAction::FinishWaveformEditFadeDrag);
        }
        if finish_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionDrag);
        }
        self.waveform_drag_mode = None;
        self.selection_drag_active = false;
        self.last_emitted_waveform_drag_action = None;
        self.map_focus_drag_active = false;
        self.last_emitted_map_drag_sample_id = None;
        self.browser_scrollbar_drag = None;
        self.last_emitted_browser_view_start = None;
        if seek_on_waveform_click_release
            && let (Some(layout), Some(point)) = (self.shell_layout.as_ref(), self.last_cursor)
        {
            let position_milli = waveform_position_milli_from_point(layout, &self.model, point);
            self.emit_model_action_with_profile(
                UiAction::SeekWaveform { position_milli },
                Some(InteractionProfileKind::Waveform),
            );
        }
    }

    /// Reveal the native window after startup sequencing reaches a stable frame.
    fn maybe_reveal_startup_window(&mut self) {
        if self.startup_window_visible || !self.first_frame_presented {
            return;
        }
        if self.startup_model_pull_pending || self.startup_deferred_model_refresh_pending {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
    }

    /// Reveal the window once the first full scene is ready on eager startup paths.
    fn maybe_reveal_startup_window_after_first_scene_ready(&mut self) {
        if self.startup_window_visible
            || self.first_frame_presented
            || self.startup_model_pull_pending
            || self.startup_deferred_model_refresh_pending
        {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
    }

    /// Force startup reveal when redraw delivery stalls while hidden.
    ///
    /// Some backends can throttle redraw delivery for hidden windows. This
    /// fallback ensures the app cannot remain hidden forever waiting on a
    /// second present.
    fn maybe_force_reveal_startup_window_on_stall(&mut self, now: Instant) {
        if self.startup_window_visible {
            return;
        }
        let Some(deadline) = self.startup_reveal_deadline else {
            return;
        };
        if now < deadline {
            return;
        }
        warn!("native vello startup reveal fallback: forcing window visible after stall");
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
        self.request_redraw_if_needed();
    }

    /// Handle one successful first present and schedule deferred startup pulls.
    fn complete_first_present(&mut self) {
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            if self.startup_model_pull_pending {
                self.startup_model_pull_pending = false;
                self.startup_deferred_model_refresh_pending = true;
                if !self.startup_window_visible {
                    self.arm_startup_reveal_deadline(Instant::now());
                }
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
        self.maybe_reveal_startup_window();
    }

    fn flush_pending_input(&mut self) -> bool {
        let mut pending_action = false;
        if self.flush_pending_volume_action() {
            pending_action = true;
        }
        if let Some(point) = self.pending_cursor.take() {
            let (_, handled) = self.process_cursor_move_immediately(point);
            if handled {
                pending_action = true;
            }
        }

        pending_action
    }

    fn mark_idle_status_refresh_if_due(&mut self, now: Instant) -> bool {
        if now < self.next_idle_status_refresh {
            return false;
        }
        let mut next_refresh = self.next_idle_status_refresh;
        while next_refresh <= now {
            next_refresh += self.idle_status_refresh_interval;
        }
        self.next_idle_status_refresh = next_refresh;
        self.frame_state.mark_motion_overlay_dirty();
        true
    }

    /// Return bridge-provided revision for one static segment.
    fn static_segment_revision(
        &self,
        segment_revisions: SegmentRevisions,
        segment: StaticFrameSegment,
    ) -> u64 {
        match segment {
            StaticFrameSegment::StatusBar => segment_revisions.status_bar,
            StaticFrameSegment::BrowserFrame => segment_revisions.browser_frame,
            StaticFrameSegment::BrowserRowsWindow => segment_revisions.browser_rows_window,
            StaticFrameSegment::MapPanel => segment_revisions.map_panel,
            StaticFrameSegment::WaveformOverlay => segment_revisions.waveform_overlay,
            StaticFrameSegment::GlobalStatic => segment_revisions.global_static,
        }
    }

    /// Return deterministic static segment identifier from cache-array index.
    fn static_segment_from_cache_index(index: usize) -> StaticFrameSegment {
        match index {
            0 => StaticFrameSegment::GlobalStatic,
            1 => StaticFrameSegment::WaveformOverlay,
            2 => StaticFrameSegment::BrowserFrame,
            3 => StaticFrameSegment::BrowserRowsWindow,
            4 => StaticFrameSegment::MapPanel,
            5 => StaticFrameSegment::StatusBar,
            _ => unreachable!("invalid static segment index {index}"),
        }
    }

    /// Build candidate fingerprints for every retained static segment.
    fn build_static_segment_fingerprints(
        &self,
        layout: &ShellLayout,
        style: &StyleTokens,
        segment_revisions: SegmentRevisions,
    ) -> [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT] {
        let layout_width_bits = layout.root.rect.width().to_bits();
        let layout_height_bits = layout.root.rect.height().to_bits();
        let layout_scale_bits = layout.ui_scale.to_bits();
        let style_signature = static_segment_style_signature(style);
        std::array::from_fn(|idx| {
            let segment = Self::static_segment_from_cache_index(idx);
            StaticSegmentCacheFingerprint {
                segment,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                style_signature,
                segment_revision: self.static_segment_revision(segment_revisions, segment),
            }
        })
    }

    /// Rebuild and encode retained static segment scenes.
    fn rebuild_static_segment_scenes(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        dirty_segments: DirtySegments,
        segment_revisions: SegmentRevisions,
        force_rebuild: bool,
    ) -> (Duration, Duration) {
        if force_rebuild {
            self.static_segment_graph.clear();
        }
        let fingerprints = self.build_static_segment_fingerprints(layout, style, segment_revisions);
        let diff_plan = self
            .static_segment_graph
            .diff(dirty_segments, force_rebuild, fingerprints);
        let mut build_duration = Duration::ZERO;
        let mut encode_duration = Duration::ZERO;
        for segment in StaticFrameSegment::ALL {
            if !diff_plan.should_rebuild(segment) {
                continue;
            }

            let segment_build_start = Instant::now();
            self.shell_state.build_static_segment_with_style_into(
                layout,
                style,
                &self.model,
                self.motion_model.as_ref(),
                segment,
                &mut self.static_segment_frame_cache,
            );
            build_duration += segment_build_start.elapsed();

            let segment_encode_start = Instant::now();
            let frame = self.static_segment_frame_cache.frame(segment);
            let entry = self.static_segment_scene_cache.entry_mut(segment);
            Self::encode_frame_to_scene(
                frame,
                &mut entry.scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += segment_encode_start.elapsed();
            self.static_segment_graph
                .commit_segment(segment, diff_plan.fingerprint(segment));
        }

        self.frame_cache.clear_color = style.clear_color;
        self.static_segment_frame_cache
            .compose_into(&mut self.frame_cache);
        self.clear_color = self.frame_cache.clear_color;
        self.static_scene.reset();
        for segment in StaticFrameSegment::ALL {
            self.static_scene
                .append(self.static_segment_scene_cache.scene(segment), None);
        }
        (build_duration, encode_duration)
    }

    /// Refresh cached motion-model projection from the latest full app model.
    fn refresh_motion_model_from_model(&mut self) {
        self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
    }

    fn rebuild_scene(
        &mut self,
        model_refresh_requested: bool,
        static_rebuild_requested: bool,
        mut rebuild_static: bool,
        mut rebuild_state_overlay: bool,
        mut rebuild_motion_overlay: bool,
    ) {
        let mut bridge_dirty_segments = DirtySegments::all();
        let should_refresh_model =
            model_refresh_requested || (!self.motion_model_supported && rebuild_motion_overlay);
        let should_refresh_motion = rebuild_motion_overlay && self.motion_model_supported;
        self.profiler.record_scene_rebuilds(
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
        );
        let previous_waveform_signature = self
            .motion_model
            .as_ref()
            .and_then(|model| model.waveform_image_signature);
        if should_refresh_model {
            self.profiler.add_bridge_model_pull_rebuild();
            let pull_start = self.profiler.now_if_enabled();
            self.profiler.add_model_refresh();
            self.model_refresh_count = self.model_refresh_count.saturating_add(1);
            if self.model_refresh_count <= 24 {
                info!(
                    "native vello refreshing model: refresh_count={} rebuild_static={} rebuild_state_overlay={} rebuild_motion_overlay={}",
                    self.model_refresh_count,
                    rebuild_static,
                    rebuild_state_overlay,
                    rebuild_motion_overlay
                );
            }
            self.model = self.bridge.project_model();
            bridge_dirty_segments = self.bridge.take_dirty_segments();
            let bridge_segment_revisions = self.bridge.take_segment_revisions();
            if bridge_segment_revisions.has_static_revisions() {
                self.segment_revisions_supported = true;
            }
            if self.segment_revisions_supported {
                self.segment_revisions = bridge_segment_revisions;
            }
            let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_model_pull(pull_duration);
            self.shell_state.sync_from_model(&self.model);
            self.refresh_motion_model_from_model();
            self.motion_model_supported = true;
            self.sync_text_input_target();
            if self.startup_deferred_model_refresh_pending {
                self.startup_deferred_model_refresh_pending = false;
                self.startup_reveal_deadline = None;
                self.startup_timing.mark_deferred_model_refresh_done();
                self.startup_timing.maybe_emit_summary();
            }
            rebuild_static = resolve_static_rebuild(
                model_refresh_requested,
                static_rebuild_requested,
                bridge_dirty_segments,
            );
            if static_rebuild_from_dirty_mask(
                model_refresh_requested,
                static_rebuild_requested,
                bridge_dirty_segments,
            ) {
                self.profiler.add_dirty_mask_static_rebuild();
            }
        } else if should_refresh_motion {
            self.profiler.add_bridge_motion_pull_rebuild();
            let pull_start = self.profiler.now_if_enabled();
            if let Some(motion_model) = self.bridge.project_motion_model() {
                let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_motion_pull(pull_duration);
                if self.motion_model.as_ref() != Some(&motion_model) {
                    if previous_waveform_signature != motion_model.waveform_image_signature {
                        rebuild_static = true;
                        rebuild_state_overlay = true;
                        rebuild_motion_overlay = true;
                    }
                    self.shell_state.sync_from_motion_model(&motion_model);
                    self.motion_model = Some(motion_model);
                }
            } else {
                let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_motion_pull(pull_duration);
                let model_pull_start = self.profiler.now_if_enabled();
                self.profiler.add_bridge_model_pull_rebuild();
                self.motion_model_supported = false;
                self.model = self.bridge.project_model();
                bridge_dirty_segments = self.bridge.take_dirty_segments();
                let bridge_segment_revisions = self.bridge.take_segment_revisions();
                if bridge_segment_revisions.has_static_revisions() {
                    self.segment_revisions_supported = true;
                }
                if self.segment_revisions_supported {
                    self.segment_revisions = bridge_segment_revisions;
                }
                let model_pull_duration =
                    model_pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_model_pull(model_pull_duration);
                self.shell_state.sync_from_model(&self.model);
                self.refresh_motion_model_from_model();
                self.sync_text_input_target();
                if self.startup_deferred_model_refresh_pending {
                    self.startup_deferred_model_refresh_pending = false;
                    self.startup_reveal_deadline = None;
                    self.startup_timing.mark_deferred_model_refresh_done();
                    self.startup_timing.maybe_emit_summary();
                }
            }
        }
        let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
            return;
        };
        let layout = layout.as_ref();
        let (layout_width_bits, layout_height_bits, layout_scale_bits) = (
            layout.root.rect.width().to_bits(),
            layout.root.rect.height().to_bits(),
            layout.ui_scale.to_bits(),
        );
        let style = self.cached_style_for_layout(layout);
        if rebuild_static {
            if self.incremental_frame_pipeline {
                let mut force_rebuild = !model_refresh_requested;
                if !self.segment_revisions_supported
                    && !self.missing_segment_revision_fallback_applied
                {
                    warn!(
                        "native vello bridge reported zero segment revisions; forcing one conservative static rebuild"
                    );
                    force_rebuild = true;
                    self.missing_segment_revision_fallback_applied = true;
                }
                let (build_duration, encode_duration) = self.rebuild_static_segment_scenes(
                    layout,
                    &style,
                    bridge_dirty_segments,
                    self.segment_revisions,
                    force_rebuild,
                );
                self.profiler.add_build_static(build_duration);
                self.profiler.add_encode_static(encode_duration);
            } else {
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_frame_with_style_into_static(
                    layout,
                    &style,
                    &self.model,
                    &mut self.frame_cache,
                );
                let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_build_static(build_duration);
                self.clear_color = self.frame_cache.clear_color;
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.frame_cache,
                    &mut self.static_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_encode_static(encode_duration);
            }
        }
        if rebuild_state_overlay {
            let state_fingerprint = StateOverlayCacheFingerprint {
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                shell: self.shell_state.state_overlay_fingerprint(),
                model_signature: state_overlay_model_signature(&self.model),
            };
            if self.state_overlay_fingerprint.as_ref() == Some(&state_fingerprint) {
                rebuild_state_overlay = false;
            } else {
                self.state_overlay_fingerprint = Some(state_fingerprint);
            }
        }
        if rebuild_state_overlay {
            let build_start = self.profiler.now_if_enabled();
            self.shell_state.build_state_overlay_into(
                layout,
                &style,
                &self.model,
                &mut self.state_overlay_frame_cache,
            );
            let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_build_state_overlay(build_duration);
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.state_overlay_frame_cache,
                &mut self.state_overlay_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_encode_state_overlay(encode_duration);
        }
        let mut rebuild_waveform_motion_overlay = rebuild_motion_overlay;
        let mut rebuild_chrome_motion_overlay = rebuild_motion_overlay;
        if rebuild_motion_overlay {
            if self.motion_model.is_none() {
                self.refresh_motion_model_from_model();
            }
            let waveform_motion_signature = {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist for waveform-motion signature");
                waveform_motion_overlay_model_signature(motion_model)
            };
            let waveform_motion_fingerprint = WaveformMotionOverlayCacheFingerprint {
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                shell: self.shell_state.waveform_motion_overlay_fingerprint(),
                motion_signature: waveform_motion_signature,
            };
            if self.waveform_motion_overlay_fingerprint.as_ref()
                == Some(&waveform_motion_fingerprint)
            {
                rebuild_waveform_motion_overlay = false;
            } else {
                self.waveform_motion_overlay_fingerprint = Some(waveform_motion_fingerprint);
            }
            let chrome_motion_signature = {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist for chrome-motion signature");
                chrome_motion_overlay_model_signature(motion_model)
            };
            let chrome_motion_fingerprint = ChromeMotionOverlayCacheFingerprint {
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                shell: self.shell_state.chrome_motion_overlay_fingerprint(),
                motion_signature: chrome_motion_signature,
            };
            if self.chrome_motion_overlay_fingerprint.as_ref() == Some(&chrome_motion_fingerprint) {
                rebuild_chrome_motion_overlay = false;
            } else {
                self.chrome_motion_overlay_fingerprint = Some(chrome_motion_fingerprint);
            }
            if !rebuild_waveform_motion_overlay && !rebuild_chrome_motion_overlay {
                self.profiler.add_motion_overlay_skip();
            }
        } else {
            rebuild_waveform_motion_overlay = false;
            rebuild_chrome_motion_overlay = false;
        }
        if rebuild_waveform_motion_overlay || rebuild_chrome_motion_overlay {
            let mut build_duration = Duration::ZERO;
            let mut encode_duration = Duration::ZERO;
            if self.motion_model.is_none() {
                self.refresh_motion_model_from_model();
            }
            if rebuild_waveform_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before waveform-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_waveform_motion_overlay_into(
                    layout,
                    &style,
                    motion_model,
                    &mut self.waveform_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.waveform_motion_overlay_frame_cache,
                    &mut self.waveform_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            if rebuild_chrome_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before chrome-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_chrome_motion_overlay_into(
                    layout,
                    &style,
                    motion_model,
                    &mut self.chrome_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.chrome_motion_overlay_frame_cache,
                    &mut self.chrome_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            self.profiler.add_build_motion_overlay(build_duration);
            self.profiler.add_encode_motion_overlay(encode_duration);
        }
        if rebuild_static
            || rebuild_state_overlay
            || rebuild_waveform_motion_overlay
            || rebuild_chrome_motion_overlay
        {
            self.scene.reset();
            self.scene.append(&self.static_scene, None);
            self.scene.append(&self.state_overlay_scene, None);
            self.scene.append(&self.waveform_motion_overlay_scene, None);
            self.scene.append(&self.chrome_motion_overlay_scene, None);
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_count = self.redraw_count.saturating_add(1);
        self.redraw_requested = false;
        let now = Instant::now();
        let delta = (now - self.last_redraw).as_secs_f32();
        self.last_redraw = now;
        let frame_started_at = Instant::now();
        let frame_profile_start = self.profiler.now_if_enabled();
        let rebuild_start = self.profiler.now_if_enabled();
        let needs_animation = self.shell_state.needs_animation();
        let (has_rebuild, mut frame_result) = self.rebuild_scene_for_redraw(needs_animation, delta);
        let rebuild_duration = rebuild_start.map_or(Duration::ZERO, |start| start.elapsed());
        if self.redraw_count <= 8 {
            info!(
                "native vello redraw start: redraw_count={} needs_animation={} has_rebuild={} delta_ms={}",
                self.redraw_count,
                needs_animation,
                has_rebuild,
                ((delta * 1000.0) as u32)
            );
        }
        if !needs_animation && !has_rebuild && self.first_frame_presented {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };

        let mut surface_error = None;
        let mut needs_resize = false;
        let mut out_of_memory = false;
        let acquire_start = self.profiler.now_if_enabled();
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                self.finish_redraw_attempt(
                    &mut frame_result,
                    frame_started_at,
                    frame_profile_start,
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    false,
                    false,
                );
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => Some(frame),
                Err(err) => {
                    surface_error = Some(err.clone());
                    if self.redraw_count <= 8 {
                        warn!("native vello surface acquire error: {:?}", err);
                    }
                    match err {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            let size = window.inner_size();
                            if let Some(render_ctx) = self.render_ctx.as_mut() {
                                render_ctx.resize_surface(
                                    surface,
                                    size.width.max(1),
                                    size.height.max(1),
                                );
                                needs_resize = true;
                            }
                        }
                        wgpu::SurfaceError::OutOfMemory => out_of_memory = true,
                        wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
                    }
                    None
                }
            }
        };
        let acquire_duration = acquire_start.map_or(Duration::ZERO, |start| start.elapsed());
        if let Some(err) = surface_error {
            if out_of_memory {
                error!("native vello out-of-memory in surface acquire: {:?}", err);
            } else if self.redraw_count <= 8 {
                info!("native vello non-fatal surface error: {:?}", err);
            }
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            if matches!(err, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)
                && needs_resize
            {
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
                self.rebuild_scene_if_needed();
            }
            if out_of_memory {
                event_loop.exit();
            }
            return;
        }
        let Some(surface_texture) = surface_texture else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };

        let Some(surface) = self.render_surface.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_start = self.profiler.now_if_enabled();
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: color_from_rgba(self.clear_color),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        if let Err(err) = render_result {
            error!("native vello render_to_texture failed: {:?}", err);
            event_loop.exit();
            let render = render_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                render,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        }
        let render_duration = render_start.map_or(Duration::ZERO, |start| start.elapsed());
        let blit_start = self.profiler.now_if_enabled();
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("native_vello_present_blit"),
                });
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        let blit_duration = blit_start.map_or(Duration::ZERO, |start| start.elapsed());
        let present_started_at = Instant::now();
        surface_texture.present();
        self.complete_first_present();
        let present_duration = present_started_at.elapsed();
        self.finish_redraw_attempt(
            &mut frame_result,
            frame_started_at,
            frame_profile_start,
            rebuild_duration,
            acquire_duration,
            render_duration,
            blit_duration,
            present_duration,
            true,
            true,
        );
    }

    fn sync_text_input_target(&mut self) {
        if self.model.confirm_prompt.visible && self.model.confirm_prompt.input_value.is_some() {
            self.text_input_target = TextInputTarget::PromptInput;
        } else if self.text_input_target == TextInputTarget::PromptInput {
            self.text_input_target = TextInputTarget::None;
        }
        if self.text_input_target != TextInputTarget::None {
            match self.text_input_target {
                TextInputTarget::BrowserSearch
                | TextInputTarget::FolderSearch
                | TextInputTarget::PromptInput => {
                    if self.text_input_buffer.is_none() {
                        self.text_input_buffer = Some(match self.text_input_target {
                            TextInputTarget::BrowserSearch => {
                                self.model.browser.search_query.clone()
                            }
                            TextInputTarget::FolderSearch => {
                                self.model.sources.folder_search_query.clone()
                            }
                            TextInputTarget::PromptInput => self
                                .model
                                .confirm_prompt
                                .input_value
                                .clone()
                                .unwrap_or_default(),
                            TextInputTarget::None | TextInputTarget::WaveformBpm => String::new(),
                        });
                    }
                }
                TextInputTarget::WaveformBpm => {
                    if self.waveform_bpm_input_buffer.is_none() {
                        self.waveform_bpm_input_buffer = Some(self.waveform_bpm_text_from_model());
                    }
                }
                TextInputTarget::None => {}
            }
            let current_text = self.current_text_value().unwrap_or_default();
            let mut editor = self
                .text_editor_state
                .take()
                .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&current_text));
            editor.clamp_to_text(&current_text);
            self.text_editor_state = Some(editor);
        } else {
            self.text_input_buffer = None;
            self.text_editor_state = None;
            self.text_input_drag_active = false;
        }
        if self.text_input_target != TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
    }

    fn current_text_value(&self) -> Option<String> {
        match self.text_input_target {
            TextInputTarget::None => None,
            TextInputTarget::BrowserSearch
            | TextInputTarget::FolderSearch
            | TextInputTarget::PromptInput => self.text_input_buffer.clone().or_else(|| match self
                .text_input_target
            {
                TextInputTarget::BrowserSearch => Some(self.model.browser.search_query.clone()),
                TextInputTarget::FolderSearch => {
                    Some(self.model.sources.folder_search_query.clone())
                }
                TextInputTarget::PromptInput => self.model.confirm_prompt.input_value.clone(),
                TextInputTarget::None | TextInputTarget::WaveformBpm => None,
            }),
            TextInputTarget::WaveformBpm => Some(
                self.waveform_bpm_input_buffer
                    .clone()
                    .unwrap_or_else(|| self.waveform_bpm_text_from_model()),
            ),
        }
    }

    fn set_text_value(&mut self, value: String) -> bool {
        let action = match self.text_input_target {
            TextInputTarget::None => return false,
            TextInputTarget::BrowserSearch => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetBrowserSearch { query: value }
            }
            TextInputTarget::FolderSearch => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetFolderSearch { query: value }
            }
            TextInputTarget::PromptInput => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetPromptInput { value }
            }
            TextInputTarget::WaveformBpm => {
                self.waveform_bpm_input_buffer = Some(value.clone());
                self.sync_waveform_bpm_editor_state();
                self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
                if let Some(parsed) = parse_waveform_bpm_input(&value) {
                    UiAction::SetWaveformBpmValue {
                        value_tenths: bpm_tenths_from_value(parsed),
                    }
                } else {
                    return true;
                }
            }
        };
        self.emit_model_action(action);
        self.sync_browser_search_editor_state();
        true
    }

    fn append_text(&mut self, appended: &str) -> bool {
        let appended = sanitize_single_line_insert(appended);
        if appended.is_empty() {
            return false;
        }
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let sanitized = if self.text_input_target == TextInputTarget::WaveformBpm {
            sanitize_waveform_bpm_insert(&value, editor.selection_range(), &appended)
        } else {
            appended
        };
        if sanitized.is_empty() {
            return false;
        }
        let next = editor.replace_selection(&value, &sanitized);
        self.set_text_value(next)
    }

    fn waveform_bpm_text_from_model(&self) -> String {
        self.model
            .waveform
            .tempo_label
            .as_deref()
            .and_then(parse_waveform_tempo_number_text)
            .unwrap_or_else(|| String::from("120.0"))
    }

    fn activate_waveform_bpm_input(&mut self) {
        self.text_input_target = TextInputTarget::WaveformBpm;
        let text = self
            .waveform_bpm_input_buffer
            .clone()
            .unwrap_or_else(|| self.waveform_bpm_text_from_model());
        self.waveform_bpm_input_buffer = Some(text.clone());
        let mut editor = SingleLineTextEditorState::collapsed_at_end(&text);
        editor.select_all(&text);
        self.text_editor_state = Some(editor);
        self.sync_waveform_bpm_editor_state();
        self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
    }

    fn activate_text_input_target(&mut self, target: TextInputTarget) {
        if matches!(target, TextInputTarget::None | TextInputTarget::WaveformBpm) {
            return;
        }
        let current_text = match target {
            TextInputTarget::BrowserSearch => self.model.browser.search_query.clone(),
            TextInputTarget::FolderSearch => self.model.sources.folder_search_query.clone(),
            TextInputTarget::PromptInput => self
                .model
                .confirm_prompt
                .input_value
                .clone()
                .unwrap_or_default(),
            TextInputTarget::None | TextInputTarget::WaveformBpm => String::new(),
        };
        self.text_input_target = target;
        self.text_input_buffer = Some(current_text.clone());
        self.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end(&current_text));
        self.waveform_bpm_input_buffer = None;
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
    }

    fn deactivate_text_input_target(&mut self) {
        let previous_target = self.text_input_target;
        let was_waveform_bpm = self.text_input_target == TextInputTarget::WaveformBpm;
        if self.text_input_target == TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        self.text_input_target = TextInputTarget::None;
        self.text_input_buffer = None;
        self.text_editor_state = None;
        self.text_input_drag_active = false;
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
        if previous_target == TextInputTarget::BrowserSearch {
            self.emit_model_action(UiAction::BlurBrowserSearch);
        }
        if was_waveform_bpm {
            self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
        }
    }

    fn step_waveform_bpm_input(&mut self, delta_tenths: i16) -> bool {
        if self.text_input_target != TextInputTarget::WaveformBpm || delta_tenths == 0 {
            return false;
        }
        let current = self
            .current_text_value()
            .and_then(|value| parse_waveform_bpm_input(&value))
            .unwrap_or(120.0);
        let next = (current + (f32::from(delta_tenths) / 10.0)).max(1.0);
        let next_text = format!("{next:.1}");
        self.waveform_bpm_input_buffer = Some(next_text.clone());
        let mut editor = SingleLineTextEditorState::collapsed_at_end(&next_text);
        editor.select_all(&next_text);
        self.text_editor_state = Some(editor);
        self.sync_waveform_bpm_editor_state();
        self.emit_model_action(UiAction::SetWaveformBpmValue {
            value_tenths: bpm_tenths_from_value(next),
        });
        true
    }

    fn build_active_text_field_visual_state(
        &mut self,
        layout: &ShellLayout,
        text_rect: UiRect,
    ) -> Option<TextFieldVisualState> {
        let text = self.current_text_value().unwrap_or_default();
        let mut editor = self
            .text_editor_state
            .take()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
        let layout_state = build_text_field_layout(
            &mut self.text_renderer,
            &mut editor,
            &text,
            StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
                .sizing
                .font_meta,
            text_rect.width(),
        );
        self.text_editor_state = Some(editor);
        Some(TextFieldVisualState {
            text: layout_state.visible_text,
            caret_offset: layout_state.caret_offset,
            selection_offsets: layout_state.selection_offsets,
        })
    }

    fn sync_waveform_bpm_editor_state(&mut self) {
        let active = self.text_input_target == TextInputTarget::WaveformBpm;
        let display = if active {
            self.waveform_bpm_input_buffer
                .clone()
                .or_else(|| Some(self.waveform_bpm_text_from_model()))
        } else {
            None
        };
        let visual = if active {
            self.with_shell_layout(|this, layout| {
                this.shell_state
                    .waveform_bpm_text_rect(layout, &this.model)
                    .and_then(|text_rect| {
                        this.build_active_text_field_visual_state(layout, text_rect)
                    })
            })
            .flatten()
        } else {
            None
        };
        self.shell_state
            .set_waveform_bpm_editor_state(active, display, visual);
    }

    fn sync_browser_search_editor_state(&mut self) {
        if self.text_input_target != TextInputTarget::BrowserSearch {
            self.shell_state.set_browser_search_editor_state(None);
            return;
        }
        let Some(visual) = self.with_shell_layout(|this, layout| {
            this.shell_state
                .browser_search_text_rect(layout, &this.model)
                .and_then(|text_rect| this.build_active_text_field_visual_state(layout, text_rect))
        }) else {
            self.shell_state.set_browser_search_editor_state(None);
            return;
        };
        self.shell_state.set_browser_search_editor_state(visual);
    }

    fn classify_action_scope(action: &UiAction) -> RuntimeInvalidationScope {
        match action {
            UiAction::SetVolume { .. }
            | UiAction::CommitVolumeSetting
            | UiAction::SetFolderSearch { .. }
            | UiAction::ReloadSourceRow { .. }
            | UiAction::HardSyncSourceRow { .. }
            | UiAction::OpenSourceFolderRow { .. }
            | UiAction::RemoveSourceRow { .. }
            | UiAction::RemoveDeadLinksForSourceRow { .. }
            | UiAction::FocusFolderRow { .. }
            | UiAction::MoveFolderFocus { .. }
            | UiAction::MoveBrowserFocus { .. }
            | UiAction::SetBrowserViewStart { .. }
            | UiAction::FocusBrowserRow { .. }
            | UiAction::ToggleBrowserRowSelection { .. }
            | UiAction::ExtendBrowserSelectionToRow { .. }
            | UiAction::AddRangeBrowserSelection { .. }
            | UiAction::ExtendBrowserSelectionFromFocus { .. }
            | UiAction::AddRangeBrowserSelectionFromFocus { .. }
            | UiAction::ToggleFocusedBrowserRowSelection
            | UiAction::SelectAllBrowserRows
            | UiAction::SetBrowserSearch { .. }
            | UiAction::BlurBrowserSearch
            | UiAction::SetBrowserTab { .. }
            | UiAction::FocusMapSample { .. }
            | UiAction::SetPromptInput { .. }
            | UiAction::SetWaveformBpmValue { .. }
            | UiAction::AdjustWaveformBpm { .. }
            | UiAction::SetWaveformSelectionRange { .. }
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
            | UiAction::FinishWaveformEditFadeDrag
            | UiAction::StartWaveformSelectionDrag { .. }
            | UiAction::UpdateWaveformSelectionDrag { .. }
            | UiAction::FinishWaveformSelectionDrag
            | UiAction::ClearWaveformSelection
            | UiAction::ClearWaveformEditSelection => RuntimeInvalidationScope::ModelAndOverlays,
            UiAction::SeekWaveform { .. }
            | UiAction::PlayFromStart
            | UiAction::PlayFromCurrentPlayhead
            | UiAction::SetWaveformCursor { .. } => RuntimeInvalidationScope::OverlayMotionOnly,
            UiAction::ZoomWaveform { .. }
            | UiAction::ZoomWaveformToSelection
            | UiAction::ZoomWaveformFull => RuntimeInvalidationScope::StaticAndOverlays,
            _ => RuntimeInvalidationScope::StaticAndOverlays,
        }
    }

    /// Classify bridge actions into tracked interaction profile groups.
    fn classify_action_interaction(action: &UiAction) -> Option<InteractionProfileKind> {
        match action {
            UiAction::SetBrowserTab { map: true } | UiAction::FocusMapSample { .. } => {
                Some(InteractionProfileKind::MapPanProxy)
            }
            UiAction::SeekWaveform { .. }
            | UiAction::PlayFromStart
            | UiAction::PlayFromCurrentPlayhead
            | UiAction::SetWaveformCursor { .. }
            | UiAction::SetWaveformSelectionRange { .. }
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
            | UiAction::SetWaveformBpmValue { .. }
            | UiAction::AdjustWaveformBpm { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
            | UiAction::FinishWaveformEditFadeDrag
            | UiAction::StartWaveformSelectionDrag { .. }
            | UiAction::UpdateWaveformSelectionDrag { .. }
            | UiAction::FinishWaveformSelectionDrag
            | UiAction::ClearWaveformSelection
            | UiAction::ClearWaveformEditSelection
            | UiAction::ZoomWaveform { .. }
            | UiAction::ZoomWaveformToSelection
            | UiAction::ZoomWaveformFull => Some(InteractionProfileKind::Waveform),
            UiAction::SetVolume { .. } => Some(InteractionProfileKind::Volume),
            _ => None,
        }
    }

    /// Apply one model action and optionally record interaction latency.
    fn emit_model_action_with_profile(
        &mut self,
        action: UiAction,
        profile_kind: Option<InteractionProfileKind>,
    ) {
        self.apply_invalidation_scope(Self::classify_action_scope(&action));
        let profile_start = profile_kind.and_then(|_| self.profiler.now_if_enabled());
        self.bridge.reduce_action(action);
        if let (Some(kind), Some(start)) = (profile_kind, profile_start) {
            self.profiler.add_interaction_latency(kind, start.elapsed());
        }
    }

    /// Apply one model action with default interaction profiling classification.
    fn emit_model_action(&mut self, action: UiAction) {
        let profile_kind = Self::classify_action_interaction(&action);
        self.emit_model_action_with_profile(action, profile_kind);
    }

    fn backspace_text(&mut self) -> bool {
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let Some(next) = editor.backspace(&value) else {
            return false;
        };
        self.set_text_value(next)
    }

    fn delete_text_forward(&mut self) -> bool {
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let Some(next) = editor.delete_forward(&value) else {
            return false;
        };
        self.set_text_value(next)
    }

    fn move_text_cursor(&mut self, key: KeyCode, extend_selection: bool) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let moved = match key {
            KeyCode::ArrowLeft => editor.move_left(&text, extend_selection),
            KeyCode::ArrowRight => editor.move_right(&text, extend_selection),
            KeyCode::Home => editor.move_home(&text, extend_selection),
            KeyCode::End => editor.move_end(&text, extend_selection),
            _ => false,
        };
        if moved {
            if self.text_input_target == TextInputTarget::WaveformBpm {
                self.sync_waveform_bpm_editor_state();
            } else {
                self.sync_browser_search_editor_state();
            }
        }
        moved
    }

    fn select_all_text(&mut self) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        editor.select_all(&text);
        if self.text_input_target == TextInputTarget::WaveformBpm {
            self.sync_waveform_bpm_editor_state();
        } else {
            self.sync_browser_search_editor_state();
        }
        true
    }

    fn copy_selected_text(&mut self) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_ref() else {
            return false;
        };
        let Some(selected) = editor.selected_text(&text) else {
            return false;
        };
        self.write_clipboard_text(&selected)
    }

    fn cut_selected_text(&mut self) -> bool {
        if !self.copy_selected_text() {
            return false;
        }
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        if !editor.has_selection() {
            return false;
        }
        let next = editor.replace_selection(&text, "");
        self.set_text_value(next)
    }

    fn paste_text(&mut self) -> bool {
        let Some(text) = self.read_clipboard_text() else {
            return false;
        };
        self.append_text(&text)
    }

    fn update_text_target_after_action(&mut self, action: &UiAction) {
        match action {
            UiAction::FocusBrowserSearch => {
                self.activate_text_input_target(TextInputTarget::BrowserSearch)
            }
            UiAction::BlurBrowserSearch => self.text_input_target = TextInputTarget::None,
            UiAction::FocusFolderSearch => {
                self.activate_text_input_target(TextInputTarget::FolderSearch)
            }
            UiAction::ConfirmPrompt | UiAction::CancelPrompt => {
                self.text_input_target = TextInputTarget::None;
                self.text_input_buffer = None;
                self.text_editor_state = None;
            }
            _ => {}
        }
        if self.text_input_target != TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        if self.text_input_target == TextInputTarget::None {
            self.text_input_buffer = None;
            self.text_editor_state = None;
            self.text_input_drag_active = false;
            self.shell_state.set_browser_search_editor_state(None);
        }
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
    }

    fn read_clipboard_text(&mut self) -> Option<String> {
        if let Some(clipboard) = self.clipboard.as_mut()
            && let Ok(text) = clipboard.get_text()
        {
            self.clipboard_fallback_text = text.clone();
            return Some(text);
        }
        if self.clipboard.is_none()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            self.clipboard_fallback_text = text.clone();
            self.clipboard = Some(clipboard);
            return Some(text);
        }
        (!self.clipboard_fallback_text.is_empty()).then(|| self.clipboard_fallback_text.clone())
    }

    fn write_clipboard_text(&mut self, text: &str) -> bool {
        self.clipboard_fallback_text = text.to_string();
        if let Some(clipboard) = self.clipboard.as_mut()
            && clipboard.set_text(text.to_string()).is_ok()
        {
            return true;
        }
        if self.clipboard.is_none()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(text.to_string());
            self.clipboard = Some(clipboard);
        }
        true
    }

    fn browser_search_click_byte_index(
        &mut self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<usize> {
        let text_rect = self
            .shell_state
            .browser_search_text_rect(layout, &self.model)?;
        let text = self
            .current_text_value()
            .unwrap_or_else(|| self.model.browser.search_query.clone());
        let font_size = self.cached_style_for_layout(layout).sizing.font_meta;
        let mut editor = self
            .text_editor_state
            .clone()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
        let layout_state = build_text_field_layout(
            &mut self.text_renderer,
            &mut editor,
            &text,
            font_size,
            text_rect.width(),
        );
        Some(byte_index_for_local_x(
            &layout_state,
            (point.x - text_rect.min.x).clamp(0.0, text_rect.width()),
        ))
    }

    fn waveform_bpm_click_byte_index(
        &mut self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<usize> {
        let text_rect = self
            .shell_state
            .waveform_bpm_text_rect(layout, &self.model)?;
        let text = self
            .current_text_value()
            .unwrap_or_else(|| self.waveform_bpm_text_from_model());
        let font_size = self.cached_style_for_layout(layout).sizing.font_meta;
        let mut editor = self
            .text_editor_state
            .clone()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
        let layout_state = build_text_field_layout(
            &mut self.text_renderer,
            &mut editor,
            &text,
            font_size,
            text_rect.width(),
        );
        Some(byte_index_for_local_x(
            &layout_state,
            (point.x - text_rect.min.x).clamp(0.0, text_rect.width()),
        ))
    }

    fn handle_browser_search_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        let Some(field_rect) = self
            .shell_state
            .browser_search_field_rect(layout, &self.model)
        else {
            return false;
        };
        if !field_rect.contains(point) {
            return false;
        }
        if self.text_input_target != TextInputTarget::BrowserSearch {
            self.emit_model_action(UiAction::FocusBrowserSearch);
            self.activate_text_input_target(TextInputTarget::BrowserSearch);
        }
        let Some(byte_index) = self.browser_search_click_byte_index(layout, point) else {
            return false;
        };
        let text = self
            .current_text_value()
            .unwrap_or_else(|| self.model.browser.search_query.clone());
        let editor = self
            .text_editor_state
            .get_or_insert_with(|| SingleLineTextEditorState::collapsed_at_end(&text));
        editor.set_cursor(&text, byte_index, extend_selection);
        self.text_input_drag_active = true;
        self.sync_browser_search_editor_state();
        self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        true
    }

    fn handle_waveform_bpm_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        let Some(field_rect) = self
            .shell_state
            .waveform_bpm_input_rect(layout, &self.model)
        else {
            return false;
        };
        if !field_rect.contains(point) {
            return false;
        }
        if self.text_input_target != TextInputTarget::WaveformBpm {
            self.activate_waveform_bpm_input();
        }
        let Some(byte_index) = self.waveform_bpm_click_byte_index(layout, point) else {
            return false;
        };
        let text = self
            .current_text_value()
            .unwrap_or_else(|| self.waveform_bpm_text_from_model());
        let editor = self
            .text_editor_state
            .get_or_insert_with(|| SingleLineTextEditorState::collapsed_at_end(&text));
        editor.set_cursor(&text, byte_index, extend_selection);
        self.text_input_drag_active = true;
        self.sync_waveform_bpm_editor_state();
        self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        true
    }

    fn process_text_input_drag(&mut self, point: Point) -> bool {
        if !self.text_input_drag_active {
            return false;
        }
        let Some((byte_index, text)) = self
            .with_shell_layout(|this, layout| match this.text_input_target {
                TextInputTarget::BrowserSearch => {
                    let byte_index = this.browser_search_click_byte_index(layout, point)?;
                    Some((
                        byte_index,
                        this.current_text_value()
                            .unwrap_or_else(|| this.model.browser.search_query.clone()),
                    ))
                }
                TextInputTarget::WaveformBpm => {
                    let byte_index = this.waveform_bpm_click_byte_index(layout, point)?;
                    Some((
                        byte_index,
                        this.current_text_value()
                            .unwrap_or_else(|| this.waveform_bpm_text_from_model()),
                    ))
                }
                _ => None,
            })
            .flatten()
        else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        editor.set_cursor(&text, byte_index, true);
        if self.text_input_target == TextInputTarget::WaveformBpm {
            self.sync_waveform_bpm_editor_state();
        } else {
            self.sync_browser_search_editor_state();
        }
        self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        true
    }
}

fn touch_image_upload_blob_cache_key(
    cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
    key: ImageUploadBlobCacheKey,
) {
    if let Some(position) = cache_order.iter().position(|existing| *existing == key) {
        let _ = cache_order.remove(position);
    }
    cache_order.push_back(key);
}

fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

fn sanitize_waveform_bpm_insert(
    current: &str,
    selection_range: (usize, usize),
    inserted: &str,
) -> String {
    let (selection_start, selection_end) = selection_range;
    let mut sanitized = String::with_capacity(inserted.len());
    let mut has_decimal =
        current[..selection_start].contains('.') || current[selection_end..].contains('.');
    for ch in inserted.chars() {
        if ch.is_ascii_digit() {
            sanitized.push(ch);
        } else if ch == '.' && !has_decimal {
            sanitized.push(ch);
            has_decimal = true;
        }
    }
    sanitized
}

fn parse_waveform_bpm_input(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(parsed)
}

fn bpm_tenths_from_value(value: f32) -> u16 {
    let scaled = (value * 10.0).round();
    if !scaled.is_finite() {
        return 0;
    }
    scaled.clamp(0.0, u16::MAX as f32) as u16
}

impl<B: NativeAppBridge> ApplicationHandler<RuntimeUserEvent> for NativeVelloRunner<B> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resumed_count = self.resumed_count.saturating_add(1);
        if self.resumed_count <= 2 {
            info!(
                "radiant native vello resumed event: resumed_count={}",
                self.resumed_count
            );
        }
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
            self.request_redraw_if_needed();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        self.window_event_count = self.window_event_count.saturating_add(1);
        match event {
            WindowEvent::CloseRequested => {
                warn!("radiant native vello close requested");
                event_loop.exit()
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if self.window_event_count <= 30 {
                    info!(
                        "scale factor changed: window_event_count={}",
                        self.window_event_count
                    );
                }
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
            }
            WindowEvent::Resized(size) => {
                if self.window_event_count <= 30 && (size.width == 0 || size.height == 0) {
                    warn!(
                        width = size.width,
                        height = size.height,
                        "radiant native vello received zero-size resize"
                    );
                }
                if size.width > 0 && size.height > 0 && self.window.is_some() {
                    if let (Some(render_ctx), Some(surface)) =
                        (self.render_ctx.as_ref(), self.render_surface.as_mut())
                    {
                        render_ctx.resize_surface(surface, size.width, size.height);
                        self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x as f32, position.y as f32);
                if self.last_cursor == Some(point) {
                    return;
                }
                self.last_cursor = Some(point);
                self.note_cursor_activity(Instant::now());
                self.update_waveform_resize_cursor(point);
                if self.volume_drag_active
                    && let Some(layout) = self.shell_layout.as_ref()
                    && let Some(action) = self.shell_state.top_bar_volume_drag_action(layout, point)
                {
                    if let UiAction::SetVolume { value_milli } = action {
                        if self.last_emitted_volume_milli != Some(value_milli) {
                            self.last_emitted_volume_milli = Some(value_milli);
                            self.emit_volume_milli_immediately(value_milli);
                        }
                    } else {
                        self.emit_model_action(action);
                    }
                } else if !self.volume_drag_active && self.browser_scrollbar_drag.is_some() {
                    let _ = self.process_browser_scrollbar_drag_immediately(point);
                }
                if !self.volume_drag_active
                    && self.browser_scrollbar_drag.is_none()
                    && self.waveform_drag_mode.is_some()
                {
                    let _ = self.process_waveform_drag_immediately(point);
                } else if !self.volume_drag_active
                    && self.browser_scrollbar_drag.is_none()
                    && self.selection_drag_active
                {
                    let _ = self.process_selection_drag_immediately(point);
                } else if !self.volume_drag_active
                    && self.browser_scrollbar_drag.is_none()
                    && self.map_focus_drag_active
                {
                    let _ = self.process_map_focus_drag_immediately(point);
                } else if !self.volume_drag_active && self.text_input_drag_active {
                    if !self.process_text_input_drag(point) {
                        let (processed, _) = self.process_cursor_move_immediately(point);
                        if !processed {
                            self.queue_cursor(point);
                        }
                    }
                } else if !self.volume_drag_active {
                    let (processed, _) = self.process_cursor_move_immediately(point);
                    if !processed {
                        self.queue_cursor(point);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
                self.pending_cursor = None;
                self.set_cursor_icon(CursorIcon::Default);
            }
            WindowEvent::MouseInput {
                button,
                state: ElementState::Pressed,
                ..
            } if matches!(button, MouseButton::Left | MouseButton::Right) => {
                if self.window.is_none() {
                    return;
                }
                if let Some(point) = self.last_cursor {
                    let _ = self.with_shell_layout(|this, layout| {
                        this.pending_volume_milli = None;
                        this.volume_drag_active = false;
                        this.last_emitted_volume_milli = None;
                        this.waveform_drag_mode = None;
                        this.selection_drag_active = false;
                        this.last_emitted_waveform_drag_action = None;
                        this.map_focus_drag_active = false;
                        this.last_emitted_map_drag_sample_id = None;
                        this.browser_scrollbar_drag = None;
                        this.last_emitted_browser_view_start = None;
                        let mut handled = false;
                        let mut action_emitted = false;
                        let mut source_menu_state_changed = false;
                        match button {
                            MouseButton::Left => {
                                let map_drag_start =
                                    this.model.map.active && layout.browser_rows.contains(point);
                                if let Some(action) = this
                                    .shell_state
                                    .source_context_menu_action_at_point(layout, &this.model, point)
                                {
                                    this.emit_model_action(action);
                                    action_emitted = true;
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
                                    handled = true;
                                } else {
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
                                }
                                if !handled {
                                    if this.handle_browser_search_pointer_press(
                                        layout,
                                        point,
                                        this.modifiers.shift_key(),
                                    ) {
                                        handled = true;
                                    } else if this.handle_waveform_bpm_pointer_press(
                                        layout,
                                        point,
                                        this.modifiers.shift_key(),
                                    ) {
                                        handled = true;
                                    }
                                }
                                if !handled {
                                    if this.shell_state.prompt_input_at_point(
                                        layout,
                                        &this.model,
                                        point,
                                    ) {
                                        this.text_input_target = TextInputTarget::PromptInput;
                                        this.text_input_buffer = Some(
                                            this.model
                                                .confirm_prompt
                                                .input_value
                                                .clone()
                                                .unwrap_or_default(),
                                        );
                                        this.text_editor_state =
                                            Some(SingleLineTextEditorState::collapsed_at_end(
                                                this.model
                                                    .confirm_prompt
                                                    .input_value
                                                    .as_deref()
                                                    .unwrap_or(""),
                                            ));
                                        this.waveform_bpm_input_buffer = None;
                                        this.sync_waveform_bpm_editor_state();
                                        this.sync_browser_search_editor_state();
                                        handled = true;
                                    } else if this.text_input_target != TextInputTarget::None {
                                        this.deactivate_text_input_target();
                                    } else if let Some(action) = this
                                        .shell_state
                                        .top_bar_volume_action_at_point(layout, point)
                                    {
                                        if let UiAction::SetVolume { value_milli } = action {
                                            this.last_emitted_volume_milli = Some(value_milli);
                                            this.emit_volume_milli_immediately(value_milli);
                                        } else {
                                            this.emit_model_action(action);
                                        }
                                        action_emitted = true;
                                        this.volume_drag_active = true;
                                        handled = true;
                                    } else if let Some(thumb_pointer_offset_y) =
                                        this.shell_state.browser_scrollbar_thumb_offset_at_point(
                                            layout,
                                            &this.model,
                                            point,
                                        )
                                    {
                                        this.browser_scrollbar_drag =
                                            Some(BrowserScrollbarDragState {
                                                thumb_pointer_offset_y,
                                            });
                                        handled = true;
                                    } else if this
                                        .process_browser_scrollbar_track_click_immediately(point)
                                    {
                                        handled = true;
                                        action_emitted = true;
                                    } else if let Some(action) = action_from_pointer_with_motion(
                                        layout,
                                        &this.model,
                                        this.motion_model.as_ref(),
                                        &mut this.shell_state,
                                        point,
                                        this.modifiers,
                                    ) {
                                        action_emitted = this
                                            .handle_pointer_press_action(action, map_drag_start);
                                        handled = true;
                                    } else if this.shell_state.handle_primary_click(layout, point)
                                        && let Some(column) = layout.column_at_point(point)
                                    {
                                        this.emit_model_action(UiAction::SelectColumn {
                                            index: column,
                                        });
                                        action_emitted = true;
                                        handled = true;
                                    }
                                }
                            }
                            MouseButton::Right => {
                                if let Some(action) = this
                                    .shell_state
                                    .source_context_menu_action_at_point(layout, &this.model, point)
                                {
                                    this.emit_model_action(action);
                                    action_emitted = true;
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
                                    handled = true;
                                } else if let Some(index) =
                                    this.shell_state
                                        .source_row_at_point(layout, &this.model, point)
                                {
                                    this.emit_model_action(UiAction::SelectSourceRow { index });
                                    this.shell_state
                                        .open_source_context_menu_for_row(index, point);
                                    source_menu_state_changed = true;
                                    action_emitted = true;
                                    handled = true;
                                } else {
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
                                }
                                if !handled
                                    && matches!(
                                        layout.hit_test(point),
                                        Some(ShellNodeKind::WaveformCard)
                                    )
                                {
                                    let action = waveform_edit_action_from_pointer(
                                        layout,
                                        &this.model,
                                        point,
                                        this.modifiers,
                                    );
                                    action_emitted =
                                        this.handle_pointer_press_action(action, false);
                                    handled = true;
                                }
                            }
                            _ => {}
                        }
                        if source_menu_state_changed {
                            this.apply_invalidation_scope(
                                RuntimeInvalidationScope::StaticAndOverlays,
                            );
                        } else if action_emitted
                            && handled
                            && !this.frame_state.has_pending_rebuild()
                        {
                            this.apply_invalidation_scope(
                                RuntimeInvalidationScope::OverlayStateOnly,
                            );
                        }
                    });
                }
            }
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if matches!(button, MouseButton::Left | MouseButton::Right) => {
                self.text_input_drag_active = false;
                self.finish_volume_drag(Some(button));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let _ = self.with_shell_layout(|this, layout| {
                    let waveform_zoom_action = this.last_cursor.and_then(|point| {
                        waveform_wheel_zoom_action(layout, &this.model, point, delta)
                    });
                    let waveform_zoom_emitted = if let Some(action) = waveform_zoom_action {
                        this.emit_model_action_with_profile(
                            action,
                            Some(InteractionProfileKind::Waveform),
                        );
                        true
                    } else {
                        false
                    };
                    if !waveform_zoom_emitted {
                        let fallback_point = Point::new(
                            (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
                            (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
                        );
                        let point = this
                            .last_cursor
                            .filter(|point| layout.browser_panel.contains(*point))
                            .unwrap_or(fallback_point);
                        let style = this.cached_style_for_layout(layout);
                        if let Some(delta) =
                            browser_wheel_row_delta(layout, &this.model, point, &style, delta)
                        {
                            let current_view_start = this
                                .shell_state
                                .browser_view_start_visible_row(layout, &this.model)
                                .or(this.model.browser.selected_visible_row)
                                .unwrap_or(0);
                            if let Some(visible_row) = browser_view_start_after_wheel(
                                current_view_start,
                                this.model.browser.visible_count,
                                delta,
                            ) {
                                let _ = this.process_wheel_rows_immediately(visible_row);
                            }
                        }
                    }
                });
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = match event.physical_key {
                    PhysicalKey::Code(code) => key_code_from_winit(code),
                    _ => None,
                };
                let allow_repeat =
                    event.repeat && key.is_some_and(|key| self.allows_key_repeat(key));
                if event.state == ElementState::Pressed && (!event.repeat || allow_repeat) {
                    let mut handled = false;
                    if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
                        if self.model.confirm_prompt.visible {
                            self.emit_model_action(UiAction::CancelPrompt);
                            self.deactivate_text_input_target();
                            handled = true;
                        } else if self.text_input_target != TextInputTarget::None {
                            self.deactivate_text_input_target();
                            handled = true;
                        } else {
                            let action = UiAction::HandleEscape;
                            self.update_text_target_after_action(&action);
                            self.emit_model_action(action);
                            handled = true;
                        }
                    }
                    if !handled && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
                        handled = self.backspace_text();
                    }
                    if !handled && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
                        handled = self.delete_text_forward();
                    }
                    if !handled
                        && matches!(event.logical_key, Key::Named(NamedKey::Enter))
                        && matches!(
                            self.text_input_target,
                            TextInputTarget::BrowserSearch
                                | TextInputTarget::FolderSearch
                                | TextInputTarget::WaveformBpm
                        )
                    {
                        self.deactivate_text_input_target();
                        handled = true;
                    }
                    if !handled && let Some(key) = key {
                        handled = match key {
                            KeyCode::ArrowUp => {
                                self.step_waveform_bpm_input(if self.modifiers.shift_key() {
                                    1
                                } else {
                                    10
                                })
                            }
                            KeyCode::ArrowDown => {
                                self.step_waveform_bpm_input(if self.modifiers.shift_key() {
                                    -1
                                } else {
                                    -10
                                })
                            }
                            _ => false,
                        };
                    }
                    if !handled
                        && self.text_input_target != TextInputTarget::None
                        && let Some(key) = key
                    {
                        handled = self.move_text_cursor(key, self.modifiers.shift_key());
                    }
                    if !handled
                        && self.text_input_target != TextInputTarget::None
                        && (self.modifiers.control_key() || self.modifiers.super_key())
                        && !self.modifiers.alt_key()
                        && let Some(key) = key
                    {
                        handled = match key {
                            KeyCode::A => self.select_all_text(),
                            KeyCode::C => self.copy_selected_text(),
                            KeyCode::V => self.paste_text(),
                            KeyCode::X => self.cut_selected_text(),
                            _ => false,
                        };
                    }
                    if !handled
                        && self.text_input_target != TextInputTarget::None
                        && !self.modifiers.control_key()
                        && !self.modifiers.super_key()
                        && !self.modifiers.alt_key()
                        && let Some(text) = event.text.as_ref()
                    {
                        let appended: String = text.chars().filter(|ch| !ch.is_control()).collect();
                        if !appended.is_empty() {
                            handled = self.append_text(&appended);
                        }
                    }
                    if !handled
                        && self.text_input_target == TextInputTarget::None
                        && let Some(key) = key
                    {
                        handled = if self.model.confirm_prompt.visible {
                            false
                        } else {
                            self.shell_state.handle_key(key)
                        };
                        if let Some(action) = action_from_key(key, self.modifiers, &self.model) {
                            self.update_text_target_after_action(&action);
                            self.emit_model_action(action);
                            handled = true;
                        }
                    }
                    if handled {
                        if !self.frame_state.has_pending_rebuild() {
                            self.apply_invalidation_scope(
                                RuntimeInvalidationScope::OverlayStateOnly,
                            );
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.repaint_event_pending.store(false, Ordering::Release);
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let has_pending_input = self.flush_pending_input();
        let needs_animation = self.shell_state.needs_animation();
        let now = Instant::now();
        self.maybe_force_reveal_startup_window_on_stall(now);
        let cursor_activity_redraw_deadline = if !needs_animation && !has_pending_input {
            self.next_cursor_activity_redraw_deadline(now)
        } else {
            None
        };
        let should_refresh_idle_status =
            !needs_animation && !has_pending_input && { self.mark_idle_status_refresh_if_due(now) };
        if needs_animation || has_pending_input || cursor_activity_redraw_deadline.is_some() {
            self.request_redraw_if_needed();
            let mut next_redraw_at = if let Some(deadline) = cursor_activity_redraw_deadline {
                deadline
            } else {
                let frame_interval = if self.shell_state.is_transport_running() {
                    self.target_frame_interval
                } else {
                    self.focus_animation_interval
                };
                self.last_redraw + frame_interval
            };
            if next_redraw_at < now {
                next_redraw_at = now;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw_at));
            return;
        }
        if should_refresh_idle_status {
            self.request_redraw_if_needed();
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
            return;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
    }
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
/// The runtime loop is owned by winit and blocks until the native window closes.
/// The host receives user input each frame through the bridge-driven action path,
/// and this function returns the host result from the event loop invocation.
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

#[cfg(test)]
mod tests;
