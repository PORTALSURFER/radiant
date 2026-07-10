use super::super::*;
use radiant::widgets::{
    BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, ColorMarkerRunWidget,
    ColorMarkerWidget, DragHandleMessage, FeedbackOverlayWidget, FocusBehavior, IconButtonWidget,
    MarkerRunWidget, PaintBounds, SelectableWidget, SliderMessage, SliderWidget, TextInputWidget,
    TextWidget, ToggleWidget, WidgetOutput, WidgetProminence, WidgetStyle, WidgetTone,
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
fn button_row_groups_app_owned_buttons_with_compact_geometry() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::column([ui::button_row([
        ui::button("Apply")
            .primary()
            .message(DemoMessage::Increment)
            .id(20)
            .width(72.0),
        ui::button("Cancel")
            .message(DemoMessage::Increment)
            .id(21)
            .width(68.0),
    ])
    .id(10)])
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 40.0)),
    );

    assert_eq!(surface.keyboard_focus_order(), vec![20, 21]);
    assert_eq!(layout.rects[&10].height(), 26.0);
    assert_eq!(layout.rects[&20].height(), 24.0);
    assert!((layout.rects[&21].min.x - layout.rects[&20].max.x - 6.0).abs() < 0.01);
    assert_eq!(
        surface.dispatch_widget_output(20, WidgetOutput::typed(ButtonMessage::Activate)),
        Some(DemoMessage::Increment)
    );
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
fn button_builder_can_filter_secondary_activation_and_map_drag() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<&'static str> = ui::button("Name")
        .click_or_drag("sort", |drag| match drag {
            DragHandleMessage::Started { .. } => "drag-start",
            DragHandleMessage::Moved { .. } => "drag-move",
            DragHandleMessage::Ended { .. } => "drag-end",
            DragHandleMessage::DoubleActivate { .. } => "drag-double",
            DragHandleMessage::Cancelled { .. } => "drag-cancel",
        })
        .id(27)
        .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(27, WidgetOutput::typed(ButtonMessage::Activate)),
        Some("sort")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0)
            }),
        ),
        None
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            WidgetOutput::typed(ButtonMessage::Drag(DragHandleMessage::Moved {
                position: ui::Point::new(3.0, 4.0)
            })),
        ),
        Some("drag-move")
    );
}

#[test]
fn constant_button_message_maps_all_enabled_button_outputs() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<&'static str> = ui::button("Run")
        .secondary_clicks()
        .draggable()
        .message("run")
        .id(28)
        .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(28, WidgetOutput::typed(ButtonMessage::Activate)),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::ActivateWithModifiers {
                modifiers: Default::default(),
            }),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::Drag(DragHandleMessage::Moved {
                position: ui::Point::new(3.0, 4.0),
            })),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(28, WidgetOutput::typed(BadgeMessage::Activate)),
        None
    );
}

#[test]
fn dynamic_button_mappers_keep_secondary_and_filtered_behavior() {
    use radiant::prelude::{self as ui, IntoView};

    let mapped: UiSurface<&'static str> = ui::button("Mapped")
        .mapped(|message| {
            if message.is_activate() {
                "activate"
            } else {
                "other"
            }
        })
        .id(29)
        .into_surface();
    assert_eq!(
        mapped.dispatch_widget_output(
            29,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some("other")
    );

    let filtered: UiSurface<&'static str> = ui::button("Filtered")
        .filter_mapped(|message| message.is_activate().then_some("activate"))
        .id(30)
        .into_surface();
    assert_eq!(
        filtered.dispatch_widget_output(30, WidgetOutput::typed(ButtonMessage::Activate)),
        Some("activate")
    );
    assert_eq!(
        filtered.dispatch_widget_output(
            30,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        None
    );
}

#[test]
fn icon_button_builder_supports_message_and_passive_apps() {
    use radiant::prelude::{self as ui, IntoView};

    let message_surface: UiSurface<DemoMessage> = ui::disclosure_button(true)
        .message(DemoMessage::Increment)
        .id(31)
        .into_surface();
    assert!(
        !widget_ref::<IconButtonWidget, _>(&message_surface, 31, "icon button")
            .common
            .state
            .active
    );
    assert_eq!(
        message_surface.dispatch_widget_output(31, WidgetOutput::typed(ButtonMessage::Activate)),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        message_surface.dispatch_widget_output(
            31,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some(DemoMessage::Increment)
    );

    let passive_surface: UiSurface<DemoState> = ui::close_button().passive().id(32).into_surface();
    assert!(passive_surface.find_widget(32).is_some());
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
fn selectable_builder_supports_color_marker_adornment() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(20, 120, 80, 255);
    let surface: UiSurface<()> = ui::selectable("Ready", true)
        .color_marker(Some(color))
        .color_marker_side(6)
        .color_marker_inset(2)
        .color_marker_align(ui::ColorMarkerAlign::Left)
        .message(|_| ())
        .id(28)
        .into_surface();

    let selectable = widget_ref::<SelectableWidget, _>(&surface, 28, "selectable");
    let marker = selectable.props.color_marker.expect("selectable marker");
    assert_eq!(marker.color, Some(color));
    assert_eq!(marker.side, 6);
    assert_eq!(marker.inset, 2);
    assert_eq!(marker.align, ui::ColorMarkerAlign::Left);

    let ordered_surface: UiSurface<()> = ui::selectable("Queued", false)
        .color_marker_side(7)
        .color_marker_inset(1)
        .color_marker(Some(color))
        .message(|_| ())
        .id(29)
        .into_surface();
    let ordered = widget_ref::<SelectableWidget, _>(&ordered_surface, 29, "selectable");
    let ordered_marker = ordered.props.color_marker.expect("selectable marker");
    assert_eq!(ordered_marker.color, Some(color));
    assert_eq!(ordered_marker.side, 7);
    assert_eq!(ordered_marker.inset, 1);
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

#[test]
fn marker_run_colors_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let first = ui::Rgba8::new(80, 180, 90, 255);
    let second = ui::Rgba8::new(180, 90, 80, 255);
    let surface: UiSurface<()> = ui::marker_run_colors([first, second])
        .side(5)
        .gap(4)
        .inset(2)
        .view()
        .id(25)
        .into_surface();

    let markers = widget_ref::<ColorMarkerRunWidget, _>(&surface, 25, "marker run colors");
    assert_eq!(markers.props.colors, vec![first, second]);
    assert_eq!(markers.props.side, 5);
    assert_eq!(markers.props.gap, 4);
    assert_eq!(markers.props.inset, 2);
    assert_eq!(
        surface.dispatch_widget_output(25, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn feedback_overlay_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let background = ui::Rgba8::new(20, 24, 28, 90);
    let fill = ui::Rgba8::new(180, 190, 200, 120);
    let edge = ui::Rgba8::new(60, 200, 120, 220);
    let surface: UiSurface<()> = ui::feedback_overlay()
        .background(background)
        .progress(0.5, fill)
        .edge(
            edge,
            2.0,
            ui::BorderSides {
                top: true,
                bottom: true,
                left: false,
                right: false,
            },
        )
        .view()
        .id(26)
        .into_surface();

    let overlay = widget_ref::<FeedbackOverlayWidget, _>(&surface, 26, "feedback overlay");
    assert_eq!(overlay.props.background, Some(background));
    assert_eq!(overlay.props.progress.expect("progress").fraction, 0.5);
    assert_eq!(overlay.props.progress.expect("progress").color, fill);
    assert_eq!(overlay.props.edge.expect("edge").color, edge);
    assert_eq!(
        surface.dispatch_widget_output(26, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn interactive_row_builder_exposes_drag_source_configuration() {
    use radiant::prelude::{self as ui, IntoView};
    use radiant::widgets::InteractiveRowWidget;

    let surface: UiSurface<DemoMessage> = ui::interactive_row()
        .draggable()
        .drag_active(true)
        .drag_source(true)
        .drag_source_motion(true)
        .suppress_hover(true)
        .clear_hover_on_sync()
        .mapped(|_| DemoMessage::Increment)
        .id(25)
        .into_surface();

    let row = widget_ref::<InteractiveRowWidget, _>(&surface, 25, "interactive row");
    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.drag_source_motion);
    assert!(row.props.suppress_hover);
    assert!(row.props.clear_hover_on_sync);
}

#[test]
fn interactive_row_builder_can_create_custom_input_layer_widget() {
    use radiant::prelude as ui;

    let row = ui::interactive_row()
        .draggable()
        .drag_active(true)
        .drag_source(true)
        .drag_source_motion(true)
        .activation_modifiers()
        .custom_paint_hit_target()
        .widget();

    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.drag_source_motion);
    assert!(row.props.activation_modifiers);
    assert_eq!(row.common.focus, FocusBehavior::None);
    assert_eq!(row.common.paint.bounds, PaintBounds::ClipToRect);
    assert!(!row.common.paint.paints_focus);
    assert!(!row.common.paint.paints_state_layers);
}
