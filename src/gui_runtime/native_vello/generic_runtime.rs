//! Generic `RuntimeBridge` native Vello runner.

use super::*;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal};

mod core;
mod input;
mod scene;
mod window;

pub(in crate::gui_runtime::native_vello) use core::{
    GenericNativeRuntimeCore, GenericRouteOutcome,
};
use input::{keypress_from_input, pointer_button_from_winit};
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
    retained_surface_cache: RetainedSurfaceFrameCache,
    last_cursor: Option<Point>,
    repaint_event_pending: Arc<std::sync::atomic::AtomicBool>,
    modifiers: winit::keyboard::ModifiersState,
    redraw_requested: bool,
    startup_timing: StartupTimingProfile,
    first_frame_presented: bool,
    last_redraw: Instant,
    last_scene_stats: RetainedSurfaceEncodeStats,
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
            retained_surface_cache: RetainedSurfaceFrameCache::default(),
            last_cursor: None,
            repaint_event_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            modifiers: winit::keyboard::ModifiersState::default(),
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            last_redraw: Instant::now(),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
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
        let viewport = self.core.runtime.viewport();
        self.last_scene_stats = encode_surface_paint_plan_to_scene(
            &plan,
            &mut self.scene,
            &mut self.text_renderer,
            self.core.runtime.bridge_mut(),
            viewport,
            &mut self.retained_surface_cache,
        );
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

    fn handle_route_outcome(&mut self, outcome: GenericRouteOutcome) {
        if outcome.needs_redraw() {
            self.rebuild_scene();
            self.request_redraw_if_needed();
        }
    }

    fn handle_keyboard_event(&mut self, event: winit::event::KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        let mut route_outcome = GenericRouteOutcome::default();
        if let PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = key_code_from_winit(code)
        {
            let outcome = self.core.route_key_press(
                keypress_from_input(key, self.modifiers),
                WidgetKey::from_key_code(key),
            );
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if let Some(text) = event.text.as_ref() {
            self.route_text_input(text, &mut route_outcome);
        } else if let Key::Character(text) = &event.logical_key {
            self.route_text_input(text.as_str(), &mut route_outcome);
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_widget_key(WidgetKey::Backspace);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_widget_key(WidgetKey::Delete);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        self.handle_route_outcome(route_outcome);
    }

    /// Route printable text from a keyboard event into the focused widget.
    fn route_text_input(&mut self, text: &str, route_outcome: &mut GenericRouteOutcome) {
        for character in text.chars().filter(|character| !character.is_control()) {
            if route_outcome.routed {
                break;
            }
            let outcome = self.core.route_character(character);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_requested = false;
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            return;
        };
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let size = window.inner_size();
                    self.resize_surface(size);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    error!("radiant generic native vello: out of memory acquiring surface");
                    event_loop.exit();
                    return;
                }
                Err(err) => {
                    warn!(
                        "radiant generic native vello: non-fatal surface acquire error: {:?}",
                        err
                    );
                    return;
                }
            }
        };
        let Some(surface) = self.render_surface.as_mut() else {
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let plan = self.core.paint_plan();
        let render_started = Instant::now();
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: color_from_rgba(plan.clear_color),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        let render_to_texture_elapsed = render_started.elapsed();
        if let Err(err) = render_result {
            error!(
                "radiant generic native vello: render_to_texture failed: {:?}",
                err
            );
            event_loop.exit();
            return;
        }
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        maybe_log_render_profile(
            "present",
            self.last_scene_stats,
            render_to_texture_elapsed,
            self.last_redraw.elapsed(),
        );
        self.last_redraw = Instant::now();
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            self.startup_timing.maybe_emit_summary();
        }
    }
}

impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>
    for GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize_surface(size),
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw_if_needed(),
            WindowEvent::CursorMoved { position, .. } => {
                let position = Point::new(position.x as f32, position.y as f32);
                self.last_cursor = Some(position);
                let outcome = self.core.route_pointer_move(position);
                self.handle_route_outcome(outcome);
            }
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let Some(position) = self.last_cursor else {
                    return;
                };
                let Some(button) = pointer_button_from_winit(button) else {
                    return;
                };
                let routed = match state {
                    ElementState::Pressed => self.core.route_pointer_press(position, button),
                    ElementState::Released => self.core.route_pointer_release(position, button),
                };
                self.handle_route_outcome(routed);
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_event(event),
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.repaint_event_pending
                    .store(false, std::sync::atomic::Ordering::Release);
                self.core.refresh_surface();
                self.rebuild_scene();
                self.request_redraw_if_needed();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if !self.core.needs_animation() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        let interval = animation_frame_interval(self.options.target_fps);
        let next_frame = self.last_redraw.checked_add(interval).unwrap_or(now);
        if now >= next_frame {
            if !self.redraw_requested {
                self.core.refresh_surface();
                self.rebuild_scene();
                self.request_redraw_if_needed();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + interval));
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
        }
    }
}

fn animation_frame_interval(target_fps: u32) -> Duration {
    let fps = target_fps.clamp(1, 240);
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

fn maybe_log_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    render_to_texture_elapsed: Duration,
    since_last_present: Duration,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        retained_bridge_calls = stats.bridge_calls,
        retained_cache_hits = stats.cache_hits,
        retained_primitives = stats.primitive_count,
        retained_text_runs = stats.text_run_count,
        render_to_texture_us = render_to_texture_elapsed.as_micros(),
        since_last_present_us = since_last_present.as_micros(),
        "radiant native render profile"
    );
}

fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}

#[cfg(test)]
mod tests;
