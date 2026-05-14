//! Backend-neutral event routing helpers for the generic native runner.

use super::{GenericNativeRuntimeCore, GenericRouteOutcome, PointerPressStamp};
use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    types::{Point, Vector2},
};
use crate::runtime::{Event, RuntimeBridge};
use crate::widgets::{PointerButton, PointerModifiers, TextEditCommand, WidgetInput, WidgetKey};
use std::time::{Duration, Instant};

const DOUBLE_CLICK_MAX_INTERVAL: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 5.0;

fn is_double_click(
    last: PointerPressStamp,
    now: Instant,
    position: Point,
    button: PointerButton,
) -> bool {
    if last.button != button || now.duration_since(last.at) > DOUBLE_CLICK_MAX_INTERVAL {
        return false;
    }
    let dx = position.x - last.position.x;
    let dy = position.y - last.position.y;
    (dx * dx + dy * dy) <= DOUBLE_CLICK_MAX_DISTANCE * DOUBLE_CLICK_MAX_DISTANCE
}

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
        let outcome = self.runtime.dispatch_pointer_move_with_outcome(position);
        GenericRouteOutcome {
            routed: outcome.routed(),
            redraw_requested: outcome.hover_changed,
            repaint_requested: outcome.repaint_requested,
            paint_only_requested: outcome.paint_only_requested,
            exit_requested: outcome.exit_requested,
            runtime_work_remaining: false,
        }
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
        let is_double_click = self
            .last_pointer_press
            .is_some_and(|last| is_double_click(last, now, position, button));
        self.last_pointer_press = Some(PointerPressStamp {
            at: now,
            position,
            button,
        });
        let event = if is_double_click {
            Event::PointerDoubleClick {
                position,
                button,
                modifiers,
            }
        } else {
            Event::PointerPress {
                position,
                button,
                modifiers,
            }
        };
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
