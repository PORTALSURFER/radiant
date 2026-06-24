use super::{PointerMoveOutcome, SurfaceRuntime};
use crate::{
    gui::types::Point,
    runtime::{CommandOutcome, NativeFileDrop, RuntimeBridge},
    widgets::{WidgetId, WidgetInput},
};

mod move_routing;
#[cfg(test)]
mod tests;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route pointer motion and return a redraw-oriented outcome for backend adapters.
    ///
    /// Use this in native or embedded backends that need to distinguish full
    /// scene rebuilds from paint-only runtime overlays. Application-level event
    /// routing can keep using [`Self::dispatch_event`].
    pub fn dispatch_pointer_move_with_outcome(&mut self, position: Point) -> PointerMoveOutcome {
        self.dispatch_pointer_move_with_refresh_outcome(position, true)
    }

    /// Route pointer motion while deferring host-surface refresh from emitted
    /// widget messages until the caller explicitly refreshes the runtime.
    ///
    /// Native backends use this during high-frequency pointer motion to
    /// coalesce many model updates into the next redraw instead of refreshing
    /// the declarative surface once per OS cursor event.
    pub fn dispatch_pointer_move_deferred_refresh_with_outcome(
        &mut self,
        position: Point,
    ) -> PointerMoveOutcome {
        self.dispatch_pointer_move_with_refresh_outcome(position, false)
    }

    fn dispatch_pointer_move_with_refresh_outcome(
        &mut self,
        position: Point,
        refresh_after_message: bool,
    ) -> PointerMoveOutcome {
        let previous_hovered_widget = self.interaction.hover.widget;
        let previous_hovered_container = self.interaction.hover.container;
        let dispatch =
            self.dispatch_pointer_move_target_with_refresh(position, refresh_after_message);
        let target = dispatch.target;
        let hover_changed = previous_hovered_widget != self.interaction.hover.widget
            || previous_hovered_container != self.interaction.hover.container;
        if hover_changed {
            self.clear_retained_hover_except(self.interaction.hover.widget);
        }
        let repaint_requested = self.take_repaint_requested();
        let exit_requested = self.take_exit_requested();
        let pointer_captured = self.interaction.pointer.capture.is_some();
        let target_prefers_paint_only =
            target.is_some_and(|widget_id| self.widget_prefers_pointer_move_paint_only(widget_id));
        let drag_preview_can_paint_only =
            self.drag_session_active() && !hover_changed && !dispatch.emitted_output;
        let paint_only_requested = repaint_requested
            && !dispatch.emitted_output
            && (target_prefers_paint_only || drag_preview_can_paint_only);
        PointerMoveOutcome {
            target,
            hover_changed,
            pointer_captured,
            repaint_requested: repaint_requested && !paint_only_requested,
            paint_only_requested,
            exit_requested,
        }
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        self.dispatch_input_at_output(point, input)
            .map(|(widget_id, _)| widget_id)
    }

    pub(super) fn dispatch_input_at_output(
        &mut self,
        point: Point,
        input: WidgetInput,
    ) -> Option<(WidgetId, bool)> {
        let widget_id = self.widget_at(point)?;
        if matches!(
            input,
            WidgetInput::PointerPress { .. } | WidgetInput::PointerDoubleClick { .. }
        ) && !self.focus_widget(widget_id)
        {
            self.clear_focus();
        }
        self.dispatch_input_output(widget_id, input)
            .map(|emitted_output| (widget_id, emitted_output))
    }

    /// Return whether a runtime-owned drag preview session is active.
    pub fn drag_session_active(&self) -> bool {
        self.interaction.drag.session.is_some()
    }

    /// Return the widget under a native file-drop pointer position.
    pub fn native_file_drop_target(&self, position: Option<Point>) -> Option<WidgetId> {
        position.and_then(|position| self.widget_at(position))
    }

    /// Route a native file-drop event to the topmost accepting declarative target.
    ///
    /// If no view-tree target accepts the drop, this falls back to the app-level
    /// native file-drop hook for compatibility with custom hosts.
    pub fn dispatch_native_file_drop(&mut self, drop: NativeFileDrop) -> CommandOutcome {
        if let Some(target) = self.native_file_drop_accepting_target(drop.position) {
            let drop = drop.clone().with_target_widget(Some(target));
            if let Some(message) = self.native_file_drop_message(target, drop.clone()) {
                return self.dispatch_message(message);
            }
            let command = self.bridge_mut().native_file_drop(drop);
            return self.execute_command(command);
        }
        let target = self.native_file_drop_target(drop.position);
        let command = self
            .bridge_mut()
            .native_file_drop(drop.with_target_widget(target));
        self.execute_command(command)
    }

    fn native_file_drop_accepting_target(&self, position: Option<Point>) -> Option<WidgetId> {
        let Some(position) = position else {
            return self.topmost_visible_native_file_drop_target();
        };
        self.traversal
            .widgets
            .native_file_drop
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|widget_id| self.widget_contains_point(*widget_id, position))
    }

    fn topmost_visible_native_file_drop_target(&self) -> Option<WidgetId> {
        self.traversal
            .widgets
            .native_file_drop
            .visible()
            .iter()
            .rev()
            .copied()
            .next()
    }

    fn native_file_drop_message(
        &self,
        widget_id: WidgetId,
        drop: NativeFileDrop,
    ) -> Option<Message> {
        self.surface_widget(widget_id)
            .and_then(|widget| widget.dispatch_native_file_drop(widget_id, drop))
    }

    /// Clear active pointer capture without routing a release event.
    ///
    /// Native external drag loops own the release that completes the OS drag, so
    /// the originating surface must not keep treating later pointer motion as a
    /// continuation of the in-window press.
    pub(crate) fn cancel_pointer_capture(&mut self) {
        let captured = self.interaction.pointer.capture.take();
        if let Some(widget_id) = captured {
            self.cancel_captured_widget_state(widget_id);
        }
        self.interaction.pointer.capture = None;
        self.interaction.pointer.capture_state = None;
        self.interaction.pointer.scroll_drag_capture = None;
    }

    fn cancel_captured_widget_state(&mut self, widget_id: WidgetId) {
        let Some(previous_state) = self
            .surface_widget(widget_id)
            .map(|widget| widget.widget_object().common().state)
        else {
            return;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ = self.dispatch_surface_input(widget_id, bounds, WidgetInput::FocusChanged(false));
        let Some(next_state) = self
            .surface_widget(widget_id)
            .map(|widget| widget.widget_object().common().state)
        else {
            return;
        };
        if previous_state != next_state {
            self.repaint_requested = true;
        }
    }

    /// Clear pointer hover ownership and retained widget hover state.
    ///
    /// Native backends call this when the pointer leaves the surface or the
    /// window loses focus, because no later pointer-move event is guaranteed to
    /// arrive to clear paint-only hover fills.
    pub(crate) fn clear_pointer_hover(&mut self) -> bool {
        let mut cleared = false;
        if let Some(widget_id) = self.interaction.hover.widget.take() {
            if let Some(widget) = self.surface.find_widget_mut(widget_id)
                && widget.widget_object().common().state.hovered
            {
                widget.widget_object_mut().common_mut().state.hovered = false;
            }
            cleared = true;
        }
        cleared |= self.clear_retained_hover_except(None);
        if self.interaction.hover.container.take().is_some() {
            cleared = true;
        }
        if self.interaction.hover.scroll_affordance.take().is_some() {
            cleared = true;
        }
        if cleared {
            self.repaint_requested = true;
        }
        cleared
    }

    fn clear_retained_hover_except(&mut self, owner: Option<WidgetId>) -> bool {
        let mut cleared = false;
        for index in 0..self.traversal.widgets.stateful_order.len() {
            let widget_id = self.traversal.widgets.stateful_order[index];
            if Some(widget_id) == owner {
                continue;
            }
            let Some(widget) = self.surface.find_widget_mut(widget_id) else {
                continue;
            };
            if !widget.widget_object().common().state.hovered {
                continue;
            }
            widget.widget_object_mut().common_mut().state.hovered = false;
            cleared = true;
        }
        if cleared {
            self.repaint_requested = true;
        }
        cleared
    }

    /// End the runtime drag preview because ownership has moved to a native
    /// external drag loop.
    pub(crate) fn take_drag_preview_for_external_drag(&mut self) -> bool {
        if self.interaction.drag.session.take().is_none() {
            return false;
        }
        self.repaint_requested = true;
        true
    }

    /// Hide the runtime drag preview while the pointer is outside this surface.
    ///
    /// The drag session stays alive so a later pointer move back into the
    /// window can show the preview again and continue routing the same drag.
    pub(crate) fn hide_drag_preview_for_cursor_left(&mut self) -> bool {
        let Some(session) = self.interaction.drag.session.as_mut() else {
            return false;
        };
        if !session.visible {
            return false;
        }
        session.visible = false;
        self.repaint_requested = true;
        true
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PointerMoveDispatch {
    pub(super) target: Option<WidgetId>,
    pub(super) emitted_output: bool,
}
