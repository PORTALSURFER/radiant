use radiant::prelude::{self as ui, IntoView};

#[derive(Clone, Debug, PartialEq)]
enum RowMessage {
    Activate,
    Secondary(ui::Point),
    Drop,
}

fn action_row() -> ui::View<RowMessage> {
    ui::interactive_row_underlay(ui::text_line("Item", 22.0))
        .input_id(44)
        .actions(
            ui::InteractiveRowActions::new()
                .activate(|| RowMessage::Activate)
                .secondary(RowMessage::Secondary)
                .drop(|| RowMessage::Drop),
        )
        .size(160.0, 22.0)
}

#[test]
fn interactive_row_actions_are_available_from_prelude() {
    let secondary = ui::Point::new(8.0, 12.0);

    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some(RowMessage::Activate)
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::SecondaryActivate {
                position: secondary,
            }),
        ),
        Some(RowMessage::Secondary(secondary))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Drop)
        ),
        Some(RowMessage::Drop)
    );
}
