use super::shared::*;
use crate::runtime::{
    NativeFrameDiagnostics, PaintPrimitive, RuntimeAnimationActivity, RuntimeAnimationHost,
    RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities, RuntimeQueueHost,
    RuntimeRetainedSurfaceHost, RuntimeTransientOverlayHost, TransientOverlayContext,
};
use crate::widgets::{TextWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Default)]
pub(super) struct CountingProjectBridge {
    pub(super) project_count: usize,
}

impl RuntimeBridge<DemoMessage> for CountingProjectBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        self.project_count += 1;
        demo_surface(&DemoState::default())
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for CountingProjectBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

#[derive(Default)]
pub(super) struct UnsupportedPreparedRefreshBridge {
    pub(super) project_count: usize,
}

#[derive(Clone)]
struct UnsupportedPreparedRefreshWidget {
    common: WidgetCommon,
}

impl UnsupportedPreparedRefreshWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::fixed(101, 120.0, 28.0),
        }
    }
}

impl Widget for UnsupportedPreparedRefreshWidget {
    fn revision(&self) -> crate::widgets::WidgetRevision {
        crate::widgets::WidgetRevision::exact((), (), (), ())
    }

    fn supports_prepared_state_synchronization(&self) -> bool {
        true
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: crate::gui::types::Rect,
        _input: WidgetInput,
    ) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: crate::gui::types::Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::GpuSurface(
            crate::runtime::PaintGpuSurface {
                widget_id: self.common.id,
                key: 1,
                revision: 1,
                rect: bounds,
                content: crate::runtime::GpuSurfaceContent::RgbaAtlas {
                    source_rect: crate::gui::types::Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
                    atlas: Arc::new(
                        crate::gui::types::ImageRgba::new(1, 1, vec![255; 4])
                            .expect("valid test image"),
                    ),
                },
                capabilities: crate::runtime::GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            },
        ));
    }
}

impl RuntimeBridge<DemoMessage> for UnsupportedPreparedRefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::static_widget(
            UnsupportedPreparedRefreshWidget::new(),
        )))
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
}

#[derive(Default)]
pub(super) struct CountingAnimationActivityBridge {
    pub(super) animation_activity_polls: usize,
}

impl RuntimeBridge<DemoMessage> for CountingAnimationActivityBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_animation()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
}

impl RuntimeAnimationHost for CountingAnimationActivityBridge {
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.animation_activity_polls += 1;
        RuntimeAnimationActivity::idle()
    }
}

#[derive(Default)]
pub(super) struct NoTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

#[derive(Default)]
pub(super) struct ExactTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

#[derive(Default)]
pub(super) struct LargeExactBridge;

impl RuntimeBridge<DemoMessage> for LargeExactBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        let rows = (0..3_000_u64)
            .map(|index| {
                SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                    10_000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )))
            })
            .collect();
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: crate::layout::ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            rows,
        )))
    }
}

impl RuntimeBridge<DemoMessage> for ExactTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            TextWidget::new(
                101,
                "Stable",
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_transient_overlays()
    }
}

impl RuntimeTransientOverlayHost for ExactTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

#[derive(Default)]
pub(super) struct RetainedSurfaceBridge {
    pub(super) render_count: usize,
}

impl RuntimeBridge<DemoMessage> for RetainedSurfaceBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            31,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            crate::widgets::RetainedSurfaceDescriptor {
                key: 7,
                revision: 1,
                dirty_mask: 0,
                volatile: false,
            },
            |_| DemoMessage::Increment,
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_retained_surfaces()
    }
}

impl RuntimeRetainedSurfaceHost for RetainedSurfaceBridge {
    fn render_retained_surface(
        &mut self,
        _descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: crate::gui::types::Rect,
        _viewport: Vector2,
    ) -> Option<crate::gui::paint::PaintFrame> {
        self.render_count += 1;
        Some(crate::gui::paint::PaintFrame {
            clear_color: crate::gui::types::Rgba8::default(),
            primitives: vec![crate::gui::paint::Primitive::Rect(
                crate::gui::paint::FillRect {
                    rect,
                    color: crate::gui::types::Rgba8 {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 255,
                    },
                },
            )],
            text_runs: Vec::new(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PreparedRefreshTerminalMessage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PreparedRefreshEvent {
    ProjectionAdmitted,
    CandidateHeld,
    ProjectionCompleted,
    LayoutAdmitted,
    LayoutCompleted,
    PaintPlanAdmitted,
    Published,
    PaintPlanCompleted,
    SceneEncode,
    SceneAdmitted,
    TerminalUpdate(PreparedRefreshTerminalMessage),
}

pub(super) type PreparedRefreshRecorder = Rc<RefCell<Vec<PreparedRefreshEvent>>>;

#[derive(Clone, Copy)]
struct PreparedRefreshTerminalOutput;

#[derive(Clone)]
struct PreparedRefreshReplacementWidget {
    common: crate::widgets::WidgetCommon,
    paint_revision: u64,
}

impl PreparedRefreshReplacementWidget {
    fn new(paint_revision: u64, root_id: u64) -> Self {
        Self {
            common: crate::widgets::WidgetCommon::fixed(root_id, 120.0, 28.0),
            paint_revision,
        }
    }
}

impl crate::widgets::Widget for PreparedRefreshReplacementWidget {
    fn revision(&self) -> crate::widgets::WidgetRevision {
        crate::widgets::WidgetRevision::exact((), (), self.paint_revision, ())
    }

    fn supports_prepared_state_synchronization(&self) -> bool {
        true
    }

    fn common(&self) -> &crate::widgets::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: crate::gui::types::Rect,
        _input: crate::widgets::WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        None
    }

    fn prepare_replacement(
        &mut self,
        successor: Option<&dyn crate::widgets::Widget>,
    ) -> Option<crate::widgets::WidgetOutput> {
        let successor = successor?.as_any().downcast_ref::<Self>()?;
        (successor.paint_revision != self.paint_revision)
            .then(|| crate::widgets::WidgetOutput::typed(PreparedRefreshTerminalOutput))
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        bounds: crate::gui::types::Rect,
        _layout: &crate::layout::LayoutOutput,
        theme: &crate::theme::ThemeTokens,
    ) {
        primitives.push(crate::runtime::PaintPrimitive::FillRect(
            crate::runtime::PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: theme.accent_mint,
            },
        ));
    }
}

fn prepared_refresh_message_mapper() -> WidgetMessageMapper<PreparedRefreshTerminalMessage> {
    WidgetMessageMapper::dynamic_mapped(
        crate::runtime::EventMapper::with_revision((), |_output: PreparedRefreshTerminalOutput| {
            PreparedRefreshTerminalMessage
        })
        .typed_mapped(),
    )
}

pub(super) struct PreparedRefreshReplacementBridge {
    pub(super) replace: bool,
    pub(super) root_id: u64,
    pub(super) project_count: usize,
    recorder: PreparedRefreshRecorder,
}

impl PreparedRefreshReplacementBridge {
    pub(super) fn new(recorder: PreparedRefreshRecorder) -> Self {
        Self {
            replace: false,
            root_id: 101,
            project_count: 0,
            recorder,
        }
    }
}

impl RuntimeBridge<PreparedRefreshTerminalMessage> for PreparedRefreshReplacementBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<PreparedRefreshTerminalMessage>> {
        self.project_count += 1;
        let paint_revision = if self.replace { 2 } else { 1 };
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            PreparedRefreshReplacementWidget::new(paint_revision, self.root_id),
            prepared_refresh_message_mapper(),
        )))
    }

    fn update(
        &mut self,
        message: PreparedRefreshTerminalMessage,
    ) -> Command<PreparedRefreshTerminalMessage> {
        self.recorder
            .borrow_mut()
            .push(PreparedRefreshEvent::TerminalUpdate(message));
        Command::none()
    }
}

pub(super) fn prepared_refresh_scene_admission_recorder() -> PreparedRefreshRecorder {
    Rc::new(RefCell::new(Vec::new()))
}

pub(super) fn record_prepared_refresh_scene_encode(recorder: &PreparedRefreshRecorder) {
    recorder
        .borrow_mut()
        .push(PreparedRefreshEvent::SceneEncode);
}

pub(super) fn record_prepared_refresh_scene_admission(recorder: &PreparedRefreshRecorder) {
    recorder
        .borrow_mut()
        .push(PreparedRefreshEvent::SceneAdmitted);
}

pub(super) fn prepared_refresh_events(
    recorder: &PreparedRefreshRecorder,
) -> Vec<PreparedRefreshEvent> {
    recorder.borrow().clone()
}

impl RuntimeBridge<DemoMessage> for NoTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for NoTransientOverlayBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeTransientOverlayHost for NoTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

#[derive(Default)]
pub(super) struct OptInTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for OptInTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_transient_overlays()
    }
}

impl RuntimeTransientOverlayHost for OptInTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

pub(super) struct NoFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for NoFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }
}

#[derive(Default)]
pub(super) struct CountingFrameDiagnosticsBridge {
    pub(super) observer_checks: Cell<usize>,
}

impl RuntimeBridge<DemoMessage> for CountingFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        self.observer_checks
            .set(self.observer_checks.get().saturating_add(1));
        RuntimeHostCapabilities::new()
    }
}

pub(super) struct OptInFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for OptInFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for OptInFrameDiagnosticsBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

#[derive(Default)]
pub(super) struct TestFrameMessageBridge {
    queued: bool,
}

impl RuntimeBridge<DemoMessage> for TestFrameMessageBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new()
            .with_animation()
            .with_queues()
            .with_frame_diagnostics()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_repaint()
    }
}

impl RuntimeAnimationHost for TestFrameMessageBridge {
    fn needs_animation(&mut self) -> bool {
        true
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queued = true;
        true
    }
}

impl RuntimeQueueHost<DemoMessage> for TestFrameMessageBridge {
    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.queued) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
    }
}

impl RuntimeFrameDiagnosticsHost for TestFrameMessageBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}
