use super::{UiSurface, WidgetDispatchResult, WidgetPath};
use crate::widgets::{CompositionSample, WidgetId, WidgetInput, WidgetOutput};
use std::time::Instant;

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

    pub(in crate::runtime) fn dispatch_widget_focus_changed_message_at(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        focused: bool,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_focus_changed_at(widget_id, bounds, focused, now))
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

    pub(in crate::runtime) fn dispatch_widget_focus_changed_message_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        bounds: crate::gui::types::Rect,
        focused: bool,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.root.dispatch_focus_changed_at_path(
            widget_id,
            child_path.as_slice(),
            bounds,
            focused,
            now,
        )
    }

    pub(in crate::runtime) fn dispatch_widget_composition_sample_message(
        &mut self,
        widget_id: WidgetId,
        sample: CompositionSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_composition_sample(widget_id, sample))
    }

    pub(in crate::runtime) fn dispatch_widget_composition_sample_message_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        sample: CompositionSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.root
            .dispatch_composition_sample_at_path(widget_id, child_path.as_slice(), sample)
    }

    pub(in crate::runtime) fn dispatch_widget_hidden_composition_update_message(
        &mut self,
        widget_id: WidgetId,
        preedit: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_hidden_composition_update(widget_id, preedit, timestamp))
    }

    pub(in crate::runtime) fn dispatch_widget_hidden_composition_update_message_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        preedit: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.root.dispatch_hidden_composition_update_at_path(
            widget_id,
            child_path.as_slice(),
            preedit,
            timestamp,
        )
    }

    pub(in crate::runtime) fn dispatch_widget_pointer_capture_cancelled_message_at(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_pointer_capture_cancelled_at(widget_id, bounds, now))
    }

    pub(in crate::runtime) fn dispatch_widget_pointer_capture_cancelled_message_at_path_with_clock(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        bounds: crate::gui::types::Rect,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.root
            .dispatch_pointer_capture_cancelled_at_path_with_clock(
                widget_id,
                child_path.as_slice(),
                bounds,
                now,
            )
    }
}
