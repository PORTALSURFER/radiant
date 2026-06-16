//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::prelude as ui;
use radiant::prelude::IntoView;
use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{Point, Rect, Vector2, layout_tree, layout_tree_with_state},
    runtime::{
        Command, Element, Event, FocusTraversal, GpuShaderSurfaceDescriptor,
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceOverlay,
        GpuSurfaceRuntimeOverlays, LayerKind, PaintPrimitive, Renderer, RepaintScope,
        RuntimeBridge, SurfaceChild, SurfaceLayer, SurfaceNode, SurfacePaintPlan, SurfaceRuntime,
        UiSurface, View, WidgetMessageMapper, declarative_command_runtime_bridge,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, CanvasMessage, DragHandleMessage, DragHandleWidget, GpuSurfaceParts,
        GpuSurfaceWidget, PointerButton, PointerModifiers, RetainedSurfaceDescriptor,
        TextEditCommand, TextInputWidget, TextWidget, Widget, WidgetCommon, WidgetCursor,
        WidgetInput, WidgetKey, WidgetOutput, WidgetProminence, WidgetSizing, WidgetState,
        WidgetStyle, WidgetTone, resolve_widget_visual_tokens,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    CanvasInput(WidgetInput),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScenePointerMessage {
    Press,
    Move,
    Release,
    Wheel,
    Dismiss,
}

#[derive(Clone, Default)]
struct SceneBridge {
    events: Arc<Mutex<Vec<ScenePointerMessage>>>,
}

impl SceneBridge {
    fn events(&self) -> Vec<ScenePointerMessage> {
        self.events.lock().expect("scene event log").clone()
    }
}

impl RuntimeBridge<ScenePointerMessage> for SceneBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ScenePointerMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::scene(
            1,
            SurfaceNode::custom_widget(
                ScenePointerWidget::new(10),
                WidgetMessageMapper::typed(|message: ScenePointerMessage| message),
            ),
            Vec::new(),
        )))
    }

    fn update(&mut self, message: ScenePointerMessage) -> Command<ScenePointerMessage> {
        self.events.lock().expect("scene event log").push(message);
        Command::none()
    }
}

struct SceneSurfaceBridge {
    root: SurfaceNode<ScenePointerMessage>,
    events: Arc<Mutex<Vec<ScenePointerMessage>>>,
}

impl SceneSurfaceBridge {
    fn new(root: SurfaceNode<ScenePointerMessage>) -> Self {
        Self {
            root,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_events(mut self, events: Arc<Mutex<Vec<ScenePointerMessage>>>) -> Self {
        self.events = events;
        self
    }
}

impl RuntimeBridge<ScenePointerMessage> for SceneSurfaceBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ScenePointerMessage>> {
        Arc::new(UiSurface::new(self.root.clone()))
    }

    fn update(&mut self, message: ScenePointerMessage) -> Command<ScenePointerMessage> {
        self.events.lock().expect("scene event log").push(message);
        Command::none()
    }
}

#[derive(Clone)]
struct ScenePointerWidget {
    common: WidgetCommon,
}

impl ScenePointerWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 40.0)))
                .with_pointer_focus()
                .without_default_chrome(),
        }
    }
}

impl Widget for ScenePointerWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerPress { .. } => {
                Some(WidgetOutput::typed(ScenePointerMessage::Press))
            }
            WidgetInput::PointerMove { .. } => Some(WidgetOutput::typed(ScenePointerMessage::Move)),
            WidgetInput::PointerRelease { .. } => {
                Some(WidgetOutput::typed(ScenePointerMessage::Release))
            }
            WidgetInput::Wheel { .. } => Some(WidgetOutput::typed(ScenePointerMessage::Wheel)),
            _ => None,
        }
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
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

#[cfg(test)]
#[path = "runtime_surface_public_api/automation.rs"]
mod automation;
#[cfg(test)]
#[path = "runtime_surface_public_api/commands.rs"]
mod commands;
#[cfg(test)]
#[path = "runtime_surface_public_api/devtools.rs"]
mod devtools;
#[cfg(test)]
#[path = "runtime_surface_public_api/focus_text.rs"]
mod focus_text;
#[cfg(test)]
#[path = "runtime_surface_public_api/interaction.rs"]
mod interaction;
#[cfg(test)]
#[path = "runtime_surface_public_api/paint_projection.rs"]
mod paint_projection;
#[cfg(test)]
#[path = "runtime_surface_public_api/pointer_motion.rs"]
mod pointer_motion;
#[cfg(test)]
#[path = "runtime_surface_public_api/virtual_scroll.rs"]
mod virtual_scroll;

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
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(16.0, 16.0)), Some(90));
}

#[test]
fn scene_base_widget_receives_pointer_press_with_no_layers() {
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge.clone(), Vector2::new(140.0, 60.0));
    let position = Point::new(16.0, 16.0);

    let target = runtime.dispatch_event(Event::PointerPress {
        position,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(target, Some(10));
    assert_eq!(bridge.events(), vec![ScenePointerMessage::Press]);
}

#[test]
fn scene_base_custom_widget_receives_press_move_release() {
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge.clone(), Vector2::new(140.0, 60.0));
    let position = Point::new(16.0, 16.0);

    runtime.dispatch_event(Event::PointerPress {
        position,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerMove { position });
    runtime.dispatch_event(Event::PointerRelease {
        position,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(
        bridge.events(),
        vec![
            ScenePointerMessage::Press,
            ScenePointerMessage::Move,
            ScenePointerMessage::Release
        ]
    );
}

#[test]
fn scene_base_pointer_capture_survives_refresh() {
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge.clone(), Vector2::new(140.0, 60.0));
    let press = Point::new(16.0, 16.0);
    let release = Point::new(120.0, 50.0);

    let press_target = runtime.dispatch_event(Event::PointerPress {
        position: press,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    let release_target = runtime.dispatch_event(Event::PointerRelease {
        position: release,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(press_target, Some(10));
    assert_eq!(release_target, Some(10));
    assert_eq!(
        bridge.events(),
        vec![ScenePointerMessage::Press, ScenePointerMessage::Release]
    );
}

#[test]
fn scene_base_stack_hit_testing_matches_plain_stack() {
    let base = SurfaceNode::stack(
        70,
        vec![
            SurfaceChild::fill(SurfaceNode::button(
                80,
                "Bottom",
                WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                ScenePointerMessage::Press,
            )),
            SurfaceChild::fill(SurfaceNode::button(
                90,
                "Top",
                WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                ScenePointerMessage::Release,
            )),
        ],
    );
    let plain = SurfaceRuntime::new(
        SceneSurfaceBridge::new(base.clone()),
        Vector2::new(140.0, 60.0),
    );
    let scene = SurfaceRuntime::new(
        SceneSurfaceBridge::new(SurfaceNode::scene(1, base, Vec::new())),
        Vector2::new(140.0, 60.0),
    );

    assert_eq!(plain.widget_at(Point::new(16.0, 16.0)), Some(90));
    assert_eq!(scene.widget_at(Point::new(16.0, 16.0)), Some(90));
}

#[test]
fn scene_interactive_layer_widgets_route_above_base_without_hiding_base_elsewhere() {
    let base = SurfaceNode::button(
        10,
        "Base",
        WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
        ScenePointerMessage::Press,
    );
    let layer = SurfaceLayer::new(
        LayerKind::Popover,
        SurfaceNode::floating_layer(
            20,
            Point::new(0.0, 0.0),
            Vector2::new(32.0, 24.0),
            SurfaceNode::button(
                30,
                "Layer",
                WidgetSizing::fixed(Vector2::new(32.0, 24.0)),
                ScenePointerMessage::Release,
            ),
            true,
        ),
    );
    let runtime = SurfaceRuntime::new(
        SceneSurfaceBridge::new(SurfaceNode::scene(1, base, vec![layer])),
        Vector2::new(140.0, 80.0),
    );

    assert_eq!(runtime.widget_at(Point::new(8.0, 8.0)), Some(30));
    assert_eq!(runtime.widget_at(Point::new(80.0, 40.0)), Some(10));
}

#[test]
fn scene_layer_pass_through_preserves_base_hit_target() {
    let root = ui::scene(
        SurfaceNode::custom_widget(
            ScenePointerWidget::new(10),
            WidgetMessageMapper::typed(|message: ScenePointerMessage| message),
        )
        .into(),
    )
    .layer(radiant::Layer::floating(
        SurfaceNode::static_widget(TextWidget::new(
            20,
            "Passive",
            WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
        ))
        .into(),
    ))
    .into_view()
    .into_surface();
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(
        SceneSurfaceBridge::new(root.into_root()).with_events(Arc::clone(&bridge.events)),
        Vector2::new(140.0, 60.0),
    );

    let target = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(16.0, 16.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(target, Some(10));
    assert_eq!(bridge.events(), vec![ScenePointerMessage::Press]);
}

#[test]
fn scene_layer_block_input_blocks_base_pointer_and_wheel() {
    let root = ui::scene(
        SurfaceNode::custom_widget(
            ScenePointerWidget::new(10),
            WidgetMessageMapper::typed(|message: ScenePointerMessage| message),
        )
        .into(),
    )
    .layer(
        radiant::Layer::modal(
            SurfaceNode::static_widget(TextWidget::new(
                20,
                "Modal",
                WidgetSizing::fixed(Vector2::new(48.0, 24.0)),
            ))
            .into(),
        )
        .block_input(),
    )
    .into_view()
    .into_surface();
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(
        SceneSurfaceBridge::new(root.into_root()).with_events(Arc::clone(&bridge.events)),
        Vector2::new(140.0, 60.0),
    );

    let press_target = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(80.0, 40.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::Scroll {
        position: Point::new(80.0, 40.0),
        delta: Vector2::new(0.0, -20.0),
    });

    assert_ne!(press_target, Some(10));
    assert!(bridge.events().is_empty());
}

#[test]
fn scene_layer_dismiss_on_outside_click_emits_message() {
    let root = ui::scene(
        SurfaceNode::custom_widget(
            ScenePointerWidget::new(10),
            WidgetMessageMapper::typed(|message: ScenePointerMessage| message),
        )
        .into(),
    )
    .layer(
        radiant::Layer::context_menu(
            SurfaceNode::floating_layer(
                20,
                Point::new(0.0, 0.0),
                Vector2::new(32.0, 24.0),
                SurfaceNode::button(
                    30,
                    "Layer",
                    WidgetSizing::fixed(Vector2::new(32.0, 24.0)),
                    ScenePointerMessage::Release,
                ),
                true,
            )
            .into(),
        )
        .dismiss_on_outside_click(ScenePointerMessage::Dismiss),
    )
    .into_view()
    .into_surface();
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(
        SceneSurfaceBridge::new(root.into_root()).with_events(Arc::clone(&bridge.events)),
        Vector2::new(140.0, 60.0),
    );

    let target = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(80.0, 40.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_ne!(target, Some(10));
    assert_eq!(bridge.events(), vec![ScenePointerMessage::Dismiss]);
}

#[test]
fn scene_layer_dismiss_surface_routes_below_foreground_content() {
    let root = ui::scene(
        SurfaceNode::custom_widget(
            ScenePointerWidget::new(10),
            WidgetMessageMapper::typed(|message: ScenePointerMessage| message),
        )
        .into(),
    )
    .layer(
        radiant::Layer::context_menu(
            SurfaceNode::floating_layer(
                20,
                Point::new(0.0, 0.0),
                Vector2::new(32.0, 24.0),
                SurfaceNode::button(
                    30,
                    "Layer",
                    WidgetSizing::fixed(Vector2::new(32.0, 24.0)),
                    ScenePointerMessage::Release,
                ),
                true,
            )
            .into(),
        )
        .dismiss_on_outside_click(ScenePointerMessage::Dismiss),
    )
    .into_view()
    .into_surface();
    let bridge = SceneBridge::default();
    let mut runtime = SurfaceRuntime::new(
        SceneSurfaceBridge::new(root.into_root()).with_events(Arc::clone(&bridge.events)),
        Vector2::new(140.0, 60.0),
    );

    let target = runtime.dispatch_pointer_click(
        Point::new(8.0, 8.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    );

    assert_eq!(target.press_target, Some(30));
    assert_eq!(target.release_target, Some(30));
    assert_eq!(bridge.events(), vec![ScenePointerMessage::Release]);
}

#[test]
fn surface_runtime_resolves_widget_cursor_at_hit_tested_point() {
    #[derive(Clone)]
    struct CursorWidget {
        common: WidgetCommon,
    }

    impl Widget for CursorWidget {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
        }

        fn cursor_for_point(&self, bounds: Rect, point: Point) -> Option<WidgetCursor> {
            (bounds.contains(point) && point.x <= bounds.center().x)
                .then_some(WidgetCursor::ResizeLeft)
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &radiant::layout::LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            let mut common = WidgetCommon::new(100, WidgetSizing::fixed(Vector2::new(80.0, 40.0)));
            common.focus = radiant::widgets::FocusBehavior::Pointer;
            Arc::new(UiSurface::new(SurfaceNode::custom_widget(
                CursorWidget { common },
                WidgetMessageMapper::none(),
            )))
        },
        |_state: &mut (), _message: DemoMessage| {},
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    assert_eq!(
        runtime.cursor_at(Point::new(8.0, 8.0)),
        WidgetCursor::ResizeLeft
    );
    assert_eq!(
        runtime.cursor_at(Point::new(72.0, 8.0)),
        WidgetCursor::Default
    );
    assert_eq!(
        runtime.cursor_at(Point::new(100.0, 50.0)),
        WidgetCursor::Default
    );
}

#[test]
fn surface_runtime_hit_testing_skips_passive_widget_leaves() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |_state: &mut DemoState| {
            Arc::new(UiSurface::new(SurfaceNode::stack(
                70,
                vec![
                    SurfaceChild::fill(SurfaceNode::button(
                        80,
                        "Interactive",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Increment,
                    )),
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        90,
                        "Passive label",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                    ))),
                ],
            )))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(16.0, 16.0)), Some(80));
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
