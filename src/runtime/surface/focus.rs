use super::*;

impl<Message> UiSurface<Message> {
    /// Return keyboard-focusable widgets in deterministic declarative tree order.
    pub fn keyboard_focus_order(&self) -> Vec<WidgetId> {
        let stats = self.root.runtime_traversal_stats();
        let mut order = Vec::with_capacity(stats.widgets);
        self.root.append_keyboard_focus_order(&mut order);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::Vector2,
        widgets::{ButtonWidget, TextWidget, WidgetSizing},
    };

    #[test]
    fn keyboard_focus_order_collects_only_keyboard_focusable_widgets() {
        let surface: UiSurface<()> = UiSurface::new(SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                    10,
                    "Label",
                    WidgetSizing::fixed(Vector2::new(120.0, 20.0)),
                ))),
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(20, "First", WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::row(
                    30,
                    0.0,
                    vec![SurfaceChild::fill(SurfaceNode::widget(
                        ButtonWidget::new(
                            40,
                            "Second",
                            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ))],
                )),
            ],
        ));

        assert_eq!(surface.keyboard_focus_order(), vec![20, 40]);
    }
}
