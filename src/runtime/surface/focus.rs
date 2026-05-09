use super::*;

impl<Message> SurfaceNode<Message> {
    fn collect_keyboard_focus_order(&self, order: &mut Vec<WidgetId>) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_keyboard_focus_order(order);
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

impl<Message> UiSurface<Message> {
    /// Return keyboard-focusable widgets in deterministic declarative tree order.
    pub fn keyboard_focus_order(&self) -> Vec<WidgetId> {
        let mut order = Vec::new();
        self.root.collect_keyboard_focus_order(&mut order);
        order
    }
}
