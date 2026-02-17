//! Native `winit + vello` runtime preview used for backend selection rollout.

use super::{NativeRunOptions, WindowIconRgba};
use crate::app::{AppModel, FrameBuildResult, NativeAppBridge, NativeMotionModel, UiAction};
use crate::gui::{
    input::{KeyCode, key_code_from_winit},
    native_shell::{
        NativeShellState, NativeViewFrame, Primitive, ShellLayout, ShellNodeKind, StyleTokens,
        TextAlign, TextRun,
    },
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Circle, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};
use std::panic::AssertUnwindSafe;
use tracing::{error, info, warn};

const REDRAW_PROFILE_INTERVAL_FRAMES: u64 = 240;
const REDRAW_PROFILE_ENV: &str = "SEMPAL_NATIVE_RENDER_PROFILE";
const FOCUS_PULSE_HZ: u64 = 60;
const IDLE_STATUS_REFRESH_HZ: u64 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    FolderSearch,
    PromptInput,
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
    /// Input/model changes need at least static scene refresh.
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

    /// Mark static content and all overlays dirty.
    fn mark_scene_dirty(&mut self) {
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

struct NativeVelloRunner<B: NativeAppBridge> {
    options: NativeRunOptions,
    bridge: B,
    model: AppModel,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    redraw_requested: bool,
    /// Retained static scene primitives (layout and stable content).
    frame_cache: NativeViewFrame,
    /// Retained state-driven overlay primitives (focus/hover and dialog state).
    state_overlay_frame_cache: NativeViewFrame,
    /// Retained motion-driven overlay primitives (lamp pulse/playhead/update).
    motion_overlay_frame_cache: NativeViewFrame,
    /// Full scene sent to Vello after combining static + overlay scenes.
    scene: Scene,
    /// Cached encoded static scene.
    static_scene: Scene,
    /// Cached encoded state-driven overlay scene.
    state_overlay_scene: Scene,
    /// Cached encoded motion-driven overlay scene.
    motion_overlay_scene: Scene,
    /// Cached latest motion-only model for lightweight overlay rebuilds.
    motion_model: Option<NativeMotionModel>,
    /// Whether the active bridge supports `pull_motion_model`.
    motion_model_supported: bool,
    text_renderer: NativeTextRenderer,
    style_cache: Option<StyleTokens>,
    frame_state: NativeVelloFrameState,
    shell_layout: Option<ShellLayout>,
    shell_state: NativeShellState,
    clear_color: Rgba8,
    last_cursor: Option<Point>,
    pending_cursor: Option<Point>,
    pending_wheel_rows_delta: i32,
    modifiers: ModifiersState,
    text_input_target: TextInputTarget,
    last_redraw: Instant,
    resumed_count: u32,
    window_event_count: u32,
    redraw_count: u32,
    target_frame_interval: Duration,
    focus_animation_interval: Duration,
    idle_status_refresh_interval: Duration,
    next_idle_status_refresh: Instant,
    model_refresh_count: u32,
    profile_redraw_enabled: bool,
    profile_redraw_frames: u64,
    profile_redraw_rebuild_ns: u128,
    profile_redraw_acquire_ns: u128,
    profile_redraw_render_ns: u128,
    profile_redraw_blit_ns: u128,
    profile_redraw_present_ns: u128,
    profile_redraw_total_ns: u128,
    profile_redraw_scene_rebuilds: u64,
    profile_redraw_state_overlay_rebuilds: u64,
    profile_redraw_motion_overlay_rebuilds: u64,
    profile_model_refreshes: u64,
    profile_redraw_model_pull_ns: u128,
    profile_redraw_motion_pull_ns: u128,
    profile_redraw_tick_ns: u128,
    profile_redraw_build_static_ns: u128,
    profile_redraw_build_state_overlay_ns: u128,
    profile_redraw_build_motion_overlay_ns: u128,
    profile_redraw_encode_static_ns: u128,
    profile_redraw_encode_state_overlay_ns: u128,
    profile_redraw_encode_motion_overlay_ns: u128,
    profile_redraw_motion_overlay_skips: u64,
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn new(options: NativeRunOptions, bridge: B) -> Self {
        let target_fps = options.target_fps.max(1);
        let frame_interval_ns = (1_000_000_000u64 / target_fps as u64).max(1);
        let target_frame_interval = Duration::from_nanos(frame_interval_ns);
        let focus_animation_interval =
            Duration::from_nanos((1_000_000_000u64 / FOCUS_PULSE_HZ).max(1));
        let idle_status_refresh_interval =
            Duration::from_nanos(1_000_000_000u64 / IDLE_STATUS_REFRESH_HZ.max(1));
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
            model: AppModel::default(),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            redraw_requested: false,
            frame_cache: NativeViewFrame {
                clear_color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            state_overlay_frame_cache: NativeViewFrame {
                clear_color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            motion_overlay_frame_cache: NativeViewFrame {
                clear_color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            scene: Scene::new(),
            static_scene: Scene::new(),
            state_overlay_scene: Scene::new(),
            motion_overlay_scene: Scene::new(),
            motion_model: None,
            motion_model_supported: true,
            text_renderer: NativeTextRenderer::new(),
            style_cache: None,
            frame_state: NativeVelloFrameState {
                model_dirty: true,
                ..NativeVelloFrameState::default()
            },
            shell_layout: None,
            shell_state: NativeShellState::new(),
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            last_cursor: None,
            pending_cursor: None,
            pending_wheel_rows_delta: 0,
            modifiers: ModifiersState::default(),
            text_input_target: TextInputTarget::None,
            last_redraw: Instant::now(),
            resumed_count: 0,
            window_event_count: 0,
            redraw_count: 0,
            target_frame_interval,
            focus_animation_interval,
            idle_status_refresh_interval,
            next_idle_status_refresh: Instant::now() + idle_status_refresh_interval,
            model_refresh_count: 0,
            profile_redraw_enabled: std::env::var(REDRAW_PROFILE_ENV)
                .ok()
                .is_some_and(|value| {
                    matches!(value.as_str(), "1" | "true" | "TRUE" | "on" | "On" | "ON" | "yes")
                }),
            profile_redraw_frames: 0,
            profile_redraw_rebuild_ns: 0,
            profile_redraw_acquire_ns: 0,
            profile_redraw_render_ns: 0,
            profile_redraw_blit_ns: 0,
            profile_redraw_present_ns: 0,
            profile_redraw_total_ns: 0,
            profile_redraw_scene_rebuilds: 0,
            profile_redraw_state_overlay_rebuilds: 0,
            profile_redraw_motion_overlay_rebuilds: 0,
            profile_model_refreshes: 0,
            profile_redraw_model_pull_ns: 0,
            profile_redraw_motion_pull_ns: 0,
            profile_redraw_tick_ns: 0,
            profile_redraw_build_static_ns: 0,
            profile_redraw_build_state_overlay_ns: 0,
            profile_redraw_build_motion_overlay_ns: 0,
            profile_redraw_encode_static_ns: 0,
            profile_redraw_encode_state_overlay_ns: 0,
            profile_redraw_encode_motion_overlay_ns: 0,
            profile_redraw_motion_overlay_skips: 0,
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
            .with_maximized(self.options.maximized);
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
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!("radiant native vello: failed to create window: {:?}", err);
                event_loop.exit();
                return;
            }
        };
        info!("radiant native vello: window created");
        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        info!(
            "radiant native vello: creating render surface with {}x{}",
            width, height
        );
        let preferred_present_mode = if self.options.target_fps >= 120 {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::AutoVsync
        };
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
        let render_surface = match create_surface_with_mode(preferred_present_mode) {
            Ok(Ok(surface)) => {
                info!(
                    "radiant native vello: render surface created using {:?}",
                    preferred_present_mode
                );
                surface
            }
            Ok(Err(preferred_err)) => {
                if preferred_present_mode == wgpu::PresentMode::AutoVsync {
                    error!(
                        "radiant native vello: failed to create primary surface: {:?}",
                        preferred_err
                    );
                    event_loop.exit();
                    return;
                }
                warn!(
                    "radiant native vello: mailbox surface creation failed (error): {:?}",
                    preferred_err
                );
                warn!("radiant native vello: retrying AutoVsync render surface");
                match create_surface_with_mode(wgpu::PresentMode::AutoVsync) {
                    Ok(Ok(surface)) => {
                        info!(
                            "radiant native vello: render surface created using AutoVsync fallback"
                        );
                        surface
                    }
                    Ok(Err(fallback_err)) => {
                        error!(
                            "radiant native vello: failed to create AutoVsync fallback surface: {:?}",
                            fallback_err
                        );
                        event_loop.exit();
                        return;
                    }
                    Err(_) => {
                        error!(
                            "radiant native vello: AutoVsync surface creation panicked during startup"
                        );
                        event_loop.exit();
                        return;
                    }
                }
            }
            Err(_) => {
                if preferred_present_mode == wgpu::PresentMode::AutoVsync {
                    error!(
                        "radiant native vello: panicked during AutoVsync surface creation"
                    );
                    event_loop.exit();
                    return;
                }
                warn!("radiant native vello: mailbox surface creation panicked; retrying AutoVsync");
                match create_surface_with_mode(wgpu::PresentMode::AutoVsync) {
                    Ok(Ok(surface)) => {
                        info!(
                            "radiant native vello: render surface created using AutoVsync fallback"
                        );
                        surface
                    }
                    Ok(Err(fallback_err)) => {
                        error!(
                            "radiant native vello: failed to create AutoVsync fallback surface: {:?}",
                            fallback_err
                        );
                        event_loop.exit();
                        return;
                    }
                    Err(_) => {
                        error!(
                            "radiant native vello: AutoVsync fallback panicked during startup"
                        );
                        event_loop.exit();
                        return;
                    }
                }
            }
        };
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
        info!("radiant native vello: renderer created");

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.frame_state.mark_layout_dirty();
        self.rebuild_scene_if_needed();
        self.last_redraw = Instant::now();
    }

    fn rebuild_layout(&mut self) {
        let Some(surface) = self.render_surface.as_ref() else {
            return;
        };

        let viewport = Vector2::new(surface.config.width as f32, surface.config.height as f32);
        let style = StyleTokens::for_viewport_with_scale(viewport.x, self.ui_scale_factor());
        self.style_cache = Some(style);
        self.shell_layout = Some(ShellLayout::build_with_style(viewport, &style));
        self.frame_state.clear_layout_dirty();
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

    fn rebuild_scene_if_needed(&mut self) {
        if self.shell_layout.is_none() || self.frame_state.layout_dirty {
            self.rebuild_layout();
        }
        let rebuild_static = self.frame_state.take_scene() || self.frame_state.take_model();
        let rebuild_state_overlay = self.frame_state.take_state_overlay() || rebuild_static;
        let rebuild_motion_overlay = self.frame_state.take_motion_overlay() || rebuild_static;
        if !rebuild_static && !rebuild_state_overlay && !rebuild_motion_overlay {
            return;
        }
        self.rebuild_scene(
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
        );
    }

    fn rebuild_scene_and_request_redraw(&mut self) {
        self.frame_state.mark_scene_dirty();
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

    fn rebuild_scene_for_redraw(&mut self, needs_animation: bool, delta_seconds: f32) -> bool {
        let profiling = self.profile_redraw_enabled;
        if !needs_animation {
            if self.frame_state.has_pending_rebuild() {
                self.rebuild_scene_if_needed();
                return true;
            }
            return false;
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let tick_start = profiling.then(Instant::now);
        let style = self.cached_style_for_layout(layout);
        self.shell_state.tick_with_style(delta_seconds, &style);
        self.rebuild_scene_for_tick();
        if profiling {
            let tick_duration = tick_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profile_redraw_tick_ns =
                self.profile_redraw_tick_ns.saturating_add(tick_duration.as_nanos());
        }
        true
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
        if !self.profile_redraw_enabled {
            return;
        }
        self.profile_redraw_frames = self.profile_redraw_frames.saturating_add(1);
        self.profile_redraw_rebuild_ns = self
            .profile_redraw_rebuild_ns
            .saturating_add(rebuild.as_nanos());
        self.profile_redraw_acquire_ns = self
            .profile_redraw_acquire_ns
            .saturating_add(acquire.as_nanos());
        self.profile_redraw_render_ns = self
            .profile_redraw_render_ns
            .saturating_add(render.as_nanos());
        self.profile_redraw_blit_ns = self
            .profile_redraw_blit_ns
            .saturating_add(blit.as_nanos());
        self.profile_redraw_present_ns = self
            .profile_redraw_present_ns
            .saturating_add(present.as_nanos());
        self.profile_redraw_total_ns = self
            .profile_redraw_total_ns
            .saturating_add(total.as_nanos());
        if self.profile_redraw_frames < REDRAW_PROFILE_INTERVAL_FRAMES {
            return;
        }

        let frames = self.profile_redraw_frames as f64;
        let total_ns = self.profile_redraw_total_ns as f64;
        if total_ns <= 0.0 {
            return;
        }

        let ms = |value_ns: u128| value_ns as f64 / 1_000_000.0;
        let avg_total_ms = ms(self.profile_redraw_total_ns) / frames;
        let avg_rebuild_ms = ms(self.profile_redraw_rebuild_ns) / frames;
        let avg_acquire_ms = ms(self.profile_redraw_acquire_ns) / frames;
        let avg_render_ms = ms(self.profile_redraw_render_ns) / frames;
        let avg_blit_ms = ms(self.profile_redraw_blit_ns) / frames;
        let avg_present_ms = ms(self.profile_redraw_present_ns) / frames;
        let avg_model_pull_ms = ms(self.profile_redraw_model_pull_ns) / frames;
        let avg_motion_pull_ms = ms(self.profile_redraw_motion_pull_ns) / frames;
        let avg_tick_ms = ms(self.profile_redraw_tick_ns) / frames;
        let avg_build_static_ms = ms(self.profile_redraw_build_static_ns) / frames;
        let avg_build_state_overlay_ms = ms(self.profile_redraw_build_state_overlay_ns) / frames;
        let avg_build_motion_overlay_ms = ms(self.profile_redraw_build_motion_overlay_ns) / frames;
        let avg_encode_static_ms = ms(self.profile_redraw_encode_static_ns) / frames;
        let avg_encode_state_overlay_ms = ms(self.profile_redraw_encode_state_overlay_ns) / frames;
        let avg_encode_motion_overlay_ms = ms(self.profile_redraw_encode_motion_overlay_ns) / frames;
        let fps = 1000.0 / avg_total_ms.max(0.001);
        let rebuild_pct = (self.profile_redraw_rebuild_ns as f64) * 100.0 / total_ns;
        let acquire_pct = (self.profile_redraw_acquire_ns as f64) * 100.0 / total_ns;
        let render_pct = (self.profile_redraw_render_ns as f64) * 100.0 / total_ns;
        let blit_pct = (self.profile_redraw_blit_ns as f64) * 100.0 / total_ns;
        let present_pct = (self.profile_redraw_present_ns as f64) * 100.0 / total_ns;
        let model_refresh_avg = self.profile_model_refreshes as f64 / frames;
        let scene_rebuild_avg = self.profile_redraw_scene_rebuilds as f64 / frames;
        let state_overlay_rebuild_avg = self.profile_redraw_state_overlay_rebuilds as f64 / frames;
        let motion_overlay_rebuild_avg = self.profile_redraw_motion_overlay_rebuilds as f64 / frames;
        let motion_overlay_skip_avg = self.profile_redraw_motion_overlay_skips as f64 / frames;
        let (text_hits, text_misses, text_evictions) = self.text_renderer.take_layout_profile_counters();
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
        eprintln!(
            "[native-vello] redraw avg over {REDRAW_PROFILE_INTERVAL_FRAMES} frames: \
             total={avg_total_ms:.2}ms ({fps:.1} fps) rebuild={avg_rebuild_ms:.2}ms ({rebuild_pct:.1}%) \
             acquire={avg_acquire_ms:.2}ms ({acquire_pct:.1}%) render={avg_render_ms:.2}ms ({render_pct:.1}%) \
             blit={avg_blit_ms:.2}ms ({blit_pct:.1}%) present={avg_present_ms:.2}ms ({present_pct:.1}%) \
             model_refresh_avg={model_refresh_avg:.2} scene_rebuild_avg={scene_rebuild_avg:.2} \
             state_overlay_rebuild_avg={state_overlay_rebuild_avg:.2} \
             motion_overlay_rebuild_avg={motion_overlay_rebuild_avg:.2} motion_overlay_skip_avg={motion_overlay_skip_avg:.2} \
             model_pull_ms={avg_model_pull_ms:.3} motion_pull_ms={avg_motion_pull_ms:.3} \
             tick_ms={avg_tick_ms:.3} build_static_ms={avg_build_static_ms:.3} \
             build_state_overlay_ms={avg_build_state_overlay_ms:.3} build_motion_overlay_ms={avg_build_motion_overlay_ms:.3} \
             encode_static_ms={avg_encode_static_ms:.3} encode_state_overlay_ms={avg_encode_state_overlay_ms:.3} \
             encode_motion_overlay_ms={avg_encode_motion_overlay_ms:.3} \
             text_layout_hits={text_hits} text_layout_misses={text_misses} text_layout_evictions={text_evictions} \
             text_hit_rate={text_cache_hit_rate:.1}% text_miss_rate={text_cache_miss_rate:.1}%"
        );
        self.profile_redraw_frames = 0;
        self.profile_redraw_rebuild_ns = 0;
        self.profile_redraw_acquire_ns = 0;
        self.profile_redraw_render_ns = 0;
        self.profile_redraw_blit_ns = 0;
        self.profile_redraw_present_ns = 0;
        self.profile_redraw_total_ns = 0;
        self.profile_redraw_scene_rebuilds = 0;
        self.profile_redraw_state_overlay_rebuilds = 0;
        self.profile_redraw_motion_overlay_rebuilds = 0;
        self.profile_model_refreshes = 0;
        self.profile_redraw_model_pull_ns = 0;
        self.profile_redraw_motion_pull_ns = 0;
        self.profile_redraw_tick_ns = 0;
        self.profile_redraw_build_static_ns = 0;
        self.profile_redraw_build_state_overlay_ns = 0;
        self.profile_redraw_build_motion_overlay_ns = 0;
        self.profile_redraw_encode_static_ns = 0;
        self.profile_redraw_encode_state_overlay_ns = 0;
        self.profile_redraw_encode_motion_overlay_ns = 0;
        self.profile_redraw_motion_overlay_skips = 0;
    }

    fn encode_frame_to_scene(
        frame: &NativeViewFrame,
        scene: &mut Scene,
        text_renderer: &mut NativeTextRenderer,
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
            }
        }
        text_renderer.draw_text_runs(scene, &frame.text_runs);
    }

    fn queue_cursor(&mut self, point: Point) {
        self.pending_cursor = Some(point);
    }

    fn queue_wheel_rows(&mut self, steps: i8) {
        if steps == 0 {
            return;
        }
        self.pending_wheel_rows_delta =
            self.pending_wheel_rows_delta
                .saturating_add(steps as i32);
    }

    fn flush_pending_input(&mut self) -> bool {
        let mut pending_action = false;
                if let Some(point) = self.pending_cursor.take() {
                    if let Some(layout) = self.shell_layout.as_ref() {
                        if self
                            .shell_state
                            .handle_cursor_move(&layout, &self.model, point)
                        {
                            self.rebuild_overlay_and_request_redraw();
                            pending_action = true;
                        }
                    }
                }

        if self.pending_wheel_rows_delta != 0 {
            let steps = self
                .pending_wheel_rows_delta
                .clamp(i8::MIN as i32, i8::MAX as i32) as i8;
            self.pending_wheel_rows_delta = 0;
            self.emit_model_action(UiAction::MoveBrowserFocus { delta: steps });
            self.rebuild_scene_and_request_redraw();
            pending_action = true;
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

    fn rebuild_scene(
        &mut self,
        mut rebuild_static: bool,
        mut rebuild_state_overlay: bool,
        mut rebuild_motion_overlay: bool,
    ) {
        let profiling = self.profile_redraw_enabled;
        let should_refresh_model = self.frame_state.take_model()
            || rebuild_static
            || rebuild_state_overlay
            || (!self.motion_model_supported && rebuild_motion_overlay);
        let should_refresh_motion = rebuild_motion_overlay && self.motion_model_supported;
        if rebuild_static {
            self.profile_redraw_scene_rebuilds = self
                .profile_redraw_scene_rebuilds
                .saturating_add(1);
        }
        if rebuild_state_overlay {
            self.profile_redraw_state_overlay_rebuilds = self
                .profile_redraw_state_overlay_rebuilds
                .saturating_add(1);
        }
        if rebuild_motion_overlay {
            self.profile_redraw_motion_overlay_rebuilds = self
                .profile_redraw_motion_overlay_rebuilds
                .saturating_add(1);
        }
        let previous_waveform_signature =
            self.motion_model
                .as_ref()
                .and_then(|model| model.waveform_image_signature);
        let mut motion_changed = false;
        if should_refresh_model {
            let pull_start = profiling.then(Instant::now);
            self.profile_model_refreshes = self.profile_model_refreshes.saturating_add(1);
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
            self.model = self.bridge.pull_model();
            if profiling {
                let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_model_pull_ns =
                    self.profile_redraw_model_pull_ns.saturating_add(pull_duration.as_nanos());
            }
            self.shell_state.sync_from_model(&self.model);
            self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
            self.motion_model_supported = true;
            self.sync_text_input_target();
        } else if should_refresh_motion {
            let pull_start = profiling.then(Instant::now);
            if let Some(motion_model) = self.bridge.pull_motion_model() {
                if profiling {
                    let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                    self.profile_redraw_motion_pull_ns =
                        self.profile_redraw_motion_pull_ns
                            .saturating_add(pull_duration.as_nanos());
                }
                if self.motion_model.as_ref() != Some(&motion_model) {
                    motion_changed = true;
                    if previous_waveform_signature != motion_model.waveform_image_signature {
                        rebuild_static = true;
                        rebuild_state_overlay = true;
                        rebuild_motion_overlay = true;
                    }
                    self.shell_state.sync_from_motion_model(&motion_model);
                    self.motion_model = Some(motion_model);
                }
            } else {
                if profiling {
                    let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                    self.profile_redraw_motion_pull_ns =
                        self.profile_redraw_motion_pull_ns
                            .saturating_add(pull_duration.as_nanos());
                }
                let model_pull_start = profiling.then(Instant::now);
                self.motion_model_supported = false;
                self.model = self.bridge.pull_model();
                if profiling {
                    let model_pull_duration =
                        model_pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                    self.profile_redraw_model_pull_ns =
                        self.profile_redraw_model_pull_ns
                            .saturating_add(model_pull_duration.as_nanos());
                }
                self.shell_state.sync_from_model(&self.model);
                self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
                self.sync_text_input_target();
            }
        }
        if should_refresh_motion && !motion_changed {
            if profiling {
                self.profile_redraw_motion_overlay_skips =
                    self.profile_redraw_motion_overlay_skips.saturating_add(1);
            }
            rebuild_motion_overlay = false;
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return;
        };
        let style = self.cached_style_for_layout(layout);
        if rebuild_static {
            let build_start = profiling.then(Instant::now);
            self.shell_state.build_frame_with_style_into_static(
                layout,
                &style,
                &self.model,
                &mut self.frame_cache,
            );
            if profiling {
                let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_build_static_ns =
                    self.profile_redraw_build_static_ns.saturating_add(build_duration.as_nanos());
            }
            self.clear_color = self.frame_cache.clear_color;
            let encode_start = profiling.then(Instant::now);
            Self::encode_frame_to_scene(
                &self.frame_cache,
                &mut self.static_scene,
                &mut self.text_renderer,
            );
            if profiling {
                let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_encode_static_ns =
                    self.profile_redraw_encode_static_ns.saturating_add(encode_duration.as_nanos());
            }
        }
        if rebuild_state_overlay {
            let build_start = profiling.then(Instant::now);
            self.shell_state.build_state_overlay_into(
                layout,
                &style,
                &self.model,
                &mut self.state_overlay_frame_cache,
            );
            if profiling {
                let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_build_state_overlay_ns = self
                    .profile_redraw_build_state_overlay_ns
                    .saturating_add(build_duration.as_nanos());
            }
            let encode_start = profiling.then(Instant::now);
            Self::encode_frame_to_scene(
                &self.state_overlay_frame_cache,
                &mut self.state_overlay_scene,
                &mut self.text_renderer,
            );
            if profiling {
                let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_encode_state_overlay_ns = self
                    .profile_redraw_encode_state_overlay_ns
                    .saturating_add(encode_duration.as_nanos());
            }
        }
        if rebuild_motion_overlay {
            let build_start = profiling.then(Instant::now);
            let motion_model = if let Some(motion_model) = self.motion_model.as_ref() {
                motion_model
            } else {
                self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
                self.motion_model.as_ref().unwrap()
            };
            self.shell_state.build_motion_overlay_into(
                layout,
                &style,
                motion_model,
                &mut self.motion_overlay_frame_cache,
            );
            if profiling {
                let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_build_motion_overlay_ns = self
                    .profile_redraw_build_motion_overlay_ns
                    .saturating_add(build_duration.as_nanos());
            }
            let encode_start = profiling.then(Instant::now);
            Self::encode_frame_to_scene(
                &self.motion_overlay_frame_cache,
                &mut self.motion_overlay_scene,
                &mut self.text_renderer,
            );
            if profiling {
                let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profile_redraw_encode_motion_overlay_ns = self
                    .profile_redraw_encode_motion_overlay_ns
                    .saturating_add(encode_duration.as_nanos());
            }
        }
        if rebuild_static || rebuild_state_overlay || rebuild_motion_overlay {
            self.scene.reset();
            self.scene.append(&self.static_scene, None);
            self.scene.append(&self.state_overlay_scene, None);
            self.scene.append(&self.motion_overlay_scene, None);
        }
        let frame_result = FrameBuildResult {
            primitive_count: self
                .frame_cache
                .primitives
                .len()
                .saturating_add(self.state_overlay_frame_cache.primitives.len())
                .saturating_add(self.motion_overlay_frame_cache.primitives.len()),
            text_run_count: self
                .frame_cache
                .text_runs
                .len()
                .saturating_add(self.state_overlay_frame_cache.text_runs.len())
                .saturating_add(self.motion_overlay_frame_cache.text_runs.len()),
            needs_animation: self.shell_state.needs_animation(),
        };
        self.bridge.on_frame_result(frame_result);
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_count = self.redraw_count.saturating_add(1);
        self.redraw_requested = false;
        let now = Instant::now();
        let delta = (now - self.last_redraw).as_secs_f32();
        self.last_redraw = now;
        let profiling = self.profile_redraw_enabled;
        let frame_start = profiling.then(Instant::now);
        let rebuild_start = profiling.then(Instant::now);
        let needs_animation = self.shell_state.needs_animation();
        let has_rebuild = self.rebuild_scene_for_redraw(needs_animation, delta);
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
        if !needs_animation && !has_rebuild {
            return;
        }

        let Some(window) = self.window.as_ref().cloned() else {
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        };

        let mut surface_error = None;
        let mut needs_resize = false;
        let mut out_of_memory = false;
        let acquire_start = profiling.then(Instant::now);
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                if let Some(frame_start) = frame_start {
                    self.maybe_record_redraw_profile(
                        rebuild_duration,
                        Duration::ZERO,
                        Duration::ZERO,
                        Duration::ZERO,
                        Duration::ZERO,
                        frame_start.elapsed(),
                    );
                }
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
                            let Some(render_ctx) = self.render_ctx.as_mut() else {
                                return;
                            };
                            render_ctx.resize_surface(surface, size.width.max(1), size.height.max(1));
                            needs_resize = true;
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
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    acquire_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            if matches!(err, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) && needs_resize {
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.rebuild_scene_if_needed();
                self.request_redraw_if_needed();
            }
            if out_of_memory {
                event_loop.exit();
            }
            return;
        }
        let Some(surface_texture) = surface_texture else {
            return;
        };

        let Some(surface) = self.render_surface.as_mut() else {
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_start = profiling.then(Instant::now);
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
            if let Some(frame_start) = frame_start {
                self.maybe_record_redraw_profile(
                    rebuild_duration,
                    acquire_duration,
                    render,
                    Duration::ZERO,
                    Duration::ZERO,
                    frame_start.elapsed(),
                );
            }
            return;
        }
        let render_duration = render_start.map_or(Duration::ZERO, |start| start.elapsed());
        let blit_start = profiling.then(Instant::now);
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
        let present_start = profiling.then(Instant::now);
        surface_texture.present();
        let present_duration = present_start.map_or(Duration::ZERO, |start| start.elapsed());
        if let Some(frame_start) = frame_start {
            self.maybe_record_redraw_profile(
                rebuild_duration,
                acquire_duration,
                render_duration,
                blit_duration,
                present_duration,
                frame_start.elapsed(),
            );
        }
    }

    fn sync_text_input_target(&mut self) {
        if self.model.confirm_prompt.visible && self.model.confirm_prompt.input_value.is_some() {
            self.text_input_target = TextInputTarget::PromptInput;
        } else if self.text_input_target == TextInputTarget::PromptInput {
            self.text_input_target = TextInputTarget::None;
        }
    }

    fn current_text_value(&self) -> Option<String> {
        match self.text_input_target {
            TextInputTarget::None => None,
            TextInputTarget::BrowserSearch => Some(self.model.browser.search_query.clone()),
            TextInputTarget::FolderSearch => Some(self.model.sources.folder_search_query.clone()),
            TextInputTarget::PromptInput => self.model.confirm_prompt.input_value.clone(),
        }
    }

    fn set_text_value(&mut self, value: String) -> bool {
        let action = match self.text_input_target {
            TextInputTarget::None => return false,
            TextInputTarget::BrowserSearch => UiAction::SetBrowserSearch { query: value },
            TextInputTarget::FolderSearch => UiAction::SetFolderSearch { query: value },
            TextInputTarget::PromptInput => UiAction::SetPromptInput { value },
        };
        self.emit_model_action(action);
        true
    }

    fn append_text(&mut self, appended: &str) -> bool {
        if appended.is_empty() {
            return false;
        }
        let Some(mut value) = self.current_text_value() else {
            return false;
        };
        value.push_str(appended);
        self.set_text_value(value)
    }

    fn emit_model_action(&mut self, action: UiAction) {
        self.frame_state.mark_model_dirty();
        self.bridge.on_action(action);
    }

    fn backspace_text(&mut self) -> bool {
        let Some(mut value) = self.current_text_value() else {
            return false;
        };
        if value.pop().is_none() {
            return false;
        }
        self.set_text_value(value)
    }

    fn update_text_target_after_action(&mut self, action: &UiAction) {
        match action {
            UiAction::FocusBrowserSearch => self.text_input_target = TextInputTarget::BrowserSearch,
            UiAction::FocusFolderSearch => self.text_input_target = TextInputTarget::FolderSearch,
            UiAction::ConfirmPrompt | UiAction::CancelPrompt => {
                self.text_input_target = TextInputTarget::None;
            }
            _ => {}
        }
    }
}

impl<B: NativeAppBridge> ApplicationHandler for NativeVelloRunner<B> {
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
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.rebuild_scene_and_request_redraw();
            }
            WindowEvent::Resized(size) => {
                if self.window_event_count <= 30 && (size.width == 0 || size.height == 0) {
                    warn!(
                        width = size.width,
                        height = size.height,
                        "radiant native vello received zero-size resize"
                    );
                }
                let window = self.window.as_ref().cloned();
                if size.width > 0
                    && size.height > 0
                    && let (Some(render_ctx), Some(surface), Some(_window)) = (
                        self.render_ctx.as_ref(),
                        self.render_surface.as_mut(),
                        window,
                    )
                {
                    render_ctx.resize_surface(surface, size.width, size.height);
                    self.frame_state.mark_layout_dirty();
                    self.frame_state.mark_model_dirty();
                    self.rebuild_scene_and_request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x as f32, position.y as f32);
                if self.last_cursor == Some(point) {
                    return;
                }
                self.last_cursor = Some(point);
                self.queue_cursor(point);
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let _window = self.window.as_ref().cloned();
                if let (Some(point), Some(layout), Some(_window)) =
                    (self.last_cursor, self.shell_layout.as_ref(), _window)
                {
                    self.text_input_target = TextInputTarget::None;
                    let mut handled = false;
                    if self
                        .shell_state
                        .prompt_input_at_point(layout, &self.model, point)
                    {
                        self.text_input_target = TextInputTarget::PromptInput;
                        handled = true;
                    } else if let Some(action) = action_from_pointer(
                        layout,
                        &self.model,
                        &mut self.shell_state,
                        point,
                        self.modifiers,
                    ) {
                        self.update_text_target_after_action(&action);
                        self.emit_model_action(action);
                        handled = true;
                    } else if self.shell_state.handle_primary_click(layout, point)
                        && let Some(column) = layout.column_at_point(point)
                    {
                        self.emit_model_action(UiAction::SelectColumn { index: column });
                        handled = true;
                    }
                    if handled {
                        self.rebuild_scene_and_request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(layout) = self.shell_layout.as_ref() {
                    let fallback_point = Point::new(
                        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
                        (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
                    );
                    let point = self
                        .last_cursor
                        .filter(|point| layout.browser_panel.contains(*point))
                        .unwrap_or(fallback_point);
                    let style = self.cached_style_for_layout(layout);
                    if let Some(delta) = browser_wheel_row_delta(layout, &self.model, point, &style, delta) {
                        self.queue_wheel_rows(delta);
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    let mut handled = false;
                    if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
                        if self.model.confirm_prompt.visible {
                            self.emit_model_action(UiAction::CancelPrompt);
                            self.text_input_target = TextInputTarget::None;
                            handled = true;
                        } else if self.text_input_target != TextInputTarget::None {
                            self.text_input_target = TextInputTarget::None;
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
                    if !handled
                        && matches!(event.logical_key, Key::Named(NamedKey::Enter))
                        && matches!(
                            self.text_input_target,
                            TextInputTarget::BrowserSearch | TextInputTarget::FolderSearch
                        )
                    {
                        self.text_input_target = TextInputTarget::None;
                        handled = true;
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
                        && let PhysicalKey::Code(code) = event.physical_key
                        && let Some(key) = key_code_from_winit(code)
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
                        self.rebuild_scene_and_request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let has_pending_input = self.flush_pending_input();
        let needs_animation = self.shell_state.needs_animation();
        let now = Instant::now();
        let should_refresh_idle_status = !needs_animation && !has_pending_input && {
            self.mark_idle_status_refresh_if_due(now)
        };
        if needs_animation || has_pending_input {
            self.request_redraw_if_needed();
            let frame_interval = if self.shell_state.is_transport_running() {
                self.target_frame_interval
            } else {
                self.focus_animation_interval
            };
            let mut next_redraw_at = self.last_redraw + frame_interval;
            if next_redraw_at < now {
                next_redraw_at = now;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                next_redraw_at,
            ));
            return;
        }
        if should_refresh_idle_status {
            self.request_redraw_if_needed();
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                self.next_idle_status_refresh,
            ));
            return;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
    }
}

fn action_from_key(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    if model.confirm_prompt.visible {
        let confirm_enabled = model
            .confirm_prompt
            .input_error
            .as_ref()
            .is_none_or(|error| error.trim().is_empty());
        return match key {
            KeyCode::Enter if confirm_enabled => Some(UiAction::ConfirmPrompt),
            KeyCode::C => Some(UiAction::CancelPrompt),
            _ => None,
        };
    }
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    match key {
        KeyCode::ArrowLeft => Some(UiAction::MoveColumn { delta: -1 }),
        KeyCode::ArrowRight => Some(UiAction::MoveColumn { delta: 1 }),
        KeyCode::ArrowUp => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: -1 })
            }
        }
        KeyCode::ArrowDown => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: 1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: 1 })
            }
        }
        KeyCode::Num1 => Some(UiAction::SelectColumn { index: 0 }),
        KeyCode::Num2 => Some(UiAction::SelectColumn { index: 1 }),
        KeyCode::Num3 => Some(UiAction::SelectColumn { index: 2 }),
        KeyCode::A => Some(UiAction::SelectAllBrowserRows),
        KeyCode::B => Some(UiAction::StartNewFolder),
        KeyCode::C => Some(UiAction::ClearWaveformSelection),
        KeyCode::D => Some(UiAction::DeleteBrowserSelection),
        KeyCode::Enter => Some(UiAction::ToggleTransport),
        KeyCode::F => Some(UiAction::FocusBrowserSearch),
        KeyCode::G => Some(UiAction::DeleteFocusedFolder),
        KeyCode::I => Some(UiAction::StartBrowserRename),
        KeyCode::L => Some(UiAction::ToggleLoopPlayback),
        KeyCode::M => Some(UiAction::ZoomWaveformToSelection),
        KeyCode::N => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Neutral,
        }),
        KeyCode::OpenBracket => Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
        }),
        KeyCode::P => model
            .progress_overlay
            .cancelable
            .then_some(UiAction::CancelProgress),
        KeyCode::CloseBracket => Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        }),
        KeyCode::Slash => Some(UiAction::ZoomWaveformFull),
        KeyCode::Quote => Some(UiAction::FocusFolderSearch),
        KeyCode::R => Some(UiAction::Redo),
        KeyCode::S => Some(UiAction::FocusSourcesPanel),
        KeyCode::T => Some(UiAction::ToggleFocusedBrowserRowSelection),
        KeyCode::U => Some(UiAction::Undo),
        KeyCode::W => Some(UiAction::FocusWaveformPanel),
        KeyCode::X => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::Y => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
        }),
        KeyCode::Z => Some(UiAction::StartFolderRename),
        _ => None,
    }
}

fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.prompt_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.progress_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.update_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_sample_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        let shift = modifiers.shift_key();
        let command = modifiers.control_key() || modifiers.super_key();
        return Some(if shift && command {
            UiAction::AddRangeBrowserSelection { visible_row }
        } else if shift {
            UiAction::ExtendBrowserSelectionToRow { visible_row }
        } else if command {
            UiAction::ToggleBrowserRowSelection { visible_row }
        } else {
            UiAction::FocusBrowserRow { visible_row }
        });
    }
    if let Some(index) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(UiAction::FocusFolderRow { index });
    }

    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => shell_state
            .source_row_at_point(layout, model, point)
            .map_or(Some(UiAction::FocusSourcesPanel), |index| {
                Some(UiAction::SelectSourceRow { index })
            }),
        ShellNodeKind::WaveformCard => {
            let inner = layout.waveform_plot;
            let width = inner.width().max(1.0);
            let ratio = ((point.x - inner.min.x) / width).clamp(0.0, 1.0);
            let position_milli = ratio_to_milli(ratio);
            let shift = modifiers.shift_key();
            let command = modifiers.control_key() || modifiers.super_key();
            if shift {
                Some(UiAction::SetWaveformSelectionRange {
                    start_milli: waveform_anchor_milli(model),
                    end_milli: position_milli,
                })
            } else if command {
                Some(UiAction::SetWaveformCursor { position_milli })
            } else {
                Some(UiAction::SeekWaveform { position_milli })
            }
        }
        ShellNodeKind::TopBar => Some(UiAction::ToggleTransport),
        ShellNodeKind::Content
        | ShellNodeKind::BrowserPanel
        | ShellNodeKind::BrowserTabs
        | ShellNodeKind::BrowserTable => Some(UiAction::FocusBrowserPanel),
        ShellNodeKind::StatusBar => Some(UiAction::FocusLoadedSampleInBrowser),
        _ => None,
    }
}

fn ratio_to_milli(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn waveform_anchor_milli(model: &AppModel) -> u16 {
    model
        .waveform
        .selection_milli
        .map(|selection| selection.start_milli)
        .or(model.waveform.cursor_milli)
        .or(model.waveform.playhead_milli)
        .unwrap_or(0)
}

fn browser_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    if model.map.active || !layout.browser_panel.contains(point) {
        return None;
    }
    let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => -y,
        MouseScrollDelta::PixelDelta(position) => -(position.y as f32) / row_stride,
    };
    if raw == 0.0 {
        return None;
    }
    let mut steps = raw.round();
    if steps.abs() < 1.0 {
        steps = raw.signum();
        if steps == 0.0 {
            return None;
        }
    }
    if steps == 0.0 {
        return None;
    }
    let clamped = if steps > 1.0 {
        steps.min(i8::MAX as f32)
    } else {
        steps.max(i8::MIN as f32)
    };
    Some(clamped as i8)
}

#[derive(Clone, Debug)]
struct GlyphLayout {
    id: u32,
    x: f32,
}

#[derive(Clone, Debug)]
struct TextLayout {
    width: f32,
    glyphs: Vec<GlyphLayout>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TextLayoutKey {
    text: String,
    font_size_bits: u32,
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;

#[derive(Clone)]
struct LoadedFont {
    font: FontData,
}

struct NativeTextRenderer {
    loaded_font: Option<LoadedFont>,
    layout_cache: HashMap<TextLayoutKey, TextLayout>,
    layout_cache_order: VecDeque<TextLayoutKey>,
    text_layout_hits: u64,
    text_layout_misses: u64,
    text_layout_evictions: u64,
}

impl NativeTextRenderer {
    fn new() -> Self {
        let loaded_font = load_native_font().map(|font| LoadedFont { font });
        if loaded_font.is_none() {
            eprintln!(
                "Native vello text renderer: no fallback font found; text runs will be skipped"
            );
        }
        Self {
            loaded_font,
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_order: VecDeque::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY),
            text_layout_hits: 0,
            text_layout_misses: 0,
            text_layout_evictions: 0,
        }
    }

    fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        let Some(loaded_font) = self.loaded_font.as_ref() else {
            return;
        };
        let font_data = &loaded_font.font;
        for run in text_runs {
            if run.text.is_empty() || run.font_size <= 0.0 {
                continue;
            }
            let Some(layout) = self.layout_for(&font_data, &run.text, run.font_size) else {
                continue;
            };
            let mut origin_x = run.position.x;
            if let Some(max_width) = run.max_width {
                let extra = (max_width - layout.width).max(0.0);
                origin_x += match run.align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                };
            }
            let clip_width = run.max_width.unwrap_or(f32::INFINITY);
            let baseline = run.position.y + run.font_size;
            let glyph_iter = layout
                .glyphs
                .iter()
                .take_while(|glyph| glyph.x <= clip_width)
                .map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin_x + glyph.x,
                    y: baseline,
                });
            scene
                .draw_glyphs(&font_data)
                .font_size(run.font_size)
                .brush(color_from_rgba(run.color))
                .draw(Fill::NonZero, glyph_iter);
        }
    }

    fn layout_for<'a>(
        &'a mut self,
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        let key = TextLayoutKey {
            text: text.to_string(),
            font_size_bits: font_size.to_bits(),
        };

        match self.layout_cache.entry(key) {
            Entry::Occupied(entry) => {
                self.text_layout_hits = self.text_layout_hits.saturating_add(1);
                return Some(entry.get());
            }
            Entry::Vacant(vacant) => {
                self.text_layout_misses = self.text_layout_misses.saturating_add(1);

                if self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
                    if let Some(evicted_key) = self.layout_cache_order.pop_front() {
                        if self.layout_cache.remove(&evicted_key).is_some() {
                            self.text_layout_evictions =
                                self.text_layout_evictions.saturating_add(1);
                        }
                    }
                }

                let Some(layout) = Self::compute_layout(font, text, font_size) else {
                    return None;
                }

                let cache_key = vacant.key().clone();
                let cached_layout = vacant.insert(layout);
                self.layout_cache_order.push_back(cache_key);
                return Some(cached_layout);
            }
        }
    }

    fn take_layout_profile_counters(&mut self) -> (u64, u64, u64) {
        let counters = (
            self.text_layout_hits,
            self.text_layout_misses,
            self.text_layout_evictions,
        );
        self.text_layout_hits = 0;
        self.text_layout_misses = 0;
        self.text_layout_evictions = 0;
        counters
    }

    fn compute_layout(font: &FontData, text: &str, font_size: f32) -> Option<TextLayout> {
        let font_ref = skrifa::FontRef::from_index(font.data.as_ref(), font.index).ok()?;
        let charmap = font_ref.charmap();
        let metrics = font_ref.glyph_metrics(FontSize::new(font_size), LocationRef::default());
        let fallback_glyph = charmap.map('?');

        let mut x = 0.0_f32;
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if ch == '\t' {
                x += font_size * 2.0;
                continue;
            }
            if ch == ' ' {
                x += font_size * 0.33;
                continue;
            }
            if ch.is_control() {
                continue;
            }
            let glyph_id = charmap.map(ch).or(fallback_glyph);
            let Some(glyph_id) = glyph_id else {
                x += font_size * 0.5;
                continue;
            };
            glyphs.push(GlyphLayout {
                id: glyph_id.to_u32(),
                x,
            });
            let advance = metrics
                .advance_width(glyph_id)
                .unwrap_or(font_size * 0.55)
                .max(0.0);
            x += advance;
        }

        Some(TextLayout { width: x, glyphs })
    }
}

fn load_native_font() -> Option<FontData> {
    for path in native_font_candidates() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        return Some(FontData::new(Blob::from(bytes), 0));
    }
    None
}

fn native_font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("SEMPAL_NATIVE_FONT_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let base = PathBuf::from(windir).join("Fonts");
            candidates.push(base.join("segoeui.ttf"));
            candidates.push(base.join("arial.ttf"));
            candidates.push(base.join("consola.ttf"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNS.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ));
        candidates.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
    }

    candidates
}

fn to_kurbo_rect(rect: UiRect) -> KurboRect {
    KurboRect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

fn color_from_rgba(color: Rgba8) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}

#[derive(Default)]
struct PreviewBridge;

impl NativeAppBridge for PreviewBridge {
    fn pull_model(&mut self) -> AppModel {
        AppModel::default()
    }
}

/// Run the native Vello backend window with a host-provided app bridge.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    info!("radiant native vello: creating event loop");
    let run_started = Instant::now();
        let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
        info!(
            "radiant native vello: event loop created with window_size={:?} min_window_size={:?} target_fps={}",
            options.inner_size,
            options.min_inner_size,
            options.target_fps
        );
    let mut runner = NativeVelloRunner::new(options, bridge);
    info!("radiant native vello: runner initialized");
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    let elapsed = run_started.elapsed();
        match &run_result {
            Ok(_) => info!("radiant native vello: event loop ended in {} ms", elapsed.as_millis()),
            Err(err) => warn!(
                "radiant native vello: event loop returned error in {} ms: {}",
                elapsed.as_millis(),
                err
            ),
        }
    info!("radiant native vello: event loop finished");
    runner.bridge.on_exit();
    run_result
}

/// Run the experimental native Vello backend window for backend-selection testing.
///
/// This preview path now renders an interactive backend-neutral shell model with
/// Vello primitives and exercises native input hit-testing without `egui`.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    run_native_vello_app(options, PreviewBridge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{
        BrowserPanelModel, ColumnModel, MapPanelModel, MapPointModel, SourcesPanelModel,
        UpdatePanelModel, UpdateStatusModel, WaveformPanelModel,
    };
    use crate::gui::types::Vector2;
    use winit::event::MouseScrollDelta;

    #[test]
    fn key_bindings_emit_waveform_zoom_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::OpenBracket, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 1,
            })
        );
        assert_eq!(
            action_from_key(KeyCode::CloseBracket, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveform {
                zoom_in: true,
                steps: 1,
            })
        );
        assert_eq!(
            action_from_key(KeyCode::M, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveformToSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::C, ModifiersState::default(), &model),
            Some(UiAction::ClearWaveformSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::Slash, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveformFull)
        );
    }

    #[test]
    fn key_bindings_emit_browser_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::D, ModifiersState::default(), &model),
            Some(UiAction::DeleteBrowserSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::I, ModifiersState::default(), &model),
            Some(UiAction::StartBrowserRename)
        );
        assert_eq!(
            action_from_key(KeyCode::N, ModifiersState::default(), &model),
            Some(UiAction::TagBrowserSelection {
                target: crate::app::BrowserTagTarget::Neutral
            })
        );
        assert_eq!(
            action_from_key(KeyCode::X, ModifiersState::default(), &model),
            Some(UiAction::TagBrowserSelection {
                target: crate::app::BrowserTagTarget::Trash
            })
        );
    }

    #[test]
    fn key_bindings_emit_folder_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::B, ModifiersState::default(), &model),
            Some(UiAction::StartNewFolder)
        );
        assert_eq!(
            action_from_key(KeyCode::G, ModifiersState::default(), &model),
            Some(UiAction::DeleteFocusedFolder)
        );
        assert_eq!(
            action_from_key(KeyCode::Quote, ModifiersState::default(), &model),
            Some(UiAction::FocusFolderSearch)
        );
        assert_eq!(
            action_from_key(KeyCode::Z, ModifiersState::default(), &model),
            Some(UiAction::StartFolderRename)
        );
    }

    #[test]
    fn prompt_visible_routes_enter_and_cancel_keys() {
        let mut model = AppModel::default();
        model.confirm_prompt.visible = true;
        assert_eq!(
            action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
            Some(UiAction::ConfirmPrompt)
        );
        assert_eq!(
            action_from_key(KeyCode::C, ModifiersState::default(), &model),
            Some(UiAction::CancelPrompt)
        );
        assert_eq!(
            action_from_key(KeyCode::W, ModifiersState::default(), &model),
            None
        );

        model.confirm_prompt.input_error = Some(String::from("Folder already exists"));
        assert_eq!(
            action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
            None
        );
    }

    #[test]
    fn waveform_click_modifiers_route_expected_actions() {
        let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
        let mut shell_state = NativeShellState::new();
        let point = Point::new(
            layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
            layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
        );
        let model = AppModel {
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Neutral", 0),
                ColumnModel::new("Keep", 0),
            ],
            sources: SourcesPanelModel::default(),
            browser: BrowserPanelModel::default(),
            waveform: WaveformPanelModel {
                selection_milli: Some(crate::app::NormalizedRangeModel::new(120, 360)),
                cursor_milli: Some(220),
                playhead_milli: Some(260),
                ..WaveformPanelModel::default()
            },
            ..AppModel::default()
        };

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::default(),
            ),
            Some(UiAction::SeekWaveform {
                position_milli: 500
            })
        );

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::CONTROL,
            ),
            Some(UiAction::SetWaveformCursor {
                position_milli: 500
            })
        );

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::SHIFT,
            ),
            Some(UiAction::SetWaveformSelectionRange {
                start_milli: 120,
                end_milli: 500,
            })
        );
    }

    #[test]
    fn waveform_anchor_prefers_selection_then_cursor_then_playhead() {
        let mut model = AppModel::default();
        assert_eq!(waveform_anchor_milli(&model), 0);

        model.waveform.playhead_milli = Some(333);
        assert_eq!(waveform_anchor_milli(&model), 333);

        model.waveform.cursor_milli = Some(222);
        assert_eq!(waveform_anchor_milli(&model), 222);

        model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(111, 444));
        assert_eq!(waveform_anchor_milli(&model), 111);
    }

    #[test]
    fn browser_tab_clicks_route_to_tab_actions() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let model = AppModel::default();
        let map_tab_point = Point::new(
            layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.75),
            layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                map_tab_point,
                ModifiersState::default(),
            ),
            Some(UiAction::SetBrowserTab { map: true })
        );

        let list_tab_point = Point::new(
            layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.25),
            layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                list_tab_point,
                ModifiersState::default(),
            ),
            Some(UiAction::SetBrowserTab { map: false })
        );
    }

    #[test]
    fn map_point_click_routes_to_focus_map_sample() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let point = Point::new(
            layout.browser_rows.min.x + (layout.browser_rows.width() * 0.5),
            layout.browser_rows.min.y + (layout.browser_rows.height() * 0.5),
        );
        let model = AppModel {
            map: MapPanelModel {
                active: true,
                summary: String::from("1 point"),
                legend_label: String::from("Render: points"),
                selection_label: String::from("Selection: source::kick.wav"),
                hover_label: String::from("Hover: source::kick.wav"),
                cluster_label: String::from("Clusters: 1"),
                viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
                error: None,
                render_mode: crate::app::MapRenderModeModel::Points,
                points: vec![MapPointModel {
                    sample_id: String::from("source::kick.wav"),
                    x_milli: 500,
                    y_milli: 500,
                    cluster_id: Some(1),
                    selected: true,
                    focused: true,
                }],
            },
            ..AppModel::default()
        };
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::default(),
            ),
            Some(UiAction::FocusMapSample {
                sample_id: String::from("source::kick.wav")
            })
        );
    }

    #[test]
    fn update_button_click_routes_update_check_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let model = AppModel {
            update: UpdatePanelModel {
                status: UpdateStatusModel::Idle,
                status_label: String::from("Updates: idle"),
                action_hint_label: String::from("Action: check"),
                release_notes_label: String::new(),
                available_tag: None,
                available_url: None,
                last_error: None,
            },
            ..AppModel::default()
        };
        let button_point = Point::new(
            layout.top_bar_action_cluster.max.x - 18.0,
            layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                button_point,
                ModifiersState::default(),
            ),
            Some(UiAction::CheckForUpdates)
        );
    }

    #[test]
    fn browser_wheel_delta_is_bounded_and_directional() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = StyleTokens::for_viewport_width(layout.root.rect.width());
        let mut model = AppModel::default();
        model.map.active = false;
        let point = Point::new(
            layout.browser_rows.min.x + 10.0,
            layout.browser_rows.min.y + 10.0,
        );

        assert_eq!(
            browser_wheel_row_delta(
                &layout,
                &model,
                point,
                &style,
                MouseScrollDelta::LineDelta(0.0, 3.0),
            ),
            Some(-3)
        );
        assert_eq!(
            browser_wheel_row_delta(
                &layout,
                &model,
                point,
                &style,
                MouseScrollDelta::LineDelta(0.0, 0.0)
            ),
            None
        );
        let header_point = Point::new(
            layout.browser_table_header.min.x + 5.0,
            layout.browser_table_header.min.y + 5.0,
        );
        assert_eq!(
            browser_wheel_row_delta(
                &layout,
                &model,
                header_point,
                &style,
                MouseScrollDelta::LineDelta(0.0, 2.0),
            ),
            Some(-2)
        );
    }
}
