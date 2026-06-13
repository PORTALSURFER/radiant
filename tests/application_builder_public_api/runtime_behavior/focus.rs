use super::*;

#[test]
fn application_builder_ui_update_context_can_move_keyboard_focus() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text_input(state.name.clone())
                    .message(|_| FocusMessage::FocusName)
                    .id(10),
                ui::button("Focus name")
                    .message(FocusMessage::FocusName)
                    .id(11),
                ui::text(format!("Name: {}", state.name))
                    .id(12)
                    .height(24.0),
            ])
        })
        .handle_message(|state, message, context| match message {
            FocusMessage::FocusName => {
                state.name = String::from("focused");
                context.focus(10);
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
    let focus = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit focus message");
    let outcome = runtime.dispatch_message(focus);

    assert!(outcome.repaint_requested);
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 12, "status").text,
        "Name: focused"
    );
}
