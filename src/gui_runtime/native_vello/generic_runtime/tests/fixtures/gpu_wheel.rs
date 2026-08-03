use super::super::*;
use crate::gui::input::InputTimestamp;
use crate::gui::types::Point;
use crate::runtime::{
    NativeFrameDiagnostics, RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
};
use crate::widgets::PointerModifiers;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in super::super) struct GpuWheelMessage {
    pub(in super::super) position: Point,
    pub(in super::super) delta: Vector2,
    pub(in super::super) modifiers: PointerModifiers,
    pub(in super::super) timestamp: Option<InputTimestamp>,
}

pub(in super::super) struct GpuWheelBridge {
    pub(in super::super) wheel_count: usize,
    pub(in super::super) project_count: usize,
    pub(in super::super) last_position: Option<Point>,
    pub(in super::super) last_delta: Vector2,
    pub(in super::super) last_modifiers: Option<PointerModifiers>,
    pub(in super::super) last_timestamp: Option<InputTimestamp>,
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
            last_position: None,
            last_delta: Vector2::new(0.0, 0.0),
            last_modifiers: None,
            last_timestamp: None,
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                coalesce_horizontal_wheel: false,
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
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
            } => Some(WidgetOutput::typed(GpuWheelMessage {
                position,
                delta,
                modifiers,
                timestamp,
            })),
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
pub(in super::super) struct PassiveGpuWheelWidget {
    common: WidgetCommon,
}

impl PassiveGpuWheelWidget {
    pub(in super::super) fn new() -> Self {
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
                coalesce_horizontal_wheel: false,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            overlays: Vec::new(),
        }));
    }
}

impl RuntimeBridge<GpuWheelMessage> for GpuWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<GpuWheelMessage>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::custom_widget(
            TestGpuWheelWidget::new(self.capabilities),
            WidgetMessageMapper::typed(|message: GpuWheelMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: GpuWheelMessage) {
        self.wheel_count += 1;
        self.last_position = Some(message.position);
        self.last_delta = message.delta;
        self.last_modifiers = Some(message.modifiers);
        self.last_timestamp = message.timestamp;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, GpuWheelMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for GpuWheelBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeBridge<String> for GpuWheelScrollBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(
            SurfaceNode::scroll_area(
                70,
                SurfaceNode::custom_widget(
                    PassiveGpuWheelWidget::new(),
                    WidgetMessageMapper::none(),
                ),
            )
            .with_scroll_message_local(Rc::new(|_| Some(String::from("scroll")))),
        ))
    }

    fn reduce_message(&mut self, message: String) {
        if message == "scroll" {
            self.scroll_count += 1;
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, String> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for GpuWheelScrollBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}
