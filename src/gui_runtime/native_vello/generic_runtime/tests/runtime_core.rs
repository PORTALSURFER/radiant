use super::*;

struct ExactPlanBridge;

impl crate::runtime::RuntimeBridge<()> for ExactPlanBridge {
    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(
            crate::runtime::SurfaceNode::widget(
                crate::widgets::TextWidget::new(
                    1,
                    "Stable",
                    crate::widgets::WidgetSizing::fixed(crate::gui::types::Vector2::new(
                        120.0, 28.0,
                    )),
                ),
                crate::runtime::WidgetMessageMapper::none(),
            ),
        ))
    }
}

struct LargeExactPlanBridge {
    surface: std::sync::Arc<crate::runtime::UiSurface<()>>,
}

impl LargeExactPlanBridge {
    fn new() -> Self {
        let rows = (0..3_000_u64)
            .map(|index| {
                crate::runtime::SurfaceChild::new(
                    crate::layout::SlotParams {
                        size_main: crate::layout::SizeModeMain::Intrinsic,
                        size_cross: crate::layout::SizeModeCross::Fill,
                        constraints: crate::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    crate::runtime::SurfaceNode::static_widget(crate::widgets::TextWidget::new(
                        10_000 + index,
                        format!("Row {index}"),
                        crate::widgets::WidgetSizing::fixed(crate::gui::types::Vector2::new(
                            180.0, 24.0,
                        )),
                    )),
                )
            })
            .collect();
        let surface = crate::runtime::UiSurface::new(crate::runtime::SurfaceNode::scroll_area(
            1,
            crate::runtime::SurfaceNode::container(
                2,
                crate::layout::ContainerPolicy {
                    kind: crate::layout::ContainerKind::Column,
                    spacing: 2.0,
                    ..crate::layout::ContainerPolicy::default()
                },
                rows,
            ),
        ));
        Self {
            surface: crate::runtime::test_arc_surface(surface),
        }
    }
}

impl crate::runtime::RuntimeBridge<()> for LargeExactPlanBridge {
    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        std::sync::Arc::clone(&self.surface)
    }
}

#[test]
fn generic_core_empty_runtime_wakeup_does_not_need_redraw() {
    let mut core = GenericNativeRuntimeCore::new(ExactPlanBridge, Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(!outcome.routed);
    assert!(!outcome.needs_redraw());
    assert!(!outcome.runtime_work_remaining);
}

#[test]
fn generic_core_keeps_paint_only_runtime_frames_off_scene_rebuild_path() {
    let mut core =
        GenericNativeRuntimeCore::new(PaintOnlyFrameBridge::default(), Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(!outcome.needs_scene_rebuild());
}

#[test]
fn generic_core_can_enable_layout_debug_before_first_frame() {
    let core = GenericNativeRuntimeCore::new_with_debug_layout(
        demo_bridge(),
        Vector2::new(320.0, 40.0),
        true,
    );

    assert_eq!(
        core.runtime.layout_debug_options(),
        LayoutDebugOptions::bounds_only()
    );
    assert!(!core.runtime.layout().debug_primitives.is_empty());
}

#[test]
fn generic_core_reuses_cached_base_paint_plan_after_exact_refresh() {
    let mut core = GenericNativeRuntimeCore::new(ExactPlanBridge, Vector2::new(320.0, 40.0));
    let mut plan = crate::runtime::SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default());

    core.paint_plan_into(&mut plan);
    let first = plan.clone();
    let first_rebuilds = core.runtime.refresh_counters().base_paint_plan_rebuilds;

    core.runtime
        .refresh_with_scope(crate::runtime::RepaintScope::Projection);
    assert!(core.runtime.base_paint_plan_reuse_eligible());
    core.paint_plan_into(&mut plan);

    assert_eq!(plan, first);
    assert_eq!(
        core.runtime.refresh_counters().base_paint_plan_rebuilds,
        first_rebuilds,
        "exact refresh should reuse the cached backend-neutral plan"
    );
}

#[test]
fn generic_core_rebuilds_cached_base_paint_plan_after_environment_veto() {
    let mut core = GenericNativeRuntimeCore::new(ExactPlanBridge, Vector2::new(320.0, 40.0));
    let mut plan = crate::runtime::SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default());
    core.paint_plan_into(&mut plan);
    let first_rebuilds = core.runtime.refresh_counters().base_paint_plan_rebuilds;

    core.runtime
        .set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            None,
            false,
            false,
        ));
    core.runtime
        .refresh_with_scope(crate::runtime::RepaintScope::Projection);
    core.paint_plan_into(&mut plan);

    assert_eq!(
        core.runtime.refresh_counters().base_paint_plan_rebuilds,
        first_rebuilds + 1,
        "environment changes must rebuild the cached plan"
    );
}

#[test]
fn generic_core_reuses_cached_base_paint_plan_for_exact_3k_cohort() {
    let mut core =
        GenericNativeRuntimeCore::new(LargeExactPlanBridge::new(), Vector2::new(960.0, 720.0));
    let mut plan = crate::runtime::SurfacePaintPlan::empty(&crate::theme::ThemeTokens::default());
    core.paint_plan_into(&mut plan);
    let first = plan.clone();
    let first_rebuilds = core.runtime.refresh_counters().base_paint_plan_rebuilds;

    for _ in 0..3 {
        core.runtime
            .refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert!(core.runtime.base_paint_plan_reuse_eligible());
        core.paint_plan_into(&mut plan);
        assert_eq!(plan, first);
    }

    assert_eq!(
        core.runtime.refresh_counters().base_paint_plan_rebuilds,
        first_rebuilds,
        "exact 3k cohort must not rebuild the cached base plan"
    );
}
