use super::super::*;

#[test]
fn application_builder_labeled_control_routes_inner_control() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::labeled_control(
        "Backend",
        ui::button("WASAPI")
            .message(DemoMessage::Increment)
            .key("backend-button")
            .fill_width()
            .height(24.0),
        45.0,
    )
    .id(1)
    .into_surface();

    let focus_order = surface.keyboard_focus_order();

    assert_eq!(focus_order.len(), 1);
    assert!(matches!(
        surface.dispatch_widget_output(
            focus_order[0],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(DemoMessage::Increment)
    ));
}

#[test]
fn application_builder_labeled_control_supports_named_overrides() {
    use radiant::prelude as ui;

    let parts: ui::LabeledControlParts<DemoMessage> =
        ui::LabeledControlParts::new("Output", ui::text("Device default"))
            .height(48.0)
            .label_height(16.0)
            .spacing(5.0)
            .label_style(ui::WidgetStyle {
                tone: ui::WidgetTone::Neutral,
                prominence: ui::WidgetProminence::Strong,
            });

    assert_eq!(parts.label, "Output");
    assert_eq!(parts.height, Some(48.0));
    assert_eq!(parts.label_height, 16.0);
    assert_eq!(parts.spacing, 5.0);
    assert_eq!(parts.label_style.tone, ui::WidgetTone::Neutral);
}
