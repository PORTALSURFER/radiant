//! Runner state and redraw coordination for the generic native Vello runtime.

use super::*;
use crate::runtime::SurfacePaintPlan;
use crate::theme::ThemeTokens;

pub(super) struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) options: NativeRunOptions,
    pub(super) core: GenericNativeRuntimeCore<Bridge, Message>,
    pub(super) runtime_wakeup: RuntimeWakeup,
    pub(super) window_id: Option<WindowId>,
    pub(super) window: Option<Arc<Window>>,
    pub(super) render_ctx: Option<RenderContext>,
    pub(super) render_surface: Option<RenderSurface<'static>>,
    pub(super) renderer: Option<Renderer>,
    pub(super) text_renderer: NativeTextRenderer,
    pub(super) scene: Scene,
    pub(super) gpu_surface_renderer: GpuSurfaceRenderer,
    pub(super) post_gpu_overlay_renderer: PostGpuOverlayRenderer,
    pub(super) last_paint_plan: SurfacePaintPlan,
    pub(super) transient_overlay_primitives: Vec<crate::runtime::PaintPrimitive>,
    pub(super) retained_surface_cache: RetainedSurfaceFrameCache,
    pub(super) last_cursor: Option<Point>,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) modifiers: winit::keyboard::ModifiersState,
    pub(super) redraw_requested: bool,
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) first_frame_presented: bool,
    pub(super) animation_origin: Instant,
    pub(super) last_redraw: Instant,
    pub(super) last_scene_stats: RetainedSurfaceEncodeStats,
    pub(super) scene_text_runs: SceneTextRunBuffer<'static>,
    pub(super) gpu_surface_interaction_regions: Vec<GpuSurfaceInteractionRegion>,
    pub(super) scene_texture_dirty: bool,
    pub(super) deferred_surface_refresh: bool,
    pub(super) pending_gpu_surface_wheel: Option<PendingGpuSurfaceWheel>,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        let text_renderer = NativeTextRenderer::with_options(&options.text);
        let debug_layout = options.debug_layout;
        Self {
            options,
            core: GenericNativeRuntimeCore::new_with_debug_layout(bridge, viewport, debug_layout),
            runtime_wakeup: RuntimeWakeup::default(),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            text_renderer,
            scene: Scene::new(),
            gpu_surface_renderer: GpuSurfaceRenderer::default(),
            post_gpu_overlay_renderer: PostGpuOverlayRenderer::default(),
            last_paint_plan: SurfacePaintPlan::empty(&ThemeTokens::default()),
            transient_overlay_primitives: Vec::new(),
            retained_surface_cache: RetainedSurfaceFrameCache::default(),
            last_cursor: None,
            clipboard: arboard::Clipboard::new().ok(),
            modifiers: winit::keyboard::ModifiersState::default(),
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            animation_origin: Instant::now(),
            last_redraw: Instant::now(),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
            scene_text_runs: SceneTextRunBuffer::new(),
            gpu_surface_interaction_regions: Vec::new(),
            scene_texture_dirty: true,
            deferred_surface_refresh: false,
            pending_gpu_surface_wheel: None,
        }
    }

    pub(super) fn request_redraw_if_needed(&mut self) {
        if self.redraw_requested {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.redraw_requested = true;
        }
    }

    pub(super) fn request_runtime_wakeup_if_needed(&self, outcome: GenericRouteOutcome) {
        self.runtime_wakeup
            .request_if(outcome.runtime_work_remaining);
    }

    pub(super) fn rebuild_scene(&mut self) {
        self.core.paint_plan_into(&mut self.last_paint_plan);
        let viewport = self.core.runtime.viewport();
        let mut scene_text_runs = std::mem::take(&mut self.scene_text_runs);
        self.last_scene_stats = encode_surface_paint_plan_to_scene(
            &self.last_paint_plan,
            SurfaceSceneEncodeContext {
                scene: &mut self.scene,
                text_renderer: &mut self.text_renderer,
                bridge: self.core.runtime.bridge_mut(),
                viewport,
                retained_cache: &mut self.retained_surface_cache,
                text_runs: &mut scene_text_runs,
                gpu_surface_interaction_regions: &mut self.gpu_surface_interaction_regions,
                animation_time: self.animation_origin.elapsed(),
            },
        );
        self.scene_text_runs = scene_text_runs.rebind();
        self.scene_texture_dirty = true;
    }

    pub(super) fn handle_route_outcome(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        if outcome.exit_requested {
            event_loop.exit();
            return;
        }
        if outcome.needs_scene_rebuild() {
            self.rebuild_scene();
        }
        if outcome.needs_redraw() {
            self.request_redraw_if_needed();
        }
        self.request_runtime_wakeup_if_needed(outcome);
    }
}
