use super::super::*;
use winit::dpi::PhysicalPosition;

#[test]
fn normal_scene_rebuild_clips_gpu_hover_interaction_regions() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ClippedGpuHoverBridge,
        Vector2::new(200.0, 40.0),
    );

    runner.rebuild_scene();

    assert!(runner.can_fast_path_native_hover_move(Point::new(20.0, 20.0)));
    assert!(!runner.can_fast_path_native_hover_move(Point::new(120.0, 20.0)));
    assert_eq!(runner.frame.gpu_surface_interaction_regions.len(), 1);
    assert_eq!(
        runner.frame.gpu_surface_interaction_regions[0].rect.width(),
        80.0
    );
}

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

#[test]
fn native_gpu_hover_hides_native_cursor_until_surface_exit() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuHoverExitBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();

    runner.handle_cursor_moved(PhysicalPosition::new(20.0, 20.0));

    assert!(
        !runner.input.native_cursor_visible,
        "GPU hover surfaces draw their own cursor overlay and should hide the native host cursor"
    );
    assert_eq!(
        runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default),
        "hidden native cursor should still be reset to default so it reappears cleanly"
    );

    runner.handle_cursor_moved(PhysicalPosition::new(220.0, 20.0));

    assert!(
        runner.input.native_cursor_visible,
        "native cursor must be restored as soon as pointer leaves the GPU hover surface"
    );
}

#[test]
fn native_gpu_hover_fast_path_respects_active_top_pointer_widget() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuHoverCoveredBridge::active_pointer_move(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(!runner.can_fast_path_native_hover_move(point));

    runner.handle_cursor_moved(PhysicalPosition::new(20.0, 20.0));

    assert_eq!(
        runner.core.runtime.bridge().pointer_moves,
        1,
        "top pointer-move widget above a GPU surface must receive native hover movement"
    );
}

#[test]
fn native_gpu_hover_fast_path_respects_passive_top_pointer_widget() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuHoverCoveredBridge::passive_pointer_hit(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(!runner.can_fast_path_native_hover_move(point));

    runner.handle_cursor_moved(PhysicalPosition::new(20.0, 20.0));
    assert_eq!(runner.core.runtime.bridge().pointer_moves, 0);

    let route = runner.route_native_mouse_input(
        winit::event::MouseButton::Left,
        winit::event::ElementState::Pressed,
    );

    assert!(route.outcome.routed);
    assert_eq!(route.diagnostic.hit_target, Some(81));
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GpuHoverExitMessage;

#[derive(Default)]
struct GpuHoverExitBridge {
    pointer_moves: usize,
}

#[derive(Default)]
struct ClippedGpuHoverBridge;

impl RuntimeBridge<GpuHoverExitMessage> for ClippedGpuHoverBridge {
    fn project_surface(&mut self) -> std::sync::Arc<UiSurface<GpuHoverExitMessage>> {
        std::sync::Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            TestGpuHoverSurface::clipped(80.0),
            WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
        )))
    }
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

struct GpuHoverCoveredBridge {
    pointer_moves: usize,
    top_widget: CoveredTopWidget,
}

#[derive(Clone, Copy, Debug)]
enum CoveredTopWidget {
    ActivePointerMove,
    PassivePointerHit,
}

impl GpuHoverCoveredBridge {
    fn active_pointer_move() -> Self {
        Self {
            pointer_moves: 0,
            top_widget: CoveredTopWidget::ActivePointerMove,
        }
    }

    fn passive_pointer_hit() -> Self {
        Self {
            pointer_moves: 0,
            top_widget: CoveredTopWidget::PassivePointerHit,
        }
    }
}

impl RuntimeBridge<GpuHoverExitMessage> for GpuHoverCoveredBridge {
    fn project_surface(&mut self) -> std::sync::Arc<UiSurface<GpuHoverExitMessage>> {
        let top = match self.top_widget {
            CoveredTopWidget::ActivePointerMove => SurfaceNode::custom_widget(
                TestPointerMoveWidget::new(),
                WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
            ),
            CoveredTopWidget::PassivePointerHit => SurfaceNode::custom_widget(
                TestPassivePointerHitWidget::new(),
                WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
            ),
        };
        std::sync::Arc::new(UiSurface::new(SurfaceNode::stack(
            1,
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::custom_widget(
                        TestGpuHoverSurface::new(),
                        WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
                    ),
                ),
                SurfaceChild::new(SlotParams::fill(), top),
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
    clip_width: Option<f32>,
}

impl TestGpuHoverSurface {
    fn new() -> Self {
        let mut common = WidgetCommon::new(61, WidgetSizing::fixed(Vector2::new(200.0, 40.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            clip_width: None,
        }
    }

    fn clipped(width: f32) -> Self {
        Self {
            clip_width: Some(width),
            ..Self::new()
        }
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
        if let Some(width) = self.clip_width {
            primitives.push(PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: self.common.id,
                rect: Rect::from_min_size(
                    bounds.min,
                    Vector2::new(width.min(bounds.width()), bounds.height()),
                ),
            }));
        }
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
        if self.clip_width.is_some() {
            primitives.push(PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd {
                node_id: self.common.id,
            }));
        }
    }
}

#[derive(Clone, Debug)]
struct TestPointerMoveWidget {
    common: WidgetCommon,
}

#[derive(Clone, Debug)]
struct TestPassivePointerHitWidget {
    common: WidgetCommon,
}

impl TestPassivePointerHitWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(81, WidgetSizing::fixed(Vector2::new(120.0, 40.0)));
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for TestPassivePointerHitWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::PointerPress { .. })
            .then_some(WidgetOutput::typed(GpuHoverExitMessage))
    }

    fn accepts_pointer_move(&self) -> bool {
        false
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
