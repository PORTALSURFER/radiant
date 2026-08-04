use super::super::*;
use radiant::application as app;
use radiant::gui::svg::IconName;
use radiant::runtime::SurfaceRuntime;
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

#[derive(Default)]
struct RuntimeDropdownState {
    open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeDropdownMessage {
    Toggle,
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
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message())
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
        .option_from_parts(app::DropdownOptionParts {
            label: "System default".into(),
            selection: ui::DropdownOptionSelection::Unselected,
            message: GalleryMessage::Pick("default"),
        })
        .option_from_parts(app::DropdownOptionParts {
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
                radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
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
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        Some(GalleryMessage::ToggleDropdown)
    );
    let trigger = widget_ref::<ButtonWidget, _>(&surface, 1, "dropdown trigger");
    assert!(trigger.props.label.is_static());
    assert_eq!(trigger.props.label, "WASAPI");
    assert!(trigger.props.trailing_label.is_none());
    assert!(trigger.trailing_icon.is_none());
    assert_eq!(
        trigger.trailing_icon_tint_cache,
        Some(IconName::ChevronDown.tint_cache())
    );
    let _parts = app::DropdownTriggerParts {
        selected_label: String::from("WASAPI").into(),
        open: true,
        toggle_message: GalleryMessage::ToggleDropdown,
    };
}

#[test]
fn keyed_runtime_dropdown_reprojects_active_state_false_true_false() {
    use radiant::prelude as ui;

    let bridge = radiant::app(RuntimeDropdownState::default())
        .view(|state| {
            ui::dropdown_trigger("WASAPI", state.open)
                .toggle_message(RuntimeDropdownMessage::Toggle)
                .build()
                .key("runtime-dropdown")
        })
        .update(|state, RuntimeDropdownMessage::Toggle| state.open = !state.open)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, ui::Vector2::new(180.0, 120.0));
    let trigger_id = runtime.surface().keyboard_focus_order()[0];
    assert!(
        !widget_ref::<ButtonWidget, _>(runtime.surface(), trigger_id, "dropdown trigger")
            .common
            .state
            .active
    );
    assert_eq!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .stroke_polylines()
            .filter(|marker| marker.widget_id == trigger_id)
            .count(),
        0
    );

    let trigger_point = ui::Point::new(8.0, 8.0);
    assert_eq!(
        runtime.dispatch_primary_click(trigger_point).press_target,
        Some(trigger_id)
    );
    let trigger_id = runtime.surface().keyboard_focus_order()[0];
    assert!(
        widget_ref::<ButtonWidget, _>(runtime.surface(), trigger_id, "dropdown trigger")
            .common
            .state
            .active
    );
    assert_eq!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .stroke_polylines()
            .filter(|marker| marker.widget_id == trigger_id)
            .count(),
        1
    );

    let trigger_id = runtime.surface().keyboard_focus_order()[0];
    let trigger_point = ui::Point::new(8.0, 8.0);
    assert_eq!(
        runtime.dispatch_primary_click(trigger_point).press_target,
        Some(trigger_id)
    );
    assert!(
        !widget_ref::<ButtonWidget, _>(runtime.surface(), trigger_id, "dropdown trigger")
            .common
            .state
            .active
    );
    assert_eq!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .stroke_polylines()
            .filter(|marker| marker.widget_id == trigger_id)
            .count(),
        0
    );
}
