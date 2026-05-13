use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, LayoutDebugOptions, Rect, SlotParams},
    runtime::{
        Command, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle,
        GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, PaintGpuSurface, PaintPrimitive,
        SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    widgets::{
        ButtonWidget, CanvasMessage, PointerButton, ScrollbarAxis, ScrollbarMessage,
        ScrollbarWidget, TextInputMessage, TextInputWidget, Widget, WidgetCommon, WidgetInput,
        WidgetOutput, WidgetSizing,
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

#[cfg(test)]
#[path = "tests/gpu_surface_runtime.rs"]
mod gpu_surface_runtime;
#[cfg(test)]
#[path = "tests/pointer_motion.rs"]
mod pointer_motion;
#[cfg(test)]
#[path = "tests/scene_cache.rs"]
mod scene_cache;

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
fn nested_button_activation_survives_surface_refresh_between_press_and_release() {
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
    core.runtime.refresh();
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().state.count, 1);
}

#[test]
fn generic_core_routes_text_edit_commands_only_to_text_inputs() {
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
    assert!(!core.route_text_edit(TextEditCommand::SelectAll).routed);

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
    assert!(core.route_text_edit(TextEditCommand::SelectAll).routed);
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
fn scrollbar_drag_state_survives_view_refresh_after_offset_message() {
    let mut core =
        GenericNativeRuntimeCore::new(ScrollbarBridge::default(), Vector2::new(240.0, 24.0));
    let press = Point::new(12.0, 7.0);
    let first_drag = Point::new(72.0, 7.0);
    let second_drag = Point::new(132.0, 7.0);

    assert!(
        core.route_pointer_press(press, PointerButton::Primary)
            .routed
    );
    let first_drag_outcome = core.route_pointer_move(first_drag);
    assert!(first_drag_outcome.routed);
    assert!(first_drag_outcome.needs_redraw());
    let first_offset = core.runtime.bridge().offset;
    assert!(first_offset > 0.0);

    let second_drag_outcome = core.route_pointer_move(second_drag);
    assert!(second_drag_outcome.routed);
    assert!(second_drag_outcome.needs_redraw());
    assert!(
        core.runtime.bridge().offset > first_offset,
        "drag should continue after the first offset message refreshes the surface"
    );
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
fn generic_core_can_enable_layout_debug_before_first_frame() {
    let core = GenericNativeRuntimeCore::new_with_debug_layout(
        demo_bridge(),
        Vector2::new(320.0, 40.0),
        true,
    );

    assert_eq!(
        core.runtime.layout_debug_options(),
        LayoutDebugOptions::bounds_only()
    );
    assert!(!core.runtime.layout().debug_primitives.is_empty());
}

#[test]
fn generic_native_window_starts_hidden_during_surface_setup() {
    let attrs = generic_window_attributes(&NativeRunOptions::default());

    assert!(!attrs.visible);
}

#[test]
fn generic_native_window_uses_configured_drag_and_drop_policy() {
    assert!(window::platform_drag_and_drop_enabled(
        &NativeRunOptions::default()
    ));
    assert!(!window::platform_drag_and_drop_enabled(&NativeRunOptions {
        drag_and_drop: false,
        ..NativeRunOptions::default()
    }));
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
struct ScrollbarBridge {
    offset: f32,
}

#[derive(Default)]
struct WheelRefreshBridge {
    wheel_count: usize,
    project_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GpuWheelMessage {
    delta: Vector2,
}

struct GpuWheelBridge {
    wheel_count: usize,
    project_count: usize,
    last_delta: Vector2,
    capabilities: GpuSurfaceCapabilities,
}

impl Default for GpuWheelBridge {
    fn default() -> Self {
        Self {
            wheel_count: 0,
            project_count: 0,
            last_delta: Vector2::new(0.0, 0.0),
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                    GpuSurfaceLineStyle {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 255,
                        },
                        width: 1.0,
                    },
                ),
            },
        }
    }
}

#[derive(Clone, Debug)]
struct TestGpuWheelWidget {
    common: WidgetCommon,
    capabilities: GpuSurfaceCapabilities,
}

impl TestGpuWheelWidget {
    fn new(capabilities: GpuSurfaceCapabilities) -> Self {
        let mut common = WidgetCommon::new(61, WidgetSizing::fixed(Vector2::new(200.0, 40.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            capabilities,
        }
    }
}

impl Widget for TestGpuWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::Wheel { delta, .. } => {
                Some(WidgetOutput::typed(GpuWheelMessage { delta }))
            }
            _ => None,
        }
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: 61,
            revision: 1,
            rect: bounds,
            content: GpuSurfaceContent::SignalBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                samples: Arc::<[f32]>::from(vec![0.0, 0.25, -0.5, 1.0]),
            },
            capabilities: self.capabilities,
            overlays: Vec::new(),
        }));
    }
}

#[derive(Default)]
struct RetainedBridge {
    render_count: usize,
    dirty_mask: u64,
    volatile: bool,
}

#[derive(Default)]
struct MultiRetainedBridge {
    render_counts: std::collections::BTreeMap<u64, usize>,
}

impl MultiRetainedBridge {
    fn render_count_for(&self, key: u64) -> usize {
        self.render_counts.get(&key).copied().unwrap_or_default()
    }
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

impl RuntimeBridge<()> for MultiRetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
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
                    SurfaceNode::retained_canvas_mapped(
                        31,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 7,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::retained_canvas_mapped(
                        32,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 8,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
            ],
        )))
    }

    fn render_retained_surface(
        &mut self,
        descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        *self.render_counts.entry(descriptor.key).or_default() += 1;
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
                    r: descriptor.key as u8,
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

impl RuntimeBridge<f32> for ScrollbarBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<f32>> {
        let mut scrollbar = ScrollbarWidget::new(
            41,
            ScrollbarAxis::Horizontal,
            WidgetSizing::fixed(Vector2::new(220.0, 14.0)),
        );
        scrollbar.props.viewport_fraction = 0.25;
        scrollbar.state.offset_fraction = self.offset;
        Arc::new(UiSurface::new(SurfaceNode::widget(
            scrollbar,
            WidgetMessageMapper::scrollbar(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => offset_fraction,
            }),
        )))
    }

    fn reduce_message(&mut self, message: f32) {
        self.offset = message;
    }
}

impl RuntimeBridge<String> for WheelRefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            51,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                _ => String::new(),
            },
        )))
    }

    fn reduce_message(&mut self, message: String) {
        if message == "wheel" {
            self.wheel_count += 1;
        }
    }
}

impl RuntimeBridge<GpuWheelMessage> for GpuWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<GpuWheelMessage>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            TestGpuWheelWidget::new(self.capabilities),
            WidgetMessageMapper::typed(|message: GpuWheelMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: GpuWheelMessage) {
        self.wheel_count += 1;
        self.last_delta = message.delta;
    }
}
