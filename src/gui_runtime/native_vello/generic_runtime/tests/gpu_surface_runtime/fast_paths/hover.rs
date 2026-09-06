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
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::custom_widget(
            TestGpuHoverSurface::clipped(80.0),
            WidgetMessageMapper::typed(|message: GpuHoverExitMessage| message),
        )))
    }
}

impl RuntimeBridge<GpuHoverExitMessage> for GpuHoverExitBridge {
    fn project_surface(&mut self) -> std::sync::Arc<UiSurface<GpuHoverExitMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
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
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::stack(
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

impl crate::widgets::WidgetPointerMotion for TestPassivePointerHitWidget {
    fn revision(&self) -> crate::widgets::WidgetPointerMotionRevision {
        crate::widgets::WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
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

    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
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

impl crate::widgets::WidgetPointerMotion for TestPointerMoveWidget {
    fn revision(&self) -> crate::widgets::WidgetPointerMotionRevision {
        crate::widgets::WidgetPointerMotionRevision::exact(true)
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

    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
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

#[test]
fn native_container_gesture_blocks_hover_and_late_wheel_replay() {
    use crate::gui_runtime::native_vello::generic_runtime::native_pointer_ingress::NativeGestureSample;
    use crate::{
        application::{IntoView, ViewNode},
        gui::pointer_ingress::{GestureKind, GestureUnit, InputDeviceId},
        widgets::GesturePolicy,
    };
    use winit::event::{MouseScrollDelta, TouchPhase};
    struct Bridge {
        inner: GpuWheelBridge,
        events: std::rc::Rc<std::cell::Cell<usize>>,
    }
    impl RuntimeBridge<GpuWheelMessage> for Bridge {
        fn project_surface(&mut self) -> Arc<UiSurface<GpuWheelMessage>> {
            let root = Arc::unwrap_or_clone(self.inner.project_surface()).into_root();
            let events = self.events.clone();
            let region = ViewNode::from(root)
                .on_gesture_with_revision(
                    GesturePolicy::none()
                        .recognize(GestureKind::Pinch, 0.1)
                        .unwrap(),
                    (),
                    move |_| {
                        events.set(events.get() + 1);
                        None
                    },
                )
                .id(100);
            crate::runtime::test_arc_surface(UiSurface::new(region.into_node()))
        }
        fn reduce_message(&mut self, message: GpuWheelMessage) {
            self.inner.reduce_message(message);
        }
    }
    let events = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        Bridge {
            inner: GpuWheelBridge::default(),
            events: events.clone(),
        },
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);
    runner.input.last_cursor = Some(point);
    runner.core.set_current_pointer_position(Some(point));
    assert!(runner.can_fast_path_native_hover_move(point));
    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, -40.0), Default::default());
    assert_eq!(runner.core.runtime.bridge().inner.wheel_count, 0);
    runner.frame.scene_texture_dirty = false;
    let device = runner
        .input
        .native_pointer_ingress
        .retain_device(
            winit::event::DeviceId::dummy(),
            crate::gui::pointer_ingress::DeviceKind::Trackpad,
        )
        .unwrap();
    let sample = |phase, value| NativeGestureSample {
        kind: GestureKind::Pinch,
        unit: GestureUnit::Scale,
        value,
        phase,
        device,
        modifiers: Default::default(),
        timestamp: crate::gui::input::InputTimestamp::capture(),
    };
    let started = runner.route_native_gesture_sample(sample(TouchPhase::Started, 1.2));
    assert!(started.outcome.routed);
    assert!(started.outcome.needs_redraw());
    assert!(started.deferred_wheel_effects.gpu_surface.is_some());
    assert!(
        !runner.frame.scene_texture_dirty,
        "lower-stage visual work waits for ticket completion"
    );
    assert_eq!(runner.core.runtime.bridge().inner.wheel_count, 1);
    assert_eq!(runner.core.runtime.pointer_capture(), None);
    assert!(!runner.can_fast_path_native_hover_move(point));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(Some(point), Point::new(30.0, 20.0)));
    assert!(!runner.can_coalesce_gpu_surface_wheel(point, Vector2::new(0.0, -40.0)));
    runner.route_native_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -2.0));
    assert!(runner.input.pending_gpu_surface_wheel.is_none());
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert_eq!(runner.core.runtime.bridge().inner.wheel_count, 1);
    let ended = runner.route_native_gesture_sample(sample(TouchPhase::Ended, 1.0));
    assert!(ended.outcome.routed);
    runner.flush_pending_wheel_input_now();
    assert_eq!(events.get(), 2);
    assert_eq!(runner.core.runtime.bridge().inner.wheel_count, 1);
    assert!(runner.can_fast_path_native_hover_move(point));
    let again = runner.route_native_gesture_sample(sample(TouchPhase::Started, 1.2));
    assert!(again.outcome.routed);
    assert_eq!(events.get(), 3);
    let invalid_start = runner.route_native_gesture_sample(sample(TouchPhase::Started, f32::NAN));
    assert!(!invalid_start.outcome.routed);
    assert_eq!(events.get(), 3);
    let foreign = runner.route_native_gesture_sample(NativeGestureSample {
        device: InputDeviceId::from_host(2).unwrap(),
        ..sample(TouchPhase::Ended, f32::NAN)
    });
    assert!(!foreign.outcome.routed);
    assert_eq!(runner.core.runtime.retained_gesture_device(), Some(device));
    let malformed = crate::gui_runtime::native_vello::generic_runtime::native_pointer_ingress::normalize_gesture(
        &mut runner.input.native_pointer_ingress, winit::event::DeviceId::dummy(),
        crate::gui_runtime::native_vello::generic_runtime::native_pointer_ingress::GestureInput::Pinch { delta: f64::NAN, phase: TouchPhase::Ended },
        crate::theme::DpiScale::ONE, Default::default(), crate::gui::input::InputTimestamp::capture(),
    ).unwrap().unwrap();
    let rejected = runner.route_native_gesture_sample(malformed);
    assert!(!rejected.outcome.routed);
    assert_eq!(runner.core.runtime.retained_gesture_device(), None);
    assert_eq!(
        events.get(),
        4,
        "malformed matching terminal cancels once using the last valid event"
    );
    runner.route_native_gesture_sample(malformed);
    assert_eq!(events.get(), 4);
    assert!(runner.can_fast_path_native_hover_move(point));
}
