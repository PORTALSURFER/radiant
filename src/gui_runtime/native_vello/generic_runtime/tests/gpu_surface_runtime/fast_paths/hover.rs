use super::super::*;
use winit::dpi::PhysicalPosition;

#[test]
fn native_gpu_hover_fast_path_is_disabled_during_pointer_capture() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(runner.can_fast_path_native_hover_move(point));
    assert!(
        runner
            .core
            .route_pointer_press(point, PointerButton::Primary)
            .needs_redraw()
    );
    assert!(runner.core.runtime.pointer_capture().is_some());
    assert!(!runner.can_fast_path_native_hover_move(Point::new(40.0, 20.0)));
}

#[test]
fn leaving_native_gpu_hover_still_routes_next_widget_move() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuHoverExitBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();

    runner.handle_cursor_moved(PhysicalPosition::new(20.0, 20.0));
    assert_eq!(runner.core.runtime.bridge().pointer_moves, 0);

    runner.handle_cursor_moved(PhysicalPosition::new(220.0, 20.0));
    assert_eq!(
        runner.core.runtime.bridge().pointer_moves,
        1,
        "leaving a native GPU hover surface must not swallow the first move over the next widget"
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GpuHoverExitMessage;

#[derive(Default)]
struct GpuHoverExitBridge {
    pointer_moves: usize,
}

impl RuntimeBridge<GpuHoverExitMessage> for GpuHoverExitBridge {
    fn project_surface(&mut self) -> std::sync::Arc<UiSurface<GpuHoverExitMessage>> {
        std::sync::Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 0.0,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::custom_widget(
                        TestGpuHoverSurface::new(),
                        WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::custom_widget(
                        TestPointerMoveWidget::new(),
                        WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, _message: GpuHoverExitMessage) {
        self.pointer_moves += 1;
    }
}

#[derive(Clone, Debug)]
struct TestGpuHoverSurface {
    common: WidgetCommon,
}

impl TestGpuHoverSurface {
    fn new() -> Self {
        let mut common = WidgetCommon::new(61, WidgetSizing::fixed(Vector2::new(200.0, 40.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for TestGpuHoverSurface {
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
            key: 61,
            revision: 1,
            rect: bounds,
            content: GpuSurfaceContent::SignalBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                samples: std::sync::Arc::<[f32]>::from(vec![0.0, 0.25, -0.5, 1.0]),
            },
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
            overlays: Vec::new(),
        }));
    }
}

#[derive(Clone, Debug)]
struct TestPointerMoveWidget {
    common: WidgetCommon,
}

impl TestPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(71, WidgetSizing::fixed(Vector2::new(120.0, 40.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for TestPointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::PointerMove { .. })
            .then(|| WidgetOutput::typed(GpuHoverExitMessage))
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }
}
