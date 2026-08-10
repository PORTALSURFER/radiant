//! Backend-neutral event routing helpers for the generic native runner.

use super::{
    FrameWorkReason, GenericNativeRuntimeCore, GenericRouteOutcome, PointerPressStamp,
    pointer_press_event,
};
use crate::gui::{
    focus::FocusSurface,
    input::{InputSequenceRange, InputTimestamp, KeyPress},
    types::{Point, Vector2},
};
use crate::runtime::WheelOrScrollRoute;
use crate::runtime::{Event, RepaintScope, RuntimeBridge};
use crate::widgets::{
    KeyboardModifiers, PointerButton, PointerModifiers, TextEditCommand, WheelSample, WidgetInput,
    WidgetKey,
};
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
        self.route_pointer_move_with_metadata(position, PointerModifiers::default(), None, None)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_move_with_metadata(
        &mut self,
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> GenericRouteOutcome {
        let outcome = self
            .runtime
            .dispatch_pointer_move_deferred_refresh_with_metadata(
                position,
                modifiers,
                timestamp,
                sequence_range,
            );
        let pending = self.runtime.take_pending_input_command_outcome();
        let captured_pointer_refresh =
            outcome.pointer_captured && pending.surface_refresh_requested;
        let scroll_drag_surface_refresh =
            self.runtime.scrollbar_drag_active() && pending.surface_refresh_requested;
        let surface_refresh_scope = pending
            .surface_invalidation()
            .repaint_scope()
            .unwrap_or(RepaintScope::Surface);
        if pending.surface_refresh_requested
            && outcome.hover_changed
            && !captured_pointer_refresh
            && !scroll_drag_surface_refresh
        {
            self.runtime.refresh_with_scope(surface_refresh_scope);
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
            route_outcome.request_interactive_surface_refresh_with_scope(
                FrameWorkReason::InteractiveSurfaceRefresh,
                surface_refresh_scope,
            );
        }
        if pending.surface_refresh_requested
            && !outcome.hover_changed
            && !captured_pointer_refresh
            && !scroll_drag_surface_refresh
        {
            route_outcome.request_surface_refresh_with_scope(
                FrameWorkReason::DeferredSurfaceRefresh,
                surface_refresh_scope,
            );
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
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::pointer_modifiers_changed_with_timestamp(
                modifiers, timestamp,
            ))
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

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_pointer_press_with_modifiers(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        self.route_pointer_press_with_timestamp(position, button, modifiers, None)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_press_with_timestamp(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let now = Instant::now();
        let event = pointer_press_event(
            self.last_pointer_press,
            now,
            position,
            button,
            modifiers,
            timestamp,
        );
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

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_pointer_release_with_modifiers(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        self.route_pointer_release_with_timestamp(position, button, modifiers, None)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_release_with_timestamp(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::pointer_release_with_timestamp(
                position, button, modifiers, timestamp,
            ))
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
        self.route_scroll_with_metadata(position, delta, modifiers, None, None)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_scroll_with_metadata(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> GenericRouteOutcome {
        let routed = self.runtime.wheel_or_scroll_at_with_metadata(
            position,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        );
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_scroll_deferred_refresh_with_modifiers(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> GenericRouteOutcome {
        let route = self
            .runtime
            .wheel_or_scroll_route_deferred_refresh_with_modifiers(position, delta, modifiers);
        self.complete_scroll_route(route)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll_deferred_refresh_with_metadata(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> GenericRouteOutcome {
        let route = self
            .runtime
            .wheel_or_scroll_route_deferred_refresh_with_metadata(
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            );
        self.complete_scroll_route(route)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll_deferred_refresh_with_sample(
        &mut self,
        position: Point,
        sample: WheelSample,
    ) -> GenericRouteOutcome {
        let route = self
            .runtime
            .wheel_or_scroll_route_deferred_refresh_with_sample(position, sample);
        self.complete_scroll_route(route)
    }

    fn complete_scroll_route(&mut self, route: WheelOrScrollRoute) -> GenericRouteOutcome {
        let pending = self.runtime.take_pending_input_command_outcome();
        let repaint_requested = self.runtime.take_repaint_requested();
        let exit_requested = self.runtime.take_exit_requested();
        let routed = route != WheelOrScrollRoute::NotRouted;
        let scroll_surface_refresh =
            route == WheelOrScrollRoute::ScrollContainer && pending.surface_refresh_requested;
        let deferred_surface_refresh = pending.surface_refresh_requested && !scroll_surface_refresh;
        let surface_refresh_scope = pending
            .surface_invalidation()
            .repaint_scope()
            .unwrap_or(RepaintScope::Surface);
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
            outcome.request_interactive_surface_refresh_with_scope(
                FrameWorkReason::InteractiveSurfaceRefresh,
                surface_refresh_scope,
            );
        }
        if deferred_surface_refresh {
            outcome.request_surface_refresh_with_scope(
                FrameWorkReason::DeferredSurfaceRefresh,
                surface_refresh_scope,
            );
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

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
    ) -> GenericRouteOutcome {
        self.route_key_press_with_timestamp(
            press,
            widget_key,
            KeyboardModifiers::from(press),
            None,
            false,
        )
    }

    pub(in crate::gui_runtime::native_vello) fn route_key_press_with_timestamp(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        widget_modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        repeat: bool,
    ) -> GenericRouteOutcome {
        let routed = self.runtime.dispatch_key_press_with_timestamp(
            press,
            widget_key,
            FocusSurface::None,
            widget_modifiers,
            timestamp,
            repeat,
        );
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_metadata_key_press_with_timestamp(
        &mut self,
        press: Option<KeyPress>,
        widget_key: Option<WidgetKey>,
        widget_modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        repeat: bool,
    ) -> Option<GenericRouteOutcome> {
        let route = self.runtime.dispatch_metadata_focused_key_press(
            press,
            widget_key,
            widget_modifiers,
            timestamp,
            repeat,
            FocusSurface::None,
        )?;
        Some(self.route_outcome(route.routed))
    }

    pub(in crate::gui_runtime::native_vello) fn route_key_release_with_metadata(
        &mut self,
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        if let Some(outcome) =
            self.route_metadata_key_release_with_metadata(Some(key), modifiers, timestamp)
        {
            return outcome;
        }
        let routed = self
            .runtime
            .dispatch_focused_input(WidgetInput::key_release_with_metadata(
                key, modifiers, timestamp,
            ))
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_metadata_key_release_with_metadata(
        &mut self,
        key: Option<WidgetKey>,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<GenericRouteOutcome> {
        let route = self
            .runtime
            .dispatch_metadata_focused_key_release(key, modifiers, timestamp)?;
        Some(self.route_outcome(route.routed))
    }

    pub(in crate::gui_runtime::native_vello) fn route_focus_lost(&mut self) -> GenericRouteOutcome {
        self.runtime.clear_focus();
        self.runtime.cancel_pointer_capture();
        self.route_outcome(true)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_widget_key(
        &mut self,
        key: WidgetKey,
    ) -> GenericRouteOutcome {
        self.route_widget_key_with_timestamp(key, None)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_widget_key_with_timestamp(
        &mut self,
        key: WidgetKey,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        self.route_widget_key_with_metadata(key, KeyboardModifiers::default(), false, timestamp)
    }

    pub(in crate::gui_runtime::native_vello) fn route_widget_key_with_metadata(
        &mut self,
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        repeat: bool,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::key_press_with_metadata(
                key, modifiers, repeat, timestamp,
            ))
            .is_some();
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> GenericRouteOutcome {
        self.route_text_edit_with_timestamp(command, None)
    }

    pub(in crate::gui_runtime::native_vello) fn route_text_edit_with_timestamp(
        &mut self,
        command: TextEditCommand,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        if self.runtime.focused_text_input_id().is_none() {
            return self.route_outcome(false);
        }
        let routed = self
            .runtime
            .dispatch_focused_input(WidgetInput::text_edit_with_timestamp(command, timestamp))
            .is_some();
        self.route_outcome(routed)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn route_character(
        &mut self,
        character: char,
    ) -> GenericRouteOutcome {
        self.route_character_with_timestamp(character, None)
    }

    pub(in crate::gui_runtime::native_vello) fn route_character_with_timestamp(
        &mut self,
        character: char,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(Event::character_with_timestamp(character, timestamp))
            .is_some();
        self.route_outcome(routed)
    }
}
