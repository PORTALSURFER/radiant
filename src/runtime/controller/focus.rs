use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Give keyboard focus to one focusable widget.
    ///
    /// Returns `false` when the widget is absent or does not participate in
    /// focus. Focus changes are routed into affected widgets so their retained
    /// interaction state can update before the next paint plan.
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> bool {
        if !self.focusable_widgets.contains(widget_id) {
            return false;
        }
        if self.focused_widget == Some(widget_id) {
            return true;
        }

        if let Some(previous) = self.focused_widget {
            self.route_focus_changed(previous, false);
        }
        self.focused_widget = Some(widget_id);
        self.route_focus_changed(widget_id, true);
        true
    }

    /// Clear keyboard focus when a surface or backend loses focus ownership.
    pub fn clear_focus(&mut self) {
        if let Some(previous) = self.focused_widget.take() {
            self.route_focus_changed(previous, false);
        }
    }

    /// Move keyboard focus through the current declarative tree.
    ///
    /// Traversal uses stable tree order and wraps at either end. Returns the new
    /// focus target, or `None` when no keyboard-focusable widgets are projected.
    pub fn traverse_focus(&mut self, direction: FocusTraversal) -> Option<WidgetId> {
        let next = next_focus_target(
            self.focused_widget,
            self.keyboard_focus_widgets.order(),
            self.keyboard_focus_widgets.rank(),
            direction,
        )?;
        self.focus_widget(next).then_some(next)
    }

    /// Route a keyboard interaction to the current focus target.
    ///
    /// Pointer events should continue to use [`SurfaceRuntime::dispatch_input_at`]
    /// or [`SurfaceRuntime::dispatch_input`], because they carry their own hit
    /// target. Keyboard events are resolved through focused widget identity.
    pub fn dispatch_focused_input(&mut self, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.focused_widget?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    /// Return whether the current focus target is a text input.
    pub fn focused_text_input_id(&self) -> Option<WidgetId> {
        let widget_id = self.focused_widget?;
        self.surface_widget(widget_id).and_then(|widget| {
            widget
                .widget_object()
                .accepts_text_input()
                .then_some(widget_id)
        })
    }

    /// Return selected text from the focused text input as a borrowed slice, if any.
    pub fn focused_text_selection_slice(&self) -> Option<&str> {
        let widget_id = self.focused_text_input_id()?;
        self.surface_widget(widget_id)
            .and_then(|widget| widget.widget_object().selected_text_slice())
    }

    /// Return selected text from the focused text input as an owned string, if any.
    pub fn focused_text_selection(&self) -> Option<String> {
        self.focused_text_selection_slice().map(str::to_owned)
    }

    /// Resolve one keypress through host-owned shortcuts before falling back to
    /// focused-widget key routing.
    ///
    /// Returns `true` when the shortcut catalog handled the press or a focused
    /// widget received it.
    pub fn dispatch_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        focus: FocusSurface,
    ) -> bool {
        let resolution = self
            .bridge
            .resolve_key_press(self.pending_key_chord, press, focus);
        self.pending_key_chord = resolution.pending_chord;
        if let Some(message) = resolution.action {
            self.dispatch_message(message);
            return true;
        }
        if resolution.handled {
            return true;
        }
        widget_key
            .and_then(|key| self.dispatch_focused_input(WidgetInput::KeyPress(key)))
            .is_some()
    }

    pub(super) fn route_focus_changed(&mut self, widget_id: WidgetId, focused: bool) {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ =
            self.dispatch_raw_surface_input(widget_id, bounds, WidgetInput::FocusChanged(focused));
    }
}

fn next_focus_target(
    current: Option<WidgetId>,
    order: &[WidgetId],
    rank: &HashMap<WidgetId, usize>,
    direction: FocusTraversal,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let current_index = current.and_then(|widget_id| rank.get(&widget_id).copied());
    let next_index = match (current_index, direction) {
        (Some(index), FocusTraversal::Forward) => (index + 1) % order.len(),
        (Some(0), FocusTraversal::Backward) => order.len() - 1,
        (Some(index), FocusTraversal::Backward) => index - 1,
        (None, FocusTraversal::Forward) => 0,
        (None, FocusTraversal::Backward) => order.len() - 1,
    };
    Some(order[next_index])
}
