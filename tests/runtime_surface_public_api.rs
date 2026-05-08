//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::prelude::IntoView;
use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{
        App, Command, Element, Event, FocusTraversal, GpuHoverCursor, GpuSurfaceCapabilities,
        GpuSurfaceContent, GpuSurfaceOverlay, PaintPrimitive, Renderer, RuntimeBridge,
        SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, View, WidgetMessageMapper,
        declarative_command_runtime_bridge, declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        BadgeMessage, ButtonMessage, ButtonWidget, CanvasMessage, CanvasWidget, DragHandleMessage,
        DragHandleWidget, GpuSurfaceWidget, ListItemMessage, ListItemWidget, PointerButton,
        RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget,
        SelectableMessage, SelectableWidget, TextEditCommand, TextInputMessage, TextInputWidget,
        TextWidget, ToggleMessage, Widget, WidgetInput, WidgetKey, WidgetProminence, WidgetSizing,
        WidgetState, WidgetStyle, WidgetTone, resolve_widget_visual_tokens,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

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
fn gpu_surface_widget_projects_generic_retained_gpu_primitive() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]).unwrap());
    let content = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 1.0)),
        atlas: Arc::clone(&atlas),
    };
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::new(
            41,
            WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            9001,
            7,
            content,
        )
        .with_capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            native_hover_cursor: Some(GpuHoverCursor {
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            }),
        })
        .with_overlays(vec![GpuSurfaceOverlay::VerticalCursor {
            ratio: 0.5,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }]),
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );
    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    let Some(PaintPrimitive::GpuSurface(gpu)) = plan.primitives.first() else {
        panic!("expected gpu surface primitive");
    };
    assert_eq!(gpu.widget_id, 41);
    assert_eq!(gpu.key, 9001);
    assert_eq!(gpu.revision, 7);
    assert!(gpu.capabilities.fast_pointer_move);
    assert!(gpu.capabilities.coalesce_vertical_wheel);
    assert!(gpu.capabilities.native_hover_cursor.is_some());
    assert_eq!(gpu.overlays.len(), 1);
    let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &gpu.content else {
        panic!("expected rgba atlas gpu content");
    };
    assert_eq!(atlas.width, 2);
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
fn surface_runtime_executes_focus_exit_and_deferred_commands() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let focus = runtime.execute_command(Command::focus(11));
    assert!(!focus.exit_requested);
    assert_eq!(runtime.focused_widget(), Some(11));

    let deferred = runtime.execute_command(Command::after(
        Duration::from_millis(1),
        DemoMessage::Increment,
    ));
    assert!(deferred.repaint_requested);
    std::thread::sleep(Duration::from_millis(20));
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 1);

    let performed = runtime.execute_command(Command::perform(
        "increment",
        || DemoMessage::Increment,
        |message| message,
    ));
    assert!(performed.repaint_requested);
    std::thread::sleep(Duration::from_millis(20));
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 2);

    let exit = runtime.execute_command(Command::exit());
    assert!(exit.exit_requested);
    assert!(runtime.take_exit_requested());
}

#[test]
fn retained_canvas_builder_projects_metadata_and_input_mapping() {
    let surface = radiant::prelude::retained_canvas(44)
        .revision(7)
        .dirty_mask(3)
        .volatile(true)
        .on_input(|message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        })
        .id(44)
        .size(120.0, 40.0)
        .into_surface();
    let plan = surface.paint_plan(
        &layout_tree(
            &surface.layout_node(),
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 40.0)),
        ),
        &ThemeTokens::default(),
    );
    let Some(PaintPrimitive::CustomSurface(custom)) = plan.primitives.first() else {
        panic!("retained canvas should project one custom surface primitive");
    };
    assert_eq!(
        custom.retained,
        Some(RetainedSurfaceDescriptor {
            key: 44,
            revision: 7,
            dirty_mask: 3,
            volatile: true,
        })
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

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
}

struct CommandDemoBridge {
    state: DemoState,
}

#[derive(Default)]
struct RuntimeCommandBridge {
    count: usize,
    pending: Arc<std::sync::Mutex<Vec<DemoMessage>>>,
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

impl RuntimeBridge<DemoMessage> for RuntimeCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::row(
            1,
            8.0,
            vec![SurfaceChild::fill(SurfaceNode::static_widget(
                ButtonWidget::new(11, "Focus", WidgetSizing::fixed(Vector2::new(80.0, 32.0))),
            ))],
        )))
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        if matches!(message, DemoMessage::Increment) {
            self.count += 1;
        }
        Command::none()
    }

    fn schedule_message(&mut self, delay: Duration, message: DemoMessage) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(message);
        });
        true
    }

    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        work: Box<dyn FnOnce() -> DemoMessage + Send + 'static>,
    ) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(work());
        });
        true
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        std::mem::take(&mut *self.pending.lock().expect("pending messages poisoned"))
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
