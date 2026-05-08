//! Generic `RuntimeBridge` native Vello runner.

use super::*;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal};
use crate::layout::Rect;
use crate::runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive, SurfacePaintPlan};
use crate::theme::ThemeTokens;

mod core;
mod gpu_surface;
mod input;
mod keyboard;
mod lifecycle;
mod present;
mod scene;
mod window;

pub(in crate::gui_runtime::native_vello) use core::{
    GenericNativeRuntimeCore, GenericRouteOutcome,
};
use gpu_surface::GpuSurfaceRenderer;
use input::{keypress_from_input, pointer_button_from_winit};
use present::RenderFrameProfile;
pub(in crate::gui_runtime::native_vello) use scene::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, encode_surface_paint_plan_to_scene,
};
use window::generic_window_attributes;

struct GenericSharedPixelBytes(Arc<[u8]>);

impl AsRef<[u8]> for GenericSharedPixelBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Run a generic [`RuntimeBridge`] through the native Vello backend.
///
/// This entrypoint is intentionally narrower than the compatibility
/// shell runner: it renders public `UiSurface` paint primitives, routes pointer
/// and keyboard input into projected widget ids, reduces host-defined messages,
/// and requests redraws when the surface changes.
pub fn run_native_vello_runtime<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> Result<(), String>
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    run_native_vello_runtime_with_artifacts(options, bridge).result
}

/// Run a generic [`RuntimeBridge`] through native Vello and return runtime artifacts.
pub fn run_native_vello_runtime_with_artifacts<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> NativeGenericRunReport
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    info!("radiant generic native vello: creating event loop");
    let run_started = Instant::now();
    let event_loop = match EventLoop::<RuntimeUserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            return NativeGenericRunReport {
                artifacts: NativeGenericRuntimeArtifacts::default(),
                result: Err(err.to_string()),
            };
        }
    };
    let viewport = initial_viewport(&options);
    let mut runner = GenericNativeVelloRunner::new(options, bridge, viewport);
    let proxy = event_loop.create_proxy();
    let repaint_signal: Arc<dyn RepaintSignal> = Arc::new(CoalescingRepaintSignal::new(
        Arc::clone(&runner.repaint_event_pending),
        move || proxy.send_event(RuntimeUserEvent::RepaintRequested).is_ok(),
    ));
    runner
        .core
        .runtime
        .bridge_mut()
        .install_repaint_signal(repaint_signal);
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    let elapsed = run_started.elapsed();
    match &run_result {
        Ok(_) => info!(
            "radiant generic native vello: event loop ended in {} ms",
            elapsed.as_millis()
        ),
        Err(err) => warn!(
            "radiant generic native vello: event loop returned error in {} ms: {}",
            elapsed.as_millis(),
            err
        ),
    }
    let shutdown_timing = runner.core.runtime.bridge_mut().on_runtime_exit();
    NativeGenericRunReport {
        artifacts: NativeGenericRuntimeArtifacts {
            startup_timing: runner.startup_timing.export_artifact(),
            shutdown_timing,
        },
        result: run_result,
    }
}

/// Structured runtime artifacts exported after one generic native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeGenericRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Result plus structured artifacts returned by one generic native runtime execution.
pub type NativeGenericRunReport =
    crate::gui_runtime::RuntimeRunReport<NativeGenericRuntimeArtifacts>;

fn initial_viewport(options: &NativeRunOptions) -> Vector2 {
    let [width, height] = options.inner_size.unwrap_or([1280.0, 720.0]);
    Vector2::new(width.max(1.0), height.max(1.0))
}

struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    options: NativeRunOptions,
    core: GenericNativeRuntimeCore<Bridge, Message>,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    text_renderer: NativeTextRenderer,
    scene: Scene,
    gpu_surface_renderer: GpuSurfaceRenderer,
    last_paint_plan: SurfacePaintPlan,
    retained_surface_cache: RetainedSurfaceFrameCache,
    last_cursor: Option<Point>,
    clipboard: Option<arboard::Clipboard>,
    repaint_event_pending: Arc<std::sync::atomic::AtomicBool>,
    modifiers: winit::keyboard::ModifiersState,
    redraw_requested: bool,
    startup_timing: StartupTimingProfile,
    first_frame_presented: bool,
    animation_origin: Instant,
    last_redraw: Instant,
    last_scene_stats: RetainedSurfaceEncodeStats,
    fast_pointer_move_gpu_surface_hit_rects: Vec<Rect>,
    scene_texture_dirty: bool,
    deferred_surface_refresh: bool,
    pending_gpu_surface_wheel: Option<PendingGpuSurfaceWheel>,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        Self {
            options,
            core: GenericNativeRuntimeCore::new(bridge, viewport),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            text_renderer: NativeTextRenderer::new(),
            scene: Scene::new(),
            gpu_surface_renderer: GpuSurfaceRenderer::default(),
            last_paint_plan: SurfacePaintPlan::empty(&ThemeTokens::default()),
            retained_surface_cache: RetainedSurfaceFrameCache::default(),
            last_cursor: None,
            clipboard: arboard::Clipboard::new().ok(),
            repaint_event_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            modifiers: winit::keyboard::ModifiersState::default(),
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            animation_origin: Instant::now(),
            last_redraw: Instant::now(),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
            fast_pointer_move_gpu_surface_hit_rects: Vec::new(),
            scene_texture_dirty: true,
            deferred_surface_refresh: false,
            pending_gpu_surface_wheel: None,
        }
    }

    fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        info!("radiant generic native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let window = match event_loop.create_window(generic_window_attributes(&self.options)) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create window: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_window_created();
        self.window_id = Some(window.id());
        self.window = Some(Arc::clone(&window));

        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.core
            .set_viewport(Vector2::new(width as f32, height as f32));
        let surface = match render_ctx.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create wgpu surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_wgpu_surface_created();
        let Some(dev_id) = pollster::block_on(render_ctx.device(Some(&surface))) else {
            error!("radiant generic native vello: no compatible render device found");
            event_loop.exit();
            return;
        };
        self.startup_timing.mark_wgpu_device_ready();
        let supported_present_modes = surface
            .get_capabilities(render_ctx.devices[dev_id].adapter())
            .present_modes;
        let present_mode = select_present_mode(self.options.target_fps, &supported_present_modes);
        let render_surface = match pollster::block_on(render_ctx.create_render_surface(
            surface,
            width,
            height,
            present_mode,
        )) {
            Ok(render_surface) => render_surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create render surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_surface_ready();
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        self.startup_timing.mark_renderer_started();
        let renderer = match Renderer::new(&dev_handle.device, startup_renderer_options()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create renderer: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_renderer_ready();
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.rebuild_scene();
        self.startup_timing.mark_first_scene_ready();
        window.set_visible(true);
        self.startup_timing.mark_window_revealed();
        self.last_redraw = Instant::now();
        self.request_redraw_if_needed();
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

    fn rebuild_scene(&mut self) {
        let plan = self.core.paint_plan();
        self.update_gpu_surface_hit_rects(&plan.primitives);
        let viewport = self.core.runtime.viewport();
        self.last_scene_stats = encode_surface_paint_plan_to_scene(
            &plan,
            &mut self.scene,
            &mut self.text_renderer,
            self.core.runtime.bridge_mut(),
            viewport,
            &mut self.retained_surface_cache,
            self.animation_origin.elapsed(),
        );
        self.scene_texture_dirty = true;
        self.last_paint_plan = plan;
    }

    fn resize_surface(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        if let (Some(render_ctx), Some(surface)) =
            (self.render_ctx.as_ref(), self.render_surface.as_mut())
        {
            render_ctx.resize_surface(surface, size.width, size.height);
            self.core
                .set_viewport(Vector2::new(size.width as f32, size.height as f32));
            self.rebuild_scene();
            self.request_redraw_if_needed();
        }
    }

    fn handle_route_outcome(&mut self, event_loop: &ActiveEventLoop, outcome: GenericRouteOutcome) {
        if outcome.exit_requested {
            event_loop.exit();
            return;
        }
        if outcome.needs_redraw() {
            self.rebuild_scene();
            self.request_redraw_if_needed();
        }
    }

    fn handle_gpu_surface_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        position: Point,
        delta: Vector2,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_route(position, delta) {
            self.deferred_surface_refresh = true;
            self.request_redraw_if_needed();
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    fn queue_gpu_surface_wheel(&mut self, position: Point, delta: Vector2) {
        match &mut self.pending_gpu_surface_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
            }
            None => {
                self.pending_gpu_surface_wheel = Some(PendingGpuSurfaceWheel { position, delta });
            }
        }
        self.update_gpu_surface_cursor_overlay(position);
        self.request_redraw_if_needed();
    }

    fn flush_pending_gpu_surface_wheel(&mut self, profile: &mut RenderFrameProfile) {
        let Some(pending) = self.pending_gpu_surface_wheel.take() else {
            return;
        };
        let started = Instant::now();
        let outcome = self
            .core
            .route_scroll_deferred_refresh(pending.position, pending.delta);
        profile.coalesced_wheel_route = started.elapsed();
        maybe_log_route_profile("coalesced_wheel", profile.coalesced_wheel_route, outcome);
        if outcome.needs_redraw() {
            self.deferred_surface_refresh = true;
        }
    }

    fn handle_gpu_surface_pointer_move_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        previous: Option<Point>,
        position: Point,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_pointer_move(previous, position) {
            self.update_gpu_surface_cursor_overlay(position);
            self.request_redraw_if_needed();
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    fn can_fast_path_gpu_surface_route(&self, position: Point, delta: Vector2) -> bool {
        let is_horizontal_pan = delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON;
        !is_horizontal_pan && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    fn can_fast_path_gpu_surface_pointer_move(
        &self,
        previous: Option<Point>,
        position: Point,
    ) -> bool {
        let Some(previous) = previous else {
            return false;
        };
        self.fast_pointer_move_gpu_surface_hit_rects
            .iter()
            .any(|rect| rect.contains(previous) && rect.contains(position))
    }

    fn paint_plan_has_coalescing_gpu_surface_at(&self, position: Point) -> bool {
        self.last_paint_plan
            .primitives
            .iter()
            .any(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface) => {
                    surface.rect.contains(position) && surface.capabilities.coalesce_vertical_wheel
                }
                _ => false,
            })
    }

    fn native_hover_surface_contains(&self, position: Point) -> bool {
        self.last_paint_plan
            .primitives
            .iter()
            .any(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface) => {
                    surface.rect.contains(position)
                        && surface.capabilities.native_hover_cursor.is_some()
                }
                _ => false,
            })
    }

    fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    fn update_gpu_surface_hit_rects(&mut self, primitives: &[PaintPrimitive]) {
        self.fast_pointer_move_gpu_surface_hit_rects =
            fast_pointer_move_gpu_surface_hit_rects(primitives);
    }

    fn update_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let Some(surface) = gpu_surface_at_mut(&mut self.last_paint_plan.primitives, position)
        else {
            return false;
        };
        let Some(cursor) = surface.capabilities.native_hover_cursor else {
            return false;
        };
        let ratio =
            ((position.x - surface.rect.min.x) / surface.rect.width().max(1.0)).clamp(0.0, 1.0);
        surface
            .overlays
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::VerticalCursor { .. }));
        surface.overlays.push(GpuSurfaceOverlay::VerticalCursor {
            ratio,
            color: cursor.color,
            width: cursor.width,
        });
        true
    }

    fn clear_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let Some(surface) = gpu_surface_at_mut(&mut self.last_paint_plan.primitives, position)
        else {
            return false;
        };
        if surface.capabilities.native_hover_cursor.is_none() {
            return false;
        }
        let previous_len = surface.overlays.len();
        surface
            .overlays
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::VerticalCursor { .. }));
        previous_len != surface.overlays.len()
    }
}

fn gpu_surface_at_mut(
    primitives: &mut [PaintPrimitive],
    position: Point,
) -> Option<&mut PaintGpuSurface> {
    primitives.iter_mut().find_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) if surface.rect.contains(position) => Some(surface),
        _ => None,
    })
}

fn fast_pointer_move_gpu_surface_hit_rects(primitives: &[PaintPrimitive]) -> Vec<Rect> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) if surface.capabilities.fast_pointer_move => {
                Some(surface.rect)
            }
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct PendingGpuSurfaceWheel {
    position: Point,
    delta: Vector2,
}

fn animation_frame_interval(target_fps: u32) -> Duration {
    let fps = target_fps.clamp(1, 240);
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

fn scroll_delta_to_logical(delta: MouseScrollDelta) -> Vector2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vector2::new(-(x * 40.0), -(y * 40.0)),
        MouseScrollDelta::PixelDelta(position) => {
            Vector2::new(-(position.x as f32), -(position.y as f32))
        }
    }
}

fn maybe_log_route_profile(reason: &'static str, elapsed: Duration, outcome: GenericRouteOutcome) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        event_route_us = elapsed.as_micros(),
        routed = outcome.routed,
        repaint_requested = outcome.repaint_requested,
        "radiant native input profile"
    );
}

fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}

#[cfg(test)]
mod tests;
