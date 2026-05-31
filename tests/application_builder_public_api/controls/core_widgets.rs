use super::super::*;
use radiant::widgets::{
    BadgeMessage, BadgeWidget, ButtonWidget, ColorMarkerWidget, MarkerRunWidget, SliderMessage,
    SliderWidget, TextInputWidget, TextWidget, ToggleWidget, WidgetOutput, WidgetProminence,
    WidgetStyle, WidgetTone,
};

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
            WidgetOutput::typed(SliderMessage::ValueChanged { value: 0.75 }),
        ),
        Some(())
    );
}

#[test]
fn passive_badge_is_prelude_accessible_and_does_not_emit_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::passive_badge("KEEP")
        .style(WidgetStyle {
            tone: WidgetTone::Warning,
            prominence: WidgetProminence::Subtle,
        })
        .id(22)
        .into_surface();

    let badge = widget_ref::<BadgeWidget, _>(&surface, 22, "badge");
    assert_eq!(badge.props.label, "KEEP");
    assert_eq!(badge.common.style.tone, WidgetTone::Warning);
    assert_eq!(badge.common.style.prominence, WidgetProminence::Subtle);
    assert_eq!(
        surface.dispatch_widget_output(22, WidgetOutput::typed(BadgeMessage::Activate)),
        None
    );
}

#[test]
fn color_marker_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(20, 40, 60, 255);
    let surface: UiSurface<()> = ui::color_marker(Some(color))
        .side(6)
        .inset(2)
        .align(ui::ColorMarkerAlign::Left)
        .view()
        .id(23)
        .into_surface();

    let marker = widget_ref::<ColorMarkerWidget, _>(&surface, 23, "color marker");
    assert_eq!(marker.props.color, Some(color));
    assert_eq!(marker.props.side, 6);
    assert_eq!(marker.props.inset, 2);
    assert_eq!(marker.props.align, ui::ColorMarkerAlign::Left);
    assert_eq!(
        surface.dispatch_widget_output(23, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn marker_run_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(80, 180, 90, 255);
    let surface: UiSurface<()> = ui::marker_run(Some(color), 3)
        .side(5)
        .gap(4)
        .inset(2)
        .view()
        .id(24)
        .into_surface();

    let markers = widget_ref::<MarkerRunWidget, _>(&surface, 24, "marker run");
    assert_eq!(markers.props.color, Some(color));
    assert_eq!(markers.props.count, 3);
    assert_eq!(markers.props.side, 5);
    assert_eq!(markers.props.gap, 4);
    assert_eq!(markers.props.inset, 2);
    assert_eq!(
        surface.dispatch_widget_output(24, WidgetOutput::typed(())),
        None
    );
}
