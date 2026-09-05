use super::{fixtures::*, shared::*};
use crate::application::IntoView;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

struct BareApplicationEnvironmentBridge {
    environment: Rc<RefCell<Option<crate::application::ApplicationEnvironment>>>,
    pulls: Rc<Cell<usize>>,
}

impl BareApplicationEnvironmentBridge {
    fn new(
        environment: Rc<RefCell<Option<crate::application::ApplicationEnvironment>>>,
        pulls: Rc<Cell<usize>>,
    ) -> Self {
        Self { environment, pulls }
    }
}

impl crate::runtime::RuntimeBridge<()> for BareApplicationEnvironmentBridge {
    fn application_environment(&mut self) -> Option<crate::application::ApplicationEnvironment> {
        self.environment.borrow().clone()
    }

    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        self.pulls.set(self.pulls.get().saturating_add(1));
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(
            crate::runtime::SurfaceNode::widget(
                crate::widgets::TextWidget::new(
                    1,
                    "bare",
                    crate::widgets::WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                ),
                crate::runtime::WidgetMessageMapper::none(),
            ),
        ))
    }
}

struct TextInputEnvironmentBridge {
    environment: Rc<RefCell<Option<crate::application::ApplicationEnvironment>>>,
    pulls: Rc<Cell<usize>>,
}

impl TextInputEnvironmentBridge {
    fn surface(&self) -> crate::runtime::UiSurface<()> {
        let mut input = crate::widgets::TextInputWidget::new(
            7,
            "candidate",
            crate::widgets::WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
        );
        input.common.state.focused = true;
        input.state.caret = 7;
        input.state.selection_anchor = 3;
        crate::runtime::UiSurface::new(crate::runtime::SurfaceNode::widget(
            input,
            crate::runtime::WidgetMessageMapper::none(),
        ))
    }
}

impl crate::runtime::RuntimeBridge<()> for TextInputEnvironmentBridge {
    fn application_environment(&mut self) -> Option<crate::application::ApplicationEnvironment> {
        self.environment.borrow().clone()
    }

    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        self.pulls.set(self.pulls.get().saturating_add(1));
        crate::runtime::test_arc_surface(self.surface())
    }
}

struct ReadyVirtualLayoutPolicy {
    query_count: Rc<Cell<usize>>,
}

impl crate::layout::VirtualLayoutPolicy for ReadyVirtualLayoutPolicy {
    fn query(
        &self,
        _input: &crate::layout::VirtualLayoutQueryInput,
        sink: &mut crate::layout::VirtualLayoutQuerySink,
    ) -> crate::layout::VirtualLayoutPolicyDecision {
        self.query_count
            .set(self.query_count.get().saturating_add(1));
        sink.visit(crate::layout::VirtualLayoutItemCandidate::new(
            crate::layout::VirtualLayoutItemKey::new(1),
            0,
            crate::gui::types::Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
            crate::layout::VirtualLayoutVisibility::Visible,
            crate::layout::VirtualLayoutBoundsConfidence::Exact,
        ))
        .expect("the test virtual-layout budget admits one item");
        sink.set_extent(crate::layout::VirtualLayoutExtentCandidate::exact(
            Vector2::new(100.0, 20.0),
        ))
        .expect("the test virtual-layout policy supplies one extent");
        crate::layout::VirtualLayoutPolicyDecision::Ready
    }
}

struct ReadyVirtualLayoutBridge {
    policy_queries: Rc<Cell<usize>>,
    project_count: usize,
}

impl ReadyVirtualLayoutBridge {
    fn new(policy_queries: Rc<Cell<usize>>) -> Self {
        Self {
            policy_queries,
            project_count: 0,
        }
    }
}

impl crate::runtime::RuntimeBridge<()> for ReadyVirtualLayoutBridge {
    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        self.project_count += 1;
        let view = crate::application::virtual_layout::virtual_layout_from_parts(
            crate::application::virtual_layout::VirtualLayoutParts::new(
                Rc::new(ReadyVirtualLayoutPolicy {
                    query_count: Rc::clone(&self.policy_queries),
                }),
                crate::layout::VirtualLayoutPolicyIdentity::new("timing-test"),
                crate::layout::VirtualLayoutOverscan::new(0.0, 0.0)
                    .expect("valid test virtual-layout overscan"),
                crate::layout::VirtualLayoutBudget::new(1),
                crate::runtime::VirtualLayoutRevisions::default(),
                Rc::new(|| crate::application::column(std::iter::empty())),
                Rc::new(|_| crate::application::text::<()>("item")),
                Rc::new(|_| crate::layout::VirtualLayoutPolicyIdentity::new("item")),
            ),
        );
        crate::runtime::test_arc_surface(view.into_surface())
    }

    fn update(&mut self, _message: ()) -> crate::runtime::Command<()> {
        crate::runtime::Command::none()
    }
}

fn valid_prepared_surface_refresh_native_evidence() -> PreparedSurfaceRefreshNativeEvidence {
    PreparedSurfaceRefreshNativeEvidence {
        window_id: Some(winit::window::WindowId::dummy()),
        adapter_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
        target_generation:
            super::super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        environment: crate::runtime::WindowEnvironment::default(),
        native_resources_present: true,
        target_fenced: false,
        pending_viewport_resize: false,
        pending_surface_resize: false,
        lifecycle: NativeLifecycle::default(),
        newer_visual_request: false,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PreparedScrollMessage {
    Settled(Vector2),
}

struct PreparedScrollBridge {
    project_count: usize,
    policy: crate::layout::ScrollPolicy,
    initial: Option<Vector2>,
    content: Vector2,
    controlled: Option<crate::layout::Controlled<Vector2>>,
    request: Option<crate::layout::ScrollRequest>,
    settled: Rc<std::cell::RefCell<Vec<Vector2>>>,
    observed_projects: Rc<Cell<usize>>,
}

impl PreparedScrollBridge {
    fn new(
        settled: Rc<std::cell::RefCell<Vec<Vector2>>>,
        observed_projects: Rc<Cell<usize>>,
    ) -> Self {
        Self {
            project_count: 0,
            policy: crate::layout::ScrollPolicy::default(),
            initial: None,
            content: Vector2::new(80.0, 400.0),
            controlled: None,
            request: None,
            settled,
            observed_projects,
        }
    }
}

impl crate::runtime::RuntimeBridge<PreparedScrollMessage> for PreparedScrollBridge {
    fn project_surface(
        &mut self,
    ) -> std::sync::Arc<crate::runtime::UiSurface<PreparedScrollMessage>> {
        self.project_count += 1;
        self.observed_projects.set(self.project_count);
        let policy = crate::layout::ContainerPolicy {
            kind: crate::layout::ContainerKind::ScrollView,
            overflow: crate::layout::OverflowPolicy::Scroll,
            scroll_policy: self.policy,
            initial_offset: self.initial,
            controlled_offset: self.controlled,
            scroll_request: self.request.clone(),
            ..crate::layout::ContainerPolicy::default()
        };
        let surface = crate::runtime::SurfaceNode::container(
            1,
            policy,
            vec![crate::runtime::SurfaceChild::fill(
                crate::runtime::SurfaceNode::widget(
                    crate::widgets::TextWidget::new(
                        2,
                        "prepared scroll",
                        crate::widgets::WidgetSizing::fixed(self.content),
                    ),
                    crate::runtime::WidgetMessageMapper::none(),
                ),
            )],
        )
        .on_offset_settled(PreparedScrollMessage::Settled);
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(surface))
    }

    fn update(
        &mut self,
        message: PreparedScrollMessage,
    ) -> crate::runtime::Command<PreparedScrollMessage> {
        let PreparedScrollMessage::Settled(offset) = message;
        self.settled.borrow_mut().push(offset);
        crate::runtime::Command::none()
    }
}

#[test]
fn prepared_native_refresh_commits_controlled_and_request_scroll_state_once() {
    let settled = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed_projects = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedScrollBridge::new(Rc::clone(&settled), Rc::clone(&observed_projects)),
        Vector2::new(100.0, 80.0),
    );
    assert!(runner.window.target_generation.advance());
    runner.rebuild_scene();
    let before_projects = runner.core.runtime.bridge().project_count;
    runner.core.runtime.bridge_mut().controlled =
        Some(crate::layout::Controlled::new(Vector2::new(0.0, 100.0), 1));
    runner.core.runtime.bridge_mut().request = Some(crate::layout::ScrollRequest::rect(
        crate::gui::types::Rect::from_xy_size(0.0, 300.0, 20.0, 20.0),
        crate::layout::ScrollAlignment::Start,
        2,
    ));
    runner.timing.deferred_surface_refresh = true;
    let phases = Rc::new(std::cell::RefCell::new(Vec::new()));
    let projects_at_publication = Rc::new(Cell::new(None));
    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let phases = Rc::clone(&phases);
            let projects_at_publication = Rc::clone(&projects_at_publication);
            move |phase| {
                phases.borrow_mut().push(phase);
                if phase == "published" {
                    projects_at_publication.set(Some(observed_projects.get()));
                }
            }
        }));

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        before_projects + 2
    );
    assert_eq!(
        projects_at_publication.get(),
        Some(before_projects + 1),
        "publication observes exactly one candidate bridge pull before callback dispatch"
    );
    assert_eq!(
        runner
            .core
            .runtime
            .layout()
            .rects
            .get(&2)
            .map(|rect| rect.min.y),
        Some(-300.0)
    );
    assert_eq!(&*settled.borrow(), &[Vector2::new(0.0, 300.0)]);
}

#[test]
fn prepared_native_scroll_gate_veto_preserves_active_state_before_retry() {
    let settled = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed_projects = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedScrollBridge::new(Rc::clone(&settled), Rc::clone(&observed_projects)),
        Vector2::new(100.0, 80.0),
    );
    assert!(runner.window.target_generation.advance());
    runner.rebuild_scene();
    let before_rect = runner.core.runtime.layout().rects[&2];
    let before_projects = runner.core.runtime.bridge().project_count;
    runner.core.runtime.bridge_mut().controlled =
        Some(crate::layout::Controlled::new(Vector2::new(0.0, 100.0), 1));
    runner.core.runtime.bridge_mut().request = Some(crate::layout::ScrollRequest::rect(
        crate::gui::types::Rect::from_xy_size(0.0, 300.0, 20.0, 20.0),
        crate::layout::ScrollAlignment::Start,
        2,
    ));
    runner.timing.deferred_surface_refresh = true;
    let mut stale = valid_prepared_surface_refresh_native_evidence();
    stale.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
        stale,
    );

    assert_eq!(runner.core.runtime.layout().rects[&2], before_rect);
    assert!(settled.borrow().is_empty());
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        before_projects + 1
    );

    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );
    assert_eq!(runner.core.runtime.layout().rects[&2].min.y, -300.0);
    assert_eq!(&*settled.borrow(), &[Vector2::new(0.0, 300.0)]);
}

#[test]
fn prepared_native_controlled_axes_publish_without_initial_or_request() {
    let settled = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed_projects = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedScrollBridge::new(Rc::clone(&settled), Rc::clone(&observed_projects)),
        Vector2::new(100.0, 80.0),
    );
    runner.core.runtime.bridge_mut().content = Vector2::new(400.0, 400.0);
    assert!(runner.window.target_generation.advance());
    runner.rebuild_scene();
    let before_rect = runner.core.runtime.layout().rects[&2];
    assert_eq!(before_rect.min, crate::gui::types::Point::new(0.0, 0.0));

    {
        let bridge = runner.core.runtime.bridge_mut();
        bridge.policy =
            crate::layout::ScrollPolicy::default().axes(crate::layout::ScrollAxis::Vertical);
        bridge.initial = None;
        bridge.request = None;
        bridge.controlled = Some(crate::layout::Controlled::new(
            Vector2::new(40.0, 10_000.0),
            1,
        ));
    }
    runner.timing.deferred_surface_refresh = true;
    let mut stale = valid_prepared_surface_refresh_native_evidence();
    stale.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
        stale,
    );
    assert_eq!(runner.core.runtime.layout().rects[&2], before_rect);
    assert!(settled.borrow().is_empty());

    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );
    assert_eq!(
        runner.core.runtime.layout().rects[&2].min,
        crate::gui::types::Point::new(0.0, -320.0)
    );
    assert!(settled.borrow().is_empty());

    {
        let bridge = runner.core.runtime.bridge_mut();
        bridge.policy =
            crate::layout::ScrollPolicy::default().axes(crate::layout::ScrollAxis::Horizontal);
        bridge.controlled = Some(crate::layout::Controlled::new(
            Vector2::new(10_000.0, 40.0),
            2,
        ));
        bridge.initial = None;
        bridge.request = None;
    }
    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );
    assert_eq!(
        runner.core.runtime.layout().rects[&2].min,
        crate::gui::types::Point::new(-300.0, 0.0)
    );
    assert!(settled.borrow().is_empty());
}

#[test]
fn prepared_native_veto_does_not_consume_candidate_only_reveal_generation() {
    let settled = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed_projects = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedScrollBridge::new(Rc::clone(&settled), Rc::clone(&observed_projects)),
        Vector2::new(100.0, 80.0),
    );
    assert!(runner.window.target_generation.advance());
    runner.rebuild_scene();
    let before_offset = runner.core.runtime.layout().rects[&2].min.y;
    runner.core.runtime.bridge_mut().request = Some(crate::layout::ScrollRequest::new(
        crate::layout::ScrollTarget::Keyed(crate::layout::VirtualLayoutItemKey::new(7_u32)),
        crate::layout::ScrollAlignment::Start,
        2,
    ));
    runner.timing.deferred_surface_refresh = true;
    let mut stale = valid_prepared_surface_refresh_native_evidence();
    stale.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
        stale,
    );
    assert_eq!(runner.core.runtime.layout().rects[&2].min.y, before_offset);

    runner.core.runtime.bridge_mut().request = Some(crate::layout::ScrollRequest::new(
        crate::layout::ScrollTarget::Edge(crate::layout::ScrollEdge::Bottom),
        crate::layout::ScrollAlignment::Start,
        2,
    ));
    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );
    assert_eq!(runner.core.runtime.layout().rects[&2].min.y, -320.0);
    assert_eq!(&*settled.borrow(), &[Vector2::new(0.0, 320.0)]);
}

#[test]
fn prepared_native_controlled_clamp_precedes_visible_nearest_request() {
    let settled = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed_projects = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedScrollBridge::new(Rc::clone(&settled), Rc::clone(&observed_projects)),
        Vector2::new(100.0, 80.0),
    );
    assert!(runner.window.target_generation.advance());
    runner.rebuild_scene();
    runner.core.runtime.bridge_mut().controlled = Some(crate::layout::Controlled::new(
        Vector2::new(0.0, 10_000.0),
        1,
    ));
    runner.core.runtime.bridge_mut().request = Some(crate::layout::ScrollRequest::rect(
        crate::gui::types::Rect::from_xy_size(0.0, 350.0, 20.0, 20.0),
        crate::layout::ScrollAlignment::Nearest,
        2,
    ));
    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(runner.core.runtime.layout().rects[&2].min.y, -320.0);
    assert!(settled.borrow().is_empty());
}

#[test]
fn bare_bridge_environment_promotes_deferred_paint_only_once_and_reuses_unchanged_source() {
    let initial = crate::application::ApplicationEnvironment::new(
        crate::application::LocaleId::new("fr").expect("valid test locale"),
    )
    .with_text_scale(crate::application::TextScale::new(1.1).expect("valid test text scale"));
    let changed = initial
        .clone()
        .with_writing_direction(crate::application::WritingDirection::Rtl)
        .with_text_scale(crate::application::TextScale::new(1.25).expect("valid test text scale"));
    let environment = Rc::new(RefCell::new(Some(initial.clone())));
    let pulls = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        BareApplicationEnvironmentBridge::new(Rc::clone(&environment), Rc::clone(&pulls)),
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runner.core.runtime.context().application_environment(),
        &initial
    );
    let startup_pulls = pulls.get();
    *environment.borrow_mut() = Some(changed.clone());
    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::PaintOnly);
    let before = runner.core.runtime.refresh_counters();
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(pulls.get(), startup_pulls + 1);
    assert_eq!(
        runner.core.runtime.context().application_environment(),
        &changed
    );
    let after_changed = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_changed.runtime_projection,
        before.runtime_projection + 1
    );
    assert_eq!(after_changed.layout, before.layout + 1);

    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::PaintOnly);
    let pulls_before_unchanged = pulls.get();
    let before_unchanged = runner.core.runtime.refresh_counters();
    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(pulls.get(), pulls_before_unchanged);
    assert_eq!(runner.core.runtime.refresh_counters(), before_unchanged);
}

#[test]
fn native_text_input_scale_direction_and_dpi_follow_one_runtime_paragraph_authority() {
    let scale_one_ltr =
        crate::application::ApplicationEnvironment::new(crate::application::LocaleId::english())
            .with_text_scale(crate::application::TextScale::new(1.0).expect("valid scale"));
    let scale_two_rtl = scale_one_ltr
        .clone()
        .with_writing_direction(crate::application::WritingDirection::Rtl)
        .with_text_scale(crate::application::TextScale::new(2.0).expect("valid scale"));
    let environment = Rc::new(RefCell::new(Some(scale_one_ltr.clone())));
    let pulls = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TextInputEnvironmentBridge {
            environment: Rc::clone(&environment),
            pulls: Rc::clone(&pulls),
        },
        Vector2::new(320.0, 120.0),
    );

    runner.rebuild_scene();
    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::Layout);
    let initial_native_evidence = runner.prepared_surface_refresh_native_evidence();
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        initial_native_evidence,
        initial_native_evidence,
    );
    assert_eq!(
        runner.core.runtime.context().application_environment(),
        &scale_one_ltr
    );

    let input = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            crate::runtime::PaintPrimitive::TextInput(input) => Some(input.clone()),
            _ => None,
        })
        .expect("real text-input paint primitive");
    assert_eq!(input.font_size, 13.0);
    assert_eq!(input.align, crate::runtime::PaintTextAlign::Left);
    let first_fence = runner
        .frame
        .current_text_input_snapshot_fence
        .expect("initial text-input fence");
    let first_snapshot = runner
        .frame
        .text_renderer
        .text_input_snapshot_for_input_aligned(
            input.widget_id,
            input.state.value.as_str(),
            input.font_size,
            crate::gui::paint::TextAlign::Left,
            input.rect,
            first_fence,
        )
        .expect("initial retained paragraph");
    assert_eq!(first_snapshot.font_size_bits, 13.0_f32.to_bits());
    let first_automation = runner.core.runtime.automation_snapshot();
    assert_eq!(first_automation.root.id.0, input.widget_id.to_string());
    assert_eq!(first_automation.root.value.as_deref(), Some("candidate"));
    let outer_bounds = *runner
        .core
        .runtime
        .context()
        .layout
        .rects
        .get(&input.widget_id)
        .expect("AX layout bounds");
    assert_eq!(
        first_automation.root.bounds,
        crate::gui::automation::AutomationBounds::from_rect(outer_bounds)
    );
    let first_scene_encode_count = runner.frame.scene_encode_count;
    let first_layout_counters = runner.frame.text_renderer.take_layout_profile_counters();
    let first_plan = runner.frame.last_paint_plan.clone();

    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::PaintOnly);
    let unchanged_native_evidence = runner.prepared_surface_refresh_native_evidence();
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        unchanged_native_evidence,
        unchanged_native_evidence,
    );
    assert_eq!(runner.frame.last_paint_plan, first_plan);
    assert_eq!(runner.frame.scene_encode_count, first_scene_encode_count);
    let repeated_fence = runner
        .frame
        .current_text_input_snapshot_fence
        .expect("unchanged text-input fence");
    let repeated_snapshot = runner
        .frame
        .text_renderer
        .text_input_snapshot_for_input_aligned(
            input.widget_id,
            input.state.value.as_str(),
            input.font_size,
            crate::gui::paint::TextAlign::Left,
            input.rect,
            repeated_fence,
        )
        .expect("unchanged retained paragraph");
    assert!(std::sync::Arc::ptr_eq(&first_snapshot, &repeated_snapshot));
    let unchanged_counters = runner.frame.text_renderer.take_layout_profile_counters();
    assert_eq!(unchanged_counters.shape.misses, 0);
    assert_eq!(unchanged_counters.view.misses, 0);
    assert!(unchanged_counters.shape.hits > 0);
    assert!(unchanged_counters.view.hits > 0);
    assert_eq!(
        runner.core.runtime.automation_snapshot(),
        first_automation,
        "unchanged environment keeps AX bounds and value stable"
    );
    assert_eq!(first_layout_counters.shape.misses, 1);

    *environment.borrow_mut() = Some(scale_two_rtl.clone());
    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::PaintOnly);
    let changed_native_evidence = runner.prepared_surface_refresh_native_evidence();
    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        changed_native_evidence,
        changed_native_evidence,
    );
    assert_eq!(
        runner.core.runtime.context().application_environment(),
        &scale_two_rtl
    );
    let scaled_input = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            crate::runtime::PaintPrimitive::TextInput(input) => Some(input.clone()),
            _ => None,
        })
        .expect("scaled text-input paint primitive");
    assert_ne!(runner.frame.last_paint_plan, first_plan);
    assert_eq!(scaled_input.font_size, 26.0);
    assert_eq!(scaled_input.align, crate::runtime::PaintTextAlign::Right);
    let scaled_counters = runner.frame.text_renderer.take_layout_profile_counters();
    assert!(scaled_counters.shape.misses > 0);
    assert!(scaled_counters.view.misses > 0);

    let scaled_fence = runner
        .frame
        .current_text_input_snapshot_fence
        .expect("scaled text-input fence");
    assert_ne!(scaled_fence, first_fence);
    assert!(
        runner
            .frame
            .text_renderer
            .text_input_snapshot_for_input_aligned(
                input.widget_id,
                input.state.value.as_str(),
                input.font_size,
                crate::gui::paint::TextAlign::Left,
                input.rect,
                first_fence,
            )
            .is_none()
    );
    let scaled_snapshot = runner
        .frame
        .text_renderer
        .text_input_snapshot_for_input_aligned(
            scaled_input.widget_id,
            scaled_input.state.value.as_str(),
            scaled_input.font_size,
            crate::gui::paint::TextAlign::Right,
            scaled_input.rect,
            scaled_fence,
        )
        .expect("scaled retained paragraph");
    assert_eq!(scaled_snapshot.font_size_bits, 26.0_f32.to_bits());
    assert!(!std::sync::Arc::ptr_eq(&first_snapshot, &scaled_snapshot));
    let pointer_position =
        crate::gui::types::Point::new(scaled_input.rect.max.x - 1.0, scaled_input.rect.center().y);
    let expected_pointer =
        crate::gui_runtime::native_vello::generic_runtime::scene::text_input_pointer_target_from_snapshot(
            &scaled_input,
            pointer_position,
            scaled_snapshot.clone(),
        )
        .expect("production pointer projection from the scaled paragraph");
    let scaled_ime = runner
        .frame
        .native_ime_cursor_area()
        .expect("scaled IME caret area");
    let scaled_hit = runner
        .frame
        .native_text_pointer_target(pointer_position, None)
        .expect("scaled pointer hit from retained paragraph");
    assert_eq!(scaled_hit.0, scaled_input.widget_id);
    assert_eq!(scaled_hit.1, scaled_input.state.value);
    assert_eq!(scaled_hit.2, expected_pointer.0);
    assert_eq!(scaled_hit.3, expected_pointer.1);

    let byte_at_scalar = |scalar: usize| {
        scaled_input
            .state
            .value
            .char_indices()
            .nth(scalar)
            .map_or(scaled_input.state.value.len(), |(byte, _)| byte)
    };
    let start_byte = byte_at_scalar(
        scaled_input
            .state
            .selection_anchor
            .min(scaled_input.state.caret),
    );
    let caret_byte = byte_at_scalar(scaled_input.state.caret);
    let end_byte = byte_at_scalar(
        scaled_input
            .state
            .selection_anchor
            .max(scaled_input.state.caret)
            .saturating_add(1)
            .min(scaled_input.state.value.chars().count()),
    );
    let mut editor =
        crate::gui_runtime::native_vello::text_edit::SingleLineTextEditorState::collapsed_at_end(
            scaled_input.state.value.as_str(),
        );
    editor.set_cursor(scaled_input.state.value.as_str(), start_byte, false);
    editor.set_cursor(scaled_input.state.value.as_str(), end_byte, true);
    let field_layout =
        crate::gui_runtime::native_vello::text_edit::build_text_field_layout_from_snapshot(
            scaled_snapshot.clone(),
            &mut editor,
            scaled_input.state.value.as_str(),
            scaled_input.font_size,
            scaled_input.rect.width(),
        );
    assert!(std::sync::Arc::ptr_eq(
        &field_layout.snapshot,
        &scaled_snapshot
    ));
    assert!(!field_layout.selection_rects().is_empty());
    let expected_selection = scaled_snapshot
        .selection_rects(start_byte, end_byte)
        .into_iter()
        .map(|(start, end)| {
            (
                (start - field_layout.scroll_x).clamp(0.0, scaled_input.rect.width()),
                (end - field_layout.scroll_x).clamp(0.0, scaled_input.rect.width()),
            )
        })
        .filter(|(start, end)| end > start)
        .collect::<Vec<_>>();
    assert_eq!(
        field_layout.selection_rects(),
        expected_selection.as_slice()
    );
    let expected_caret_offset = (scaled_snapshot.caret_x(
        caret_byte,
        crate::gui_runtime::native_vello::CaretAffinity::Downstream,
    ) - field_layout.scroll_x)
        .clamp(0.0, scaled_input.rect.width());
    assert_eq!(
        scaled_ime.min.x,
        scaled_input.rect.min.x + expected_caret_offset
    );
    assert!(scaled_ime.min.x >= scaled_input.rect.min.x);
    assert!(scaled_ime.max.x <= scaled_input.rect.max.x);
    let scaled_automation = runner.core.runtime.automation_snapshot();
    assert_eq!(scaled_automation.root.value.as_deref(), Some("candidate"));
    assert_eq!(
        scaled_automation.root.bounds,
        crate::gui::automation::AutomationBounds::from_rect(outer_bounds)
    );

    runner.rebuild_scene();
    assert!(runner.frame.scene_encode_count > first_scene_encode_count);
    assert_ne!(
        runner.frame.scene_build_outcome,
        super::super::super::frame_state::NativeSceneBuildOutcome::WholeSceneReuse
    );
    let encoded_fence = runner
        .frame
        .current_text_input_snapshot_fence
        .expect("encoded text-input fence");

    let logical_transforms = runner.frame.scene.encoding().transforms.clone();
    let logical_snapshot_before_dpi = runner
        .frame
        .text_renderer
        .text_input_snapshot_for_input_aligned(
            scaled_input.widget_id,
            scaled_input.state.value.as_str(),
            scaled_input.font_size,
            crate::gui::paint::TextAlign::Right,
            scaled_input.rect,
            encoded_fence,
        )
        .expect("logical paragraph before DPI projection");
    runner.update_native_dpi_scale(1.5);
    assert!(runner.timing.deferred_scene_rebuild);
    let scaled_scene_transforms = runner
        .frame
        .scene_for_dpi_scale(crate::theme::DpiScale::new(1.5))
        .encoding()
        .transforms
        .clone();
    assert_eq!(runner.frame.scene.encoding().transforms, logical_transforms);
    assert_eq!(
        scaled_scene_transforms
            .last()
            .map(|transform| transform.to_kurbo()),
        Some(vello::kurbo::Affine::scale(1.5)),
        "native scene must apply one physical DPI transform"
    );
    let dpi_counters = runner.frame.text_renderer.take_layout_profile_counters();
    assert_eq!(dpi_counters.shape.misses, 0);
    assert_eq!(dpi_counters.view.misses, 0);
    assert!(dpi_counters.shape.hits > 0);
    assert!(dpi_counters.view.hits > 0);
    let logical_snapshot_after_dpi = runner
        .frame
        .text_renderer
        .text_input_snapshot_for_input_aligned(
            scaled_input.widget_id,
            scaled_input.state.value.as_str(),
            scaled_input.font_size,
            crate::gui::paint::TextAlign::Right,
            scaled_input.rect,
            encoded_fence,
        )
        .expect("logical paragraph after DPI projection");
    assert!(std::sync::Arc::ptr_eq(
        &logical_snapshot_before_dpi,
        &logical_snapshot_after_dpi
    ));
    assert_eq!(pulls.get(), 3, "initial, unchanged, and changed refreshes");
}

#[test]
fn native_candidate_source_change_vetoes_without_replay_or_active_mutation() {
    let initial = crate::application::ApplicationEnvironment::new(
        crate::application::LocaleId::new("fr").expect("valid test locale"),
    )
    .with_text_scale(crate::application::TextScale::new(1.1).expect("valid test text scale"));
    let changed = initial
        .clone()
        .with_writing_direction(crate::application::WritingDirection::Rtl)
        .with_text_scale(crate::application::TextScale::new(1.25).expect("valid test text scale"));
    let environment = Rc::new(RefCell::new(Some(initial.clone())));
    let pulls = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        BareApplicationEnvironmentBridge::new(Rc::clone(&environment), Rc::clone(&pulls)),
        Vector2::new(120.0, 40.0),
    );
    let startup_pulls = pulls.get();
    let before_counters = runner.core.runtime.refresh_counters();
    let before_application_environment = runner
        .core
        .runtime
        .surface()
        .application_environment()
        .clone();
    let before_window_environment = runner.core.runtime.context().window_environment();
    let before_plan = runner.frame.last_paint_plan.clone();
    *environment.borrow_mut() = Some(changed);
    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let environment = Rc::clone(&environment);
            move |phase| {
                if phase == "candidate-held" {
                    *environment.borrow_mut() = None;
                }
            }
        }));
    runner.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::PaintOnly);

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(pulls.get(), startup_pulls + 1);
    assert_eq!(runner.core.runtime.refresh_counters(), before_counters);
    assert_eq!(
        runner.core.runtime.surface().application_environment(),
        &before_application_environment
    );
    assert_eq!(
        runner.core.runtime.context().window_environment(),
        before_window_environment
    );
    assert_eq!(runner.frame.last_paint_plan, before_plan);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn transient_overlay_hint_skips_empty_app_overlay_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
}

#[test]
fn empty_overlay_paint_skips_app_and_runtime_overlay_callbacks() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
    assert!(runner.frame.transient_overlay_primitives.is_empty());
}

#[test]
fn explicit_transient_overlay_capability_runs_custom_bridge_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        OptInTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 1);
}

#[test]
fn exact_scene_refresh_reuses_encoded_scene_and_preserves_derived_state() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    let initial_stats = runner.frame.last_scene_stats;
    runner.frame.post_gpu_overlay_suffix_start = Some(7);
    runner.frame.post_gpu_overlay_has_replayable_suffix = true;
    runner.frame.scene_texture_dirty = false;

    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();
    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 1);
    assert_eq!(runner.frame.last_scene_stats, initial_stats);
    assert_eq!(runner.frame.post_gpu_overlay_suffix_start, Some(7));
    assert!(runner.frame.post_gpu_overlay_has_replayable_suffix);
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(runner.core.runtime.bridge().paint_calls, 1);
}

#[test]
fn ordinary_retained_surface_rebuild_does_not_clone_populated_cache() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RetainedSurfaceBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    assert_eq!(runner.core.runtime.bridge().render_count, 1);
    assert_eq!(runner.frame.retained_surface_cache.entry_count(), 1);
    assert_eq!(runner.frame.last_scene_stats.cache_hits, 0);

    RetainedSurfaceFrameCache::reset_test_clone_count();
    runner.rebuild_scene();

    assert_eq!(runner.core.runtime.bridge().render_count, 1);
    assert_eq!(runner.frame.last_scene_stats.cache_hits, 1);
    assert_eq!(RetainedSurfaceFrameCache::test_clone_count(), 0);
}

#[test]
fn prepared_plan_admission_encodes_once_without_a_second_plan_build() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());

    runner.rebuild_scene();
    let before_refresh = runner.core.runtime.refresh_counters();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("prepared refresh candidate");
    let terminal_messages = runner
        .core
        .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared);
    assert!(terminal_messages.is_some());
    let after_plan = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_plan.base_paint_plan_rebuilds,
        before_refresh.base_paint_plan_rebuilds + 1
    );
    runner.frame.scene_texture_dirty = false;

    runner.complete_prepared_surface_refresh(terminal_messages.unwrap());

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .base_paint_plan_rebuilds,
        after_plan.base_paint_plan_rebuilds
    );
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(
        runner.frame.test_phase_trace(),
        [
            Some(super::super::super::frame_state::NativeVelloTestPhase::EligibilityObserved),
            Some(super::super::super::frame_state::NativeVelloTestPhase::SceneEncode),
        ]
    );
}

#[test]
fn full_prepared_refresh_accepts_a_legitimate_appearance_change() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(recorder),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let previous = runner.core.resolved_appearance();
    runner
        .core
        .set_test_appearance_policy(crate::theme::AppearancePolicy::fixed(
            crate::theme::ThemeTokens::light(),
        ));
    runner
        .core
        .runtime
        .refresh_with_scope(crate::runtime::RepaintScope::Projection);

    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("full candidate with changed appearance");
    assert!(matches!(
        &prepared,
        crate::runtime::PreparedSurfaceRefresh::Full { .. }
    ));
    let terminal_messages = runner
        .core
        .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared)
        .expect("changed appearance remains publishable");
    assert!(terminal_messages.is_empty());
    assert_ne!(runner.core.resolved_appearance(), previous);
}

#[test]
fn interaction_candidate_is_vetoed_when_appearance_drifts_before_publish() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactInteractionBridge {
            revision: false,
            exact: true,
            project_count: 0,
            pull_update_count: 0,
            drop_probe: None,
            application_environment: None,
            surface_application_environment: None,
        },
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner.core.runtime.bridge_mut().revision = true;
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("interaction candidate");
    assert!(matches!(
        &prepared,
        crate::runtime::PreparedSurfaceRefresh::Interaction { .. }
    ));
    runner
        .core
        .set_test_appearance_policy(crate::theme::AppearancePolicy::fixed(
            crate::theme::ThemeTokens::light(),
        ));
    runner
        .core
        .set_test_resolved_appearance(crate::theme::ResolvedAppearance::fixed(
            crate::theme::ThemeTokens::light(),
        ));

    assert!(
        runner
            .core
            .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared)
            .is_none()
    );
    assert_eq!(runner.core.runtime.bridge().pull_update_count, 1);
}

#[test]
fn prepared_refresh_veto_keeps_the_combined_refresh_fallback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let project_count = runner.core.runtime.bridge().project_count;
    runner.timing.deferred_surface_refresh = true;

    // Startup has no native adapter/window/resource evidence, so Projection
    // admission must veto before the prepared transaction and use the
    // existing combined refresh path.
    runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn stale_native_evidence_drops_held_candidate_without_publication_or_replay() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    let native_evidence = valid_prepared_surface_refresh_native_evidence();
    let mut stale_native_evidence = native_evidence;
    stale_native_evidence.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        native_evidence,
        stale_native_evidence,
    );

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1,
        "candidate preparation pulls once and must not replay combined projection"
    );
    let after_refresh = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_refresh.application_projection, before_refresh.application_projection,
        "stale native evidence must not publish the candidate transaction"
    );
    assert_eq!(
        after_refresh.runtime_projection,
        before_refresh.runtime_projection,
    );
    assert_eq!(
        after_refresh.widget_state_sync,
        before_refresh.widget_state_sync
    );
    assert_eq!(after_refresh.layout, before_refresh.layout);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn native_interaction_publication_reuses_plan_and_skips_full_runtime_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactInteractionBridge {
            revision: false,
            exact: true,
            project_count: 0,
            pull_update_count: 0,
            drop_probe: None,
            application_environment: None,
            surface_application_environment: None,
        },
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let before_plan = runner.frame.last_paint_plan.clone();
    let before_automation = runner.core.runtime.automation_snapshot();
    let before_sibling = runner
        .core
        .runtime
        .surface()
        .find_widget(20)
        .expect("unchanged sibling")
        .widget() as *const dyn crate::widgets::Widget as *const ();
    let before = runner.core.runtime.refresh_counters();
    let before_generation = runner.core.runtime.fresh_surface_active_generation();
    let before_pulls = runner.core.runtime.bridge().pull_update_count;
    runner.core.runtime.bridge_mut().revision = true;
    runner.timing.deferred_surface_refresh = true;
    runner.timing.deferred_surface_refresh_scope = Some(crate::runtime::RepaintScope::Projection);

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    let after = runner.core.runtime.refresh_counters();
    assert_eq!(
        runner.core.runtime.bridge().pull_update_count,
        before_pulls + 1
    );
    assert_eq!(after.runtime_projection, before.runtime_projection);
    assert_eq!(after.widget_state_sync, before.widget_state_sync);
    assert_eq!(after.layout, before.layout);
    assert_eq!(
        after.base_paint_plan_rebuilds,
        before.base_paint_plan_rebuilds
    );
    assert_eq!(runner.frame.last_paint_plan, before_plan);
    let after_sibling = runner
        .core
        .runtime
        .surface()
        .find_widget(20)
        .expect("unchanged sibling")
        .widget() as *const dyn crate::widgets::Widget as *const ();
    assert_eq!(after_sibling, before_sibling);
    assert_ne!(runner.core.runtime.automation_snapshot(), before_automation);
    assert_eq!(
        runner.core.runtime.fresh_surface_active_generation(),
        before_generation + 1
    );
    assert_eq!(
        runner
            .core
            .runtime
            .surface()
            .find_widget(10)
            .unwrap()
            .revision(),
        crate::widgets::WidgetRevision::exact((), (), (), true)
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn native_sampled_application_environment_changes_match_forced_full_twins() {
    let key = crate::application::TextKey::new("save", "Save");
    let old_catalog = crate::application::TextCatalog::default()
        .with_generation(7)
        .insert(crate::application::LocaleId::english(), key.clone(), "Save");
    let new_catalog = crate::application::TextCatalog::default()
        .with_generation(7)
        .insert(
            crate::application::LocaleId::english(),
            key.clone(),
            "Store",
        );
    let old_environment =
        crate::application::ApplicationEnvironment::default().with_catalog(Arc::new(old_catalog));
    let cases = [
        (
            "locale",
            crate::application::ApplicationEnvironment::new(
                crate::application::LocaleId::new("fr").expect("valid locale"),
            ),
            "Save",
        ),
        (
            "same locale and catalog generation",
            crate::application::ApplicationEnvironment::default()
                .with_catalog(Arc::new(new_catalog)),
            "Store",
        ),
    ];

    for (name, new_environment, expected_localized) in cases {
        let make_runner = |exact| {
            let mut runner = GenericNativeVelloRunner::new(
                NativeRunOptions::default(),
                ExactInteractionBridge {
                    revision: false,
                    exact,
                    project_count: 0,
                    pull_update_count: 0,
                    drop_probe: None,
                    application_environment: Some(old_environment.clone()),
                    surface_application_environment: Some(old_environment.clone()),
                },
                Vector2::new(120.0, 40.0),
            );
            runner.rebuild_scene();
            runner.core.runtime.bridge_mut().revision = true;
            runner.core.runtime.bridge_mut().application_environment =
                Some(new_environment.clone());
            runner.timing.deferred_surface_refresh = true;
            runner.timing.deferred_surface_refresh_scope =
                Some(crate::runtime::RepaintScope::Projection);
            runner
        };
        let mut exact = make_runner(true);
        let mut full = make_runner(false);
        let exact_before = exact.core.runtime.refresh_counters();
        let full_before = full.core.runtime.refresh_counters();

        exact.refresh_deferred_surface_if_needed_for_test(
            &mut RenderFrameProfile::default(),
            valid_prepared_surface_refresh_native_evidence(),
        );
        full.refresh_deferred_surface_if_needed_for_test(
            &mut RenderFrameProfile::default(),
            valid_prepared_surface_refresh_native_evidence(),
        );

        assert_eq!(
            exact.core.runtime.bridge().pull_update_count,
            1,
            "{name} exact should pull once"
        );
        assert_eq!(
            full.core.runtime.bridge().pull_update_count,
            1,
            "{name} full should pull once"
        );
        assert_eq!(
            exact.core.runtime.refresh_counters(),
            full.core.runtime.refresh_counters(),
            "{name} counters"
        );
        assert_eq!(
            exact.frame.last_paint_plan, full.frame.last_paint_plan,
            "{name} paint"
        );
        assert_eq!(
            exact.core.runtime.automation_snapshot(),
            full.core.runtime.automation_snapshot(),
            "{name} automation"
        );
        assert_eq!(
            exact.core.runtime.context().application_environment(),
            &new_environment,
            "{name} installed environment"
        );
        assert_eq!(
            exact
                .core
                .runtime
                .context()
                .application_environment()
                .localized(&key)
                .as_str(),
            expected_localized,
            "{name} localized value"
        );
        assert_eq!(
            exact.core.runtime.refresh_counters().runtime_projection,
            exact_before.runtime_projection + 1
        );
        assert_eq!(
            full.core.runtime.refresh_counters().runtime_projection,
            full_before.runtime_projection + 1
        );
        assert!(!exact.frame_stage_owner.has_in_flight());
        assert!(!full.frame_stage_owner.has_in_flight());
    }
}

#[test]
fn native_unchanged_some_environment_keeps_exact_fast_path() {
    let environment = crate::application::ApplicationEnvironment::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactInteractionBridge {
            revision: false,
            exact: true,
            project_count: 0,
            pull_update_count: 0,
            drop_probe: None,
            application_environment: Some(environment.clone()),
            surface_application_environment: Some(environment),
        },
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let before = runner.core.runtime.refresh_counters();
    let before_plan = runner.frame.last_paint_plan.clone();
    runner.core.runtime.bridge_mut().revision = true;
    runner.timing.deferred_surface_refresh = true;
    runner.timing.deferred_surface_refresh_scope = Some(crate::runtime::RepaintScope::Projection);

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    let after = runner.core.runtime.refresh_counters();
    assert_eq!(runner.core.runtime.bridge().pull_update_count, 1);
    assert_eq!(after.runtime_projection, before.runtime_projection);
    assert_eq!(after.layout, before.layout);
    assert_eq!(after.widget_state_sync, before.widget_state_sync);
    assert_eq!(
        after.base_paint_plan_rebuilds,
        before.base_paint_plan_rebuilds
    );
    assert_eq!(runner.frame.last_paint_plan, before_plan);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn retired_interaction_candidate_drops_after_native_publication_diagnostics() {
    let published = Rc::new(Cell::new(false));
    let dropped = Rc::new(Cell::new(0));
    let premature = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactInteractionBridge {
            revision: false,
            exact: true,
            project_count: 0,
            pull_update_count: 0,
            drop_probe: Some((
                Rc::clone(&published),
                Rc::clone(&dropped),
                Rc::clone(&premature),
            )),
            application_environment: None,
            surface_application_environment: None,
        },
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let published = Rc::clone(&published);
            move |phase| {
                if phase == "interaction-published" {
                    published.set(true);
                }
            }
        }));
    runner.core.runtime.bridge_mut().revision = true;
    runner.timing.deferred_surface_refresh = true;
    runner.timing.deferred_surface_refresh_scope = Some(crate::runtime::RepaintScope::Projection);

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert!(
        published.get(),
        "native diagnostics observer ran before retirement"
    );
    assert!(
        dropped.get() > 0,
        "candidate retirement should release displaced leaves"
    );
    assert_eq!(
        premature.get(),
        0,
        "candidate leaves must not drop before publication"
    );
    assert!(runner.core.interaction_refresh_applied());
}

#[test]
fn native_stale_interaction_gate_discards_without_replay() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactInteractionBridge {
            revision: true,
            exact: true,
            project_count: 0,
            pull_update_count: 0,
            drop_probe: None,
            application_environment: None,
            surface_application_environment: None,
        },
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let before = runner.core.runtime.refresh_counters();
    let before_plan = runner.frame.last_paint_plan.clone();
    let before_pulls = runner.core.runtime.bridge().pull_update_count;
    let native_evidence = valid_prepared_surface_refresh_native_evidence();
    let mut stale = native_evidence;
    stale.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.timing.deferred_surface_refresh = true;
    runner.timing.deferred_surface_refresh_scope = Some(crate::runtime::RepaintScope::Projection);

    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        native_evidence,
        stale,
    );

    assert_eq!(
        runner.core.runtime.bridge().pull_update_count,
        before_pulls + 1
    );
    assert_eq!(runner.core.runtime.refresh_counters(), before);
    assert_eq!(runner.frame.last_paint_plan, before_plan);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn post_projection_layout_veto_discards_without_running_active_refresh_tail() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;
    let before_plan = runner.frame.last_paint_plan.clone();
    let before_scene_stats = runner.frame.last_scene_stats;
    let before_scene_encode_count = runner.frame.scene_encode_count;
    let before_scene_reuse_count = runner.frame.scene_reuse_count;
    let before_scene_texture_dirty = runner.frame.scene_texture_dirty;
    let before_composited_base_dirty = runner.frame.composited_base_dirty;
    let before_refresh = runner.core.runtime.refresh_counters();
    let before_frame_work = runner.timing.pending_frame_work;
    let before_project_count = runner.core.runtime.bridge().project_count;
    let automation_export_path = std::env::temp_dir().join(format!(
        "radiant_post_projection_layout_veto_{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&automation_export_path);
    runner.automation_targets =
        NativeAutomationTargetExporter::new(Some(automation_export_path.clone()), false);

    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let recorder = Rc::clone(&recorder);
            move |phase| {
                let event = match phase {
                    "projection-admitted" => PreparedRefreshEvent::ProjectionAdmitted,
                    "projection-complete" => PreparedRefreshEvent::ProjectionCompleted,
                    _ => panic!("unexpected prepared refresh phase: {phase}"),
                };
                recorder.borrow_mut().push(event);
            }
        }));
    runner.core.runtime.bridge_mut().root_id = 102;
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        before_project_count + 1,
        "the held candidate may pull once, but a post-Projection veto must not replay projection"
    );
    assert_eq!(runner.frame.last_paint_plan, before_plan);
    assert_eq!(runner.frame.last_scene_stats, before_scene_stats);
    assert_eq!(runner.frame.scene_encode_count, before_scene_encode_count);
    assert_eq!(runner.frame.scene_reuse_count, before_scene_reuse_count);
    assert_eq!(runner.frame.scene_texture_dirty, before_scene_texture_dirty);
    assert_eq!(
        runner.frame.composited_base_dirty,
        before_composited_base_dirty
    );
    assert_eq!(runner.core.runtime.refresh_counters(), before_refresh);
    assert_eq!(runner.timing.pending_frame_work, before_frame_work);
    assert!(!runner.timing.deferred_surface_refresh);
    assert_eq!(
        prepared_refresh_events(&recorder),
        vec![
            PreparedRefreshEvent::ProjectionAdmitted,
            PreparedRefreshEvent::ProjectionCompleted,
        ],
        "a Layout veto must not publish scene or terminal work"
    );
    assert!(runner.automation_targets.path().is_some());
    assert!(!automation_export_path.exists());
    assert!(!runner.frame_stage_owner.has_in_flight());

    let _ = std::fs::remove_file(automation_export_path);
}

#[test]
fn active_virtual_layout_vetoes_prepared_admission_and_materializes_combined_refresh() {
    let policy_queries = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ReadyVirtualLayoutBridge::new(Rc::clone(&policy_queries)),
        Vector2::new(240.0, 80.0),
    );
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    let before_project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    assert!(!runner.core.runtime.prepared_surface_refresh_is_eligible());
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    let after_refresh = runner.core.runtime.refresh_counters();
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        before_project_count + 1,
        "active virtual content must use the combined projection refresh"
    );
    assert_eq!(
        after_refresh.application_projection,
        before_refresh.application_projection + 1,
    );
    assert!(
        after_refresh.runtime_projection > before_refresh.runtime_projection,
        "combined virtual refresh must perform the runtime projection"
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn admitted_gpu_candidate_does_not_replay_combined_projection() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        UnsupportedPreparedRefreshBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    // Candidate preparation pulls once. A post-admission fallback would pull
    // and project a second time through the combined refresh path.
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1
    );
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .application_projection,
        before_refresh.application_projection + 1
    );
    assert!(
        runner
            .frame
            .last_paint_plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn prepared_refresh_dispatches_replacement_terminal_after_scene_admission() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner.frame.set_test_scene_encode_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_encode(&recorder)
    }));
    runner.frame.set_test_scene_admission_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_admission(&recorder)
    }));
    let before_refresh = runner.core.runtime.refresh_counters();
    runner.core.runtime.bridge_mut().replace = true;
    // The native owner needs a real window/device resource bundle; exercise
    // the same prepared transaction directly and keep completion on the
    // production ordering helper below.
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("prepared refresh candidate");
    let terminal_messages = runner
        .core
        .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared);
    let terminal_messages = terminal_messages.expect("prepared replacement terminal messages");
    assert_eq!(terminal_messages.len(), 1);

    let after_plan = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_plan.base_paint_plan_rebuilds,
        before_refresh.base_paint_plan_rebuilds + 1
    );
    runner.frame.scene_texture_dirty = false;
    runner.complete_prepared_surface_refresh(terminal_messages);

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
    assert_eq!(
        prepared_refresh_events(&recorder),
        vec![
            PreparedRefreshEvent::SceneEncode,
            PreparedRefreshEvent::SceneAdmitted,
            PreparedRefreshEvent::TerminalUpdate(PreparedRefreshTerminalMessage),
        ]
    );
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .base_paint_plan_rebuilds,
        after_plan.base_paint_plan_rebuilds
    );
}

#[test]
fn prepared_refresh_orders_projection_candidate_layout_publication_scene_and_terminal() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let recorder = Rc::clone(&recorder);
            move |phase| {
                let event = match phase {
                    "projection-admitted" => PreparedRefreshEvent::ProjectionAdmitted,
                    "candidate-held" => PreparedRefreshEvent::CandidateHeld,
                    "projection-complete" => PreparedRefreshEvent::ProjectionCompleted,
                    "layout-admitted" => PreparedRefreshEvent::LayoutAdmitted,
                    "layout-complete" => PreparedRefreshEvent::LayoutCompleted,
                    "paint-plan-admitted" => PreparedRefreshEvent::PaintPlanAdmitted,
                    "published" => PreparedRefreshEvent::Published,
                    "paint-plan-complete" => PreparedRefreshEvent::PaintPlanCompleted,
                    _ => panic!("unexpected prepared refresh phase: {phase}"),
                };
                recorder.borrow_mut().push(event);
            }
        }));
    runner.frame.set_test_scene_encode_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_encode(&recorder)
    }));
    runner.frame.set_test_scene_admission_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_admission(&recorder)
    }));
    runner.core.runtime.bridge_mut().replace = true;
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(
        prepared_refresh_events(&recorder),
        vec![
            PreparedRefreshEvent::ProjectionAdmitted,
            PreparedRefreshEvent::CandidateHeld,
            PreparedRefreshEvent::ProjectionCompleted,
            PreparedRefreshEvent::LayoutAdmitted,
            PreparedRefreshEvent::LayoutCompleted,
            PreparedRefreshEvent::PaintPlanAdmitted,
            PreparedRefreshEvent::Published,
            PreparedRefreshEvent::PaintPlanCompleted,
            PreparedRefreshEvent::SceneEncode,
            PreparedRefreshEvent::SceneAdmitted,
            PreparedRefreshEvent::TerminalUpdate(PreparedRefreshTerminalMessage),
        ]
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn prepared_refresh_vetoes_source_some_to_none_and_catalog_replacement_before_publication() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );
    let before = runner.core.runtime.refresh_counters();
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("source snapshot should prepare");
    runner.core.runtime.bridge_mut().application_environment = None;
    assert!(
        runner
            .core
            .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared)
            .is_none()
    );
    assert_eq!(runner.core.runtime.refresh_counters(), before);

    runner.core.runtime.bridge_mut().application_environment = Some(
        crate::application::ApplicationEnvironment::default().with_catalog(std::sync::Arc::new(
            crate::application::TextCatalog::default()
                .with_generation(7)
                .insert(
                    crate::application::LocaleId::english(),
                    crate::application::TextKey::new("prepared", "Prepared"),
                    "Changed",
                ),
        )),
    );
    let before_catalog = runner.core.runtime.refresh_counters();
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("catalog source snapshot should prepare");
    runner.core.runtime.bridge_mut().application_environment = Some(
        crate::application::ApplicationEnvironment::default().with_catalog(std::sync::Arc::new(
            crate::application::TextCatalog::default()
                .with_generation(7)
                .insert(
                    crate::application::LocaleId::english(),
                    crate::application::TextKey::new("prepared", "Prepared"),
                    "Replaced",
                ),
        )),
    );
    assert!(
        runner
            .core
            .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared)
            .is_none()
    );
    assert_eq!(runner.core.runtime.refresh_counters(), before_catalog);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn eligibility_observation_precedes_encode_without_changing_scene_counters() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());

    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 0);

    runner.rebuild_scene();
    assert_eq!(
        runner.frame.last_native_paint_segment_eligibility.outcome,
        super::super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::FullSceneFallback(
            super::super::super::retained_paint_segments::NativePaintSegmentFallbackReason::PaintConservative,
        )
    );
    assert_eq!(
        runner.frame.last_native_paint_segment_eligibility.entries,
        [None; crate::runtime::MAX_PAINT_SEGMENTS]
    );
    assert_eq!(
        runner
            .frame
            .last_native_paint_segment_eligibility
            .entry_count,
        0
    );
    assert_eq!(
        runner.frame.test_phase_trace(),
        [
            Some(super::super::super::frame_state::NativeVelloTestPhase::EligibilityObserved),
            Some(super::super::super::frame_state::NativeVelloTestPhase::SceneEncode),
        ]
    );
    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn environment_change_vetoes_exact_scene_reuse_and_reencodes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        OptInTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .runtime
        .set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            Some(crate::runtime::WindowColorScheme::Light),
            false,
            false,
        ));
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn exact_3k_runner_refresh_cohort_reuses_scene_encoding() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        LargeExactBridge,
        Vector2::new(960.0, 720.0),
    );

    runner.rebuild_scene();
    for _ in 0..3 {
        runner
            .core
            .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
        runner.rebuild_scene_after_surface_refresh();
    }

    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 3);
    assert!(runner.frame.last_scene_stats.paint_plan_primitives > 0);
}

#[test]
fn invalidated_native_target_vetoes_exact_scene_reuse() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner.frame.invalidate_native_scene_context();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn standalone_scene_rebuild_without_exact_refresh_reencodes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();
    runner.rebuild_scene();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 1);
}

#[test]
fn minimal_bridge_skips_frame_diagnostics_callback_work() {
    let core = GenericNativeRuntimeCore::new(NoFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(!core.has_frame_diagnostics_observer());
}

#[test]
fn explicit_frame_diagnostics_capability_enables_callback_work() {
    let core =
        GenericNativeRuntimeCore::new(OptInFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(core.has_frame_diagnostics_observer());
}
