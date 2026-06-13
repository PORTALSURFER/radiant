use super::*;

#[test]
fn application_builder_background_spawn_routes_worker_result() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text(format!("Loaded: {}", state.name))
                    .id(10)
                    .height(24.0),
                ui::button("Load")
                    .message(LoadingMessage::Start)
                    .id(11)
                    .height(28.0),
            ])
        })
        .handle_message(|state, message, context| match message {
            LoadingMessage::Start => {
                state.name = "loading".to_string();
                context
                    .business()
                    .background("test-loader")
                    .run(|_| "ready".to_string(), LoadingMessage::Loaded);
                context.request_repaint();
            }
            LoadingMessage::Loaded(value) => {
                state.name = value;
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 80.0));
    let start = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("load button should emit a start message");

    let started = runtime.dispatch_message(start);
    assert!(started.repaint_requested);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: loading"
    );

    let finished = wait_for_runtime_message(&mut runtime);
    assert_eq!(finished.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: ready"
    );
}
