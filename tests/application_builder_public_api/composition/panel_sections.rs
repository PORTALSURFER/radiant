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
fn closeable_panel_section_routes_standard_close_button_message() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::closeable_panel_section_from_parts(
        ui::PanelSectionParts::new("Job Details", ui::text("Ready")).height(80.0),
        DemoMessage::Increment,
    )
    .id(1)
    .into_surface();
    let focus_order = surface.keyboard_focus_order();

    assert_eq!(focus_order.len(), 1);
    assert_eq!(
        surface.dispatch_widget_output(
            focus_order[0],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(DemoMessage::Increment)
    );
}

#[test]
fn panel_section_layer_exports_anchored_panel_geometry() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::new("Inspector", ui::text("Ready").id(90)),
            ui::Vector2::new(160.0, 90.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::End)
        .vertical(ui::LayerVerticalAnchor::End)
        .inset(12.0, 10.0),
    )
    .into_surface();
    let frame = surface.frame_at_size(ui::Vector2::new(220.0, 140.0), &Default::default());
    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            radiant::runtime::PaintPrimitive::Text(text) if text.widget_id == 90 => Some(text.rect),
            _ => None,
        })
        .expect("anchored panel content should paint");

    assert!((text_rect.min.x - 54.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 70.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn closeable_panel_section_layer_routes_standard_close_button_message() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::new("Inspector", ui::text("Ready")),
            ui::Vector2::new(160.0, 90.0),
        ),
        DemoMessage::Increment,
    )
    .into_surface();
    let focus_order = surface.keyboard_focus_order();

    assert_eq!(focus_order.len(), 1);
    assert_eq!(
        surface.dispatch_widget_output(
            focus_order[0],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(DemoMessage::Increment)
    );
}

#[test]
fn dialog_layer_helpers_project_standard_dialogs() {
    use radiant::prelude::{self as ui, IntoView};

    let frame = ui::column([
        ui::dialog_layer(
            "Info",
            ui::text("Plain dialog body"),
            ui::WidgetTone::Neutral,
            ui::Vector2::new(180.0, 96.0),
        ),
        ui::closeable_dialog_layer(
            "Warning",
            ui::text("Closeable dialog body"),
            ui::WidgetTone::Warning,
            ui::Vector2::new(180.0, 96.0),
            DemoMessage::Increment,
        ),
    ])
    .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 220.0));

    assert!(frame.paint_plan.contains_text("Info"));
    assert!(frame.paint_plan.contains_text("Warning"));
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
