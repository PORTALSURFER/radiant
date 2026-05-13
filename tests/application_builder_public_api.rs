//! Public API coverage for Radiant application builder ergonomics.

use radiant::{
    layout::{
        LayoutDebugOptions, LayoutState, Point, Rect, Vector2, layout_tree, layout_tree_with_state,
    },
    runtime::{RuntimeBridge, UiSurface, WidgetMessageMapper},
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CardWidget, SelectableMessage,
        SelectableWidget, SliderMessage, SliderWidget, TextInputMessage, TextInputWidget,
        TextWidget, ToggleWidget, Widget, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
    },
};

#[path = "application_builder_public_api/collection_layout.rs"]
mod collection_layout;
#[path = "application_builder_public_api/composition.rs"]
mod composition;
#[path = "application_builder_public_api/prelude_exports.rs"]
mod prelude_exports;
#[path = "application_builder_public_api/runtime_behavior.rs"]
mod runtime_behavior;
#[path = "application_builder_public_api/runtime_options.rs"]
mod runtime_options;
#[path = "application_builder_public_api/typography.rs"]
mod typography;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GalleryMessage {
    Badge,
    Selected(bool),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

fn widget_ref<'a, T, Message>(surface: &'a UiSurface<Message>, id: u64, expected: &str) -> &'a T
where
    T: Widget + 'static,
{
    surface
        .find_widget(id)
        .unwrap_or_else(|| panic!("expected {expected} widget {id} to exist"))
        .widget()
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("expected widget {id} to be {expected}"))
}

#[test]
fn application_builder_accepts_widgets_through_widget_view_trait() {
    use radiant::prelude::{self as ui, IntoView, MappedWidget};

    let surface: UiSurface<DemoMessage> = ui::row([
        ui::widget(TextWidget::new(
            0,
            "Direct",
            WidgetSizing::fixed(Vector2::new(80.0, 20.0)).with_baseline(14.0),
        ))
        .id(20),
        ui::widget(MappedWidget::new(
            ButtonWidget::new(0, "Mapped", WidgetSizing::fixed(Vector2::new(96.0, 28.0))),
            WidgetMessageMapper::button(|_| DemoMessage::Increment),
        ))
        .id(21),
    ])
    .id(10)
    .into_surface();

    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 20, "text").common.id,
        20
    );
    assert_eq!(
        surface.dispatch_widget_output(
            21,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Increment)
    );
}

#[test]
fn application_view_builders_lower_into_runtime_surface_nodes() {
    use radiant::prelude::{self as ui, IntoView};

    let surface = ui::row([
        ui::text("Title").size(96.0, 24.0).baseline(17.0),
        ui::button("Increment")
            .message(DemoMessage::Increment)
            .id(42),
    ])
    .id(1)
    .into_surface();

    assert_eq!(surface.root().id(), 1);
    assert!(surface.find_widget(2).is_some());
    assert!(surface.find_widget(42).is_some());

    let message = surface
        .dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit the configured host message");
    assert_eq!(message, DemoMessage::Increment);
}

#[test]
fn application_builders_support_direct_callbacks_scroll_and_sizing_helpers() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Direct")
        .view(|state| {
            ui::scroll(
                ui::column([
                    ui::text(format!("Count: {}", state.count))
                        .id(10)
                        .fixed(120.0, 24.0)
                        .baseline(17.0),
                    ui::button("Increment")
                        .on_click(|state: &mut DemoState| state.count += 1)
                        .id(11)
                        .size(96.0, 32.0),
                    ui::text_input(state.name.clone())
                        .bind_submit(
                            |state: &mut DemoState| &mut state.name,
                            |state: &mut DemoState| state.count += 1,
                        )
                        .id(12)
                        .min_size(120.0, 28.0)
                        .preferred_size(180.0, 28.0),
                ])
                .id(2),
            )
            .id(1)
        })
        .into_bridge();

    let before = bridge.project_surface();
    assert_eq!(before.root().id(), 1);
    assert!(before.find_widget(10).is_some());
    assert!(before.find_widget(11).is_some());
    assert!(before.find_widget(12).is_some());

    let increment = before
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("direct button should emit a state action");
    let command = bridge.update(increment);
    assert!(command.requests_repaint());

    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Count: 1"
    );

    let submit = after
        .dispatch_widget_output(
            12,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Launch now"),
            }),
        )
        .expect("direct text input submit should emit a state action");
    let command = bridge.update(submit);
    assert!(command.requests_repaint());

    let after_submit = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&after_submit, 12, "text input")
            .state
            .value,
        "Launch now"
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&after_submit, 10, "text").text,
        "Count: 2"
    );
}

#[test]
fn application_bridge_pulls_owned_surfaces_for_runtime_projection() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([ui::text(format!("Count: {}", state.count))
                .id(10)
                .fixed(120.0, 24.0)
                .baseline(17.0)])
            .id(1)
        })
        .update(|state, DemoMessage::Increment| {
            state.count += 1;
        })
        .into_bridge();

    let before = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&before, 10, "text").text,
        "Count: 0"
    );

    bridge.update(DemoMessage::Increment);
    let after = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Count: 1"
    );
}

#[test]
fn details_columns_use_logical_widths() {
    use radiant::prelude::DetailsColumn;

    assert_eq!(
        DetailsColumn::fixed("kind", "Kind", 120.5),
        DetailsColumn {
            id: String::from("kind"),
            label: String::from("Kind"),
            width: Some(120.5),
        }
    );
    assert_eq!(DetailsColumn::flexible("name", "Name").width, None);
}

#[test]
fn application_builders_scope_keys_and_bind_text_inputs_to_state_fields() {
    use radiant::prelude::{self as ui, IntoView};

    let surface = ui::column_key(
        "todos",
        [
            ui::row_key(
                1_u64,
                [
                    ui::text("First").key("label"),
                    ui::button("Delete")
                        .on_click(|state: &mut DemoState| state.count += 1)
                        .key("delete"),
                ],
            ),
            ui::row_key(
                2_u64,
                [
                    ui::text("Second").key("label"),
                    ui::button("Delete")
                        .on_click(|state: &mut DemoState| state.count += 1)
                        .key("delete"),
                ],
            ),
            ui::text_input(String::from("Draft"))
                .bind(|state: &mut DemoState| &mut state.name)
                .key("draft"),
        ],
    )
    .into_surface();

    let ids = surface
        .keyboard_focus_order()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(ids.len(), 3);
    for id in ids {
        assert!(surface.find_widget(id).is_some());
    }
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
