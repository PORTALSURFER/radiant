use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DemoMessage {
    Increment,
    Rename(String),
}

#[derive(Default)]
pub(super) struct DemoState {
    pub(super) count: usize,
    pub(super) name: String,
}

#[derive(Default)]
pub(super) struct DemoBridge {
    pub(super) state: DemoState,
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

pub(super) fn demo_bridge() -> DemoBridge {
    DemoBridge::default()
}

#[derive(Default)]
pub(super) struct RepaintBridge {
    pub(super) state: DemoState,
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
pub(super) struct CanvasBridge {
    pub(super) text: String,
}

#[derive(Default)]
pub(super) struct ScrollbarBridge {
    pub(super) offset: f32,
}

#[derive(Default)]
pub(super) struct WheelRefreshBridge {
    pub(super) wheel_count: usize,
    pub(super) project_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct GpuWheelMessage {
    pub(super) delta: Vector2,
}

pub(super) struct GpuWheelBridge {
    pub(super) wheel_count: usize,
    pub(super) project_count: usize,
    pub(super) last_delta: Vector2,
    pub(super) capabilities: GpuSurfaceCapabilities,
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

pub(super) struct AnimatingBridge;

pub(super) struct PaintOnlyFrameBridge {
    pub(super) pending_frame: bool,
}

impl Default for PaintOnlyFrameBridge {
    fn default() -> Self {
        Self {
            pending_frame: true,
        }
    }
}

impl RuntimeBridge<DemoMessage> for AnimatingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
    }
}

impl RuntimeBridge<DemoMessage> for PaintOnlyFrameBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_paint_only()
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.pending_frame) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
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
