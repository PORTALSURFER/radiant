//! Public API coverage for Radiant application builder ergonomics.

use radiant::{
    layout::{
        LayoutDebugOptions, LayoutState, Point, Rect, Vector2, layout_tree, layout_tree_with_state,
    },
    runtime::{
        Command, DEFAULT_NATIVE_WINDOW_TITLE, NativeGpuBackend, NativeGpuOptions, NativeRunOptions,
        NativeTextOptions, PaintPrimitive, RuntimeBridge, SurfaceRuntime, UiSurface,
        WidgetMessageMapper, WindowManifest, WindowSpec,
    },
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CardWidget, SelectableMessage,
        SelectableWidget, TextAlign, TextInputMessage, TextInputWidget, TextWidget, TextWrap,
        ToggleWidget, Widget, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusMessage {
    FocusName,
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

#[derive(Clone)]
struct CustomTextPolicyWidget {
    common: radiant::widgets::WidgetCommon,
    wrap: TextWrap,
    align: TextAlign,
}

impl CustomTextPolicyWidget {
    fn new(id: u64) -> Self {
        Self {
            common: radiant::widgets::WidgetCommon::new(
                id,
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            ),
            wrap: TextWrap::None,
            align: TextAlign::Left,
        }
    }
}

impl Widget for CustomTextPolicyWidget {
    fn common(&self) -> &radiant::widgets::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut radiant::widgets::WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: Rect,
        _input: radiant::widgets::WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        None
    }

    fn set_text_wrap(&mut self, wrap: TextWrap) -> bool {
        self.wrap = wrap;
        true
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        self.align = align;
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &radiant::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::Text(radiant::runtime::PaintTextRun {
            widget_id: self.common.id,
            text: "custom".into(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(17.0),
            color: theme.text_primary,
            align: match self.align {
                TextAlign::Left => radiant::runtime::PaintTextAlign::Left,
                TextAlign::Center => radiant::runtime::PaintTextAlign::Center,
                TextAlign::Right => radiant::runtime::PaintTextAlign::Right,
            },
            wrap: self.wrap,
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
fn native_run_options_expose_gpu_backend_policy() {
    let options = NativeRunOptions {
        gpu: NativeGpuOptions {
            backend: NativeGpuBackend::Dx12,
        },
        ..NativeRunOptions::default()
    };
    let spec = WindowSpec::new("main", "Main").gpu_backend(NativeGpuBackend::Vulkan);

    assert_eq!(
        NativeRunOptions::default().gpu.backend,
        NativeGpuBackend::Auto
    );
    assert_eq!(options.gpu.backend, NativeGpuBackend::Dx12);
    assert_eq!(spec.native_options().gpu.backend, NativeGpuBackend::Vulkan);
}

#[test]
fn native_run_options_expose_text_font_policy() {
    let options = NativeRunOptions {
        text: NativeTextOptions {
            font_paths: vec!["fonts/App.ttf".into()],
        },
        ..NativeRunOptions::default()
    };
    let spec = WindowSpec::new("main", "Main").font_path("fonts/Spec.ttf");

    assert!(NativeRunOptions::default().text.font_paths.is_empty());
    assert_eq!(
        options.text.font_paths[0],
        std::path::PathBuf::from("fonts/App.ttf")
    );
    assert_eq!(
        spec.native_options().text.font_paths[0],
        std::path::PathBuf::from("fonts/Spec.ttf")
    );
}

#[test]
fn window_specs_describe_multiple_windows_without_opening_runtime() {
    let main = radiant::window("Main")
        .size(800, 600)
        .min_size(640, 480)
        .spec("main");
    let inspector = WindowSpec::new("inspector", "Inspector")
        .logical_size(320.5, 500.25)
        .min_logical_size(300.25, 420.5)
        .drag_and_drop(false)
        .target_fps(60);

    assert_eq!(main.key, "main");
    assert_eq!(main.title(), "Main");
    assert_eq!(main.inner_size(), Some([800.0, 600.0]));
    assert_eq!(main.min_inner_size(), Some([640.0, 480.0]));
    assert_eq!(inspector.title(), "Inspector");
    assert_eq!(inspector.inner_size(), Some([320.5, 500.25]));
    assert_eq!(inspector.min_inner_size(), Some([300.25, 420.5]));
    assert!(!inspector.drag_and_drop_enabled());
    assert_eq!(inspector.target_frame_rate(), 60);
    let options: NativeRunOptions = inspector.into();
    assert_eq!(options.inner_size, Some([320.5, 500.25]));
    assert_eq!(options.min_inner_size, Some([300.25, 420.5]));
}

#[test]
fn window_manifest_validates_stable_unique_window_keys() {
    let manifest = WindowManifest::from_specs([
        WindowSpec::new("main", "Main").size(800, 600),
        WindowSpec::new("inspector", "Inspector").size(320, 500),
    ])
    .expect("unique keys should be valid");

    assert_eq!(manifest.len(), 2);
    assert_eq!(manifest.keys().collect::<Vec<_>>(), ["main", "inspector"]);
    assert_eq!(
        manifest.get("inspector").unwrap().inner_size(),
        Some([320.0, 500.0])
    );
    assert!(manifest.validate().is_ok());
}

#[test]
fn window_manifest_rejects_duplicate_window_keys() {
    let duplicate = WindowManifest::from_specs([
        WindowSpec::new("main", "Main"),
        WindowSpec::new("main", "Duplicate"),
    ]);

    assert_eq!(duplicate, Err(String::from("duplicate window key 'main'")));
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
fn application_builder_update_context_can_move_keyboard_focus() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text_input(state.name.clone())
                    .message(|_| FocusMessage::FocusName)
                    .id(10),
                ui::button("Focus name")
                    .message(FocusMessage::FocusName)
                    .id(11),
                ui::text(format!("Name: {}", state.name))
                    .id(12)
                    .height(24.0),
            ])
        })
        .update_with(|state, message, context| match message {
            FocusMessage::FocusName => {
                state.name = String::from("focused");
                context.focus(10);
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
    let focus = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit focus message");
    let outcome = runtime.dispatch_message(focus);

    assert!(outcome.repaint_requested);
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 12, "status").text,
        "Name: focused"
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
fn application_builder_list_row_id_uses_direct_numeric_identity() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::list_row_id(
        42,
        [ui::button("Open").message(DemoMessage::Increment).id(420)],
    )
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 64.0)),
    );

    assert!(layout.rects.contains_key(&42));
    assert!(surface.find_widget(420).is_some());
}

#[test]
fn application_builder_grid_lowers_to_fixed_column_tile_layout() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::grid_with_gaps(
        (0..5).map(|index| {
            ui::text(format!("Tile {index}"))
                .id(100 + index)
                .fill_width()
                .height(28.0)
        }),
        2,
        10.0,
        6.0,
    )
    .id(10)
    .padding(4.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 160.0)),
    );
    let first = layout.rects[&100];
    let second = layout.rects[&101];
    let third = layout.rects[&102];

    assert_eq!(layout.rects[&10].min.x, 0.0);
    assert!(second.min.x > first.max.x);
    assert_eq!(first.min.y, second.min.y);
    assert!(third.min.y > first.min.y);
    assert_eq!(first.height(), 28.0);
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
            ui::text("Name")
                .id(12)
                .size(80.0, 28.0)
                .baseline(19.0)
                .align_text(TextAlign::Right),
            ui::text("Radiant")
                .id(13)
                .fill_width()
                .height(28.0)
                .baseline(19.0)
                .align_text(TextAlign::Center),
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
    assert_eq!(wrapped.align, TextAlign::Left);
    assert_eq!(wrapped.common.sizing.baseline, Some(18.0));
    assert_eq!(truncated.wrap, TextWrap::None);
    assert_eq!(truncated.common.sizing.baseline, Some(19.0));
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 12, "label").align,
        TextAlign::Right
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 13, "value").align,
        TextAlign::Center
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 12, "label")
            .common
            .sizing
            .baseline,
        Some(19.0)
    );
    let paint = surface.paint_plan(&layout, &radiant::theme::ThemeTokens::default());
    assert!(paint.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(text)
                if text.widget_id == 12 && text.align == radiant::runtime::PaintTextAlign::Right
        )
    }));
    assert!(paint.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(text)
                if text.widget_id == 13 && text.align == radiant::runtime::PaintTextAlign::Center
        )
    }));
    assert_eq!(layout.rects[&10].height(), 64.0);
    assert_eq!(layout.rects[&11].height(), 28.0);
    assert_eq!(layout.rects[&12].height(), layout.rects[&13].height());
    assert!(layout.rects[&13].min.x >= layout.rects[&12].max.x + 8.0);
}

#[test]
fn application_builder_text_policy_modifiers_use_widget_contract() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::widget(CustomTextPolicyWidget::new(0))
        .wrap()
        .align_text(TextAlign::Right)
        .id(10)
        .into_surface();

    let custom = widget_ref::<CustomTextPolicyWidget, _>(&surface, 10, "custom text policy");

    assert_eq!(custom.wrap, TextWrap::Word);
    assert_eq!(custom.align, TextAlign::Right);
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
