use super::super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in super::super) struct GpuWheelMessage {
    pub(in super::super) delta: Vector2,
}

pub(in super::super) struct GpuWheelBridge {
    pub(in super::super) wheel_count: usize,
    pub(in super::super) project_count: usize,
    pub(in super::super) last_delta: Vector2,
    pub(in super::super) capabilities: GpuSurfaceCapabilities,
}

#[derive(Default)]
pub(in super::super) struct GpuWheelScrollBridge {
    pub(in super::super) scroll_count: usize,
    pub(in super::super) project_count: usize,
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

#[derive(Clone, Debug)]
struct PassiveGpuWheelWidget {
    common: WidgetCommon,
}

impl PassiveGpuWheelWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(71, WidgetSizing::fixed(Vector2::new(200.0, 80.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for PassiveGpuWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
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
            key: 71,
            revision: 1,
            rect: bounds,
            content: GpuSurfaceContent::SignalBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                samples: Arc::<[f32]>::from(vec![0.0, 0.25, -0.5, 1.0]),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: false,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            overlays: Vec::new(),
        }));
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

impl RuntimeBridge<String> for GpuWheelScrollBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(
            SurfaceNode::scroll_area(
                70,
                SurfaceNode::custom_widget(
                    PassiveGpuWheelWidget::new(),
                    WidgetMessageMapper::none(),
                ),
            )
            .with_scroll_message(Arc::new(|_| String::from("scroll"))),
        ))
    }

    fn reduce_message(&mut self, message: String) {
        if message == "scroll" {
            self.scroll_count += 1;
        }
    }
}
