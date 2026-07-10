use radiant::prelude::{self as ui, IntoView};

#[derive(Clone, Debug, PartialEq)]
enum RowMessage {
    Activate,
    Secondary(ui::Point),
    Drop,
}

fn action_row() -> ui::View<RowMessage> {
    let accent = ui::Rgba8::new(80, 160, 220, 255);
    ui::interactive_row_underlay(ui::text_line("Item", 22.0))
        .input_id(44)
        .visual_state(ui::InteractiveRowVisualStateParts {
            selected: true,
            ..ui::InteractiveRowVisualStateParts::default()
        })
        .dense_chrome_palette(ui::DenseRowPalette::new().selected(accent.with_alpha(96)))
        .leading_marker(ui::DenseRowMarkerStyle::new(
            ui::DenseRowMarkerParts::leading(2.0),
            accent,
        ))
        .outline(ui::DenseRowOutlineStyle::new(0.5, accent, 1.5))
        .actions(
            ui::InteractiveRowActions::new()
                .activate(|| RowMessage::Activate)
                .secondary(RowMessage::Secondary)
                .drop(|| RowMessage::Drop),
        )
        .size(160.0, 22.0)
}

fn dense_policy_row() -> ui::View<RowMessage> {
    ui::interactive_row_underlay(ui::text_line("Item", 22.0))
        .dense_row_policy(
            ui::DenseRowPolicy::selectable(true)
                .activation_modifiers()
                .tracked_drag_source(false, false),
        )
        .input_id(45)
        .actions(ui::row_actions().activate(|| RowMessage::Activate))
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

#[test]
fn dense_row_policy_is_available_from_prelude() {
    assert_eq!(
        dense_policy_row().view_dispatch_widget_output(
            45,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some(RowMessage::Activate)
    );
}
