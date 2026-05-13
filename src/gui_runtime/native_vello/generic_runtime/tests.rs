use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, LayoutDebugOptions, Rect, SlotParams},
    runtime::{
        Command, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle,
        GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, PaintGpuSurface, PaintPrimitive,
        SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    widgets::{
        ButtonWidget, CanvasMessage, PointerButton, ScrollbarAxis, ScrollbarMessage,
        ScrollbarWidget, TextInputMessage, TextInputWidget, Widget, WidgetCommon, WidgetInput,
        WidgetOutput, WidgetSizing,
    },
};
use winit::{dpi::Position, window::WindowLevel};

#[cfg(test)]
#[path = "tests/event_routing.rs"]
mod event_routing;
#[path = "tests/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "tests/gpu_surface_runtime.rs"]
mod gpu_surface_runtime;
#[cfg(test)]
#[path = "tests/pointer_motion.rs"]
mod pointer_motion;
#[cfg(test)]
#[path = "tests/scene_cache.rs"]
mod scene_cache;
use fixtures::*;

#[test]
fn generic_core_empty_runtime_wakeup_does_not_need_redraw() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(!outcome.routed);
    assert!(!outcome.needs_redraw());
    assert!(!outcome.runtime_work_remaining);
}

#[test]
fn generic_core_is_repaint_driven_when_host_reports_no_animation() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    assert!(!core.animation_activity().needs_animation());
}

#[test]
fn generic_core_preserves_animation_when_host_requests_it() {
    let mut core = GenericNativeRuntimeCore::new(AnimatingBridge, Vector2::new(320.0, 40.0));

    assert!(core.animation_activity().needs_animation());
}

#[test]
fn generic_core_turns_message_free_animation_into_paint_only_redraw() {
    let mut core = GenericNativeRuntimeCore::new(AnimatingBridge, Vector2::new(320.0, 40.0));

    let activity = core.animation_activity();
    let outcome = core.drain_timed_frame(activity, false);

    assert!(!outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(!outcome.needs_scene_rebuild());
}

#[test]
fn generic_core_turns_text_caret_animation_into_scene_rebuild_redraw() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    assert!(core.runtime.focus_widget(12));
    let outcome = core.drain_timed_frame(
        crate::runtime::RuntimeAnimationActivity::idle(),
        core.has_focused_text_input(),
    );

    assert!(!outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(outcome.needs_scene_rebuild());
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
fn generic_native_window_starts_hidden_during_surface_setup() {
    let attrs = generic_window_attributes(&NativeRunOptions::default());

    assert!(!attrs.visible);
}

#[test]
fn generic_native_window_uses_configured_drag_and_drop_policy() {
    assert!(window::platform_drag_and_drop_enabled(
        &NativeRunOptions::default()
    ));
    assert!(!window::platform_drag_and_drop_enabled(&NativeRunOptions {
        drag_and_drop: false,
        ..NativeRunOptions::default()
    }));
}

#[test]
fn generic_native_window_applies_floating_popup_policy() {
    let attrs = generic_window_attributes(
        &NativeRunOptions::popup("Drag Preview").popup_policy(
            NativePopupOptions::default()
                .position(64.0, 96.0)
                .initially_focused(true),
        ),
    );

    assert_eq!(attrs.title, "Drag Preview");
    assert!(!attrs.decorations);
    assert!(!attrs.resizable);
    assert!(attrs.transparent);
    assert!(attrs.active);
    assert_eq!(attrs.window_level, WindowLevel::AlwaysOnTop);
    assert!(
        matches!(attrs.position, Some(Position::Logical(position)) if position.x == 64.0 && position.y == 96.0)
    );
}

#[test]
fn generic_runtime_clamps_animation_frame_interval() {
    assert_eq!(
        frame_cadence::animation_frame_interval(0),
        Duration::from_secs(1)
    );
    assert_eq!(
        frame_cadence::animation_frame_interval(120),
        Duration::from_secs_f64(1.0 / 120.0)
    );
    assert_eq!(
        frame_cadence::animation_frame_interval(1_000),
        Duration::from_secs_f64(1.0 / 240.0)
    );
}
