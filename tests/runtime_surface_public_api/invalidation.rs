use super::*;
use radiant::application::{ApplicationEnvironment, LocaleId, TextScale, WritingDirection};
use radiant::runtime::{IdentityAudit, SurfaceInvalidation, SurfaceRefreshCounters};
use radiant::widgets::DragHandleMessage;
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

#[derive(Clone, Debug, PartialEq)]
enum TreeRowRuntimeMessage {
    Activate,
    Drag(DragHandleMessage),
}

struct TreeRowRuntimeBridge {
    environment: ApplicationEnvironment,
    events: Vec<TreeRowRuntimeMessage>,
    project_count: usize,
    drag_started: bool,
}

impl RuntimeBridge<TreeRowRuntimeMessage> for TreeRowRuntimeBridge {
    fn application_environment(&mut self) -> Option<ApplicationEnvironment> {
        Some(self.environment.clone())
    }

    fn project_surface(&mut self) -> Arc<UiSurface<TreeRowRuntimeMessage>> {
        self.project_count += 1;
        let underlay = ui::interactive_row_underlay(ui::text("Underlay"))
            .input_id(9001)
            .selected(true)
            .active_target(true)
            .mapped(|_| TreeRowRuntimeMessage::Activate);
        let tree_row = ui::tree_row("Folder")
            .depth(2)
            .has_children(true)
            .expanded(true)
            .row_height(22.0)
            .selected(true)
            .stable_row_identity(91, "tree-row")
            .drag_drop_state(ui::TreeRowDragDropState {
                drag_active: true,
                drag_source: true,
                drop_candidate: true,
                drop_target: true,
                drop_target_active: true,
            })
            .interactive_actions(
                ui::InteractiveRowActions::new()
                    .activate(|| TreeRowRuntimeMessage::Activate)
                    .drag(TreeRowRuntimeMessage::Drag),
            );
        arc_surface(ui::column([underlay, tree_row]).into_surface())
    }

    fn update(&mut self, message: TreeRowRuntimeMessage) -> Command<TreeRowRuntimeMessage> {
        self.events.push(message.clone());
        match message {
            TreeRowRuntimeMessage::Activate => Command::none(),
            TreeRowRuntimeMessage::Drag(message)
                if !message.is_finished() && !self.drag_started =>
            {
                self.drag_started = true;
                Command::begin_drag(radiant::runtime::DragRequest::new(
                    radiant::runtime::DragPreview::sized("Folder", Vector2::new(96.0, 22.0)),
                    Point::new(40.0, 33.0),
                ))
            }
            TreeRowRuntimeMessage::Drag(message) if message.is_finished() => {
                self.drag_started = false;
                Command::end_drag()
            }
            TreeRowRuntimeMessage::Drag(_) => Command::none(),
        }
    }
}

#[test]
fn tree_row_runtime_refresh_preserves_capture_drag_semantics_and_physical_geometry() {
    let scale_one = ApplicationEnvironment::new(LocaleId::english())
        .with_text_scale(TextScale::new(1.0).expect("valid scale"));
    let scale_one_rtl = scale_one
        .clone()
        .with_writing_direction(WritingDirection::Rtl)
        .with_text_scale(TextScale::new(1.5).expect("valid scale"));
    let scale_two_rtl = scale_one
        .clone()
        .with_writing_direction(WritingDirection::Rtl)
        .with_text_scale(TextScale::new(2.0).expect("valid scale"));
    let events = Vec::new();
    let mut runtime = SurfaceRuntime::new(
        TreeRowRuntimeBridge {
            environment: scale_one,
            events,
            project_count: 0,
            drag_started: false,
        },
        Vector2::new(360.0, 44.0),
    );
    let tree_id = ui::stable_widget_id(91, "tree-row");
    let tree_bounds = runtime.layout().rects[&tree_id];
    assert_eq!(tree_bounds.height(), 22.0);
    let initial_plan = runtime.paint_plan(&ThemeTokens::default());
    fn text_for(
        plan: &radiant::runtime::SurfacePaintPlan,
        widget_id: radiant::widgets::WidgetId,
    ) -> Option<&radiant::runtime::PaintTextRun> {
        plan.primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == widget_id => Some(text),
                _ => None,
            })
    }
    let underlay_id = initial_plan
        .text_runs()
        .find(|text| text.text.as_str() == "Underlay")
        .map(|text| text.widget_id)
        .expect("actual underlay text widget");
    assert_ne!(underlay_id, tree_id, "underlay keeps a separate text owner");
    assert_eq!(
        text_for(&initial_plan, underlay_id).unwrap().font_size,
        13.0
    );
    assert_eq!(text_for(&initial_plan, tree_id).unwrap().font_size, 13.0);

    let initial_targets = runtime.automation_target_snapshot().targets;
    let initial_target = initial_targets
        .iter()
        .find(|target| target.id.0 == tree_id.to_string())
        .expect("actual TreeRow automation target");
    assert_eq!(initial_target.role, radiant::runtime::AutomationRole::Row);
    assert_eq!(initial_target.label.as_deref(), Some("Folder"));
    assert!(initial_target.selected);
    assert_eq!(initial_target.bounds.height, 22.0);
    assert_eq!(
        initial_targets
            .iter()
            .filter(|target| target.id.0 == tree_id.to_string())
            .count(),
        1,
        "TreeRow keeps one automation owner for its stable identity"
    );

    let pointer = tree_bounds.center();
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(pointer)),
        Some(tree_id)
    );
    assert_eq!(runtime.pointer_capture(), Some(tree_id));
    assert!(
        runtime
            .surface()
            .find_widget(tree_id)
            .unwrap()
            .widget()
            .common()
            .state
            .pressed
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(Point::new(
            pointer.x + 8.0,
            pointer.y + 4.0
        ))),
        Some(tree_id)
    );
    assert!(runtime.drag_session_active());
    let before_refresh = runtime.refresh_counters();
    let before_projects = runtime.bridge().project_count;
    let before_chrome: Vec<_> = runtime
        .paint_plan(&ThemeTokens::default())
        .fill_rects_for_widget(tree_id)
        .map(|fill| (fill.rect, fill.color))
        .collect();
    assert!(
        !before_chrome.is_empty(),
        "selected TreeRow chrome is painted"
    );

    runtime.bridge_mut().environment = scale_one_rtl.clone();
    runtime.refresh_with_scope(RepaintScope::PaintOnly);

    let after_refresh = runtime.refresh_counters();
    assert_eq!(
        after_refresh.runtime_projection,
        before_refresh.runtime_projection + 1
    );
    assert_eq!(after_refresh.layout, before_refresh.layout + 1);
    assert_eq!(runtime.bridge().project_count, before_projects + 1);
    assert_eq!(runtime.pointer_capture(), Some(tree_id));
    assert!(runtime.drag_session_active());
    assert!(
        runtime
            .surface()
            .find_widget(tree_id)
            .unwrap()
            .widget()
            .common()
            .state
            .pressed
    );
    assert_eq!(runtime.layout().rects[&tree_id].height(), 22.0);

    let after_plan = runtime.paint_plan(&ThemeTokens::default());
    assert_eq!(text_for(&after_plan, underlay_id).unwrap().font_size, 19.5);
    assert_eq!(text_for(&after_plan, tree_id).unwrap().font_size, 19.5);
    let after_chrome: Vec<_> = after_plan
        .fill_rects_for_widget(tree_id)
        .map(|fill| (fill.rect, fill.color))
        .collect();
    assert_eq!(after_chrome.len(), before_chrome.len());
    for ((after_rect, after_color), (before_rect, before_color)) in
        after_chrome.iter().zip(before_chrome.iter())
    {
        assert_eq!(
            *after_color, *before_color,
            "TreeRow chrome keeps its color"
        );
        assert_eq!(after_rect.min.x, 360.0 - before_rect.max.x);
        assert_eq!(after_rect.max.x, 360.0 - before_rect.min.x);
        assert_eq!(after_rect.min.y, before_rect.min.y);
        assert_eq!(after_rect.max.y, before_rect.max.y);
    }

    let refreshed_target = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.id.0 == tree_id.to_string())
        .expect("TreeRow automation target after refresh");
    assert_eq!(refreshed_target.role, radiant::runtime::AutomationRole::Row);
    assert_eq!(refreshed_target.label.as_deref(), Some("Folder"));
    assert!(refreshed_target.selected);
    assert_eq!(refreshed_target.bounds.height, 22.0);

    let release = Point::new(pointer.x + 8.0, pointer.y + 4.0);
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(release)),
        Some(tree_id)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert!(!runtime.drag_session_active());
    assert!(
        !runtime
            .surface()
            .find_widget(tree_id)
            .unwrap()
            .widget()
            .common()
            .state
            .pressed
    );
    assert_eq!(
        runtime
            .bridge()
            .events
            .iter()
            .filter(|message| matches!(
                message,
                TreeRowRuntimeMessage::Drag(DragHandleMessage::Ended { .. })
            ))
            .count(),
        1,
        "one mapped TreeRow drag terminal is released"
    );

    runtime.bridge_mut().environment = scale_two_rtl.clone();
    let before_scale_two = runtime.refresh_counters();
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    let after_scale_two = runtime.refresh_counters();
    assert_eq!(
        after_scale_two.runtime_projection,
        before_scale_two.runtime_projection + 1
    );
    assert_eq!(after_scale_two.layout, before_scale_two.layout + 1);
    assert_eq!(runtime.layout().rects[&tree_id].height(), 22.0);
    let scale_two_plan = runtime.paint_plan(&ThemeTokens::default());
    assert_eq!(text_for(&scale_two_plan, tree_id).unwrap().font_size, 26.0);

    let before_unchanged = runtime.refresh_counters();
    let before_unchanged_projects = runtime.bridge().project_count;
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    assert_eq!(runtime.refresh_counters(), before_unchanged);
    assert_eq!(runtime.bridge().project_count, before_unchanged_projects);
}
