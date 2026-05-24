mod layout;
mod lifecycle;
mod traversal;

use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn capture_pointer_capture_state(&mut self, widget_id: WidgetId) {
        if self.interaction.pointer.capture != Some(widget_id) {
            return;
        }
        let Some(widget) = self.surface_widget(widget_id) else {
            self.interaction.pointer.capture_state = None;
            return;
        };
        self.interaction.pointer.capture_state =
            Some((widget_id, widget.widget_object().common().state));
    }

    pub(super) fn restore_pointer_capture_state(&mut self) {
        let Some((widget_id, state)) = self.interaction.pointer.capture_state else {
            return;
        };
        if self.interaction.pointer.capture != Some(widget_id) {
            self.interaction.pointer.capture_state = None;
            return;
        }
        let Some(widget) = self.surface_widget_mut(widget_id) else {
            self.interaction.pointer.capture_state = None;
            return;
        };
        widget.widget_object_mut().common_mut().state = state;
    }
}
