use super::{
    super::{
        FrameWork, FrameWorkReason, GenericNativeRuntimeCore, GenericNativeVelloRunner,
        RenderFrameProfile, SceneRebuildMode, demo_bridge,
    },
    fixtures::{
        AdjacentTreeRowsBridge, DisclosureAndTreeRowBridge, LocalPointerMoveBridge,
        PointerMoveBridge, VirtualTreeRowsBridge,
    },
};
use crate::{
    layout::{Point, Vector2},
    runtime::{NativeRunOptions, PaintPrimitive, RepaintScope, SurfaceInvalidation},
    widgets::PointerButton,
};
use std::time::Instant;
use winit::dpi::PhysicalPosition;

#[test]
fn pointer_move_inside_same_widget_does_not_request_redundant_redraw() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_rect = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .copied()
        .expect("button should be laid out");
    let first_point = Point::new(button_rect.min.x + 2.0, button_rect.min.y + 2.0);
    let second_point = Point::new(button_rect.min.x + 4.0, button_rect.min.y + 2.0);

    let first = core.route_pointer_move(first_point);
    assert!(first.routed);
    assert!(first.needs_redraw());

    let second = core.route_pointer_move(second_point);
    assert!(second.routed);
    assert!(!second.needs_redraw());
}

#[test]
fn pointer_move_message_inside_same_widget_still_requests_redraw() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
    assert_eq!(core.runtime.bridge().moves, 2);
}

#[test]
fn pointer_move_messages_defer_surface_refresh_until_redraw_after_hover_enters() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    assert_eq!(core.runtime.bridge().project_count, 1);
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().project_count, 2);

    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
    assert!(!second.needs_scene_rebuild());
    assert!(second.is_deferred_surface_refresh());
    assert_eq!(core.runtime.bridge().moves, 2);
    assert_eq!(
        core.runtime.bridge().project_count,
        2,
        "stable pointer-move messages should reduce immediately but coalesce surface projection until redraw"
    );
}

#[test]
fn pointer_move_preserves_typed_refresh_scope() {
    for (scope, expected_layout_passes) in
        [(RepaintScope::Projection, 0), (RepaintScope::Layout, 1)]
    {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            PointerMoveBridge {
                repaint_scope: Some(scope),
                ..PointerMoveBridge::default()
            },
            Vector2::new(120.0, 40.0),
        );
        let point = runner
            .core
            .runtime
            .layout()
            .rects
            .get(&71)
            .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
            .expect("pointer widget should be laid out");

        let first = runner.core.route_pointer_move(point);
        assert!(first.needs_scene_rebuild());
        let layout_before_second_move = runner.core.runtime.refresh_counters().layout;

        let routed = runner
            .core
            .route_pointer_move(Point::new(point.x + 1.0, point.y));
        assert!(routed.needs_scene_rebuild());
        assert_eq!(routed.surface_refresh_scope_or_surface(), scope);

        runner.apply_route_outcome(routed);

        assert_eq!(
            runner.core.runtime.refresh_counters().layout,
            layout_before_second_move + expected_layout_passes
        );
        assert_eq!(
            runner.core.runtime.last_refresh_diagnostics().invalidation,
            match scope {
                RepaintScope::Projection => SurfaceInvalidation::Projection,
                RepaintScope::Layout => SurfaceInvalidation::Layout,
                RepaintScope::Surface => SurfaceInvalidation::Surface,
                RepaintScope::PaintOnly => SurfaceInvalidation::PaintOnly,
            }
        );
    }
}

#[test]
fn captured_pointer_move_message_marks_interactive_refresh_for_resizes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PointerMoveBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let point = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let press = runner
        .core
        .route_pointer_press(point, PointerButton::Primary);
    assert!(press.routed);

    let project_count_before_move = runner.core.runtime.bridge().project_count;
    let drag_move = runner
        .core
        .route_pointer_move(Point::new(point.x + 4.0, point.y));

    assert!(drag_move.routed);
    assert!(drag_move.needs_scene_rebuild());
    assert!(!drag_move.is_deferred_surface_refresh());
    assert!(drag_move.is_interactive_surface_refresh());
    assert!(drag_move.is_interactive_scene_rebuild());
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count_before_move,
        "captured pointer routing should let the runner cadence-limit surface projection"
    );

    runner.handle_gpu_surface_pointer_move_outcome(
        drag_move,
        Some(point),
        Point::new(point.x + 4.0, point.y),
    );

    assert_eq!(runner.core.runtime.bridge().moves, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count_before_move + 1,
        "the first captured resize move still refreshes immediately to keep live redraw"
    );
}

#[test]
fn captured_pointer_move_surface_refresh_respects_interactive_cadence() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PointerMoveBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let point = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    assert!(
        runner
            .core
            .route_pointer_press(point, PointerButton::Primary)
            .routed
    );
    runner.timing.last_interactive_scene_rebuild = Instant::now();

    let project_count_before_move = runner.core.runtime.bridge().project_count;
    let drag_position = Point::new(point.x + 4.0, point.y);
    let drag_move = runner.core.route_pointer_move(drag_position);

    assert!(drag_move.is_interactive_surface_refresh());
    assert!(drag_move.is_interactive_scene_rebuild());

    runner.handle_gpu_surface_pointer_move_outcome(drag_move, Some(point), drag_position);

    assert_eq!(runner.core.runtime.bridge().moves, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count_before_move,
        "cadence-limited captured pointer refreshes should not reproject at raw mouse frequency"
    );
    assert!(
        runner.timing.deferred_surface_refresh,
        "the next rebuild should refresh the projected surface before painting"
    );
    assert!(
        runner.timing.deferred_scene_rebuild,
        "cadence-limited captured pointer moves should defer scene work"
    );
}

#[test]
fn deferred_pointer_move_refresh_invalidates_scene_texture() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PointerMoveBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;
    let point = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = runner.core.route_pointer_move(point);
    assert!(first.needs_scene_rebuild());
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;

    let second = runner
        .core
        .route_pointer_move(Point::new(point.x + 1.0, point.y));
    assert!(second.is_deferred_surface_refresh());
    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

    assert!(runner.frame.scene_texture_dirty);
    assert!(runner.frame.composited_base_dirty);
}

#[test]
fn deferred_pointer_move_repaint_refreshes_before_scene_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PointerMoveBridge {
            request_repaint_on_update: true,
            ..PointerMoveBridge::default()
        },
        Vector2::new(120.0, 40.0),
    );
    let point = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = runner.core.route_pointer_move(point);
    assert!(first.needs_scene_rebuild());
    runner.handle_gpu_surface_pointer_move_outcome(first, None, point);

    let project_count_before_second = runner.core.runtime.bridge().project_count;
    let second_position = Point::new(point.x + 1.0, point.y);
    let second = runner.core.route_pointer_move(second_position);

    assert_eq!(
        second.frame_work(),
        FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
        }
    );
    runner.handle_gpu_surface_pointer_move_outcome(second, Some(point), second_position);

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count_before_second + 1,
        "deferred pointer repaint rebuilds should refresh the projected surface before painting"
    );
}

#[test]
fn local_pointer_move_state_inside_same_widget_requests_redraw() {
    let mut core = GenericNativeRuntimeCore::new(LocalPointerMoveBridge, Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("local pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
}

#[test]
fn adjacent_tree_row_hover_transition_clears_previous_row_and_rebuilds_scene() {
    let mut core = GenericNativeRuntimeCore::new(AdjacentTreeRowsBridge, Vector2::new(220.0, 48.0));
    let first = core
        .runtime
        .layout()
        .rects
        .get(&81)
        .copied()
        .expect("first tree row should be laid out");
    let second = core
        .runtime
        .layout()
        .rects
        .get(&82)
        .copied()
        .expect("second tree row should be laid out");

    let enter_first = core.route_pointer_move(first.center());
    assert!(enter_first.needs_scene_rebuild());
    assert_eq!(core.runtime.hovered_widget(), Some(81));
    assert!(
        core.runtime
            .surface()
            .find_widget(81)
            .expect("first row")
            .widget()
            .common()
            .state
            .hovered
    );

    let enter_second = core.route_pointer_move(second.center());

    assert!(
        enter_second.needs_scene_rebuild(),
        "moving between adjacent hover-painted tree rows must rebuild the scene"
    );
    assert_eq!(core.runtime.hovered_widget(), Some(82));
    assert!(
        !core
            .runtime
            .surface()
            .find_widget(81)
            .expect("first row")
            .widget()
            .common()
            .state
            .hovered,
        "previous tree row hover must clear when the pointer enters another row"
    );
    assert!(
        core.runtime
            .surface()
            .find_widget(82)
            .expect("second row")
            .widget()
            .common()
            .state
            .hovered,
        "current tree row should own the hover state"
    );
}

#[test]
fn disclosure_and_tree_row_hover_transitions_clear_previous_target() {
    let mut core =
        GenericNativeRuntimeCore::new(DisclosureAndTreeRowBridge, Vector2::new(220.0, 28.0));
    let disclosure = core
        .runtime
        .layout()
        .rects
        .get(&83)
        .copied()
        .expect("disclosure should be laid out");
    let row = core
        .runtime
        .layout()
        .rects
        .get(&84)
        .copied()
        .expect("tree row should be laid out");

    let enter_disclosure = core.route_pointer_move(disclosure.center());
    assert!(enter_disclosure.needs_scene_rebuild());
    assert_eq!(core.runtime.hovered_widget(), Some(83));

    let enter_row = core.route_pointer_move(row.center());
    assert!(
        enter_row.needs_scene_rebuild(),
        "moving from disclosure to row label must rebuild hover paint"
    );
    assert_eq!(core.runtime.hovered_widget(), Some(84));
    assert!(
        !core
            .runtime
            .surface()
            .find_widget(83)
            .expect("disclosure")
            .widget()
            .common()
            .state
            .hovered,
        "disclosure hover must clear after the pointer enters the row"
    );
    assert!(
        core.runtime
            .surface()
            .find_widget(84)
            .expect("tree row")
            .widget()
            .common()
            .state
            .hovered
    );

    let return_to_disclosure = core.route_pointer_move(disclosure.center());
    assert!(
        return_to_disclosure.needs_scene_rebuild(),
        "moving from row label back to disclosure must rebuild hover paint"
    );
    assert_eq!(core.runtime.hovered_widget(), Some(83));
    assert!(
        !core
            .runtime
            .surface()
            .find_widget(84)
            .expect("tree row")
            .widget()
            .common()
            .state
            .hovered,
        "row hover must clear after the pointer enters the disclosure"
    );
    assert!(
        core.runtime
            .surface()
            .find_widget(83)
            .expect("disclosure")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn native_runner_tree_row_hover_transition_rebuilds_visible_paint_plan() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AdjacentTreeRowsBridge,
        Vector2::new(220.0, 48.0),
    );
    runner.rebuild_scene();
    let first = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&81)
        .copied()
        .expect("first tree row should be laid out");
    let second = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&82)
        .copied()
        .expect("second tree row should be laid out");

    runner.handle_cursor_moved(physical_position(first.center()));
    assert!(row_fill_visible(
        &runner.frame.last_paint_plan.primitives,
        81
    ));

    runner.handle_cursor_moved(physical_position(second.center()));

    assert!(
        !row_fill_visible(&runner.frame.last_paint_plan.primitives, 81),
        "native runner paint plan must drop the previous tree-row hover fill"
    );
    assert!(row_fill_visible(
        &runner.frame.last_paint_plan.primitives,
        82
    ));
}

#[test]
fn native_runner_virtual_tree_row_hover_transition_rebuilds_visible_paint_plan() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        VirtualTreeRowsBridge,
        Vector2::new(220.0, 72.0),
    );
    runner.rebuild_scene();

    runner.handle_cursor_moved(physical_position(Point::new(40.0, 33.0)));
    let first_hover = runner
        .core
        .runtime
        .hovered_widget()
        .expect("first virtual tree row should be hovered");
    assert!(row_fill_visible(
        &runner.frame.last_paint_plan.primitives,
        first_hover
    ));

    runner.handle_cursor_moved(physical_position(Point::new(40.0, 55.0)));
    let second_hover = runner
        .core
        .runtime
        .hovered_widget()
        .expect("second virtual tree row should be hovered");

    assert_ne!(first_hover, second_hover);
    assert!(
        !row_fill_visible(&runner.frame.last_paint_plan.primitives, first_hover),
        "native runner virtual tree paint plan must drop the previous hover fill"
    );
    assert!(row_fill_visible(
        &runner.frame.last_paint_plan.primitives,
        second_hover
    ));
}

fn physical_position(point: Point) -> PhysicalPosition<f64> {
    PhysicalPosition::new(point.x as f64, point.y as f64)
}

fn row_fill_visible(primitives: &[PaintPrimitive], widget_id: u64) -> bool {
    primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == widget_id && fill.rect.has_finite_positive_area()
        )
    })
}
