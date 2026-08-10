use super::super::SurfaceRuntime;
use super::super::focus::FocusTransition;
use super::super::pointer::PointInputDispatch;
use crate::{
    gui::input::InputTimestamp,
    gui::types::Point,
    layout::LayoutInput,
    runtime::RuntimeBridge,
    widgets::PointerCapturePolicy,
    widgets::{PointerButton, PointerModifiers, PointerPressPreflight, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller::events) fn dispatch_pointer_press_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        if self.scroll_affordance_at(position).is_some()
            && self.clear_focus_with_transition() == FocusTransition::Vetoed
        {
            self.cancel_layout_pointer_capture();
            self.unwind_provisional_pointer_capture();
            return None;
        }
        if self.start_scrollbar_drag_at(position) {
            self.cancel_layout_pointer_capture();
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.reset_tooltip_hover_intent();
            self.clear_focus();
            return None;
        }
        let input =
            WidgetInput::pointer_press_with_timestamp(position, button, modifiers, timestamp);
        if self.layout_input_target_at(position)
            && let Some(widget_id) = self.interaction.pointer.capture
        {
            let routed = self.dispatch_input(widget_id, input);
            return routed.then_some(widget_id);
        }
        if self.layout_pointer_capture_active() {
            let _ = self.dispatch_captured_layout_input(
                LayoutInput::PointerPress {
                    position,
                    button,
                    modifiers,
                    timestamp,
                },
                true,
            );
            return None;
        }
        if self
            .dispatch_layout_input_at(
                position,
                LayoutInput::PointerPress {
                    position,
                    button,
                    modifiers,
                    timestamp,
                },
                true,
            )
            .handled
        {
            return None;
        }
        let Some(widget_id) = self.widget_at_for_input(position, &input) else {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.interaction.pointer.scroll_drag_capture = None;
            self.reset_tooltip_hover_intent();
            self.clear_focus();
            return None;
        };
        if self.preflight_pointer_press_for_widget(widget_id, &input)
            == PointerPressPreflight::Consume
        {
            return Some(widget_id);
        }
        self.interaction.pointer.capture = Some(widget_id);
        self.reset_tooltip_hover_intent();
        match self.dispatch_input_at_output(position, input) {
            PointInputDispatch::Routed(widget_id, _) => Some(widget_id),
            PointInputDispatch::FocusVetoed => {
                self.unwind_provisional_pointer_capture();
                None
            }
            PointInputDispatch::Miss => None,
        }
    }

    pub(in crate::runtime::controller::events) fn dispatch_pointer_double_click_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        if self.scroll_affordance_at(position).is_some()
            && self.clear_focus_with_transition() == FocusTransition::Vetoed
        {
            self.cancel_layout_pointer_capture();
            self.unwind_provisional_pointer_capture();
            return None;
        }
        if self.start_scrollbar_drag_at(position) {
            self.cancel_layout_pointer_capture();
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.reset_tooltip_hover_intent();
            self.clear_focus();
            return None;
        }
        let input = WidgetInput::pointer_double_click_with_timestamp(
            position, button, modifiers, timestamp,
        );
        if self.layout_input_target_at(position)
            && let Some(widget_id) = self.interaction.pointer.capture
        {
            let routed = self.dispatch_input(widget_id, input);
            return routed.then_some(widget_id);
        }
        if self.layout_pointer_capture_active() {
            let _ = self.dispatch_captured_layout_input(
                LayoutInput::PointerDoubleClick {
                    position,
                    button,
                    modifiers,
                    timestamp,
                },
                true,
            );
            return None;
        }
        if self
            .dispatch_layout_input_at(
                position,
                LayoutInput::PointerDoubleClick {
                    position,
                    button,
                    modifiers,
                    timestamp,
                },
                true,
            )
            .handled
        {
            return None;
        }
        let Some(widget_id) = self.widget_at_for_input(position, &input) else {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.reset_tooltip_hover_intent();
            self.clear_focus();
            return None;
        };
        self.interaction.pointer.capture = Some(widget_id);
        self.reset_tooltip_hover_intent();
        let routed = self.dispatch_input_at_output(position, input);
        match routed {
            PointInputDispatch::Routed(widget_id, true) => Some(widget_id),
            PointInputDispatch::Routed(_, false) => self
                .dispatch_input_output(
                    widget_id,
                    WidgetInput::pointer_press_with_timestamp(
                        position, button, modifiers, timestamp,
                    ),
                )
                .map(|_| widget_id),
            PointInputDispatch::FocusVetoed => {
                self.unwind_provisional_pointer_capture();
                None
            }
            PointInputDispatch::Miss => self
                .dispatch_input_output(
                    widget_id,
                    WidgetInput::pointer_press_with_timestamp(
                        position, button, modifiers, timestamp,
                    ),
                )
                .map(|_| widget_id),
        }
    }

    pub(in crate::runtime::controller::events) fn dispatch_pointer_release_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        if self
            .interaction
            .pointer
            .scroll_drag_capture
            .take()
            .is_some()
        {
            self.cancel_layout_pointer_capture();
            self.reset_tooltip_hover_intent();
            return None;
        }
        if self.interaction.pointer.capture.is_none() && self.layout_pointer_capture_active() {
            let _ = self.dispatch_captured_layout_input(
                LayoutInput::PointerRelease {
                    position,
                    button,
                    modifiers,
                    timestamp,
                },
                true,
            );
            self.rearm_tooltip_hover_intent();
            return None;
        }
        if self.interaction.pointer.capture.is_none()
            && self
                .dispatch_layout_input_at(
                    position,
                    LayoutInput::PointerRelease {
                        position,
                        button,
                        modifiers,
                        timestamp,
                    },
                    true,
                )
                .handled
        {
            self.rearm_tooltip_hover_intent();
            return None;
        }
        if matches!(button, PointerButton::Secondary | PointerButton::Auxiliary)
            && let Some(captured_id) = self.interaction.pointer.capture
            && self.widget_pointer_capture_policy(captured_id) == PointerCapturePolicy::Exclusive
            && self.preflight_pointer_press_for_widget(
                captured_id,
                &WidgetInput::pointer_press_with_timestamp(
                    position,
                    PointerButton::Primary,
                    PointerModifiers {
                        alt: true,
                        ..PointerModifiers::default()
                    },
                    None,
                ),
            ) == PointerPressPreflight::Consume
        {
            let routed = self.dispatch_input(
                captured_id,
                WidgetInput::pointer_release_with_timestamp(position, button, modifiers, timestamp),
            );
            return routed.then_some(captured_id);
        }
        let captured = self.interaction.pointer.capture.take();
        let drop_target = captured.and_then(|captured_id| {
            self.widget_at(position)
                .filter(|target_id| *target_id != captured_id)
        });
        if let Some(drop_target) = drop_target {
            let _ = self.dispatch_input(
                drop_target,
                WidgetInput::pointer_drop_with_timestamp(position, button, modifiers, timestamp),
            );
        }
        let widget_id = captured.or_else(|| self.widget_at(position))?;
        self.interaction.pointer.capture_state = None;
        let routed = self.dispatch_input(
            widget_id,
            WidgetInput::pointer_release_with_timestamp(position, button, modifiers, timestamp),
        );
        if captured.is_some() {
            self.reconcile_pointer_hover_after_capture_release(position);
        }
        self.rearm_tooltip_hover_intent();
        routed.then_some(widget_id)
    }

    pub(in crate::runtime::controller::events) fn dispatch_pointer_modifiers_changed(
        &mut self,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        if let Some(widget_id) = self.interaction.pointer.capture {
            let routed = self.dispatch_input(
                widget_id,
                WidgetInput::pointer_modifiers_changed_with_timestamp(modifiers, timestamp),
            );
            if routed {
                self.repaint_requested = true;
                return Some(widget_id);
            }
            return None;
        }
        if self.layout_pointer_capture_active() {
            let _ = self.dispatch_captured_layout_input(
                LayoutInput::PointerModifiersChanged {
                    modifiers,
                    timestamp,
                },
                true,
            );
            return None;
        }
        if let Some(position) = self.current_pointer_position()
            && self
                .dispatch_layout_input_at(
                    position,
                    LayoutInput::PointerModifiersChanged {
                        modifiers,
                        timestamp,
                    },
                    true,
                )
                .handled
        {
            return None;
        }
        let widget_id = self.interaction.hover.widget?;
        let routed = self.dispatch_input(
            widget_id,
            WidgetInput::pointer_modifiers_changed_with_timestamp(modifiers, timestamp),
        );
        if routed {
            self.repaint_requested = true;
            Some(widget_id)
        } else {
            None
        }
    }
}
