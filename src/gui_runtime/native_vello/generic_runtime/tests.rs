use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, SlotParams},
    runtime::{Command, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasMessage, PointerButton, TextInputMessage, TextInputWidget, WidgetInput,
        WidgetSizing,
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
    assert!(core.route_character(' ').routed);
    assert!(core.route_widget_key(WidgetKey::Enter).routed);
    assert_eq!(core.runtime.bridge().state.name, "R ");
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
fn generic_canvas_receives_wheel_before_scroll_fallback() {
    let bridge = CanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&21)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(
        core.route_scroll(canvas_point, Vector2::new(0.0, -40.0))
            .routed
    );

    assert_eq!(core.runtime.bridge().text, "wheel");
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
fn generic_core_is_repaint_driven_when_host_reports_no_animation() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    assert!(!core.needs_animation());
}

#[test]
fn generic_core_preserves_animation_when_host_requests_it() {
    let mut core = GenericNativeRuntimeCore::new(AnimatingBridge, Vector2::new(320.0, 40.0));

    assert!(core.needs_animation());
}

#[test]
fn generic_paint_plan_encodes_to_vello_scene() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();

    encode_surface_paint_plan_to_scene(
        &core.paint_plan(),
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
}

#[test]
fn retained_custom_surface_cache_skips_unchanged_bridge_render() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    let second = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 1);
    assert_eq!(core.runtime.bridge().render_count, 1);
}

#[test]
fn retained_custom_surface_cache_rejects_current_dirty_descriptor() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let second = encode_surface_paint_plan_to_scene(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}

#[test]
fn retained_custom_surface_cache_rejects_volatile_descriptor() {
    let mut core = GenericNativeRuntimeCore::new(
        RetainedBridge {
            volatile: true,
            ..RetainedBridge::default()
        },
        Vector2::new(320.0, 40.0),
    );
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );
    let second = encode_surface_paint_plan_to_scene(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        Duration::ZERO,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
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
    let button = ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
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

#[derive(Default)]
struct RetainedBridge {
    render_count: usize,
    dirty_mask: u64,
    volatile: bool,
}

struct AnimatingBridge;

impl RuntimeBridge<DemoMessage> for AnimatingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
    }
}

impl RuntimeBridge<()> for RetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            31,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            crate::widgets::RetainedSurfaceDescriptor {
                key: 7,
                revision: 1,
                dirty_mask: self.dirty_mask,
                volatile: self.volatile,
            },
            |_| (),
        )))
    }

    fn render_retained_surface(
        &mut self,
        _descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        self.render_count += 1;
        Some(PaintFrame {
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            primitives: vec![Primitive::Rect(crate::gui::paint::FillRect {
                rect,
                color: Rgba8 {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            })],
            text_runs: Vec::new(),
        })
    }
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
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                _ => String::new(),
            },
        )))
    }

    fn update(&mut self, message: String) -> Command<String> {
        self.text.push_str(&message);
        Command::none()
    }
}
