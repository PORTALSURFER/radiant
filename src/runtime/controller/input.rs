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
    pub(super) fn resolve_widget_dispatch(
        &mut self,
        result: WidgetDispatchResult<Message>,
    ) -> crate::runtime::ResolvedWidgetDispatchResult<Message> {
        use crate::runtime::ResolvedWidgetDispatchResult as Resolved;
        match result {
            WidgetDispatchResult::NoOutput => Resolved::NoOutput,
            WidgetDispatchResult::UnmappedOutput => Resolved::UnmappedOutput,
            WidgetDispatchResult::Message(message) => Resolved::Message(message),
            WidgetDispatchResult::Command(activation) => self
                .resolve_command_request(
                    activation.request(),
                    crate::gui::focus::FocusSurface::None,
                )
                .message
                .map(Resolved::Message)
                .unwrap_or(Resolved::UnmappedOutput),
        }
    }

    pub(super) fn dispatch_surface_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        let is_pointer_input = matches!(
            &input,
            WidgetInput::PointerMove { .. }
                | WidgetInput::PointerPress { .. }
                | WidgetInput::PointerDoubleClick { .. }
                | WidgetInput::PointerRelease { .. }
        );
        if self.gesture_owns_pointer_capture()
            && (is_pointer_input
                || matches!(
                    &input,
                    WidgetInput::Wheel { .. } | WidgetInput::PointerModifiersChanged { .. }
                ))
        {
            return None;
        }
        if is_pointer_input {
            self.apply_native_text_pointer_caret(widget_id);
        } else {
            self.pending_native_text_pointer_caret = None;
        }
        let result = if let WidgetInput::FocusChanged(focused) = input {
            self.dispatch_surface_focus_changed(widget_id, bounds, focused)
        } else if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            self.surface
                .dispatch_widget_input_message_at_path(widget_id, child_path, bounds, input)
        } else {
            self.surface
                .dispatch_widget_input_message(widget_id, bounds, input)
        };
        if is_pointer_input {
            self.publish_accepted_native_text_pointer_caret(widget_id);
        }
        result
    }

    fn apply_native_text_pointer_caret(&mut self, widget_id: WidgetId) {
        let Some(super::NativeTextPointerCaret::Pending(
            pending_widget_id,
            source,
            caret,
            affinity,
        )) = self.pending_native_text_pointer_caret.take()
        else {
            return;
        };
        if pending_widget_id != widget_id {
            self.pending_native_text_pointer_caret = Some(super::NativeTextPointerCaret::Pending(
                pending_widget_id,
                source,
                caret,
                affinity,
            ));
            return;
        }
        let Some(widget) = self.surface_widget_mut(widget_id) else {
            return;
        };
        let Some(text_input) = widget.native_text_input_delegate_mut() else {
            return;
        };
        if text_input.state.value == source {
            text_input.set_native_pointer_caret(caret, affinity);
            self.pending_native_text_pointer_caret = Some(super::NativeTextPointerCaret::Applied(
                widget_id, source, affinity,
            ));
        }
    }

    fn publish_accepted_native_text_pointer_caret(&mut self, widget_id: WidgetId) {
        let Some(super::NativeTextPointerCaret::Applied(applied_widget_id, source, affinity)) =
            self.pending_native_text_pointer_caret.take()
        else {
            return;
        };
        if applied_widget_id != widget_id {
            return;
        }
        let accepted = self
            .surface_widget_mut(widget_id)
            .and_then(|widget| widget.native_text_input_delegate_mut())
            .is_some_and(|text_input| {
                let accepted = text_input.state.value == source
                    && text_input.take_native_pointer_caret_acceptance() == Some(affinity);
                if !accepted {
                    text_input.clear_native_pointer_caret();
                }
                accepted
            });
        if accepted {
            self.pending_native_text_pointer_caret =
                Some(super::NativeTextPointerCaret::Accepted(widget_id, affinity));
        }
    }

    pub(crate) fn take_accepted_native_text_pointer_caret(
        &mut self,
    ) -> Option<(WidgetId, crate::widgets::NativeCaretAffinity)> {
        match self.pending_native_text_pointer_caret.take() {
            Some(super::NativeTextPointerCaret::Accepted(widget_id, affinity)) => {
                Some((widget_id, affinity))
            }
            Some(super::NativeTextPointerCaret::Pending(..))
            | Some(super::NativeTextPointerCaret::Applied(..))
            | None => None,
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
