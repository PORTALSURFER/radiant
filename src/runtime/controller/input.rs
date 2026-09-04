use super::SurfaceRuntime;
use crate::{
    gui::types::Rect,
    runtime::{RuntimeBridge, SurfaceWidget, WidgetDispatchResult},
    widgets::{CompositionSample, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn dispatch_surface_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        if matches!(
            &input,
            WidgetInput::PointerMove { .. }
                | WidgetInput::PointerPress { .. }
                | WidgetInput::PointerDoubleClick { .. }
                | WidgetInput::PointerRelease { .. }
        ) {
            self.apply_native_text_pointer_caret(widget_id);
        } else {
            self.pending_native_text_pointer_caret = None;
        }
        if let WidgetInput::FocusChanged(focused) = input {
            return self.dispatch_surface_focus_changed(widget_id, bounds, focused);
        }
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_input_message(widget_id, bounds, input);
        };
        self.surface
            .dispatch_widget_input_message_at_path(widget_id, child_path, bounds, input)
    }

    fn apply_native_text_pointer_caret(&mut self, widget_id: WidgetId) {
        let Some((pending_widget_id, source, caret, affinity)) =
            self.pending_native_text_pointer_caret.take()
        else {
            return;
        };
        if pending_widget_id != widget_id {
            return;
        }
        let Some(widget) = self.surface_widget_mut(widget_id) else {
            return;
        };
        let Some(text_input) = widget
            .widget_object_mut_runtime()
            .as_any_mut()
            .downcast_mut::<crate::widgets::TextInputWidget>()
        else {
            return;
        };
        if text_input.state.value == source {
            text_input.set_native_pointer_caret(caret, affinity);
        }
    }

    fn dispatch_surface_focus_changed(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        focused: bool,
    ) -> Option<WidgetDispatchResult<Message>> {
        let now = self.timed_repaint_now();
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_focus_changed_message_at(widget_id, bounds, focused, now);
        };
        self.surface.dispatch_widget_focus_changed_message_at_path(
            widget_id, child_path, bounds, focused, now,
        )
    }

    pub(super) fn dispatch_surface_composition_sample(
        &mut self,
        widget_id: WidgetId,
        sample: CompositionSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            self.surface
                .dispatch_widget_composition_sample_message_at_path(widget_id, child_path, sample)
        } else {
            self.surface
                .dispatch_widget_composition_sample_message(widget_id, sample)
        }
    }

    pub(super) fn dispatch_surface_hidden_composition_update(
        &mut self,
        widget_id: WidgetId,
        preedit: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            self.surface
                .dispatch_widget_hidden_composition_update_message_at_path(
                    widget_id, child_path, preedit, timestamp,
                )
        } else {
            self.surface
                .dispatch_widget_hidden_composition_update_message(widget_id, preedit, timestamp)
        }
    }

    pub(super) fn dispatch_surface_pointer_capture_cancelled(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
    ) -> Option<WidgetDispatchResult<Message>> {
        let now = self.timed_repaint_now();
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_pointer_capture_cancelled_message_at(widget_id, bounds, now);
        };
        self.surface
            .dispatch_widget_pointer_capture_cancelled_message_at_path_with_clock(
                widget_id, child_path, bounds, now,
            )
    }

    pub(super) fn surface_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.traversal
            .widgets
            .paths
            .current
            .get(&widget_id)
            .and_then(|child_path| self.surface.find_widget_at_path(widget_id, child_path))
            .or_else(|| self.surface.find_widget(widget_id))
    }

    pub(super) fn surface_widget_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> Option<&mut SurfaceWidget<Message>> {
        let surface = &mut self.surface;
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            return surface.find_widget_mut_at_path(widget_id, child_path);
        }
        surface.find_widget_mut(widget_id)
    }
}
