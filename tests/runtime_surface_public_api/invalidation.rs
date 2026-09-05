use super::*;
use radiant::application::{ApplicationEnvironment, LocaleId, TextScale};
use radiant::runtime::{IdentityAudit, SurfaceInvalidation, SurfaceRefreshCounters};
use std::time::Duration;

#[test]
fn identity_audit_is_a_runtime_only_copyable_policy() {
    let mut runtime = SurfaceRuntime::new(
        RevisionBridge {
            label: "Audit",
            height: 24.0,
            widget_id: 10,
        },
        Vector2::new(180.0, 80.0),
    );
    assert_eq!(runtime.identity_audit(), IdentityAudit::default());
    let strict = IdentityAudit::strict();
    let copied = strict;
    runtime.set_identity_audit(copied);
    assert_eq!(runtime.identity_audit(), strict);

    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .identity
            .replacement_count,
        0
    );
}

struct RevisionBridge {
    label: &'static str,
    height: f32,
    widget_id: u64,
}

impl RuntimeBridge<()> for RevisionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::arc_surface(UiSurface::new(SurfaceNode::column(
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
    let requested_scope = RepaintScope::Projection;
    // TextWidget's exact geometry revision includes text content, so changing
    // the label promotes this projection request to an effective layout pass.
    let effective_scope = RepaintScope::Layout;
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
    runtime.refresh_with_scope(requested_scope);

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
            reconciliation_attempts: before_counters.reconciliation_attempts,
            reconciliation_applied: before_counters.reconciliation_applied,
            reconciliation_unsupported: before_counters.reconciliation_unsupported,
            reconciliation_fallbacks: before_counters.reconciliation_fallbacks + 1,
            widget_state_sync: before_counters.widget_state_sync + 1,
            layout: before_counters.layout
                + if effective_scope.refreshes_layout() {
                    1
                } else {
                    0
                },
            base_paint_plan_rebuilds: before_counters.base_paint_plan_rebuilds,
        }
    );
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        SurfaceInvalidation::Projection
    );
    assert_ne!(
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
            crate::arc_surface(UiSurface::new(SurfaceNode::static_widget(
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

struct TextScaleButtonBridge {
    environment: ApplicationEnvironment,
    activations: Arc<Mutex<Vec<DemoMessage>>>,
}

impl RuntimeBridge<DemoMessage> for TextScaleButtonBridge {
    fn application_environment(&mut self) -> Option<ApplicationEnvironment> {
        Some(self.environment.clone())
    }

    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        crate::arc_surface(
            UiSurface::new(SurfaceNode::widget(
                ButtonWidget::new(70, "Scale", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::button(|_| DemoMessage::Increment),
            ))
            .with_application_environment(self.environment.clone()),
        )
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        self.activations
            .lock()
            .expect("button activation log")
            .push(message);
        Command::none()
    }
}

fn text_scale_environment(scale: f32) -> ApplicationEnvironment {
    ApplicationEnvironment::new(LocaleId::english())
        .with_text_scale(TextScale::new(scale).expect("valid text scale"))
}

#[test]
fn application_environment_text_scale_promotes_paint_only_to_one_layout_per_change() {
    let activations = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        TextScaleButtonBridge {
            environment: text_scale_environment(1.0),
            activations,
        },
        Vector2::new(120.0, 40.0),
    );

    let initial = runtime.refresh_counters();
    for (scale, expected_layout) in [(1.5, initial.layout + 1), (2.0, initial.layout + 2)] {
        runtime.bridge_mut().environment = text_scale_environment(scale);
        runtime.refresh_with_scope(RepaintScope::PaintOnly);
        let counters = runtime.refresh_counters();
        let transitions = expected_layout - initial.layout;
        assert_eq!(
            counters,
            SurfaceRefreshCounters {
                application_projection: initial.application_projection + transitions,
                runtime_projection: initial.runtime_projection + transitions,
                reconciliation_attempts: initial.reconciliation_attempts,
                reconciliation_applied: initial.reconciliation_applied,
                reconciliation_unsupported: initial.reconciliation_unsupported,
                reconciliation_fallbacks: initial.reconciliation_fallbacks + transitions,
                widget_state_sync: initial.widget_state_sync + transitions,
                layout: expected_layout,
                base_paint_plan_rebuilds: initial.base_paint_plan_rebuilds,
            }
        );
    }

    let unchanged = runtime.refresh_counters();
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    assert_eq!(runtime.refresh_counters(), unchanged);
}

#[test]
fn button_capture_survives_scale_refresh_and_activates_once_at_same_physical_point() {
    let activations = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        TextScaleButtonBridge {
            environment: text_scale_environment(1.0),
            activations: activations.clone(),
        },
        Vector2::new(120.0, 40.0),
    );
    let point = Point::new(20.0, 10.0);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(70)
    );
    assert_eq!(runtime.pointer_capture(), Some(70));
    let before_refresh = runtime.refresh_counters();

    runtime.bridge_mut().environment = text_scale_environment(1.5);
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    assert_eq!(runtime.pointer_capture(), Some(70));
    assert_eq!(runtime.refresh_counters().layout, before_refresh.layout + 1);

    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(70)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        activations
            .lock()
            .expect("button activation log")
            .as_slice(),
        [DemoMessage::Increment]
    );
}

#[test]
fn projection_stage_reuses_scrolled_geometry_and_scroll_state() {
    struct ScrollBridge;

    impl RuntimeBridge<()> for ScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::arc_surface(UiSurface::new(SurfaceNode::scroll_area(
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
