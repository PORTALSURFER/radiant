use super::{DemoMessage, DemoState, widget_ref};
use radiant::{
    layout::Vector2,
    runtime::{RuntimeBridge, UiSurface, WidgetMessageMapper},
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, WidgetSizing,
    },
};

#[test]
fn application_builder_accepts_widgets_through_widget_view_trait() {
    use radiant::prelude::{self as ui, IntoView, MappedWidget};

    let surface: UiSurface<DemoMessage> = ui::row([
        ui::widget(TextWidget::new(
            0,
            "Direct",
            WidgetSizing::fixed(Vector2::new(80.0, 20.0)).with_baseline(14.0),
        ))
        .id(20),
        ui::widget(MappedWidget::new(
            ButtonWidget::new(0, "Mapped", WidgetSizing::fixed(Vector2::new(96.0, 28.0))),
            WidgetMessageMapper::button(|_| DemoMessage::Increment),
        ))
        .id(21),
    ])
    .id(10)
    .into_surface();

    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 20, "text").common.id,
        20
    );
    assert_eq!(
        surface.dispatch_widget_output(
            21,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Increment)
    );
}

#[test]
fn application_view_builders_lower_into_runtime_surface_nodes() {
    use radiant::prelude::{self as ui, IntoView};

    let surface = ui::row([
        ui::text("Title").size(96.0, 24.0).baseline(17.0),
        ui::button("Increment")
            .message(DemoMessage::Increment)
            .id(42),
    ])
    .id(1)
    .into_surface();

    assert_eq!(surface.root().id(), 1);
    assert!(surface.find_widget(2).is_some());
    assert!(surface.find_widget(42).is_some());

    let message = surface
        .dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit the configured host message");
    assert_eq!(message, DemoMessage::Increment);
}

#[test]
fn application_builders_support_message_callbacks_scroll_and_sizing_helpers() {
    use radiant::prelude as ui;

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Increment,
        TextInput(TextInputMessage),
    }

    let mut bridge = ui::app(DemoState::default())
        .title("Direct")
        .view(|state| {
            ui::scroll(
                ui::column([
                    ui::text(format!("Count: {}", state.count))
                        .id(10)
                        .fixed(120.0, 24.0)
                        .baseline(17.0),
                    ui::button("Increment")
                        .message(Message::Increment)
                        .id(11)
                        .size(96.0, 32.0),
                    ui::text_input(state.name.clone())
                        .message_event(Message::TextInput)
                        .id(12)
                        .min_size(120.0, 28.0)
                        .preferred_size(180.0, 28.0),
                ])
                .id(2),
            )
            .id(1)
        })
        .update(|state, message| match message {
            Message::Increment => state.count += 1,
            Message::TextInput(message) => {
                let submitted = message.is_submitted();
                state.name = message.value().to_owned();
                if submitted {
                    state.count += 1;
                }
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    assert_eq!(before.root().id(), 1);
    assert!(before.find_widget(10).is_some());
    assert!(before.find_widget(11).is_some());
    assert!(before.find_widget(12).is_some());

    let increment = before
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("direct button should emit a state action");
    let command = bridge.update(increment);
    assert!(command.requests_repaint());

    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Count: 1"
    );

    let submit = after
        .dispatch_widget_output(
            12,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Launch now"),
            }),
        )
        .expect("direct text input submit should emit a state action");
    let command = bridge.update(submit);
    assert!(command.requests_repaint());

    let after_submit = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&after_submit, 12, "text input")
            .state
            .value,
        "Launch now"
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&after_submit, 10, "text").text,
        "Count: 2"
    );
}

#[test]
fn application_bridge_pulls_owned_surfaces_for_runtime_projection() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([ui::text(format!("Count: {}", state.count))
                .id(10)
                .fixed(120.0, 24.0)
                .baseline(17.0)])
            .id(1)
        })
        .update(|state, DemoMessage::Increment| {
            state.count += 1;
        })
        .into_bridge();

    let before = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&before, 10, "text").text,
        "Count: 0"
    );

    bridge.update(DemoMessage::Increment);
    let after = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Count: 1"
    );
}

#[test]
fn application_view_projection_is_repeatable_over_immutable_host_state() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState {
        count: 7,
        name: String::from("immutable"),
    })
    .view(|state: &DemoState| ui::text(format!("{}: {}", state.name, state.count)).id(10))
    .update(|_state, DemoMessage::Increment| {})
    .into_bridge();

    for _ in 0..2 {
        let surface = bridge.pull_surface();
        assert_eq!(
            widget_ref::<TextWidget, _>(&surface, 10, "text").text,
            "immutable: 7"
        );
    }
}

#[test]
fn application_builders_scope_keys_and_bind_text_inputs_to_state_fields() {
    use radiant::prelude::{self as ui, IntoView};

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Delete,
        NameChanged(String),
    }

    let surface = ui::column_key(
        "todos",
        [
            ui::row_key(
                1_u64,
                [
                    ui::text("First").key("label"),
                    ui::button("Delete").message(Message::Delete).key("delete"),
                ],
            ),
            ui::row_key(
                2_u64,
                [
                    ui::text("Second").key("label"),
                    ui::button("Delete").message(Message::Delete).key("delete"),
                ],
            ),
            ui::text_input(String::from("Draft"))
                .message(Message::NameChanged)
                .key("draft"),
        ],
    )
    .into_surface();

    let ids = surface
        .keyboard_focus_order()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(ids.len(), 3);
    for id in ids {
        assert!(surface.find_widget(id).is_some());
    }
}
