use super::super::read_runtime_source;

#[test]
fn native_timed_frame_drain_does_not_recompute_selected_cadence() {
    let lifecycle =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs");
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");

    assert!(
        lifecycle.contains("let cadence = timed_frame_cadence(")
            && lifecycle.contains("TimedFrameCadence::DrainNow { next_wake }")
            && lifecycle.contains("self.drain_timed_frame_now("),
        "native lifecycle should compute timed-frame cadence once and drain directly when due"
    );
    assert!(
        runner.contains("fn drain_timed_frame_now")
            && !runner.contains("fn drain_due_timed_frame")
            && !runner.contains("match timed_frame_cadence("),
        "runner timed-frame drain should not recompute cadence already selected by lifecycle"
    );
}

#[test]
fn native_lifecycle_uses_explicit_imports() {
    let lifecycle =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs");
    let native_pointer =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/native_pointer.rs");

    assert!(
        lifecycle.contains("use super::{")
            && lifecycle.contains("AuxiliaryWindowEventResult")
            && lifecycle.contains("GenericNativeVelloRunner")
            && lifecycle.contains("RuntimeUserEvent")
            && lifecycle.contains("TimedFrameCadence")
            && lifecycle.contains("should_start_popup_window_drag")
            && lifecycle.contains("timed_frame_cadence")
            && lifecycle.contains("timed_frame_target_fps")
            && lifecycle.contains("use crate::runtime::RuntimeBridge;")
            && lifecycle.contains("use std::time::Instant;")
            && lifecycle.contains("use tracing::warn;")
            && lifecycle.contains("use winit::{")
            && !lifecycle.starts_with("use super::*;"),
        "native lifecycle should name runner, auxiliary routing, runtime event, cadence, input conversion, popup policy, bridge, timing, logging, and Winit dependencies"
    );
    assert!(
        native_pointer.contains("use super::{")
            && native_pointer.contains("maybe_log_route_profile")
            && native_pointer.contains("pointer_button_from_winit")
            && native_pointer.contains("scroll_delta_to_logical")
            && !native_pointer.starts_with("use super::*;"),
        "native pointer routing should explicitly import route profiling and pointer/wheel conversion helpers"
    );
    assert!(
        lifecycle.contains("impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>")
            && lifecycle.contains("fn window_event(")
            && lifecycle.contains("fn user_event(")
            && lifecycle.contains("fn about_to_wait(")
            && lifecycle.contains("ControlFlow::WaitUntil")
            && !lifecycle.contains("winit::event::WindowEvent"),
        "native lifecycle should keep Winit callbacks and timed-frame wake policy focused"
    );
}

#[test]
fn native_runner_keeps_window_input_and_timing_state_grouped() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");
    let state = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner_state.rs");

    assert!(
        module.contains("mod runner_state;")
            && module.contains(
                "use runner_state::{NativeRunnerInputState, NativeRunnerTimingState, NativeRunnerWindowState};"
            ),
        "generic runtime should expose focused native runner state groups"
    );
    assert!(
        runner.contains("window: NativeRunnerWindowState")
            && runner.contains("input: NativeRunnerInputState")
            && runner.contains("timing: NativeRunnerTimingState")
            && !runner.contains("window_id: Option<WindowId>")
            && !runner.contains("last_cursor: Option<Point>")
            && !runner.contains("startup_timing: StartupTimingProfile"),
        "native runner root should group window resources, input state, and timing state"
    );
    assert!(
        runner.contains("use super::{")
            && runner.contains("AuxiliaryNativeWindow")
            && runner.contains("GenericNativeRuntimeCore")
            && runner.contains("GenericRouteOutcome")
            && runner.contains("NativeRunnerInputState")
            && runner.contains("NativeRunnerTimingState")
            && runner.contains("NativeRunnerWindowState")
            && runner.contains("NativeVelloFrameState")
            && runner.contains("RuntimeWakeup")
            && runner.contains("SurfaceSceneEncodeContext")
            && runner.contains("TimedFrameCadence")
            && runner.contains("encode_surface_paint_plan_to_scene")
            && runner.contains("timed_frame_cadence")
            && runner.contains("timed_frame_target_fps")
            && runner.contains("gui::types::Vector2")
            && runner.contains("gui_runtime::native_vello::NativeTextRenderer")
            && runner.contains("RuntimeAnimationActivity")
            && runner.contains("RuntimeBridge")
            && runner.contains("NativeRunOptions")
            && runner.contains("use std::time::Instant;")
            && runner.contains("use winit::event_loop::ActiveEventLoop;")
            && !runner.starts_with("use super::*;"),
        "native runner should name runtime state, frame state, route outcome, scene rebuild, timed-frame, text renderer, runtime, timing, and event-loop dependencies"
    );
    assert!(
        state.contains("struct NativeRunnerWindowState")
            && state.contains("struct NativeRunnerInputState")
            && state.contains("struct NativeRunnerTimingState")
            && state.contains("use super::PendingGpuSurfaceWheel;")
            && state.contains("use crate::gui::types::Point;")
            && state.contains("use vello::{")
            && state.contains("use winit::{")
            && !state.starts_with("use super::*;"),
        "native runner state groups should stay in runner_state.rs with explicit runtime, window, and timing dependencies"
    );
}

#[test]
fn native_auxiliary_windows_use_explicit_runtime_imports() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let auxiliary =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs");
    let native_pointer =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/native_pointer.rs");

    assert!(
        module.contains("mod auxiliary;")
            && module
                .contains("use auxiliary::{AuxiliaryNativeWindow, AuxiliaryWindowEventResult};"),
        "generic runtime should expose auxiliary windows as a focused module"
    );
    assert!(
        auxiliary.contains("use super::{")
            && auxiliary.contains("GenericNativeVelloRunner")
            && auxiliary.contains("GenericRouteOutcome")
            && auxiliary.contains("owner_window_handle")
            && auxiliary.contains(
                "use crate::runtime::{AuxiliaryWindow, NativeRunOptions, RuntimeBridge};"
            )
            && auxiliary.contains("use winit::{")
            && !auxiliary.starts_with("use super::*;"),
        "auxiliary windows should name their runner, runtime helper, public model, and winit dependencies"
    );
    assert!(
        native_pointer.contains("scroll_delta_to_logical"),
        "auxiliary windows should reuse the shared native pointer wheel routing contract"
    );
    assert!(
        auxiliary.contains("fn dispatch_auxiliary_messages")
            && auxiliary.contains("fn sync_auxiliary_windows")
            && auxiliary.contains("project_auxiliary_windows")
            && !runner_contains_auxiliary_sync_root(),
        "auxiliary projection sync and dispatched auxiliary messages should stay with the auxiliary window module"
    );
}

fn runner_contains_auxiliary_sync_root() -> bool {
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");
    runner.contains("fn dispatch_auxiliary_messages")
        || runner.contains("fn sync_auxiliary_windows")
}
