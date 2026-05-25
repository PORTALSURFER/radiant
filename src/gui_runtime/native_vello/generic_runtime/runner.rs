//! Runner state and redraw coordination for the generic native Vello runtime.

use super::{
    AuxiliaryNativeWindow, GenericNativeRuntimeCore, GenericRouteOutcome, NativeRunnerInputState,
    NativeRunnerTimingState, NativeRunnerWindowState, NativeVelloFrameState, RuntimeWakeup,
    SurfaceSceneEncodeContext, TimedFrameCadence, encode_surface_paint_plan_to_scene,
    timed_frame_cadence, timed_frame_target_fps,
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
        let animation_activity = self.core.animation_activity();
        let needs_text_caret_animation = self.core.has_focused_text_input();
        if !animation_activity.needs_animation() && !needs_text_caret_animation {
            return;
        }
        let now = Instant::now();
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
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_texture_dirty();
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
        if outcome.needs_scene_rebuild() {
            self.rebuild_scene();
            self.sync_auxiliary_windows(event_loop);
        }
        if outcome.needs_redraw() {
            self.request_redraw_if_needed();
        }
        self.request_runtime_wakeup_if_needed(outcome);
    }

    pub(super) fn dispatch_auxiliary_messages(
        &mut self,
        event_loop: &ActiveEventLoop,
        messages: Vec<Message>,
    ) {
        let mut outcome = GenericRouteOutcome::default();
        for message in messages {
            let command_outcome = self.core.runtime.dispatch_message(message);
            outcome.merge(self.core.route_command_outcome(command_outcome));
        }
        self.handle_route_outcome(event_loop, outcome);
        self.sync_auxiliary_windows(event_loop);
    }

    pub(super) fn sync_auxiliary_windows(&mut self, event_loop: &ActiveEventLoop) {
        let projections = self.core.runtime.bridge_mut().project_auxiliary_windows();
        let mut projected_keys = Vec::with_capacity(projections.len());
        for projection in projections {
            projected_keys.push(projection.key.clone());
            if let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| window.key() == projection.key)
            {
                window.update_projection(projection);
            } else {
                let parent_window = self.window.window.as_deref();
                let mut window = AuxiliaryNativeWindow::new(projection, &self.options);
                window.initialize_runtime(event_loop, parent_window);
                self.auxiliary_windows.push(window);
            }
        }
        for window in &mut self.auxiliary_windows {
            if !projected_keys.iter().any(|key| key == window.key()) {
                window.hide();
            }
        }
    }
}
