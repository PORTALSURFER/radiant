use super::*;

#[cfg(test)]
#[path = "focus/tests.rs"]
mod tests;

impl<Message> UiSurface<Message> {
    /// Append keyboard-focusable widgets in deterministic declarative tree
    /// order into caller-owned storage.
    ///
    /// This is useful for diagnostics, tests, or host integrations that inspect
    /// focus order repeatedly and want to reuse allocation capacity.
    pub fn keyboard_focus_order_into(&self, order: &mut Vec<WidgetId>) {
        order.clear();
        self.root.append_keyboard_focus_order(order);
    }

    /// Return keyboard-focusable widgets in deterministic declarative tree order.
    pub fn keyboard_focus_order(&self) -> Vec<WidgetId> {
        let mut order = Vec::new();
        self.keyboard_focus_order_into(&mut order);
        order
    }
}

impl<Message> SurfaceNode<Message> {
    fn append_keyboard_focus_order(&self, order: &mut Vec<WidgetId>) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.append_keyboard_focus_order(order);
                }
            }
            Self::Widget(widget) => {
                if widget.is_keyboard_focusable() {
                    order.push(widget.id());
                }
            }
            Self::Overlay(_) => {}
        }
    }
}
