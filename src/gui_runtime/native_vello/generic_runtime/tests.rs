use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, SlotParams},
    runtime::{Command, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasMessage, PointerButton, TextInputMessage, TextInputWidget, WidgetInput,
        WidgetSizing, WidgetSpec,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    Increment,
    Rename(String),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

#[test]
fn generic_core_routes_pointer_and_key_input_to_host_messages() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );
    assert_eq!(core.runtime.bridge().state.count, 1);

    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_character('R').routed);
    assert!(core.route_widget_key(WidgetKey::Enter).routed);
    assert_eq!(core.runtime.bridge().state.name, "R");
}

#[test]
fn generic_canvas_can_receive_keyboard_focus_and_text_input() {
    let bridge = CanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&21)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(core.runtime.surface().keyboard_focus_order().contains(&21));
    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_character('K').routed);

    assert_eq!(core.runtime.bridge().text, "K");
}

#[test]
fn generic_core_drains_command_repaint_requests_after_routing() {
    let bridge = RepaintBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_release(button_point, PointerButton::Primary);

    assert!(outcome.routed);
    assert!(outcome.repaint_requested);
    assert!(!core.runtime.repaint_requested());
    assert_eq!(core.runtime.bridge().state.count, 1);
}

#[test]
fn generic_paint_plan_encodes_to_vello_scene() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let viewport = core.runtime.viewport();

    encode_surface_paint_plan_to_scene(
        &core.paint_plan(),
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
    );
}

#[test]
fn generic_native_window_starts_hidden_during_surface_setup() {
    let attrs = generic_window_attributes(&NativeRunOptions::default());

    assert!(!attrs.visible);
}

#[test]
fn generic_runtime_clamps_animation_frame_interval() {
    assert_eq!(animation_frame_interval(0), Duration::from_secs(1));
    assert_eq!(
        animation_frame_interval(120),
        Duration::from_secs_f64(1.0 / 120.0)
    );
    assert_eq!(
        animation_frame_interval(1_000),
        Duration::from_secs_f64(1.0 / 240.0)
    );
}

#[derive(Default)]
struct DemoBridge {
    state: DemoState,
}

impl RuntimeBridge<DemoMessage> for DemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn reduce_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::Increment => self.state.count += 1,
            DemoMessage::Rename(name) => self.state.name = name,
        }
    }
}

fn demo_bridge() -> DemoBridge {
    DemoBridge::default()
}

#[derive(Default)]
struct RepaintBridge {
    state: DemoState,
}

impl RuntimeBridge<DemoMessage> for RepaintBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        match message {
            DemoMessage::Increment => {
                self.state.count += 1;
                Command::request_repaint()
            }
            DemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }
}

fn demo_surface(state: &DemoState) -> Arc<UiSurface<DemoMessage>> {
    let button = WidgetSpec::Button(ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let input = WidgetSpec::TextInput(TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    ));
    Arc::new(UiSurface::new(SurfaceNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    button,
                    WidgetMessageMapper::button(|_| DemoMessage::Increment),
                ),
            ),
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    input,
                    WidgetMessageMapper::text_input(|message| match message {
                        TextInputMessage::Changed { value }
                        | TextInputMessage::Submitted { value } => DemoMessage::Rename(value),
                    }),
                ),
            ),
        ],
    )))
}

#[derive(Default)]
struct CanvasBridge {
    text: String,
}

impl RuntimeBridge<String> for CanvasBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            21,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Character(character),
                } => character.to_string(),
                _ => String::new(),
            },
        )))
    }

    fn update(&mut self, message: String) -> Command<String> {
        self.text.push_str(&message);
        Command::none()
    }
}
