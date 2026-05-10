//! Generic `RuntimeBridge` native Vello runner.

use super::*;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal};
use crate::layout::Rect;
use crate::runtime::SurfacePaintPlan;
use crate::theme::ThemeTokens;

mod core;
mod gpu_surface;
mod gpu_surface_interaction;
mod input;
mod keyboard;
mod lifecycle;
mod present;
mod runtime_helpers;
mod scene;
mod surface;
mod window;

pub(in crate::gui_runtime::native_vello) use core::{
    GenericNativeRuntimeCore, GenericRouteOutcome,
};
use gpu_surface::GpuSurfaceRenderer;
use gpu_surface_interaction::PendingGpuSurfaceWheel;
use input::{key_code_from_winit, keypress_from_input, pointer_button_from_winit};
use present::RenderFrameProfile;
use runtime_helpers::{
    animation_frame_interval, fast_pointer_move_gpu_surface_hit_rects, maybe_log_route_profile,
    render_profile_enabled, scroll_delta_to_logical,
};
pub(in crate::gui_runtime::native_vello) use scene::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, SurfaceSceneEncodeContext,
    encode_surface_paint_plan_to_scene,
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
        let text_renderer = NativeTextRenderer::with_options(&options.text);
        Self {
            options,
            core: GenericNativeRuntimeCore::new(bridge, viewport),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            text_renderer,
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
        let mut scene_text_runs = Vec::with_capacity(plan.primitives.len().min(64));
        self.last_scene_stats = encode_surface_paint_plan_to_scene(
            &plan,
            SurfaceSceneEncodeContext {
                scene: &mut self.scene,
                text_renderer: &mut self.text_renderer,
                bridge: self.core.runtime.bridge_mut(),
                viewport,
                retained_cache: &mut self.retained_surface_cache,
                text_runs: &mut scene_text_runs,
                animation_time: self.animation_origin.elapsed(),
            },
        );
        self.scene_texture_dirty = true;
        self.last_paint_plan = plan;
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

    fn update_gpu_surface_hit_rects(&mut self, primitives: &[PaintPrimitive]) {
        self.fast_pointer_move_gpu_surface_hit_rects =
            fast_pointer_move_gpu_surface_hit_rects(primitives);
    }
}

#[cfg(test)]
mod tests;
