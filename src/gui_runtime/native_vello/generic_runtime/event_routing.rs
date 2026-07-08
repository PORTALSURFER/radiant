//! Backend-neutral event routing helpers for the generic native runner.

use super::{
    FrameWorkReason, GenericNativeRuntimeCore, GenericRouteOutcome, PointerPressStamp,
    pointer_press_event,
};
use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    types::{Point, Vector2},
};
use crate::runtime::WheelOrScrollRoute;
use crate::runtime::{Event, RuntimeBridge};
use crate::widgets::{PointerButton, PointerModifiers, TextEditCommand, WidgetInput, WidgetKey};
use std::time::Instant;

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn route_outcome(&mut self, routed: bool) -> GenericRouteOutcome {
        let pending = self.runtime.take_pending_input_command_outcome();
        let repaint_requested = self.runtime.take_repaint_requested();
        let mut outcome = GenericRouteOutcome {
            routed,
            exit_requested: self.runtime.take_exit_requested() || pending.exit_requested,
            runtime_work_remaining: pending.runtime_work_remaining,
            dpi_scale_override: pending.dpi_scale_override,
            window_logical_size: pending.window_logical_size,
            ..GenericRouteOutcome::default()
        };
        if routed {
            outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);
        }
        if pending.surface_refresh_requested {
            outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRefresh);
        }
        if repaint_requested || pending.surface_repaint_requested {
            outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        }
        if pending.paint_only_requested {
            outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        }
        if pending.window_logical_size.is_some() {
            outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        }
        if outcome.exit_requested {
            outcome.request_exit();
        }
        outcome
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_move(
        &mut self,
        position: Point,
    ) -> GenericRouteOutcome {
        let outcome = self
            .runtime
            .dispatch_pointer_move_deferred_refresh_with_outcome(position);
        let pending = self.runtime.take_pending_input_command_outcome();
        let captured_pointer_refresh =
            outcome.pointer_captured && pending.surface_refresh_requested;
        let scroll_drag_surface_refresh =
            self.runtime.scrollbar_drag_active() && pending.surface_refresh_requested;
        if pending.surface_refresh_requested
            && outcome.hover_changed
            && !captured_pointer_refresh
            && !scroll_drag_surface_refresh
        {
            self.runtime.refresh();
        }
        let mut route_outcome = GenericRouteOutcome {
            routed: outcome.routed(),
            exit_requested: outcome.exit_requested || pending.exit_requested,
            runtime_work_remaining: pending.runtime_work_remaining,
            dpi_scale_override: pending.dpi_scale_override,
            window_logical_size: pending.window_logical_size,
            ..GenericRouteOutcome::default()
        };
        if outcome.hover_changed {
            route_outcome.request_scene_rebuild(FrameWorkReason::PointerHover);
        }
        if captured_pointer_refresh || scroll_drag_surface_refresh {
            route_outcome
                .request_interactive_surface_refresh(FrameWorkReason::InteractiveSurfaceRefresh);
        }
        if pending.surface_refresh_requested
            && !outcome.hover_changed
            && !captured_pointer_refresh
            && !scroll_drag_surface_refresh
        {
            route_outcome.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        }
        if outcome.repaint_requested || pending.surface_repaint_requested {
            route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        }
        if outcome.paint_only_requested || pending.paint_only_requested {
            route_outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        }
        if pending.window_logical_size.is_some() {
            route_outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        }
        if route_outcome.exit_requested {
            route_outcome.request_exit();
        }
        route_outcome
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_modifiers_changed(
        &mut self,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::PointerModifiersChanged { modifiers })
            .is_some();
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_pointer_press(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        self.route_pointer_press_with_modifiers(position, button, PointerModifiers::default())
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_press_with_modifiers(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let now = Instant::now();
        let event = pointer_press_event(self.last_pointer_press, now, position, button, modifiers);
        self.last_pointer_press = Some(PointerPressStamp {
            at: now,
            position,
            button,
        });
        let routed = self.runtime.dispatch_event(event).is_some();
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_pointer_release(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        self.route_pointer_release_with_modifiers(position, button, PointerModifiers::default())
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_release_with_modifiers(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::PointerRelease {
                position,
                button,
                modifiers,
            })
            .is_some();
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_scroll_with_modifiers(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .wheel_or_scroll_at_with_modifiers(position, delta, modifiers);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll_deferred_refresh_with_modifiers(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let route = self
            .runtime
            .wheel_or_scroll_route_deferred_refresh_with_modifiers(position, delta, modifiers);
        let pending = self.runtime.take_pending_input_command_outcome();
        let repaint_requested = self.runtime.take_repaint_requested();
        let exit_requested = self.runtime.take_exit_requested();
        let routed = route != WheelOrScrollRoute::NotRouted;
        let scroll_surface_refresh =
            route == WheelOrScrollRoute::ScrollContainer && pending.surface_refresh_requested;
        let deferred_surface_refresh = pending.surface_refresh_requested && !scroll_surface_refresh;
        let mut outcome = GenericRouteOutcome {
            routed,
            exit_requested: exit_requested || pending.exit_requested,
            runtime_work_remaining: pending.runtime_work_remaining,
            dpi_scale_override: pending.dpi_scale_override,
            window_logical_size: pending.window_logical_size,
            ..GenericRouteOutcome::default()
        };
        if routed && !deferred_surface_refresh {
            outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);
        }
        if scroll_surface_refresh {
            outcome.request_interactive_surface_refresh(FrameWorkReason::InteractiveSurfaceRefresh);
        }
        if deferred_surface_refresh {
            outcome.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
        }
        if (repaint_requested || pending.surface_repaint_requested)
            && !deferred_surface_refresh
            && !scroll_surface_refresh
        {
            outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        }
        if pending.paint_only_requested {
            outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        }
        if pending.window_logical_size.is_some() {
            outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        }
        if outcome.exit_requested {
            outcome.request_exit();
        }
        outcome
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

    pub(in crate::gui_runtime::native_vello) fn route_focus_lost(&mut self) -> GenericRouteOutcome {
        self.runtime.clear_focus();
        self.runtime.cancel_pointer_capture();
        self.route_outcome(true)
    }

    pub(in crate::gui_runtime::native_vello) fn route_widget_key(
        &mut self,
        key: WidgetKey,
    ) -> GenericRouteOutcome {
        let routed = self.runtime.dispatch_event(Event::KeyPress(key)).is_some();
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

    pub(in crate::gui_runtime::native_vello) fn route_character(
        &mut self,
        character: char,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::Character(character))
            .is_some();
        self.route_outcome(routed)
    }
}
