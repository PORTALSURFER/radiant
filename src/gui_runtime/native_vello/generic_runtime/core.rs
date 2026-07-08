//! Runtime state and event routing for the generic native Vello runner.

use super::{FrameWorkReason, GenericRouteOutcome};
use crate::gui::types::{Point, Vector2};
use crate::runtime::{
    CommandOutcome, DevtoolsOverlayOptions, RuntimeAnimationActivity, RuntimeBridge, SurfaceRuntime,
};
use crate::theme::ThemeTokens;
use crate::widgets::{PointerButton, WidgetKey};
use std::time::Instant;

pub(in crate::gui_runtime::native_vello) struct GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) runtime: SurfaceRuntime<Bridge, Message>,
    pub(in crate::gui_runtime::native_vello) last_pointer_press: Option<PointerPressStamp>,
    theme: ThemeTokens,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gui_runtime::native_vello) struct PointerPressStamp {
    pub(in crate::gui_runtime::native_vello) at: Instant,
    pub(in crate::gui_runtime::native_vello) position: Point,
    pub(in crate::gui_runtime::native_vello) button: PointerButton,
}

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn new(bridge: Bridge, viewport: Vector2) -> Self {
        Self::new_with_debug_layout(bridge, viewport, false)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn new_with_debug_layout(
        bridge: Bridge,
        viewport: Vector2,
        debug_layout: bool,
    ) -> Self {
        Self::new_with_frame_options(
            bridge,
            viewport,
            debug_layout,
            DevtoolsOverlayOptions::default(),
        )
    }

    pub(in crate::gui_runtime::native_vello) fn new_with_frame_options(
        bridge: Bridge,
        viewport: Vector2,
        debug_layout: bool,
        devtools_overlay: DevtoolsOverlayOptions,
    ) -> Self {
        let mut runtime = SurfaceRuntime::new(bridge, viewport);
        if debug_layout {
            runtime.set_layout_debug_options(crate::layout::LayoutDebugOptions::bounds_only());
        }
        runtime.set_devtools_overlay_options(devtools_overlay);
        Self {
            runtime,
            last_pointer_press: None,
            theme: ThemeTokens::default(),
        }
    }

    pub(super) fn set_viewport(&mut self, viewport: Vector2) -> bool {
        self.runtime.set_viewport_and_report_relayout(viewport)
    }

    pub(super) fn set_current_pointer_position(&mut self, position: Option<Point>) {
        self.runtime.set_current_pointer_position(position);
    }

    #[cfg(test)]
    pub(super) fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime.paint_plan(&self.theme)
    }

    pub(super) fn paint_plan_into(&self, plan: &mut crate::runtime::SurfacePaintPlan) {
        self.runtime.base_paint_plan_into(&self.theme, plan);
    }

    pub(super) fn paint_transient_overlay(
        &mut self,
        plan: &crate::runtime::SurfacePaintPlan,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        animation_time: std::time::Duration,
    ) {
        let viewport = self.runtime.viewport();
        self.runtime.bridge_mut().paint_transient_overlay(
            crate::runtime::TransientOverlayContext::new(plan, viewport, animation_time),
            primitives,
        );
    }

    pub(super) fn has_transient_overlay_painter(&self) -> bool {
        self.runtime.bridge().has_transient_overlay_painter()
    }

    pub(super) fn paint_runtime_overlay(
        &self,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
    ) {
        self.runtime
            .runtime_overlay_paint_into(&self.theme, primitives);
    }

    pub(super) fn has_runtime_overlay_paint(&self) -> bool {
        self.runtime.has_runtime_overlay_paint()
    }

    pub(super) fn has_frame_diagnostics_observer(&self) -> bool {
        self.runtime.bridge().has_frame_diagnostics_observer()
    }

    pub(super) fn refresh_surface(&mut self) {
        self.runtime.refresh();
    }

    pub(super) fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.runtime.bridge_mut().animation_activity()
    }

    pub(super) fn queue_animation_frame(&mut self) -> bool {
        self.runtime.bridge_mut().queue_animation_frame()
    }

    pub(super) fn drain_timed_frame(
        &mut self,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        if animation_activity.needs_frame_message() {
            self.queue_animation_frame();
        }
        let mut outcome = self.drain_runtime_messages();
        if !outcome.needs_redraw() && needs_text_caret_animation {
            outcome.request_scene_rebuild(FrameWorkReason::TextCaretAnimation);
        } else if !outcome.needs_redraw() && animation_activity.needs_animation() {
            outcome.request_paint_only(FrameWorkReason::TimedPaintOnlyAnimation);
        }
        outcome
    }

    pub(in crate::gui_runtime::native_vello) fn drain_runtime_messages(
        &mut self,
    ) -> GenericRouteOutcome {
        let outcome = self.runtime.drain_runtime_messages();
        self.route_command_outcome(outcome)
    }

    pub(in crate::gui_runtime::native_vello) fn route_command_outcome(
        &mut self,
        outcome: CommandOutcome,
    ) -> GenericRouteOutcome {
        let _ = self.runtime.take_repaint_requested();
        let mut route_outcome = GenericRouteOutcome {
            routed: outcome.messages_dispatched > 0,
            exit_requested: outcome.exit_requested,
            runtime_work_remaining: outcome.runtime_work_remaining,
            dpi_scale_override: outcome.dpi_scale_override,
            window_logical_size: outcome.window_logical_size,
            ..GenericRouteOutcome::default()
        };
        if outcome.surface_refresh_requested {
            route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRefresh);
        }
        if outcome.surface_repaint_requested {
            route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        }
        if outcome.paint_only_requested {
            route_outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        }
        if outcome.window_logical_size.is_some() {
            route_outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        }
        if outcome.exit_requested {
            route_outcome.request_exit();
        }
        route_outcome
    }

    pub(in crate::gui_runtime::native_vello) fn focused_text_selection(&self) -> Option<String> {
        self.runtime.focused_text_selection()
    }

    pub(in crate::gui_runtime::native_vello) fn has_focused_text_input(&self) -> bool {
        self.runtime.focused_text_input_id().is_some()
    }

    pub(in crate::gui_runtime::native_vello) fn focused_widget_preempts_host_shortcut_key(
        &self,
        key: WidgetKey,
    ) -> bool {
        self.runtime.focused_widget_preempts_host_shortcut_key(key)
    }
}
