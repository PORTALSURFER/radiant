use super::*;
use radiant::runtime::{SurfaceInvalidation, SurfaceRefreshCounters};
use std::time::Duration;

struct RevisionBridge {
    label: &'static str,
    height: f32,
    widget_id: u64,
}

impl RuntimeBridge<()> for RevisionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::column(
            1,
            0.0,
            vec![SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::static_widget(TextWidget::new(
                    self.widget_id,
                    self.label,
                    WidgetSizing::fixed(Vector2::new(120.0, self.height)),
                )),
            )],
        )))
    }
}

#[test]
fn projection_stage_refreshes_paint_content_without_running_layout() {
    let mut runtime = SurfaceRuntime::new(
        RevisionBridge {
            label: "Before",
            height: 24.0,
            widget_id: 10,
        },
        Vector2::new(180.0, 80.0),
    );
    let before_layout = runtime.layout().rects[&10];
    let before_counters = runtime.refresh_counters();

    runtime.bridge_mut().label = "After";
    runtime.refresh_with_scope(RepaintScope::Projection);

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "After"
    );
    assert_eq!(runtime.layout().rects[&10], before_layout);
    assert_eq!(
        runtime.refresh_counters(),
        SurfaceRefreshCounters {
            application_projection: before_counters.application_projection + 1,
            runtime_projection: before_counters.runtime_projection + 1,
            widget_state_sync: before_counters.widget_state_sync + 1,
            layout: before_counters.layout,
        }
    );
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        SurfaceInvalidation::Projection
    );
    assert_eq!(
        runtime.last_refresh_diagnostics().timings.layout,
        Duration::ZERO
    );
}

#[test]
fn layout_stage_reprojects_and_recomputes_geometry() {
    let mut runtime = SurfaceRuntime::new(
        RevisionBridge {
            label: "Row",
            height: 24.0,
            widget_id: 10,
        },
        Vector2::new(180.0, 80.0),
    );
    let before_layout_count = runtime.refresh_counters().layout;

    runtime.bridge_mut().height = 40.0;
    runtime.refresh_with_scope(RepaintScope::Layout);

    assert_eq!(runtime.layout().rects[&10].height(), 40.0);
    assert_eq!(runtime.refresh_counters().layout, before_layout_count + 1);
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        SurfaceInvalidation::Layout
    );
}

#[test]
fn surface_stage_clears_focus_when_structural_revision_removes_identity() {
    struct FocusBridge {
        widget_id: u64,
    }

    impl RuntimeBridge<()> for FocusBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            Arc::new(UiSurface::new(SurfaceNode::static_widget(
                TextInputWidget::new(
                    self.widget_id,
                    String::from("Focus"),
                    WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
                ),
            )))
        }
    }

    let mut runtime = SurfaceRuntime::new(FocusBridge { widget_id: 10 }, Vector2::new(180.0, 80.0));
    assert!(runtime.focus_widget(10));

    runtime.bridge_mut().widget_id = 11;
    runtime.refresh_with_scope(RepaintScope::Surface);

    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        SurfaceInvalidation::Surface
    );
}

#[test]
fn projection_stage_retains_focus_hover_and_pointer_capture_for_stable_identity() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let button = Point::new(150.0, 10.0);

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(button)),
        Some(11)
    );
    assert_eq!(runtime.hovered_widget(), Some(11));
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(button)),
        Some(11)
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), Some(11));

    runtime.refresh_with_scope(RepaintScope::Projection);

    assert_eq!(runtime.hovered_widget(), Some(11));
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), Some(11));
}

#[test]
fn projection_stage_reuses_scrolled_geometry_and_scroll_state() {
    struct ScrollBridge;

    impl RuntimeBridge<()> for ScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            Arc::new(UiSurface::new(SurfaceNode::scroll_area(
                31,
                SurfaceNode::text(
                    32,
                    "Long content",
                    WidgetSizing::fixed(Vector2::new(220.0, 320.0)),
                ),
            )))
        }
    }

    let mut runtime = SurfaceRuntime::new(ScrollBridge, Vector2::new(100.0, 80.0));
    assert!(runtime.scroll_at(Point::new(10.0, 10.0), Vector2::new(0.0, 48.0)));
    let scrolled = runtime.layout().rects[&32];

    runtime.refresh_with_scope(RepaintScope::Projection);

    assert_eq!(runtime.layout().rects[&32], scrolled);
    assert!(runtime.layout().rects[&32].min.y < 0.0);
}
