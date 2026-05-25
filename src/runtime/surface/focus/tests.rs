use crate::{
    layout::Vector2,
    runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
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
                    ButtonWidget::new(40, "Second", WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
                    WidgetMessageMapper::none(),
                ))],
            )),
        ],
    ));

    assert_eq!(surface.keyboard_focus_order(), vec![20, 40]);
}

#[test]
fn keyboard_focus_order_into_reuses_existing_storage() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::widget(
            ButtonWidget::new(20, "First", WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
            WidgetMessageMapper::none(),
        ))],
    ));
    let mut order = Vec::with_capacity(8);
    order.extend([99, 100]);
    let capacity = order.capacity();

    surface.keyboard_focus_order_into(&mut order);

    assert_eq!(order, vec![20]);
    assert_eq!(order.capacity(), capacity);
}
