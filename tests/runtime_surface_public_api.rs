//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::ImageRgba,
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
        BadgeMessage, ButtonMessage, ButtonWidget, CanvasMessage, ListItemMessage, PointerButton,
        ScrollbarAxis, ScrollbarMessage, SelectableMessage, TextInputMessage, TextInputWidget,
        TextWidget, ToggleMessage, WidgetInput, WidgetKey, WidgetSizing, WidgetSpec, WidgetState,
        WidgetStyle, resolve_widget_visual_tokens,
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
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.title, "Radiant");
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
            radiant::widgets::WidgetOutput::TextInput(TextInputMessage::Changed {
                value: String::from("Projects"),
            }),
        )
        .expect("text input should emit a host-defined rename message");
    bridge.reduce_message(rename);

    let increment = bridge
        .project_surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::Button(ButtonMessage::Activate),
        )
        .expect("button should emit a host-defined increment message");
    bridge.reduce_message(increment);

    let surface_after = bridge.project_surface();
    let title = surface_after
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = surface_after
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();

    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "Projects (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "Projects"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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
    let header = WidgetSpec::Text(TextWidget::new(
        20,
        "Header",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    ));
    let primary = WidgetSpec::Button(ButtonWidget::new(
        21,
        "Primary",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let secondary = WidgetSpec::Button(ButtonWidget::new(
        22,
        "Secondary",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));

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
    let title = WidgetSpec::Text(TextWidget::new(
        30,
        "Status",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    ));
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::static_widget(title));

    assert!(surface.find_widget(30).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::Button(ButtonMessage::Activate)
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
            radiant::widgets::WidgetOutput::Button(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::Button(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Mapped")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            43,
            radiant::widgets::WidgetOutput::Badge(BadgeMessage::Activate)
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            44,
            radiant::widgets::WidgetOutput::Badge(BadgeMessage::Activate)
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
        ],
    ));

    assert_eq!(
        surface.dispatch_widget_output(
            50,
            radiant::widgets::WidgetOutput::TextInput(TextInputMessage::Changed {
                value: String::from("Edited"),
            })
        ),
        Some(DemoMessage::Rename(String::from("Edited")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            51,
            radiant::widgets::WidgetOutput::TextInput(TextInputMessage::Submitted {
                value: String::from("Submitted"),
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:Submitted")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            52,
            radiant::widgets::WidgetOutput::Toggle(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            53,
            radiant::widgets::WidgetOutput::Toggle(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(false))
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

    assert!(matches!(
        surface.find_widget(60).map(|widget| widget.widget()),
        Some(WidgetSpec::Scrollbar(_))
    ));
    assert!(matches!(
        surface.find_widget(62).map(|widget| widget.widget()),
        Some(WidgetSpec::ListItem(_))
    ));
    assert!(matches!(
        surface.find_widget(64).map(|widget| widget.widget()),
        Some(WidgetSpec::ListItem(_))
    ));
    assert!(matches!(
        surface.find_widget(65).map(|widget| widget.widget()),
        Some(WidgetSpec::Selectable(_))
    ));
    assert!(matches!(
        surface.find_widget(63).map(|widget| widget.widget()),
        Some(WidgetSpec::Canvas(_))
    ));
    assert!(matches!(
        surface.find_widget(66).map(|widget| widget.widget()),
        Some(WidgetSpec::Canvas(_))
    ));
    assert_eq!(
        surface.dispatch_widget_output(
            60,
            radiant::widgets::WidgetOutput::Scrollbar(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.25,
            })
        ),
        Some(DemoMessage::Rename(String::from("offset:0.25")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            61,
            radiant::widgets::WidgetOutput::Scrollbar(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.5,
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:0.5")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            64,
            radiant::widgets::WidgetOutput::ListItem(ListItemMessage::Invoked)
        ),
        Some(DemoMessage::Rename(String::from("row")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            65,
            radiant::widgets::WidgetOutput::Selectable(SelectableMessage::SelectionChanged {
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
            canvas_input,
        )
        .expect("canvas should forward routed input");
    assert_eq!(
        canvas_output,
        radiant::widgets::WidgetOutput::Canvas(CanvasMessage::Input {
            input: canvas_input
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

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "Untitled (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }

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

    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "Q"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();
    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "R (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "R"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();
    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "Commands (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "Commands"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();

    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "F (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "F"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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
    assert_eq!(
        texts,
        vec![(10, "Crates (2)"), (11, "Increment"), (12, "Crates")]
    );

    let fills: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some((fill.widget_id, fill.color)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        vec![(11, theme.surface_raised), (12, theme.surface_raised)]
    );
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<DemoMessage>> {
    let title = WidgetSpec::Text(TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    ));
    let button = WidgetSpec::Button(ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
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
    match surface
        .find_widget(widget_id)
        .expect("button widget should exist")
        .widget()
    {
        WidgetSpec::Button(button) => button.common.state.hovered,
        other => panic!("expected button widget, got {other:?}"),
    }
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
    let title = WidgetSpec::Text(TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    ));
    let button = WidgetSpec::Button(ButtonWidget::new(
        11,
        "Run",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let input = WidgetSpec::TextInput(TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    ));

    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
            )),
            SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::None)),
        ],
    )))
}
