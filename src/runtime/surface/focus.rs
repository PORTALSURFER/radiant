use super::*;

impl<Message> UiSurface<Message> {
    /// Return keyboard-focusable widgets in deterministic declarative tree order.
    pub fn keyboard_focus_order(&self) -> Vec<WidgetId> {
        self.runtime_traversal_index().keyboard_focus_order
    }
}
