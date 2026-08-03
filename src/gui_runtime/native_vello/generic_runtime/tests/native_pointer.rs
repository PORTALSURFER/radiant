use super::*;
use crate::application::IntoView;
use crate::gui::{
    focus::FocusSurface,
    input::{InputTimestamp, KeyCode, KeyPress},
    shortcuts::ShortcutResolution,
};
use crate::runtime::{RuntimeHostCapabilities, RuntimeInputHost};
use crate::{
    layout::LayoutOutput,
    theme::ThemeTokens,
    widgets::{
        PointerModifiers, Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{MouseButton, MouseScrollDelta},
    keyboard::ModifiersState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModifierWheelMessage {
    Wheel,
}

#[derive(Clone, Debug)]
struct ModifierWheelWidget {
    common: WidgetCommon,
}

impl ModifierWheelWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
        }
    }
}

impl Widget for ModifierWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::Wheel { .. })
            .then(|| WidgetOutput::typed(ModifierWheelMessage::Wheel))
    }

    fn accepts_pointer_input(&self, input: &WidgetInput) -> bool {
        matches!(input, WidgetInput::Wheel { modifiers, .. } if modifiers.shift)
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
struct ModifierWheelBridge;

impl RuntimeBridge<ModifierWheelMessage> for ModifierWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ModifierWheelMessage>> {
        let rows = (0..20)
            .map(|index| {
                crate::application::text(format!("Row {index}"))
                    .height(20.0)
                    .fill_width()
            })
            .collect();
        crate::application::overlay_stack(crate::application::bounded_scroll_column(
            rows, 4, 20.0, 0.0,
        ))
        .input(crate::application::custom_widget_direct(
            ModifierWheelWidget::new(4),
        ))
        .into_view()
        .into_surface()
        .into()
    }

    fn reduce_message(&mut self, _message: ModifierWheelMessage) {}
}

#[derive(Default)]
struct PressTimestampBridge {
    timestamps: Vec<Option<InputTimestamp>>,
}

#[derive(Clone)]
struct PressTimestampWidget {
    common: WidgetCommon,
}

impl PressTimestampWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
        }
    }
}

impl Widget for PressTimestampWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerPress {
                position,
                timestamp,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(timestamp)),
            _ => None,
        }
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

impl RuntimeBridge<Option<InputTimestamp>> for PressTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<Option<InputTimestamp>>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            PressTimestampWidget::new(1),
            WidgetMessageMapper::typed(|timestamp: Option<InputTimestamp>| timestamp),
        )))
    }

    fn reduce_message(&mut self, timestamp: Option<InputTimestamp>) {
        self.timestamps.push(timestamp);
    }
}

#[test]
fn scroll_coalescing_preserves_modifier_sensitive_wheel_ownership() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ModifierWheelBridge,
        Vector2::new(120.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let delta = Vector2::new(0.0, -40.0);

    assert!(runner.can_coalesce_scroll_container_wheel(point, delta, PointerModifiers::default()));
    assert!(!runner.can_coalesce_scroll_container_wheel(
        point,
        delta,
        PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        }
    ));
}

#[derive(Default)]
struct PointerSnapshotShortcutBridge {
    snapshots: Vec<Option<Point>>,
}

impl RuntimeBridge<()> for PointerSnapshotShortcutBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
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

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for PointerSnapshotShortcutBridge {
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

#[derive(Default)]
struct FocusRegainedBridge {
    focus_regained_calls: usize,
    reduced_messages: usize,
}

impl RuntimeBridge<()> for FocusRegainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, (): ()) {
        self.reduced_messages += 1;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for FocusRegainedBridge {
    fn native_focus_regained(&mut self) -> Command<()> {
        self.focus_regained_calls += 1;
        Command::message(())
    }
}

#[test]
fn native_focus_regained_notifies_host_and_routes_its_command() {
    let mut harness =
        NativePointerHarness::new(FocusRegainedBridge::default(), Vector2::new(320.0, 40.0));

    harness.focus_lost();
    harness.focus_regained();

    let bridge = harness.runner.core.runtime.bridge();
    assert_eq!(bridge.focus_regained_calls, 1);
    assert_eq!(bridge.reduced_messages, 1);
}

#[test]
fn initial_native_focus_does_not_report_focus_regained() {
    let mut harness =
        NativePointerHarness::new(FocusRegainedBridge::default(), Vector2::new(320.0, 40.0));

    harness.focus_regained();

    let bridge = harness.runner.core.runtime.bridge();
    assert_eq!(bridge.focus_regained_calls, 0);
    assert_eq!(bridge.reduced_messages, 0);
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
fn native_pointer_press_delivers_one_captured_timestamp_to_widget_input() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    let point = Point::new(8.0, 8.0);

    harness.cursor_moved_logical(point);
    assert!(
        harness
            .mouse_pressed_route(MouseButton::Left)
            .outcome
            .routed
    );

    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert!(timestamps[0].is_some());
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
#[cfg(target_os = "macos")]
fn macos_control_left_is_one_latched_secondary_gesture() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let point = Point::new(12.0, 12.0);
    harness.cursor_moved_logical(point);
    harness.modifiers_changed(
        ModifiersState::CONTROL
            | ModifiersState::SUPER
            | ModifiersState::SHIFT
            | ModifiersState::ALT,
    );

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(
        press.diagnostic.modifiers,
        PointerModifiers {
            command: true,
            shift: true,
            alt: true,
        }
    );
    assert_eq!(
        harness.runner.input.effective_pointer_gesture,
        Some(
            crate::gui_runtime::native_vello::generic_runtime::input::NativePointerGestureLatch {
                physical_button: MouseButton::Left,
                gesture:
                    crate::gui_runtime::native_vello::generic_runtime::input::NativePointerGesture {
                        button: PointerButton::Secondary,
                        consume_control: true,
                    },
            }
        )
    );

    // macOS can report the Control release before the matching mouse-up. The
    // latched effective button must still drive the release.
    harness.modifiers_changed(ModifiersState::SUPER | ModifiersState::SHIFT);
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn macos_command_left_without_control_remains_primary() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::SUPER);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Primary));
    assert!(press.diagnostic.modifiers.command);
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Primary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_secondary_double_click_uses_converted_button_identity() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);

    let first_press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(!first_press.double_click);
    let _first_release = harness.mouse_released_route(MouseButton::Left);
    let second_press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(second_press.double_click);
    assert_eq!(second_press.button, Some(PointerButton::Secondary));
    let _second_release = harness.mouse_released_route(MouseButton::Left);
}

#[test]
#[cfg(target_os = "macos")]
fn macos_control_left_release_keeps_latched_button_during_capture() {
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
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(harness.runner.core.runtime.pointer_capture(), Some(11));

    harness.cursor_moved_logical(Point::new(button_point.x + 40.0, button_point.y + 8.0));
    harness.modifiers_changed(ModifiersState::empty());
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn macos_control_left_latch_survives_interleaved_physical_buttons() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert!(!press.diagnostic.modifiers.command);
    assert!(harness.runner.input.effective_pointer_gesture.is_some());

    for interleaved in [MouseButton::Right, MouseButton::Other(5)] {
        let interleaved_press = harness.mouse_pressed_route(interleaved);
        let interleaved_release = harness.mouse_released_route(interleaved);
        if interleaved == MouseButton::Right {
            assert_eq!(interleaved_press.button, Some(PointerButton::Secondary));
            assert_eq!(interleaved_release.button, Some(PointerButton::Secondary));
        } else {
            assert_eq!(interleaved_press.button, None);
            assert_eq!(interleaved_release.button, None);
        }
        assert!(harness.runner.input.effective_pointer_gesture.is_some());
    }

    harness.modifiers_changed(ModifiersState::empty());
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(!release.diagnostic.modifiers.command);
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(not(target_os = "macos"))]
fn non_macos_control_left_remains_primary() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let point = Point::new(12.0, 12.0);
    harness.cursor_moved_logical(point);
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Primary));
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Primary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_physical_right_remains_secondary_without_control_consumption() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL | ModifiersState::SHIFT);

    let press = harness.mouse_pressed_route(MouseButton::Right);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(
        press.diagnostic.modifiers,
        PointerModifiers {
            command: true,
            shift: true,
            alt: false,
        }
    );
    let release = harness.mouse_released_route(MouseButton::Right);
    assert_eq!(release.button, Some(PointerButton::Secondary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_focus_loss_clears_effective_pointer_gesture_latch() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);
    let _press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(harness.runner.input.effective_pointer_gesture.is_some());

    let _ = harness.focus_lost();

    assert!(harness.runner.input.effective_pointer_gesture.is_none());
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
