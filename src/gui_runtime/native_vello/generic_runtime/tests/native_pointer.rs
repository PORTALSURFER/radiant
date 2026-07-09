use super::*;
use crate::gui::{
    focus::FocusSurface,
    input::{KeyCode, KeyPress},
    shortcuts::ShortcutResolution,
};
use crate::widgets::WidgetKey;
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{MouseButton, MouseScrollDelta},
    keyboard::ModifiersState,
};

#[derive(Default)]
struct PointerSnapshotShortcutBridge {
    snapshots: Vec<Option<Point>>,
}

impl RuntimeBridge<()> for PointerSnapshotShortcutBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn update_with_runtime(
        &mut self,
        _message: (),
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<()> {
        self.snapshots.push(snapshot.current_pointer_position());
        Command::none()
    }

    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<()> {
        if press.key == KeyCode::W {
            ShortcutResolution::action(())
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[test]
fn native_pointer_harness_routes_cursor_and_mouse_to_runner_state() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");

    harness.cursor_moved_logical(button_point);
    assert_eq!(harness.runner.input.last_cursor, Some(button_point));
    assert!(harness.mouse_pressed(MouseButton::Left).routed);
    assert!(harness.mouse_released(MouseButton::Left).routed);

    assert_eq!(harness.runner.core.runtime.bridge().state.count, 1);
}

#[test]
fn native_keypress_update_snapshot_uses_hover_cursor_without_mouse_press() {
    let mut harness = NativePointerHarness::new(
        PointerSnapshotShortcutBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let hover = Point::new(88.0, 18.0);

    harness.runner.input.last_cursor = Some(hover);
    harness
        .runner
        .core
        .runtime
        .set_current_pointer_position(None);
    harness.runner.sync_runtime_pointer_from_native_cursor();
    let outcome = harness.runner.core.route_key_press(
        KeyPress::new(KeyCode::W),
        WidgetKey::from_key_code(KeyCode::W),
    );
    harness.runner.apply_route_outcome(outcome);

    assert_eq!(
        harness.runner.core.runtime.bridge().snapshots,
        vec![Some(hover)]
    );
}

#[test]
fn native_pointer_harness_uses_physical_to_logical_cursor_conversion() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.window.dpi_scale = crate::theme::DpiScale::new(2.0);

    harness.cursor_moved_physical(PhysicalPosition::new(40.0, 24.0));

    assert_eq!(
        harness.runner.input.last_cursor,
        Some(Point::new(20.0, 12.0))
    );
}

#[test]
fn native_pointer_enter_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    harness.runner.input.native_cursor_visible = false;
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_entered();

    assert!(harness.runner.input.native_cursor_visible);
    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert_eq!(
        harness.runner.input.native_cursor_apply_count,
        updates_before + 1,
        "cursor entry must reclaim native cursor ownership even when the cached logical cursor did not change"
    );
}

#[test]
fn native_pointer_first_move_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_moved_logical(Point::new(4.0, 4.0));

    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert!(
        harness.runner.input.native_cursor_apply_count > updates_before,
        "first pointer motion after an absent cursor must not trust a stale native cursor cache"
    );
}

#[test]
fn native_pointer_repeated_hover_move_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(4.0, 4.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_moved_logical(Point::new(5.0, 4.0));

    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert!(
        harness.runner.input.native_cursor_apply_count > updates_before,
        "hover motion inside the app must reclaim native cursor ownership even when the cached logical cursor did not change"
    );
}

#[test]
fn native_pointer_harness_drops_mouse_input_without_cursor() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));

    let route = harness.mouse_pressed_route(MouseButton::Left);

    assert!(!route.outcome.routed);
    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MousePress);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::NoCursor);
    assert_eq!(route.diagnostic.position, None);
    assert_eq!(route.diagnostic.hit_target, None);
    assert_eq!(harness.runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn native_pointer_diagnostics_report_hit_target_and_capture_state() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");
    harness.cursor_moved_logical(button_point);

    let press = harness.mouse_pressed_route(MouseButton::Left);

    assert!(press.outcome.routed);
    assert_eq!(press.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(press.diagnostic.position, Some(button_point));
    assert_eq!(press.diagnostic.button, Some(PointerButton::Primary));
    assert_eq!(press.diagnostic.hit_target, Some(11));
    assert_eq!(press.diagnostic.captured_widget, None);
    assert!(press.diagnostic.outcome.needs_redraw());

    let release = harness.mouse_released_route(MouseButton::Left);

    assert!(release.outcome.routed);
    assert_eq!(release.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(release.diagnostic.hit_target, Some(11));
    assert_eq!(release.diagnostic.captured_widget, Some(11));
}

#[test]
fn native_pointer_harness_routes_wheel_with_modifiers() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    harness.modifiers_changed(ModifiersState::SHIFT);

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));
    harness
        .runner
        .flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(route.outcome.frame_work(), FrameWork::None);
    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MouseWheel);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert_eq!(route.diagnostic.hit_target, Some(61));
    assert_eq!(harness.runner.core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        harness.runner.core.runtime.bridge().last_delta,
        Vector2::new(0.0, 80.0)
    );
}

#[test]
fn canceled_coalesced_wheel_does_not_retain_synthetic_lifecycle_work() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    harness.runner.take_pending_frame_work();

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert_eq!(route.outcome.frame_work(), FrameWork::None);
    assert!(harness.runner.input.pending_gpu_surface_wheel.is_some());

    harness.runner.apply_route_outcome(route.outcome);
    assert_eq!(harness.runner.timing.pending_frame_work, FrameWork::None);

    harness.focus_lost();

    assert!(harness.runner.input.pending_gpu_surface_wheel.is_none());
    assert_eq!(
        harness.runner.timing.pending_frame_work,
        FrameWork::None,
        "focus-loss cancellation must not leave synthetic wheel work for presentation"
    );
}

#[test]
fn native_pointer_harness_refreshes_scroll_area_wheel_surface_interactively() {
    let mut harness =
        NativePointerHarness::new(ScrollRefreshBridge::default(), Vector2::new(240.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));

    assert!(route.outcome.routed);
    assert!(!route.outcome.is_deferred_surface_refresh());
    assert!(route.outcome.is_interactive_surface_refresh());
    assert!(route.outcome.is_interactive_scene_rebuild());
    assert!(route.outcome.needs_scene_rebuild());
    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MouseWheel);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        harness.runner.core.runtime.bridge().project_count,
        2,
        "native wheel routing should refresh the projected surface on the first interactive frame"
    );
    assert!(!harness.runner.timing.deferred_surface_refresh);
    assert!(!harness.runner.timing.deferred_scene_rebuild);

    harness
        .runner
        .rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());
    assert_eq!(
        harness.runner.core.runtime.bridge().project_count,
        2,
        "no extra deferred scene rebuild should be queued after the immediate interactive refresh"
    );
}

#[test]
fn native_wheel_flushes_coalesced_scroll_when_redraw_is_starved() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -100.0));

    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(harness.runner.core.runtime.bridge().project_count > 1);
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "starved wheel redraw should refresh virtual rows immediately"
    );
}

#[test]
fn native_wheel_flushes_stale_coalesced_scroll_before_new_wheel_input() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now());

    let queued = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -20.0));
    assert_eq!(
        queued.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    assert_eq!(queued.outcome.frame_work(), FrameWork::None);
    assert_eq!(
        harness.runner.core.runtime.bridge().scroll_count,
        0,
        "fresh pending redraws may coalesce scroll input until paint"
    );

    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));
    let routed = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -20.0));

    assert_eq!(routed.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none(),
        "new wheel input must not leave an older coalesced scroll delta pending"
    );
    assert!(
        harness.runner.core.runtime.bridge().scroll_count >= 2,
        "stale coalesced scroll should flush before routing the fresh wheel event"
    );
    assert!(
        harness.runner.core.runtime.bridge().window.viewport_start >= 80,
        "ordered wheel deltas should advance the app-owned virtual window"
    );
    let paint = harness.runner.core.runtime.paint_plan(&Default::default());
    assert!(
        paint.text_runs().count() > 0 && paint.contains_text("Row 80"),
        "ordered wheel flushing should keep virtual-list rows rendered after a large jump"
    );
}

#[test]
fn native_pointer_press_flushes_coalesced_scroll_before_click_routing() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now());

    let queued = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -100.0));
    assert_eq!(
        queued.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_some(),
        "fresh wheel input should be pending before the click"
    );
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 0);

    let _press = harness.mouse_pressed_route(MouseButton::Left);

    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none(),
        "mouse press should commit pending scroll before hit testing the click"
    );
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(
        harness.runner.core.runtime.bridge().window.viewport_start >= 80,
        "the coalesced scroll should update the app-owned virtual window before click routing"
    );
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "click routing should see freshly materialized bottom rows"
    );
}

#[test]
fn native_scrollbar_drag_flushes_when_redraw_is_starved() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    let scroll_rect = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&81)
        .copied()
        .expect("virtual list scroll area should be laid out");
    let press = Point::new(scroll_rect.max.x - 2.0, scroll_rect.min.y + 6.0);
    let drag = Point::new(press.x, scroll_rect.max.y - 6.0);

    harness.cursor_moved_logical(press);
    harness.mouse_pressed(MouseButton::Left);
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));
    harness.cursor_moved_logical(drag);

    assert!(harness.runner.input.pending_scrollbar_drag.is_none());
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(harness.runner.core.runtime.bridge().project_count > 1);
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "starved scrollbar redraw should refresh virtual rows immediately"
    );
}

#[test]
fn native_pointer_harness_exercises_gpu_hover_fast_path_before_press() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    let point = Point::new(40.0, 20.0);

    assert!(harness.runner.can_fast_path_native_hover_move(point));
    harness.cursor_moved_logical(point);
    assert_eq!(harness.runner.input.last_cursor, Some(point));
    assert!(
        harness.runner.frame.composited_base_dirty,
        "native GPU hover fast path should update cached overlay state"
    );

    let pressed = harness.mouse_pressed(MouseButton::Left);

    assert!(
        pressed.routed,
        "press after native GPU hover fast path should still route through the runtime"
    );
}

#[test]
fn native_pointer_harness_focus_loss_clears_native_pointer_state() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    assert!(!harness.runner.input.native_cursor_visible);
    harness.modifiers_changed(ModifiersState::ALT);

    let outcome = harness.focus_lost();

    assert!(outcome.routed);
    assert_eq!(harness.runner.input.last_cursor, None);
    assert!(harness.runner.input.native_cursor_visible);
    assert!(harness.runner.input.modifiers.is_empty());
    harness.focus_regained();
}

#[test]
fn native_pointer_focus_loss_clears_retained_widget_hover() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");

    harness.cursor_moved_logical(button_point);
    assert_eq!(harness.runner.core.runtime.hovered_widget(), Some(11));
    assert!(
        harness
            .runner
            .core
            .runtime
            .surface()
            .find_widget(11)
            .expect("hovered button")
            .widget()
            .common()
            .state
            .hovered
    );

    let outcome = harness.focus_lost();

    assert!(outcome.needs_scene_rebuild());
    assert_eq!(harness.runner.input.last_cursor, None);
    assert_eq!(harness.runner.core.runtime.hovered_widget(), None);
    assert!(
        !harness
            .runner
            .core
            .runtime
            .surface()
            .find_widget(11)
            .expect("previous hovered button")
            .widget()
            .common()
            .state
            .hovered
    );
}
