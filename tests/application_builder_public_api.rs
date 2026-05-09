//! Public API coverage for Radiant application builder ergonomics.

use radiant::{
    layout::{
        LayoutDebugOptions, LayoutState, Point, Rect, Vector2, layout_tree, layout_tree_with_state,
    },
    runtime::{
        Command, DEFAULT_NATIVE_WINDOW_TITLE, NativeRunOptions, RuntimeBridge, SurfaceRuntime,
        UiSurface, WidgetMessageMapper,
    },
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, TextWrap,
        ToggleWidget, Widget, WidgetProminence, WidgetSizing, WidgetTone,
    },
};
use std::{thread, time::Duration};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LoadingMessage {
    Start,
    Loaded(String),
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
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.title, "Radiant");
    assert!(options.drag_and_drop);
}

#[test]
fn native_run_options_expose_platform_neutral_drag_and_drop_policy() {
    let options = NativeRunOptions {
        drag_and_drop: false,
        ..NativeRunOptions::default()
    };

    assert!(!options.drag_and_drop);
}

#[test]
fn application_builder_animation_frames_route_through_public_app_path() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().needs_animation());
    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 1"
    );
}

#[test]
fn application_builder_background_spawn_routes_worker_result() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text(format!("Loaded: {}", state.name))
                    .id(10)
                    .height(24.0),
                ui::button("Load")
                    .message(LoadingMessage::Start)
                    .id(11)
                    .height(28.0),
            ])
        })
        .update_with(|state, message, context| match message {
            LoadingMessage::Start => {
                state.name = "loading".to_string();
                context.spawn(
                    "test-loader",
                    || "ready".to_string(),
                    LoadingMessage::Loaded,
                );
                context.request_repaint();
            }
            LoadingMessage::Loaded(value) => {
                state.name = value;
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 80.0));
    let start = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("load button should emit a start message");

    let started = runtime.dispatch_message(start);
    assert!(started.repaint_requested);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: loading"
    );

    thread::sleep(Duration::from_millis(10));
    let finished = runtime.drain_runtime_messages();
    assert_eq!(finished.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: ready"
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
fn application_builder_lists_keep_row_heights_stable_across_item_counts() {
    use radiant::prelude::{self as ui, IntoView};

    fn surface(count: u64) -> UiSurface<()> {
        ui::column([ui::list(0..count, |index| {
            ui::list_row(
                index,
                [
                    ui::text(format!("Item {index}"))
                        .id(100 + index)
                        .fill_width(),
                    ui::button("Delete").danger().message(()).id(200 + index),
                ],
            )
            .id(10 + index)
        })
        .id(2)])
        .id(1)
        .padding(12.0)
        .into_surface()
    }

    let two = layout_tree(
        &surface(2).layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 200.0)),
    );
    let ten = layout_tree(
        &surface(10).layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(300.0, 200.0)),
    );

    assert_eq!(two.rects[&10].height(), 52.0);
    assert_eq!(two.rects[&11].height(), 52.0);
    assert_eq!(ten.rects[&10].height(), 52.0);
    assert_eq!(ten.rects[&11].height(), 52.0);
}

#[test]
fn application_builder_virtual_list_records_virtual_window() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::virtual_list(
        0..512_u64,
        |index| {
            ui::list_row(
                index,
                [ui::button(format!("Row {index:03}"))
                    .message(DemoMessage::Increment)
                    .id(1_000 + index)],
            )
            .id(10_000 + index)
        },
        64.0,
    )
    .id(2)
    .into_surface();
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(2, Vector2::new(0.0, 640.0));

    let output = layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&2)
        .expect("virtual_list should lower to a virtualized scroll viewport");

    assert_eq!(window.total_children, 512);
    assert!(window.first_index > 0);
    assert!(window.culled_after > 0);
    assert!(surface.find_widget(1_000).is_some());
}

#[test]
fn application_builder_property_panel_routes_row_selection() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::selectable_property_panel(
                "Inspector",
                [
                    ui::PropertyRow::new("name", "Name", state.name.clone())
                        .selected(state.name == "name"),
                    ui::PropertyRow::new("count", "Count", state.count.to_string())
                        .selected(state.name == "count"),
                ],
                Some(|state: &mut DemoState, id| state.name = id),
            )
        })
        .into_bridge();

    let surface = bridge.project_surface();
    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 2);

    let message = surface
        .dispatch_widget_output(
            focus_order[1],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("property value button should emit a state action");
    let command = bridge.update(message);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<ButtonWidget, _>(&after, focus_order[0], "button")
            .props
            .label,
        "count"
    );
}

#[test]
fn application_builder_property_panel_read_only_rows_do_not_join_focus_order() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<ui::StateAction<DemoState>> = ui::property_panel(
        "Inspector",
        [
            ui::PropertyRow::new("name", "Name", "Layer 12"),
            ui::PropertyRow::new("kind", "Kind", "Signal track").selected(true),
        ],
    )
    .id(1)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(280.0, 120.0)),
    );

    assert!(surface.keyboard_focus_order().is_empty());
    assert_eq!(layout.rects[&1].min.x, 0.0);
    assert!(layout.rects[&1].height() <= 120.0);
}

#[test]
fn application_builder_context_menu_overlay_routes_items() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::stack([
                ui::text(format!("Selected: {}", state.name))
                    .id(10)
                    .height(24.0)
                    .fill_width(),
                ui::context_menu_overlay(
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
                    Point::new(260.0, 150.0),
                    Vector2::new(140.0, 92.0),
                    "Actions",
                    [
                        ui::MenuItem::new("Inspect", |state: &mut DemoState| {
                            state.name = "inspect".to_string()
                        })
                        .primary(),
                        ui::MenuItem::new("Delete", |state: &mut DemoState| {
                            state.name = "delete".to_string()
                        })
                        .danger(),
                    ],
                )
                .id(20),
            ])
        })
        .into_bridge();

    let surface = bridge.project_surface();
    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 2);

    let message = surface
        .dispatch_widget_output(
            focus_order[1],
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("context menu item should emit a state action");
    let command = bridge.update(message);
    assert!(command.requests_repaint());

    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Selected: delete"
    );

    let layout = layout_tree(
        &after.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 180.0)),
    );
    assert_eq!(layout.rects[&20].min.x, 0.0);
}

#[test]
fn application_builder_todo_layout_does_not_overlap_header_input_and_list() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::row([
            ui::text("Todos").id(10).size(140.0, 28.0),
            ui::text("1/3 done").id(11).size(120.0, 28.0),
        ])
        .id(2)
        .fill_width(),
        ui::row([
            ui::text_input("Review public API")
                .message(|_| ())
                .id(12)
                .min_size(260.0, 32.0)
                .preferred_size(420.0, 32.0)
                .fill_width(),
            ui::button("Add")
                .primary()
                .message(())
                .id(13)
                .size(80.0, 32.0),
        ])
        .id(3)
        .fill_width(),
        ui::list(0..3, |index| {
            ui::list_row(
                index,
                [
                    ui::checkbox(false)
                        .message(|_| ())
                        .id(20 + index)
                        .size(24.0, 24.0),
                    ui::text(format!("Item {index}"))
                        .id(60 + index)
                        .fill_width(),
                    ui::button("Delete")
                        .danger()
                        .message(())
                        .id(30 + index)
                        .size(84.0, 30.0),
                ],
            )
            .id(40 + index)
        })
        .id(4),
    ])
    .id(1)
    .padding(16.0)
    .spacing(12.0)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(560.0, 360.0)),
    );

    let header = layout.rects[&2];
    let input = layout.rects[&3];
    let list = layout.rects[&4];
    let first_row = layout.rects[&40];

    assert_eq!(header.height(), 28.0);
    assert_eq!(input.height(), 32.0);
    assert!(input.min.y >= header.max.y + 12.0);
    assert!(list.min.y >= input.max.y + 12.0);
    assert!(first_row.min.y >= list.min.y);
    assert_eq!(first_row.height(), 52.0);
}

#[test]
fn application_builder_typography_helpers_lower_text_policies_and_baselines() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::text("Wrapped text that should stay inside the assigned text rectangle")
            .wrap()
            .id(10)
            .fill_width()
            .height(64.0)
            .baseline(18.0),
        ui::text("Truncated text that keeps one line")
            .truncate()
            .id(11)
            .fill_width()
            .height(28.0)
            .baseline(19.0),
        ui::row([
            ui::text("Name").id(12).size(80.0, 28.0).baseline(19.0),
            ui::text("Radiant")
                .id(13)
                .fill_width()
                .height(28.0)
                .baseline(19.0),
        ])
        .id(20)
        .fill_width()
        .spacing(8.0),
    ])
    .id(1)
    .padding(16.0)
    .spacing(10.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(360.0, 180.0)),
    );

    let wrapped = widget_ref::<TextWidget, _>(&surface, 10, "wrapped text");
    let truncated = widget_ref::<TextWidget, _>(&surface, 11, "truncated text");
    assert_eq!(wrapped.wrap, TextWrap::Word);
    assert_eq!(wrapped.common.sizing.baseline, Some(18.0));
    assert_eq!(truncated.wrap, TextWrap::None);
    assert_eq!(truncated.common.sizing.baseline, Some(19.0));
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 12, "label")
            .common
            .sizing
            .baseline,
        Some(19.0)
    );
    assert_eq!(layout.rects[&10].height(), 64.0);
    assert_eq!(layout.rects[&11].height(), 28.0);
    assert_eq!(layout.rects[&12].height(), layout.rects[&13].height());
    assert!(layout.rects[&13].min.x >= layout.rects[&12].max.x + 8.0);
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
}

#[test]
fn prelude_supports_hello_world_imports() {
    use radiant::prelude::*;

    fn hello_body() -> impl IntoView<()> {
        text("Hello, world!")
    }

    let surface = hello_body().into_surface();

    assert!(surface.find_widget(1).is_some());
}

#[test]
fn hello_world_example_stays_on_application_builders() {
    let source = include_str!("../examples/hello_world.rs");

    assert!(source.contains("use radiant::prelude::*;"));
    assert!(source.contains("radiant::window(\"Radiant Hello World\")"));
    assert!(source.contains(".run(text(\"Hello, world!\"))"));
    assert!(!source.contains("NativeRunOptions"));
    assert!(!source.contains("RuntimeBridge"));
    assert!(!source.contains("SurfaceChild"));
    assert!(!source.contains("WidgetSizing"));
    assert!(!source.contains("declarative_command_runtime_bridge"));
}

#[test]
fn stateful_app_builder_projects_updates_and_preserves_commands() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Counter")
        .size(320, 120)
        .view(|state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)),
                ui::button("Increment").message(DemoMessage::Increment),
            ])
        })
        .update_command(|state, message| match message {
            DemoMessage::Increment => {
                state.count += 1;
                Command::request_repaint()
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let increment = before
        .dispatch_widget_output(
            3,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 2, "text").text,
        "Count: 1"
    );
}
