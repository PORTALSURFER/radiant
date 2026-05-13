//! Runtime state and event routing for the generic native Vello runner.

use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    types::{Point, Vector2},
};
use crate::runtime::{RuntimeBridge, SurfaceRuntime};
use crate::theme::ThemeTokens;
use crate::widgets::{PointerButton, TextEditCommand, WidgetInput, WidgetKey};

pub(in crate::gui_runtime::native_vello) struct GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) runtime: SurfaceRuntime<Bridge, Message>,
    theme: ThemeTokens,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) routed: bool,
    pub(in crate::gui_runtime::native_vello) redraw_requested: bool,
    pub(in crate::gui_runtime::native_vello) repaint_requested: bool,
    pub(in crate::gui_runtime::native_vello) paint_only_requested: bool,
    pub(in crate::gui_runtime::native_vello) exit_requested: bool,
    pub(in crate::gui_runtime::native_vello) runtime_work_remaining: bool,
}

impl GenericRouteOutcome {
    pub(super) fn needs_redraw(self) -> bool {
        self.needs_scene_rebuild() || self.paint_only_requested
    }

    pub(super) fn needs_scene_rebuild(self) -> bool {
        self.redraw_requested || self.repaint_requested
    }

    pub(super) fn merge(&mut self, other: Self) {
        self.routed |= other.routed;
        self.redraw_requested |= other.redraw_requested;
        self.repaint_requested |= other.repaint_requested;
        self.paint_only_requested |= other.paint_only_requested;
        self.exit_requested |= other.exit_requested;
        self.runtime_work_remaining |= other.runtime_work_remaining;
    }
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

    pub(super) fn needs_animation(&mut self) -> bool {
        self.runtime.bridge_mut().needs_animation()
    }

    pub(super) fn queue_animation_frame(&mut self) -> bool {
        self.runtime.bridge_mut().queue_animation_frame()
    }

    fn route_outcome(&mut self, routed: bool) -> GenericRouteOutcome {
        GenericRouteOutcome {
            routed,
            redraw_requested: routed,
            repaint_requested: self.runtime.take_repaint_requested(),
            paint_only_requested: false,
            exit_requested: self.runtime.take_exit_requested(),
            runtime_work_remaining: false,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_move(
        &mut self,
        position: Point,
    ) -> GenericRouteOutcome {
        let previous_hovered_widget = self.runtime.hovered_widget();
        let previous_hovered_container = self.runtime.hovered_container();
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerMove { position })
            .is_some();
        let repaint_requested = self.runtime.take_repaint_requested();
        let exit_requested = self.runtime.take_exit_requested();
        let hover_changed = previous_hovered_widget != self.runtime.hovered_widget()
            || previous_hovered_container != self.runtime.hovered_container();
        GenericRouteOutcome {
            routed,
            redraw_requested: hover_changed || self.runtime.pointer_capture().is_some(),
            repaint_requested,
            paint_only_requested: false,
            exit_requested,
            runtime_work_remaining: false,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_press(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerPress { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_release(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerRelease { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll(
        &mut self,
        position: Point,
        delta: Vector2,
    ) -> GenericRouteOutcome {
        let routed = self.runtime.wheel_or_scroll_at(position, delta);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll_deferred_refresh(
        &mut self,
        position: Point,
        delta: Vector2,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .wheel_or_scroll_at_deferred_refresh(position, delta);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_key_press(press, widget_key, FocusSurface::None);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_widget_key(
        &mut self,
        key: WidgetKey,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::KeyPress(key))
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> GenericRouteOutcome {
        if self.runtime.focused_text_input_id().is_none() {
            return self.route_outcome(false);
        }
        let routed = self
            .runtime
            .dispatch_focused_input(WidgetInput::TextEdit(command))
            .is_some();
        self.route_outcome(routed)
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

    pub(in crate::gui_runtime::native_vello) fn route_character(
        &mut self,
        character: char,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::Character(character))
            .is_some();
        self.route_outcome(routed)
    }
}
