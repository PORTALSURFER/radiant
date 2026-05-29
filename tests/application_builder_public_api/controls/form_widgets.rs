use super::super::*;
use radiant::widgets::{SliderWidget, TextInputMessage, TextInputWidget};

#[test]
fn application_builders_expose_interactive_row_scrollbar_icon_button_and_compact_slider() {
    use radiant::prelude::{self as ui, IntoView};

    let icon = ui::SvgIcon::from_svg(
        r##"<svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg"><path d="M1 0 L4 2 L1 4 Z"/></svg>"##,
    )
    .expect("icon");
    let surface: UiSurface<&'static str> = ui::column([
        ui::interactive_row()
            .draggable()
            .droppable(true)
            .mapped(|message| match message {
                ui::InteractiveRowMessage::Activate => "row",
                ui::InteractiveRowMessage::DoubleActivate => "double-row",
                ui::InteractiveRowMessage::Drag(_) => "drag",
                ui::InteractiveRowMessage::SecondaryActivate { .. } => "secondary",
                ui::InteractiveRowMessage::Drop => "drop",
                ui::InteractiveRowMessage::HoverDropTarget => "hover-drop",
            })
            .id(20),
        ui::scrollbar(ui::ScrollbarAxis::Horizontal)
            .viewport_fraction(0.25)
            .offset_fraction(0.5)
            .mapped(|_| "scroll")
            .id(21),
        ui::icon_button(icon).active(true).message("icon").id(22),
        ui::slider(0.25).compact().message(|_| "slider").id(23),
    ])
    .into_surface();

    let row = widget_ref::<ui::InteractiveRowWidget, _>(&surface, 20, "interactive row");
    assert!(row.props.draggable);
    assert!(row.props.droppable);
    assert!(row.props.drag_active);
    assert_eq!(
        surface.dispatch_widget_output(
            20,
            radiant::widgets::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some("row")
    );

    let scrollbar = widget_ref::<radiant::widgets::ScrollbarWidget, _>(&surface, 21, "scrollbar");
    assert_eq!(scrollbar.props.viewport_fraction, 0.25);
    assert_eq!(scrollbar.state.offset_fraction, 0.5);
    assert_eq!(
        surface.dispatch_widget_output(
            21,
            radiant::widgets::WidgetOutput::typed(
                radiant::widgets::ScrollbarMessage::OffsetChanged {
                    offset_fraction: 0.75,
                }
            ),
        ),
        Some("scroll")
    );

    let icon_button = widget_ref::<ui::IconButtonWidget, _>(&surface, 22, "icon button");
    assert!(icon_button.common.state.active);
    assert_eq!(
        surface.dispatch_widget_output(
            22,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate),
        ),
        Some("icon")
    );

    let slider = widget_ref::<SliderWidget, _>(&surface, 23, "slider");
    assert_eq!(slider.common.sizing.preferred, Vector2::new(92.0, 20.0));
}

#[test]
fn text_input_builder_can_seed_selection_and_route_full_input_events() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<TextInputMessage> = ui::text_input("Rename me")
        .select_all()
        .message_event(|message| message)
        .id(10)
        .into_surface();

    let input = widget_ref::<TextInputWidget, _>(&surface, 10, "text input");
    assert_eq!(input.state.selection_anchor, 0);
    assert_eq!(input.state.caret, "Rename me".chars().count());
    assert_eq!(
        surface.dispatch_widget_output(
            10,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Renamed"),
            }),
        ),
        Some(TextInputMessage::Submitted {
            value: String::from("Renamed"),
        })
    );
}
