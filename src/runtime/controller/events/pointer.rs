use super::SurfaceRuntime;
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn dispatch_pointer_press_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Option<WidgetId> {
        if self.start_scrollbar_drag_at(position) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.clear_focus();
            return None;
        }
        let Some(widget_id) = self.widget_at(position) else {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.interaction.pointer.scroll_drag_capture = None;
            self.clear_focus();
            return None;
        };
        self.interaction.pointer.capture = Some(widget_id);
        self.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button,
                modifiers,
            },
        )
    }

    pub(super) fn dispatch_pointer_double_click_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Option<WidgetId> {
        let Some(widget_id) = self.widget_at(position) else {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
            self.clear_focus();
            return None;
        };
        self.interaction.pointer.capture = Some(widget_id);
        let routed = self.dispatch_input_at_output(
            position,
            WidgetInput::PointerDoubleClick {
                position,
                button,
                modifiers,
            },
        );
        match routed {
            Some((widget_id, true)) => Some(widget_id),
            _ => self.dispatch_input_at(
                position,
                WidgetInput::PointerPress {
                    position,
                    button,
                    modifiers,
                },
            ),
        }
    }

    pub(super) fn dispatch_pointer_release_event(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Option<WidgetId> {
        if self
            .interaction
            .pointer
            .scroll_drag_capture
            .take()
            .is_some()
        {
            return None;
        }
        let captured = self.interaction.pointer.capture.take();
        let drop_target = captured.and_then(|captured_id| {
            self.widget_at(position)
                .filter(|target_id| *target_id != captured_id)
        });
        if let Some(drop_target) = drop_target {
            let _ = self.dispatch_input(
                drop_target,
                WidgetInput::PointerDrop {
                    position,
                    button,
                    modifiers,
                },
            );
        }
        let widget_id = captured.or_else(|| self.widget_at(position))?;
        self.interaction.pointer.capture_state = None;
        self.dispatch_input(
            widget_id,
            WidgetInput::PointerRelease {
                position,
                button,
                modifiers,
            },
        )
        .then_some(widget_id)
    }
}
