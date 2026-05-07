//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{
        Constraints, LayoutDebugOptions, LayoutState, Point, Rect, SizeModeCross, SizeModeMain,
        SlotParams, Vector2, VirtualizationAxis, layout_tree, layout_tree_with_state,
    },
    runtime::{
        App, Command, DEFAULT_NATIVE_WINDOW_TITLE, Element, Event, FocusTraversal,
        NativeRunOptions, PaintPrimitive, Renderer, RuntimeBridge, SurfaceChild, SurfaceNode,
        SurfaceRuntime, UiSurface, View, WidgetMessageMapper, declarative_command_runtime_bridge,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        BadgeMessage, ButtonMessage, ButtonWidget, CanvasMessage, CanvasWidget, DragHandleMessage,
        DragHandleWidget, ListItemMessage, ListItemWidget, PointerButton,
        RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget,
        SelectableMessage, SelectableWidget, TextEditCommand, TextInputMessage, TextInputWidget,
        TextWidget, ToggleMessage, ToggleWidget, Widget, WidgetCommon, WidgetInput, WidgetKey,
        WidgetOutput, WidgetProminence, WidgetSizing, WidgetState, WidgetStyle, WidgetTone,
        resolve_widget_visual_tokens,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    SetActive(bool),
    CanvasInput(WidgetInput),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CustomWidgetMessage {
    Activated,
}

#[derive(Clone)]
struct CustomStatusWidget {
    common: WidgetCommon,
    label: &'static str,
}

impl CustomStatusWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
        common.focus = radiant::widgets::FocusBehavior::Keyboard;
        Self {
            common,
            label: "custom",
        }
    }
}

impl Widget for CustomStatusWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::KeyPress(WidgetKey::Enter) if self.common.state.focused => {
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(radiant::runtime::PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: if self.common.state.hovered {
                theme.accent_danger
            } else {
                theme.surface_base
            },
        }));
        primitives.push(PaintPrimitive::Text(radiant::runtime::PaintTextRun {
            widget_id: self.common.id,
            text: self.label.to_owned(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(18.0),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Center,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }
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
fn generic_runtime_surface_projects_layout_without_legacy_app_contracts() {
    let surface = project_surface(&mut DemoState::default());
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0)),
    );

    assert!(output.rects.contains_key(&10));
    assert!(output.rects.contains_key(&11));
    assert!(output.rects.contains_key(&12));
}

#[test]
fn retained_canvas_metadata_reaches_backend_neutral_paint_plan() {
    let retained = RetainedSurfaceDescriptor {
        key: 42,
        revision: 7,
        dirty_mask: 0b101,
        volatile: false,
    };
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::retained_canvas_mapped(
        90,
        WidgetSizing::fixed(Vector2::new(240.0, 120.0)),
        retained,
        |message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        },
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let Some(PaintPrimitive::CustomSurface(custom)) = plan.primitives.first() else {
        panic!("retained canvas should emit one custom surface primitive");
    };
    assert_eq!(custom.widget_id, 90);
    assert_eq!(custom.retained, Some(retained));
}

#[test]
fn custom_widget_travels_through_runtime_input_message_and_paint_paths() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::custom_widget(
        CustomStatusWidget::new(91),
        WidgetMessageMapper::dynamic(|output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|message| DemoMessage::Rename(format!("{message:?}")))
        }),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0)),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    assert!(matches!(
        plan.primitives.first(),
        Some(PaintPrimitive::FillRect(fill)) if fill.widget_id == 91
    ));

    let mut interactive = surface.clone();
    let output = interactive
        .dispatch_widget_input(
            91,
            layout.rects[&91],
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 12.0),
                button: PointerButton::Primary,
            },
        )
        .expect("custom widget should emit output");
    let message = surface
        .dispatch_widget_output(91, output)
        .expect("custom output should map to a host message");

    assert_eq!(message, DemoMessage::Rename("Activated".to_owned()));
}

#[test]
fn application_builder_accepts_custom_widgets_with_generated_and_explicit_ids() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::column([
        ui::custom_widget(CustomStatusWidget::new(1), |output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|_| DemoMessage::SetActive(true))
        })
        .key("generated-custom"),
        ui::custom_widget(CustomStatusWidget::new(2), |output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|_| DemoMessage::SetActive(false))
        })
        .id(77),
    ])
    .id(10)
    .into_surface();

    assert!(surface.find_widget(77).is_some());
    assert_eq!(
        surface.find_widget(77).unwrap().widget_object().common().id,
        77
    );
    assert_eq!(surface.keyboard_focus_order().len(), 2);
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
fn checkbox_hover_changes_paint_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::checkbox(false)
        .message(|_| DemoMessage::SetActive(false))
        .id(10)
        .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(64.0, 64.0));
    let theme = ThemeTokens::default();
    let before = widget_fill_color(&runtime.paint_plan(&theme), 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = widget_fill_color(&runtime.paint_plan(&theme), 10);

    assert_ne!(before, after);
}

#[test]
fn styled_list_row_hover_changes_container_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> =
        ui::list_row("row", [ui::text("Hover row").id(20).fill_width()])
            .id(10)
            .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 52.0));
    let theme = ThemeTokens::default();
    let before_plan = runtime.paint_plan(&theme);
    let before = widget_fill_color(&before_plan, 10);
    let border_before = widget_stroke_color(&before_plan, 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after_plan = runtime.paint_plan(&theme);
    let after = widget_fill_color(&after_plan, 10);
    let border_after = widget_stroke_color(&after_plan, 10);
    let hover_markers: Vec<_> = after_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == 10
                    && fill.color == theme.accent_danger
                    && fill.rect.width() <= 3.0 =>
            {
                Some(fill)
            }
            _ => None,
        })
        .collect();

    assert_eq!(runtime.hovered_container(), Some(10));
    assert_ne!(before, after);
    assert_eq!(border_before, border_after);
    assert_eq!(hover_markers.len(), 1);
}

#[test]
fn static_styled_container_does_not_hover_without_opt_in() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::row([ui::text("Header").id(20)])
        .id(10)
        .style(WidgetStyle::default())
        .padding(8.0)
        .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 52.0));
    let theme = ThemeTokens::default();
    let before = widget_fill_color(&runtime.paint_plan(&theme), 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = widget_fill_color(&runtime.paint_plan(&theme), 10);

    assert_eq!(runtime.hovered_container(), None);
    assert_eq!(before, after);
}

#[test]
fn control_hover_suppresses_surrounding_container_hover_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::column([ui::text_input("")
        .placeholder("Type")
        .message(DemoMessage::Rename)
        .id(20)
        .size(180.0, 32.0)])
    .id(10)
    .style(WidgetStyle::default())
    .padding(8.0)
    .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 64.0));
    let theme = ThemeTokens::default();
    let before = runtime.paint_plan(&theme);
    let body_before = widget_fill_color(&before, 10);
    let input_before = widget_fill_color(&before, 20);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = runtime.paint_plan(&theme);

    assert_eq!(runtime.hovered_widget(), Some(20));
    assert_eq!(runtime.hovered_container(), None);
    assert_eq!(body_before, widget_fill_color(&after, 10));
    assert_ne!(input_before, widget_fill_color(&after, 20));
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
            DemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
            DemoMessage::SetActive(_) | DemoMessage::CanvasInput(_) => Command::none(),
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

#[test]
fn generic_runtime_bridge_projects_and_reduces_host_defined_messages() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );

    let surface_before = bridge.project_surface();
    let rename = surface_before
        .dispatch_widget_output(
            12,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Projects"),
            }),
        )
        .expect("text input should emit a host-defined rename message");
    bridge.reduce_message(rename);

    let increment = bridge
        .project_surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit a host-defined increment message");
    bridge.reduce_message(increment);

    let surface_after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface_after, 10, "text").text,
        "Projects (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&surface_after, 12, "text input")
            .state
            .value,
        "Projects"
    );
}

#[test]
fn runtime_bridge_is_the_public_app_contract() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );

    let surface = project_app_once(&mut bridge);
    assert!(surface.find_widget(10).is_some());
}

#[test]
fn runtime_bridge_accepts_repaint_signal_for_host_background_work() {
    let called = Arc::new(AtomicBool::new(false));
    let mut bridge = RepaintSignalBridge::default();

    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::clone(&called),
    }));
    bridge.request_worker_repaint();

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn runtime_bridge_exposes_host_owned_runtime_exit_artifact() {
    let mut bridge = RuntimeExitBridge;

    assert_eq!(
        bridge.on_runtime_exit(),
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    );
}

#[test]
fn view_and_element_aliases_match_runtime_surface_types() {
    let surface: Arc<View<DemoMessage>> = project_surface(&mut DemoState::default());
    let root: &Element<DemoMessage> = surface.root();

    assert_eq!(root.id(), 1);
    assert!(surface.find_widget(11).is_some());
}

#[test]
fn surface_node_row_column_and_fill_helpers_project_layout() {
    let header = TextWidget::new(
        20,
        "Header",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    );
    let primary = ButtonWidget::new(21, "Primary", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let secondary = ButtonWidget::new(
        22,
        "Secondary",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );

    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::column(
        2,
        6.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(header)),
            SurfaceChild::fill(SurfaceNode::row(
                3,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(
                        primary,
                        WidgetMessageMapper::button(|_| DemoMessage::Increment),
                    )),
                    SurfaceChild::fill(SurfaceNode::widget(
                        secondary,
                        WidgetMessageMapper::button(|_| DemoMessage::Increment),
                    )),
                ],
            )),
        ],
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 80.0)),
    );

    assert!(output.rects.contains_key(&2));
    assert!(output.rects.contains_key(&3));
    assert!(output.rects.contains_key(&20));
    assert!(output.rects.contains_key(&21));
    assert!(output.rects.contains_key(&22));
}

#[test]
fn surface_node_grid_helper_projects_tile_layout() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::grid(
        28,
        2,
        10.0,
        5.0,
        vec![
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(29, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(30, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::card(35, WidgetSizing::fixed(Vector2::new(40.0, 24.0))),
            ),
        ],
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    let first = output.rects.get(&29).expect("first tile");
    let second = output.rects.get(&30).expect("second tile");
    let third = output.rects.get(&35).expect("third tile");

    assert!(second.min.x > first.min.x);
    assert_eq!(first.min.y, second.min.y);
    assert!(third.min.y > first.min.y);
}

#[test]
fn surface_node_stack_and_card_helpers_project_grouped_surface() {
    let image = Arc::new(ImageRgba::new(1, 1, vec![0, 128, 255, 255]).unwrap());
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::stack(
        23,
        vec![
            SurfaceChild::fill(SurfaceNode::card(
                24,
                WidgetSizing::fixed(Vector2::new(180.0, 96.0)),
            )),
            SurfaceChild::fill(SurfaceNode::column(
                25,
                4.0,
                vec![SurfaceChild::fill(SurfaceNode::text(
                    26,
                    "Overview",
                    WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
                ))],
            )),
            SurfaceChild::fill(SurfaceNode::image(
                27,
                Arc::clone(&image),
                WidgetSizing::fixed(Vector2::new(16.0, 16.0)),
            )),
        ],
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 96.0)),
    );
    let theme = ThemeTokens::default();
    let plan = surface.paint_plan(&output, &theme);

    assert_eq!(output.rects.get(&24), output.rects.get(&25));
    assert!(surface.find_widget(24).is_some());
    assert_eq!(
        plan.primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill.widget_id),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![24]
    );
    assert_eq!(
        plan.primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::Image(draw) => Some((draw.widget_id, draw.image.width)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(27, 1)]
    );
}

#[test]
fn overlay_panel_nodes_paint_without_joining_widget_hit_testing() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::stack(
        1,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                2,
                "Content",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            )),
            SurfaceChild::fill(SurfaceNode::overlay_panel(
                3,
                Rect::from_min_size(Point::new(12.0, 18.0), Vector2::new(180.0, 44.0)),
                "Dragging",
                WidgetStyle {
                    tone: WidgetTone::Accent,
                    prominence: WidgetProminence::Subtle,
                },
            )),
        ],
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 96.0)),
    );
    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    assert!(surface.find_widget(3).is_none());
    assert!(
        plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.widget_id == 3 && text.text == "Dragging")
        )
    );
}

#[test]
fn surface_runtime_hit_testing_prefers_topmost_declarative_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |_state: &mut DemoState| {
            Arc::new(UiSurface::new(SurfaceNode::stack(
                70,
                vec![
                    SurfaceChild::fill(SurfaceNode::button(
                        80,
                        "Bottom",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Increment,
                    )),
                    SurfaceChild::fill(SurfaceNode::button(
                        90,
                        "Top",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Rename(String::from("top")),
                    )),
                ],
            )))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(16.0, 16.0)), Some(90));
}

#[test]
fn surface_node_scroll_area_helpers_project_scroll_view_layout() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(220.0, 160.0)),
        ),
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );
    let overflow = output
        .overflow_flags
        .get(&31)
        .expect("scroll area should report overflow");

    assert!(surface.find_widget(32).is_some());
    assert!(overflow.x);
    assert!(overflow.y);
}

#[test]
fn surface_paint_plan_clips_scroll_content_and_draws_scrollbar_affordance() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
            (0..8)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should start a clip");
    let PaintPrimitive::ClipStart(clip) = &plan.primitives[clip_start] else {
        unreachable!("clip_start index was matched above");
    };
    let clip_end = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should end the clip");
    let scrollbar_fills: Vec<_> = plan
        .primitives
        .iter()
        .skip(clip_end + 1)
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill),
            _ => None,
        })
        .collect();

    assert!(clip_start < clip_end);
    assert_eq!(clip.rect, layout.rects[&31]);
    assert_eq!(scrollbar_fills.len(), 1);
    assert!(scrollbar_fills[0].rect.width() <= 3.0);
    assert_eq!(scrollbar_fills[0].rect.max.x, layout.rects[&31].max.x);
}

#[test]
fn surface_runtime_routes_scroll_delta_to_scroll_view_under_pointer() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
                (0..12)
                    .map(|index| {
                        SurfaceChild::new(
                            intrinsic_slot(),
                            SurfaceNode::text(
                                100 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                            ),
                        )
                    })
                    .collect(),
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let before = runtime.layout().rects[&100];

    assert!(runtime.scroll_at(Point::new(20.0, 20.0), Vector2::new(0.0, 48.0)));
    let after = runtime.layout().rects[&100];

    assert!(after.min.y < before.min.y);
    assert_eq!(before.min.y - after.min.y, 48.0);
}

#[test]
fn surface_runtime_does_not_hit_scrolled_content_outside_scroll_viewport() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::column(
            1,
            8.0,
            vec![
                SurfaceChild::new(
                    intrinsic_slot(),
                    SurfaceNode::button(
                        10,
                        "Header",
                        WidgetSizing::fixed(Vector2::new(220.0, 32.0)),
                        DemoMessage::Increment,
                    ),
                ),
                SurfaceChild::fill(SurfaceNode::scroll_area(
                    20,
                    SurfaceNode::column(
                        21,
                        2.0,
                        (0..12)
                            .map(|index| {
                                SurfaceChild::new(
                                    intrinsic_slot(),
                                    SurfaceNode::button(
                                        100 + index,
                                        format!("Row {index}"),
                                        WidgetSizing::fixed(Vector2::new(220.0, 30.0)),
                                        DemoMessage::Increment,
                                    ),
                                )
                            })
                            .collect(),
                    ),
                )),
            ],
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));

    assert!(runtime.scroll_at(Point::new(20.0, 60.0), Vector2::new(0.0, 80.0)));

    assert_eq!(runtime.widget_at(Point::new(20.0, 16.0)), Some(10));
}

#[test]
fn surface_node_virtual_scroll_area_helper_records_virtual_window() {
    let rows = (0..256)
        .map(|index| {
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::text(
                    1000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 10.0)),
                ),
            )
        })
        .collect::<Vec<_>>();
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        33,
        SurfaceNode::column(34, 1.0, rows),
        VirtualizationAxis::Vertical,
        0.0,
    ));
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(33, Vector2::new(0.0, 400.0));

    let output = layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&33)
        .expect("virtual scroll area should report a virtual window");

    assert_eq!(window.total_children, 256);
    assert!(window.first_index > 0);
    assert!(window.culled_after > 0);
}

#[test]
fn static_widget_helper_builds_non_emitting_leaf() {
    let title = TextWidget::new(
        30,
        "Status",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    );
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::static_widget(title));

    assert!(surface.find_widget(30).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        None
    );
}

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

#[test]
fn text_and_button_helpers_build_common_leaf_nodes() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        4,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                40,
                "Counter",
                WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
            )),
            SurfaceChild::fill(SurfaceNode::button(
                41,
                "Increment",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::Increment,
            )),
            SurfaceChild::fill(SurfaceNode::button_mapped(
                42,
                "Rename",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                |_| DemoMessage::Rename(String::from("Mapped")),
            )),
            SurfaceChild::fill(SurfaceNode::badge(
                43,
                "Active",
                WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
                DemoMessage::SetActive(true),
            )),
            SurfaceChild::fill(SurfaceNode::badge_mapped(
                44,
                "Mapped badge",
                WidgetSizing::fixed(Vector2::new(112.0, 24.0)),
                |_| DemoMessage::Rename(String::from("Badge")),
            )),
        ],
    ));

    assert!(surface.find_widget(40).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            41,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Mapped")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            43,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            44,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Badge")))
    );
}

#[test]
fn text_input_and_toggle_helpers_map_value_messages() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        5,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text_input(
                50,
                "Draft",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                DemoMessage::Rename,
            )),
            SurfaceChild::fill(SurfaceNode::text_input_mapped(
                51,
                "Raw",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                |message| match message {
                    TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                        DemoMessage::Rename(format!("raw:{value}"))
                    }
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle(
                52,
                "Enabled",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped(
                53,
                "Raw toggle",
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle_with_checked(
                54,
                "Done",
                true,
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped_with_checked(
                55,
                "Raw done",
                true,
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
        ],
    ));

    assert_eq!(
        surface.dispatch_widget_output(
            50,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Edited"),
            })
        ),
        Some(DemoMessage::Rename(String::from("Edited")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            51,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Submitted"),
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:Submitted")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            52,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            53,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(false))
    );
    assert_eq!(
        surface
            .find_widget(54)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
    assert_eq!(
        surface
            .find_widget(55)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
}

#[test]
fn scrollbar_list_item_and_canvas_helpers_build_common_leaf_nodes() {
    let mut surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::column(
        6,
        4.0,
        vec![
            SurfaceChild::fill(SurfaceNode::scrollbar(
                60,
                ScrollbarAxis::Vertical,
                WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
                |offset| DemoMessage::Rename(format!("offset:{offset:.2}")),
            )),
            SurfaceChild::fill(SurfaceNode::scrollbar_mapped(
                61,
                ScrollbarAxis::Horizontal,
                WidgetSizing::fixed(Vector2::new(120.0, 12.0)),
                |message| match message {
                    ScrollbarMessage::OffsetChanged { offset_fraction } => {
                        DemoMessage::Rename(format!("raw:{offset_fraction:.1}"))
                    }
                },
            )),
            SurfaceChild::fill(SurfaceNode::list_item(
                62,
                "Row",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            )),
            SurfaceChild::fill(SurfaceNode::list_item_mapped(
                64,
                "Mapped row",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
                |_| DemoMessage::Rename(String::from("row")),
            )),
            SurfaceChild::fill(SurfaceNode::selectable(
                65,
                "Choice",
                false,
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::canvas(
                63,
                WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
            )),
            SurfaceChild::fill(SurfaceNode::canvas_mapped(
                66,
                WidgetSizing::fixed(Vector2::new(160.0, 90.0)),
                |message| match message {
                    CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
                },
            )),
        ],
    ));

    assert!(
        widget_ref::<ScrollbarWidget, _>(&surface, 60, "scrollbar")
            .common()
            .id
            == 60
    );
    assert!(
        widget_ref::<ListItemWidget, _>(&surface, 62, "list item")
            .common()
            .id
            == 62
    );
    assert!(
        widget_ref::<ListItemWidget, _>(&surface, 64, "list item")
            .common()
            .id
            == 64
    );
    assert!(
        widget_ref::<SelectableWidget, _>(&surface, 65, "selectable")
            .common()
            .id
            == 65
    );
    assert!(
        widget_ref::<CanvasWidget, _>(&surface, 63, "canvas")
            .common()
            .id
            == 63
    );
    assert!(
        widget_ref::<CanvasWidget, _>(&surface, 66, "canvas")
            .common()
            .id
            == 66
    );
    assert_eq!(
        surface.dispatch_widget_output(
            60,
            radiant::widgets::WidgetOutput::typed(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.25,
            })
        ),
        Some(DemoMessage::Rename(String::from("offset:0.25")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            61,
            radiant::widgets::WidgetOutput::typed(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.5,
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:0.5")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            64,
            radiant::widgets::WidgetOutput::typed(ListItemMessage::Invoked)
        ),
        Some(DemoMessage::Rename(String::from("row")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            65,
            radiant::widgets::WidgetOutput::typed(SelectableMessage::SelectionChanged {
                selected: true,
            })
        ),
        Some(DemoMessage::SetActive(true))
    );

    let canvas_input = WidgetInput::PointerPress {
        position: Point::new(12.0, 8.0),
        button: PointerButton::Primary,
    };
    let canvas_output = surface
        .dispatch_widget_input(
            66,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 90.0)),
            canvas_input.clone(),
        )
        .expect("canvas should forward routed input");
    assert_eq!(
        canvas_output.typed_ref::<CanvasMessage>(),
        Some(&CanvasMessage::Input {
            input: canvas_input.clone()
        })
    );
    assert_eq!(
        surface.dispatch_widget_output(66, canvas_output),
        Some(DemoMessage::CanvasInput(canvas_input))
    );
}

#[test]
fn runtime_context_and_renderer_cover_paint_plan_boundary() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 3,
            name: String::from("Panels"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let context = runtime.context();

    assert_eq!(context.viewport.width(), 420.0);
    assert!(context.surface.find_widget(11).is_some());
    assert!(context.layout.rects.contains_key(&11));

    let plan = runtime.paint_plan(&theme);
    let mut renderer = CountingRenderer::default();
    renderer
        .render(&plan)
        .expect("counting renderer cannot fail");
    assert_eq!(renderer.rendered_primitives, plan.primitives.len());
}

#[test]
fn surface_runtime_manages_focus_and_routes_keyboard_to_focused_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(11));
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(12));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Backward), Some(11));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(11)
    );

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Untitled (1)"
    );

    assert!(runtime.focus_widget(12));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('Q')),
        Some(12)
    );
    runtime.clear_focus();
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('X')),
        None
    );

    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Q"
    );
}

#[test]
fn surface_runtime_preserves_text_input_caret_selection_across_value_refreshes() {
    let bridge = declarative_runtime_bridge(
        DemoState {
            name: String::from("abcd"),
            ..DemoState::default()
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::Increment | DemoMessage::SetActive(_) | DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert!(runtime.focus_widget(12));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::MoveHome {
            extend_selection: false,
        })),
        Some(12)
    );
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::MoveRight {
            extend_selection: true,
        })),
        Some(12)
    );
    assert_eq!(runtime.focused_text_selection().as_deref(), Some("ab"));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::InsertText(
            String::from("z")
        ))),
        Some(12)
    );

    let input = widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input");
    assert_eq!(input.state.value, "zcd");
    assert_eq!(input.state.caret, 1);
    assert_eq!(input.state.selection_anchor, 1);
}

#[test]
fn surface_runtime_resolves_host_shortcuts_before_widget_key_routing() {
    let mut runtime = SurfaceRuntime::new(ShortcutDemoBridge::default(), Vector2::new(420.0, 32.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(runtime.bridge().state.count, 1);
}

#[test]
fn surface_runtime_routes_backend_neutral_events() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(
        runtime.dispatch_event(Event::Resize {
            viewport: Vector2::new(360.0, 40.0),
        }),
        None
    );
    assert_eq!(runtime.viewport(), Vector2::new(360.0, 40.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(150.0, 10.0),
            button: PointerButton::Primary,
        }),
        Some(11)
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), Some(11));
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(150.0, 10.0),
            button: PointerButton::Primary,
        }),
        Some(11)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.dispatch_event(Event::TraverseFocus(FocusTraversal::Forward)),
        Some(12)
    );
    assert_eq!(runtime.dispatch_event(Event::Character('R')), Some(12));

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "R (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "R"
    );
}

#[test]
fn surface_runtime_clears_hover_when_pointer_leaves_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(150.0, 10.0),
        }),
        Some(11)
    );
    assert_eq!(runtime.hovered_widget(), Some(11));
    assert!(button_hovered(runtime.surface(), 11));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(410.0, 80.0),
        }),
        None
    );
    assert_eq!(runtime.hovered_widget(), None);
    assert!(!button_hovered(runtime.surface(), 11));
}

#[test]
fn surface_runtime_preserves_captured_drag_state_across_repaint_refreshes() {
    let bridge = declarative_command_runtime_bridge(
        Vec::<DragHandleMessage>::new(),
        |_| {
            Arc::new(UiSurface::new(SurfaceNode::widget(
                DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(24.0, 24.0))),
                WidgetMessageMapper::drag_handle(|message| message),
            )))
        },
        |messages: &mut Vec<DragHandleMessage>, message| {
            messages.push(message);
            Command::request_repaint()
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 120.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
        }),
        Some(10)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 72.0),
        }),
        Some(10)
    );

    assert_eq!(
        runtime.bridge().state().as_slice(),
        &[
            DragHandleMessage::Started {
                position: Point::new(12.0, 12.0),
            },
            DragHandleMessage::Moved {
                position: Point::new(12.0, 72.0),
            },
        ]
    );
}

#[test]
fn surface_runtime_executes_command_messages_and_repaint_requests() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert!(runtime.repaint_requested());
    assert!(runtime.take_repaint_requested());
    assert!(!runtime.repaint_requested());

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Commands (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Commands"
    );
}

#[test]
fn declarative_command_bridge_supports_command_update_flow() {
    let bridge = declarative_command_runtime_bridge(
        DemoState::default(),
        project_demo_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Closure"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Closure");
}

#[test]
fn surface_runtime_routes_widget_input_and_reprojects_surface() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let input_bounds = runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should have layout bounds");
    let input_point = Point::new(
        input_bounds.min.x + input_bounds.width() * 0.5,
        input_bounds.min.y + input_bounds.height() * 0.5,
    );

    assert_eq!(runtime.widget_at(Point::new(150.0, 10.0)), Some(11));
    assert!(runtime.dispatch_input(12, WidgetInput::FocusChanged(true)));
    assert!(runtime.dispatch_input(12, WidgetInput::Character('F')));
    assert!(runtime.dispatch_input(11, WidgetInput::FocusChanged(true)));
    assert_eq!(
        runtime.dispatch_input_at(input_point, WidgetInput::FocusChanged(true)),
        Some(12)
    );
    assert!(runtime.dispatch_input(11, WidgetInput::KeyPress(WidgetKey::Enter)));

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "F (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "F"
    );
}

#[derive(Default)]
struct CountingRenderer {
    rendered_primitives: usize,
}

impl Renderer for CountingRenderer {
    type Error = std::convert::Infallible;

    fn render(&mut self, plan: &radiant::runtime::SurfacePaintPlan) -> Result<(), Self::Error> {
        self.rendered_primitives += plan.primitives.len();
        Ok(())
    }
}

fn project_app_once(app: &mut impl App<DemoMessage>) -> Arc<UiSurface<DemoMessage>> {
    app.project_surface()
}

#[test]
fn generic_public_surface_resolves_theme_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let visuals = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            focused: true,
            ..WidgetState::default()
        },
    );

    assert_eq!(visuals.border, theme.border_emphasis);
}

#[test]
fn generic_surface_projects_deterministic_paint_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 2,
            name: String::from("Crates"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::SetActive(active) => {
                state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let direct_plan = runtime.surface().paint_plan(runtime.layout(), &theme);
    let runtime_plan = runtime.paint_plan(&theme);

    assert_eq!(runtime_plan, direct_plan);
    assert_eq!(runtime_plan.clear_color, theme.clear_color);
    assert_eq!(runtime_plan.primitives.len(), 7);

    let texts: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some((text.widget_id, text.text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec![(10, "Crates (2)"), (11, "Increment")]);
    let text_inputs: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some((input.widget_id, input.state.value.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(text_inputs, vec![(12, "Crates")]);

    let fills: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some((fill.widget_id, fill.color)),
            _ => None,
        })
        .collect();
    assert_eq!(fills, vec![(12, theme.bg_primary)]);

    let button_polygons: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillPolygon(fill) => Some((fill.widget_id, fill.points.len())),
            _ => None,
        })
        .collect();
    assert_eq!(button_polygons, vec![(11, 5)]);
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<DemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| DemoMessage::Increment),
            )),
            SurfaceChild::fill(SurfaceNode::text_input(
                12,
                state.name.clone(),
                WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
                DemoMessage::Rename,
            )),
        ],
    )))
}

fn display_name(state: &DemoState) -> &str {
    if state.name.is_empty() {
        "Untitled"
    } else {
        &state.name
    }
}

fn button_hovered(surface: &UiSurface<DemoMessage>, widget_id: u64) -> bool {
    widget_ref::<ButtonWidget, _>(surface, widget_id, "button")
        .common
        .state
        .hovered
}

fn widget_fill_color(plan: &radiant::runtime::SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == widget_id => Some(fill.color),
            PaintPrimitive::FillPolygon(fill) if fill.widget_id == widget_id => Some(fill.color),
            _ => None,
        })
}

fn widget_stroke_color(plan: &radiant::runtime::SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::StrokeRect(stroke) if stroke.widget_id == widget_id => {
                Some(stroke.color)
            }
            PaintPrimitive::StrokePolygon(stroke) if stroke.widget_id == widget_id => {
                Some(stroke.color)
            }
            _ => None,
        })
}

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
}

struct CommandDemoBridge {
    state: DemoState,
}

#[derive(Default)]
struct ShortcutDemoBridge {
    state: DemoState,
}

impl RuntimeBridge<DemoMessage> for ShortcutDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut self.state)
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        match message {
            DemoMessage::Increment => self.state.count += 1,
            DemoMessage::Rename(name) => self.state.name = name,
            DemoMessage::SetActive(active) => {
                self.state.name = if active { "active" } else { "inactive" }.to_owned()
            }
            DemoMessage::CanvasInput(_) => {}
        }
        Command::none()
    }

    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<DemoMessage> {
        if press == KeyPress::with_command(KeyCode::I) {
            return ShortcutResolution {
                action: Some(DemoMessage::Increment),
                handled: true,
                pending_chord: None,
            };
        }
        ShortcutResolution::unhandled()
    }
}

#[derive(Default)]
struct RepaintSignalBridge {
    signal: Option<Arc<dyn RepaintSignal>>,
}

impl RepaintSignalBridge {
    fn request_worker_repaint(&self) {
        if let Some(signal) = self.signal.as_ref() {
            signal.request_repaint();
        }
    }
}

impl RuntimeBridge<DemoMessage> for RepaintSignalBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.signal = Some(signal);
    }
}

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

struct RuntimeExitBridge;

impl RuntimeBridge<DemoMessage> for RuntimeExitBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    }
}

impl RuntimeBridge<CommandDemoMessage> for CommandDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<CommandDemoMessage>> {
        project_demo_surface(&mut self.state)
    }

    fn update(&mut self, message: CommandDemoMessage) -> Command<CommandDemoMessage> {
        match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Commands"))),
                Command::request_repaint(),
                Command::message(CommandDemoMessage::Increment),
            ]),
            CommandDemoMessage::Increment => {
                self.state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }
}

fn project_demo_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(11, "Run", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    );

    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
            )),
            SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::none())),
        ],
    )))
}
