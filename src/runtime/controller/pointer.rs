use super::*;

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
        let previous_hovered_widget = self.hovered_widget();
        let previous_hovered_container = self.hovered_container();
        let target = self.dispatch_pointer_move_target(position);
        let repaint_requested = self.take_repaint_requested();
        let exit_requested = self.take_exit_requested();
        let hover_changed = previous_hovered_widget != self.hovered_widget()
            || previous_hovered_container != self.hovered_container();
        let pointer_captured = self.pointer_capture().is_some();
        let target_prefers_paint_only =
            target.is_some_and(|widget_id| self.widget_prefers_pointer_move_paint_only(widget_id));
        let paint_only_requested = repaint_requested && target_prefers_paint_only && !hover_changed;
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
        ) {
            let _ = self.focus_widget(widget_id);
        }
        self.dispatch_input_output(widget_id, input)
            .map(|emitted_output| (widget_id, emitted_output))
    }

    pub(super) fn dispatch_pointer_move_target(&mut self, position: Point) -> Option<WidgetId> {
        if self.drag_scrollbar_to(position) {
            return None;
        }
        let hovered_scroll_affordance = self.scroll_affordance_at(position);
        if self.hovered_scroll_affordance != hovered_scroll_affordance {
            self.hovered_scroll_affordance = hovered_scroll_affordance;
            self.repaint_requested = true;
        }
        let pointer_widget = self.pointer_widget_at_for_move(position);
        let hover_container = if self.widget_suppresses_container_hover(pointer_widget) {
            None
        } else {
            self.styled_container_at(position)
        };
        if self.hovered_container != hover_container {
            self.hovered_container = hover_container;
            self.repaint_requested = true;
        }
        let hover_widget = self
            .pointer_capture
            .filter(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(position))
            })
            .or_else(|| {
                self.pointer_capture
                    .is_none()
                    .then_some(pointer_widget)
                    .flatten()
            });
        let hover_changed = self.hovered_widget != hover_widget;
        if hover_changed {
            if let Some(previous) = self.hovered_widget {
                let _ = self.dispatch_input(previous, WidgetInput::PointerMove { position });
            }
            self.hovered_widget = hover_widget;
        }

        if let (Some(captured), Some(pointer_widget)) = (self.pointer_capture, pointer_widget)
            && pointer_widget != captured
            && self.widget_accepts_stable_pointer_move(pointer_widget)
        {
            let routed = self.dispatch_input(pointer_widget, WidgetInput::PointerMove { position });
            if routed {
                self.repaint_requested = true;
            }
        }

        let target = self.pointer_capture.or(pointer_widget)?;
        let accepts_stable_pointer_move = self.widget_accepts_stable_pointer_move(target);
        if !hover_changed && self.pointer_capture.is_none() && !accepts_stable_pointer_move {
            return Some(target);
        }
        let routed = self.dispatch_input(target, WidgetInput::PointerMove { position });
        if routed && (accepts_stable_pointer_move || self.pointer_capture.is_some()) {
            // Stable pointer-move widgets may update local paint-only hover
            // state without emitting host messages. Captured drags can also
            // update local preview state even when the widget opts out of
            // stable hover motion. Request repaint here so cursor, handle, and
            // drag previews stay responsive without reducer churn.
            self.repaint_requested = true;
        }
        routed.then_some(target)
    }
}
