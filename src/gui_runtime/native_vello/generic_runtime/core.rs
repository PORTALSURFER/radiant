//! Runtime state and event routing for the generic native Vello runner.

use super::GenericRouteOutcome;
use crate::gui::types::{Point, Vector2};
use crate::runtime::{RuntimeAnimationActivity, RuntimeBridge, SurfaceRuntime};
use crate::theme::ThemeTokens;
use crate::widgets::PointerButton;
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

    pub(in crate::gui_runtime::native_vello) fn new_with_debug_layout(
        bridge: Bridge,
        viewport: Vector2,
        debug_layout: bool,
    ) -> Self {
        let mut runtime = SurfaceRuntime::new(bridge, viewport);
        if debug_layout {
            runtime.set_layout_debug_options(crate::layout::LayoutDebugOptions::bounds_only());
        }
        Self {
            runtime,
            last_pointer_press: None,
            theme: ThemeTokens::default(),
        }
    }

    pub(super) fn set_viewport(&mut self, viewport: Vector2) {
        let _ = self
            .runtime
            .dispatch_event(crate::runtime::Event::Resize { viewport });
    }

    #[cfg(test)]
    pub(super) fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime.paint_plan(&self.theme)
    }

    pub(super) fn paint_plan_into(&self, plan: &mut crate::runtime::SurfacePaintPlan) {
        self.runtime.paint_plan_into(&self.theme, plan);
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
        needs_scene_animation: bool,
    ) -> GenericRouteOutcome {
        if animation_activity.needs_frame_message() {
            self.queue_animation_frame();
        }
        let mut outcome = self.drain_runtime_messages();
        if !outcome.needs_redraw() && needs_scene_animation {
            outcome.redraw_requested = true;
        } else if !outcome.needs_redraw() && animation_activity.needs_animation() {
            outcome.paint_only_requested = true;
        }
        outcome
    }

    pub(in crate::gui_runtime::native_vello) fn drain_runtime_messages(
        &mut self,
    ) -> GenericRouteOutcome {
        let outcome = self.runtime.drain_runtime_messages();
        let _ = self.runtime.take_repaint_requested();
        GenericRouteOutcome {
            routed: outcome.messages_dispatched > 0,
            redraw_requested: outcome.surface_refresh_requested,
            repaint_requested: outcome.surface_repaint_requested,
            paint_only_requested: outcome.paint_only_requested,
            exit_requested: outcome.exit_requested,
            runtime_work_remaining: outcome.runtime_work_remaining,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn focused_text_selection(&self) -> Option<String> {
        self.runtime.focused_text_selection()
    }

    pub(in crate::gui_runtime::native_vello) fn has_focused_text_input(&self) -> bool {
        self.runtime.focused_text_input_id().is_some()
    }
}
