use super::super::*;
use radiant::widgets::{
    BadgeMessage, BadgeWidget, CardWidget, SelectableMessage, SelectableWidget,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum GalleryMessage {
    Badge,
    Selected(bool),
    ToggleDropdown,
    Pick(&'static str),
}

#[test]
fn application_builder_gallery_widgets_lower_and_route_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<GalleryMessage> = ui::column([
        ui::badge("Ready")
            .active(true)
            .message(GalleryMessage::Badge)
            .id(10),
        ui::selectable("Option", false)
            .message(GalleryMessage::Selected)
            .id(11),
        ui::card().id(12).size(160.0, 72.0),
    ])
    .id(1)
    .into_surface();

    let badge = widget_ref::<BadgeWidget, _>(&surface, 10, "badge");
    assert_eq!(badge.props.label, "Ready");
    assert!(badge.common.state.active);
    assert_eq!(
        surface.dispatch_widget_output(
            10,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(GalleryMessage::Badge)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            10,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate)
        ),
        None
    );

    let selectable = widget_ref::<SelectableWidget, _>(&surface, 11, "selectable");
    assert_eq!(selectable.props.label, "Option");
    assert!(!selectable.common.state.selected);
    assert_eq!(
        surface.dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(SelectableMessage::SelectionChanged {
                selected: true,
            })
        ),
        Some(GalleryMessage::Selected(true))
    );

    let card = widget_ref::<CardWidget, _>(&surface, 12, "card");
    assert!(!card.common.paint.paints_focus);
    assert!(card.common.paint.suppresses_container_hover);
    assert_eq!(surface.keyboard_focus_order(), vec![10, 11]);
}

#[test]
fn application_builder_dropdown_exports_and_routes_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<GalleryMessage> = ui::dropdown("WASAPI", true)
        .toggle_message(GalleryMessage::ToggleDropdown)
        .option_from_parts(ui::DropdownOptionParts {
            label: "System default".into(),
            selection: ui::DropdownOptionSelection::Unselected,
            message: GalleryMessage::Pick("default"),
        })
        .option_from_parts(ui::DropdownOptionParts {
            label: "WASAPI".into(),
            selection: ui::DropdownOptionSelection::Selected,
            message: GalleryMessage::Pick("wasapi"),
        })
        .build()
        .id(1)
        .into_surface();

    let focus_order = surface.keyboard_focus_order();
    let routed = focus_order
        .iter()
        .filter_map(|widget_id| {
            surface.dispatch_widget_output(
                *widget_id,
                radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
            )
        })
        .collect::<Vec<_>>();
    assert!(routed.contains(&GalleryMessage::ToggleDropdown));
    assert!(routed.contains(&GalleryMessage::Pick("wasapi")));
    assert_eq!(ui::dropdown_height(true, 2), 24.0);
    assert_eq!(ui::dropdown_menu_height(2), 55.0);
}

#[test]
fn application_builder_dropdown_trigger_exports_and_routes_message() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<GalleryMessage> = ui::dropdown_trigger("WASAPI", true)
        .toggle_message(GalleryMessage::ToggleDropdown)
        .build()
        .id(1)
        .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(
            1,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(GalleryMessage::ToggleDropdown)
    );
    let _parts = ui::DropdownTriggerParts {
        selected_label: String::from("WASAPI"),
        open: true,
        toggle_message: GalleryMessage::ToggleDropdown,
    };
}
