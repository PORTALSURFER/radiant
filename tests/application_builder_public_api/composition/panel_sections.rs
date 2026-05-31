use super::super::*;

#[test]
fn application_builder_panel_section_applies_fixed_height_and_routes_trailing_action() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Collections",
            ui::text("Recently used").fill_width().height(20.0),
        )
        .trailing(
            ui::button("+")
                .message(DemoMessage::Increment)
                .key("panel-section-action")
                .size(24.0, 20.0),
        )
        .height(76.0),
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
fn application_builder_panel_section_supports_convenience_constructor() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> =
        ui::panel_section("Filter", ui::text("Type: Audio"), 72.0)
            .id(1)
            .into_surface();
    assert!(surface.keyboard_focus_order().is_empty());
}

#[test]
fn panel_section_parts_support_named_overrides() {
    use radiant::prelude as ui;

    let parts: ui::PanelSectionParts<DemoMessage> =
        ui::PanelSectionParts::new("Inspector", ui::text("Ready"))
            .height(90.0)
            .padding(8.0)
            .spacing(6.0)
            .header_spacing(5.0)
            .title_height(22.0)
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Strong,
            });

    assert_eq!(parts.title, "Inspector");
    assert_eq!(parts.height, Some(90.0));
    assert_eq!(parts.padding, 8.0);
    assert_eq!(parts.spacing, 6.0);
    assert_eq!(parts.header_spacing, 5.0);
    assert_eq!(parts.title_height, 22.0);
    assert_eq!(parts.style.tone, ui::WidgetTone::Accent);
}
