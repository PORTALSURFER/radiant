use super::*;
use radiant::widgets::{
    BadgeMessage, BadgeWidget, ButtonWidget, CardWidget, SelectableMessage, SelectableWidget,
    SliderMessage, SliderWidget, TextInputWidget, ToggleWidget, WidgetProminence, WidgetStyle,
    WidgetTone,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum GalleryMessage {
    Badge,
    Selected(bool),
}

#[test]
fn application_builder_dense_control_panel_uses_generic_focusable_widgets() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::row([
            ui::toggle("Enabled", true).message(|_| ()).id(10),
            ui::toggle("Link", false).message(|_| ()).id(11),
        ])
        .id(2)
        .fill_width(),
        ui::grid_with_gaps(
            (0..3).map(|index| {
                ui::column([
                    ui::text(format!("Param {index}"))
                        .id(100 + index)
                        .height(22.0),
                    ui::row([
                        ui::button("-").subtle().message(()).id(200 + index * 2),
                        ui::button("+").primary().message(()).id(201 + index * 2),
                    ]),
                ])
                .id(50 + index)
                .style(WidgetStyle {
                    tone: WidgetTone::Neutral,
                    prominence: WidgetProminence::Subtle,
                })
                .padding(8.0)
                .height(96.0)
            }),
            3,
            8.0,
            8.0,
        )
        .id(3)
        .fill_width(),
    ])
    .id(1)
    .padding(12.0)
    .spacing(10.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 180.0)),
    );

    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 8);
    assert!(focus_order.contains(&10));
    assert!(focus_order.contains(&205));
    assert_eq!(layout.rects[&50].min.y, layout.rects[&51].min.y);
    assert!(layout.rects[&51].min.x > layout.rects[&50].max.x);
    assert_eq!(layout.rects[&50].height(), 96.0);
}

#[test]
fn application_builder_gallery_widgets_lower_and_route_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<GalleryMessage> = ui::column([
        ui::badge("Ready").message(GalleryMessage::Badge).id(10),
        ui::selectable("Option", false)
            .message(GalleryMessage::Selected)
            .id(11),
        ui::card().id(12).size(160.0, 72.0),
    ])
    .id(1)
    .into_surface();

    let badge = widget_ref::<BadgeWidget, _>(&surface, 10, "badge");
    assert_eq!(badge.props.label, "Ready");
    assert_eq!(
        surface.dispatch_widget_output(
            10,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(GalleryMessage::Badge)
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
fn application_builders_expose_padding_style_and_text_policy_helpers() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::text("Long title").wrap().id(10),
        ui::button("Add").primary().message(()).id(11),
        ui::button("Delete").danger().message(()).id(12),
        ui::checkbox(true).message(|_| ()).id(13),
        ui::text_input("")
            .placeholder("What needs to be done?")
            .message(|_| ())
            .id(14),
        ui::slider(0.4).primary().message(|_| ()).id(15),
    ])
    .id(1)
    .padding(16.0)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 160.0)),
    );

    assert_eq!(layout.rects[&10].min.x, 16.0);
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 10, "text").wrap,
        radiant::widgets::TextWrap::Word
    );
    let primary = widget_ref::<ButtonWidget, _>(&surface, 11, "button");
    assert_eq!(primary.common.style.tone, WidgetTone::Accent);
    assert_eq!(primary.common.style.prominence, WidgetProminence::Strong);
    assert_eq!(
        widget_ref::<ButtonWidget, _>(&surface, 12, "button")
            .common
            .style
            .tone,
        WidgetTone::Danger
    );
    let toggle = widget_ref::<ToggleWidget, _>(&surface, 13, "toggle");
    assert_eq!(toggle.props.label, "");
    assert!(toggle.state.checked);
    assert_eq!(toggle.common.sizing.preferred, Vector2::new(22.0, 22.0));
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&surface, 14, "text input")
            .props
            .placeholder
            .as_deref(),
        Some("What needs to be done?")
    );
    let slider = widget_ref::<SliderWidget, _>(&surface, 15, "slider");
    assert_eq!(slider.state.value, 0.4);
    assert_eq!(slider.common.style.tone, WidgetTone::Accent);
    assert_eq!(slider.common.style.prominence, WidgetProminence::Strong);
    assert_eq!(
        surface.dispatch_widget_output(
            15,
            radiant::widgets::WidgetOutput::typed(SliderMessage::ValueChanged { value: 0.75 }),
        ),
        Some(())
    );
}

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
                ui::InteractiveRowMessage::Drag(_) => "drag",
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
