use super::*;

#[test]
fn stateful_app_builder_projects_updates_and_preserves_commands() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Counter")
        .size(320, 120)
        .view(|state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)),
                ui::button("Increment").message(DemoMessage::Increment),
            ])
        })
        .update_command(|state, message| match message {
            DemoMessage::Increment => {
                state.count += 1;
                Command::request_repaint()
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let increment = before
        .dispatch_widget_output(
            3,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 2, "text").text,
        "Count: 1"
    );
}
