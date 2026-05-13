//! Backend-neutral event routing helpers for the generic native runner.

use super::{GenericNativeRuntimeCore, GenericRouteOutcome};
use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    types::{Point, Vector2},
};
use crate::runtime::{Event, RuntimeBridge};
use crate::widgets::{PointerButton, TextEditCommand, WidgetInput, WidgetKey};

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
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
            .dispatch_event(Event::PointerMove { position })
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
            .dispatch_event(Event::PointerPress { position, button })
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
            .dispatch_event(Event::PointerRelease { position, button })
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
