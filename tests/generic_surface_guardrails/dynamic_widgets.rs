use super::*;

#[test]
fn runtime_widgets_accept_boxed_widgets_and_dynamic_messages() {
    #[derive(Clone, Debug, PartialEq)]
    struct CustomPayload(&'static str);

    #[derive(Debug, PartialEq)]
    enum HostMessage {
        Custom(&'static str),
    }

    let widget_id = 91;
    let boxed_widget = Box::new(TextWidget::new(
        widget_id,
        "boxed",
        WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
    ));
    let surface = UiSurface::new(SurfaceNode::custom_widget_box(
        boxed_widget,
        WidgetMessageMapper::dynamic(|output| {
            output
                .typed_ref::<CustomPayload>()
                .map(|payload| HostMessage::Custom(payload.0))
        }),
    ));

    assert!(surface.find_widget(widget_id).is_some());
    assert_eq!(
        surface.dispatch_widget_output(widget_id, WidgetOutput::custom(CustomPayload("open"))),
        Some(HostMessage::Custom("open"))
    );
}
