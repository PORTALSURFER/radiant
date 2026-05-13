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
#[path = "tests/event_routing.rs"]
mod event_routing;
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
fn generic_core_empty_runtime_wakeup_does_not_need_redraw() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(!outcome.routed);
    assert!(!outcome.needs_redraw());
    assert!(!outcome.runtime_work_remaining);
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

struct AnimatingBridge;

impl RuntimeBridge<DemoMessage> for AnimatingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
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
