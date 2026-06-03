use super::{UiSurface, WidgetDispatchResult, WidgetPath};
use crate::widgets::{WidgetId, WidgetInput, WidgetOutput};

impl<Message> UiSurface<Message> {
    /// Map one widget output back into a host-defined message.
    pub fn dispatch_widget_output(
        &self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message> {
        self.root.dispatch_output(widget_id, &output)
    }

    /// Route one backend-neutral interaction into a projected widget.
    pub fn dispatch_widget_input(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.root.handle_input(widget_id, bounds, input)
    }

    pub(in crate::runtime) fn dispatch_widget_input_message(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_input(widget_id, bounds, input))
    }

    pub(in crate::runtime) fn dispatch_widget_input_message_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.root
            .dispatch_input_at_path(widget_id, child_path.as_slice(), bounds, input)
    }
}
