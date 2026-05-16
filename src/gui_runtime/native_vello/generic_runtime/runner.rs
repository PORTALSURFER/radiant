//! Runner state and redraw coordination for the generic native Vello runtime.

use super::*;

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
    pub(super) frame: NativeVelloFrameState,
    pub(super) last_cursor: Option<Point>,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) modifiers: winit::keyboard::ModifiersState,
    pub(super) redraw_requested: bool,
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) first_frame_presented: bool,
    pub(super) animation_origin: Instant,
    pub(super) last_redraw: Instant,
    pub(super) last_timed_frame_drain: Instant,
    pub(super) last_navigation_key_repeat: Option<Instant>,
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
        let retained_surface_cache = options.retained_surface_cache;
        Self {
            options,
            core: GenericNativeRuntimeCore::new_with_debug_layout(bridge, viewport, debug_layout),
            runtime_wakeup: RuntimeWakeup::default(),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            frame: NativeVelloFrameState::new(text_renderer, retained_surface_cache),
            last_cursor: None,
            clipboard: arboard::Clipboard::new().ok(),
            modifiers: winit::keyboard::ModifiersState::default(),
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            animation_origin: Instant::now(),
            last_redraw: Instant::now(),
            last_timed_frame_drain: Instant::now(),
            last_navigation_key_repeat: None,
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

    pub(super) fn drain_timed_frame_now(
        &mut self,
        now: Instant,
        animation_activity: crate::runtime::RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        self.last_timed_frame_drain = now;
        self.core
            .drain_timed_frame(animation_activity, needs_text_caret_animation)
    }

    pub(super) fn request_runtime_wakeup_if_needed(&self, outcome: GenericRouteOutcome) {
        self.runtime_wakeup
            .request_if(outcome.runtime_work_remaining);
    }

    pub(super) fn rebuild_scene(&mut self) {
        self.core.paint_plan_into(&mut self.frame.last_paint_plan);
        let viewport = self.core.runtime.viewport();
        self.frame.last_scene_stats = encode_surface_paint_plan_to_scene(
            &self.frame.last_paint_plan,
            SurfaceSceneEncodeContext {
                scene: &mut self.frame.scene,
                text_renderer: &mut self.frame.text_renderer,
                bridge: self.core.runtime.bridge_mut(),
                viewport,
                retained_cache: &mut self.frame.retained_surface_cache,
                text_runs: &mut self.frame.scene_text_runs,
                gpu_surface_interaction_regions: &mut self.frame.gpu_surface_interaction_regions,
                animation_time: self.animation_origin.elapsed(),
            },
        );
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_texture_dirty();
    }

    fn restore_native_hover_cursor_overlay(&mut self) {
        let Some(position) = self.last_cursor else {
            return;
        };
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
        }
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
