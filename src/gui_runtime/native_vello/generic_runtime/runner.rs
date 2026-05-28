//! Runner state and redraw coordination for the generic native Vello runtime.

use super::{
    AuxiliaryNativeWindow, GenericNativeRuntimeCore, GenericRouteOutcome, NativeRunnerInputState,
    NativeRunnerTimingState, NativeRunnerWindowState, NativeVelloFrameState, RuntimeWakeup,
    SurfaceSceneEncodeContext, TimedFrameCadence, animation_frame_interval,
    encode_surface_paint_plan_to_scene, timed_frame_cadence, timed_frame_target_fps,
};
use crate::{
    gui::types::Vector2,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{NativeRunOptions, RuntimeAnimationActivity, RuntimeBridge},
};
use std::time::Instant;
use winit::event_loop::ActiveEventLoop;

pub(super) struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) options: NativeRunOptions,
    pub(super) core: GenericNativeRuntimeCore<Bridge, Message>,
    pub(super) runtime_wakeup: RuntimeWakeup,
    pub(super) window: NativeRunnerWindowState,
    pub(super) frame: NativeVelloFrameState,
    pub(super) input: NativeRunnerInputState,
    pub(super) timing: NativeRunnerTimingState,
    pub(super) auxiliary_windows: Vec<AuxiliaryNativeWindow<Message>>,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        let text_renderer = NativeTextRenderer::with_options(&options.text);
        let debug_layout = options.frame.debug_layout;
        let retained_surface_cache = options.frame.retained_surface_cache;
        Self {
            options,
            core: GenericNativeRuntimeCore::new_with_debug_layout(bridge, viewport, debug_layout),
            runtime_wakeup: RuntimeWakeup::default(),
            window: NativeRunnerWindowState::default(),
            frame: NativeVelloFrameState::new(text_renderer, retained_surface_cache),
            input: NativeRunnerInputState::default(),
            timing: NativeRunnerTimingState::default(),
            auxiliary_windows: Vec::new(),
        }
    }

    pub(super) fn request_redraw_if_needed(&mut self) {
        if self.timing.redraw_requested {
            return;
        }
        if let Some(window) = self.window.window.as_ref() {
            window.request_redraw();
            self.timing.redraw_requested = true;
        }
    }

    pub(super) fn drain_timed_frame_now(
        &mut self,
        now: Instant,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        self.timing.last_timed_frame_drain = now;
        self.core
            .drain_timed_frame(animation_activity, needs_text_caret_animation)
    }

    pub(super) fn merge_due_timed_frame_for_route(&mut self, outcome: &mut GenericRouteOutcome) {
        let now = Instant::now();
        let native_frame_interval = animation_frame_interval(self.options.normalized_target_fps());
        if now.duration_since(self.timing.last_timed_frame_drain) < native_frame_interval {
            return;
        }
        let animation_activity = self.core.animation_activity();
        let needs_text_caret_animation = self.core.has_focused_text_input();
        if !animation_activity.needs_animation() && !needs_text_caret_animation {
            return;
        }
        let frame_target_fps = timed_frame_target_fps(
            self.options.normalized_target_fps(),
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            self.timing.last_timed_frame_drain,
            frame_target_fps,
            true,
        );
        if !matches!(cadence, TimedFrameCadence::DrainNow { .. }) {
            return;
        }
        outcome.merge(self.drain_timed_frame_now(
            now,
            animation_activity,
            needs_text_caret_animation,
        ));
    }

    pub(super) fn request_runtime_wakeup_if_needed(&self, outcome: GenericRouteOutcome) {
        if self.core.runtime.interactive_pointer_route_active() {
            return;
        }
        self.runtime_wakeup
            .request_if(outcome.runtime_work_remaining);
    }

    pub(super) fn rebuild_scene(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.deferred_scene_rebuild_requires_encode = false;
        let _ = self.apply_pending_viewport_resize_if_needed();
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
                animation_time: self.timing.animation_origin.elapsed(),
            },
        );
        self.frame.refresh_post_gpu_overlay_cache();
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_content_dirty();
    }

    pub(super) fn rebuild_scene_for_interactive_route_now(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.last_interactive_scene_rebuild = Instant::now();
        self.rebuild_scene();
    }

    pub(super) fn refresh_and_rebuild_scene_for_interactive_route_now(&mut self) {
        if self.timing.deferred_surface_refresh {
            self.timing.deferred_surface_refresh = false;
        }
        self.core.refresh_surface();
        self.rebuild_scene_for_interactive_route_now();
    }

    pub(super) fn should_rebuild_interactive_scene_now(&self, now: Instant) -> bool {
        let interval = animation_frame_interval(self.options.normalized_target_fps());
        now.duration_since(self.timing.last_interactive_scene_rebuild) >= interval
    }

    pub(super) fn defer_scene_rebuild(&mut self) {
        self.timing.deferred_scene_rebuild = true;
        self.timing.deferred_scene_rebuild_requires_encode = true;
    }

    pub(super) fn defer_viewport_resize(&mut self, viewport: Vector2) {
        self.timing.pending_viewport_resize = Some(viewport);
        self.timing.deferred_scene_rebuild = true;
    }

    pub(super) fn apply_pending_viewport_resize_if_needed(&mut self) -> Option<bool> {
        let viewport = self.timing.pending_viewport_resize.take()?;
        Some(self.core.set_viewport(viewport))
    }

    pub(super) fn defer_interactive_scene_rebuild(&mut self) {
        self.timing.deferred_surface_refresh = true;
        self.defer_scene_rebuild();
    }

    fn restore_native_hover_cursor_overlay(&mut self) {
        let Some(position) = self.input.last_cursor else {
            return;
        };
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
        }
    }

    pub(super) fn handle_route_outcome(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut outcome: GenericRouteOutcome,
    ) {
        self.merge_due_timed_frame_for_route(&mut outcome);
        if outcome.exit_requested {
            event_loop.exit();
            return;
        }
        if let Some(scale) = outcome.dpi_scale_override {
            self.set_dpi_scale_override(scale);
        }
        if let Some(size) = outcome.window_logical_size {
            self.set_window_logical_size(size);
        }
        let mut sync_auxiliary_windows_now = false;
        if outcome.needs_scene_rebuild() {
            if outcome.interactive_scene_rebuild_requested {
                let now = Instant::now();
                if self.should_rebuild_interactive_scene_now(now) {
                    if outcome.interactive_surface_refresh_requested {
                        self.refresh_and_rebuild_scene_for_interactive_route_now();
                    } else {
                        self.rebuild_scene_for_interactive_route_now();
                    }
                    self.defer_auxiliary_window_sync();
                } else {
                    self.defer_interactive_scene_rebuild();
                    self.defer_auxiliary_window_sync();
                }
            } else {
                self.rebuild_scene();
                sync_auxiliary_windows_now = true;
            }
        } else if outcome.deferred_surface_refresh_requested {
            self.timing.deferred_surface_refresh = true;
        }
        if sync_auxiliary_windows_now {
            self.sync_auxiliary_windows(event_loop);
        }
        if outcome.needs_redraw() {
            self.request_redraw_if_needed();
        }
        self.request_runtime_wakeup_if_needed(outcome);
    }
}
