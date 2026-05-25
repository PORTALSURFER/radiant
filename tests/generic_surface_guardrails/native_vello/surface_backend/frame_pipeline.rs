use super::read_runtime_source;

#[test]
fn native_generic_runtime_root_uses_explicit_facade_imports() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");

    assert!(
        module.contains("use super::{")
            && module.contains("NativeRunOptions")
            && module.contains("NativeRunOptionsError")
            && module.contains("NativeStartupTimingArtifact")
            && module.contains("RuntimeUserEvent")
            && module.contains("gui::{repaint::RepaintSignal, types::Vector2}")
            && module.contains("gui_runtime::RuntimeRunReport")
            && module.contains("runtime::RuntimeBridge")
            && module.contains("sync::Arc")
            && module.contains("time::Instant")
            && module.contains("use tracing::{info, warn};")
            && module.contains("use winit::event_loop::EventLoop;")
            && module.contains("#[cfg(test)]")
            && module.contains("gui::types::{Point, Rect as UiRect, Rgba8}")
            && module.contains("gui_runtime::native_vello::NativeTextRenderer")
            && module.contains("use std::time::Duration;")
            && module.contains("use vello::Scene;")
            && module.contains("type NativeGenericRunReport =")
            && module
                .contains("RuntimeRunReport<NativeGenericRuntimeArtifacts, NativeGenericRunError>")
            && !module.starts_with("use super::*;"),
        "generic native runtime root should name facade, runtime, geometry, timing, event-loop, and tracing dependencies"
    );
}

#[test]
fn native_vello_scene_texture_rendering_stays_out_of_present_driver() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let scene_texture =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/scene_texture.rs");

    assert!(
        module.contains("mod scene_texture;")
            && present.contains("use super::scene_texture::render_scene_texture_if_needed;"),
        "generic runtime should expose the Vello scene texture renderer as a focused module and the present driver should import it directly"
    );
    assert!(
        !present.contains("renderer.render_to_texture(")
            && scene_texture.contains("renderer.render_to_texture(")
            && scene_texture.contains("frame.scene_texture_dirty = false")
            && scene_texture.contains("frame.mark_composited_base_dirty();"),
        "present driver should delegate dirty scene texture rendering to the focused scene_texture module"
    );
    assert!(
        scene_texture.contains("use super::NativeVelloFrameState;")
            && scene_texture.contains("use crate::gui_runtime::native_vello::color_from_rgba;")
            && scene_texture.contains("use std::time::{Duration, Instant};")
            && scene_texture.contains("use tracing::error;")
            && scene_texture.contains("AaConfig")
            && scene_texture.contains("RenderParams")
            && scene_texture.contains("Renderer")
            && scene_texture.contains("util::RenderSurface")
            && scene_texture.contains("wgpu")
            && scene_texture.contains("use winit::event_loop::ActiveEventLoop;")
            && !scene_texture.starts_with("use super::*;"),
        "scene texture rendering should name its frame, color, timing, tracing, Vello, WGPU, and event-loop dependencies"
    );
}

#[test]
fn native_present_driver_uses_explicit_imports() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");

    assert!(
        present.contains("use super::{")
            && present.contains("GenericNativeVelloRunner")
            && present.contains("RenderFrameProfile")
            && present.contains("RenderSurfacePixelSize")
            && present.contains("hide_window_after_first_present")
            && present.contains("maybe_log_render_profile")
            && present.contains("post_gpu_overlay")
            && present.contains("reveal_window_after_first_present")
            && present.contains("use crate::runtime::RuntimeBridge;")
            && present.contains("use std::time::Instant;")
            && present.contains("use vello::wgpu;")
            && present.contains("use winit::event_loop::ActiveEventLoop;")
            && present.contains("use super::composited_base::{")
            && present.contains("BaseFramePresentState")
            && present.contains("BaseFramePresentTarget")
            && present.contains("present_base_frame")
            && present.contains("use super::scene_texture::render_scene_texture_if_needed;")
            && !present.starts_with("use super::*;"),
        "native present driver should name runner, frame profile, surface sizing, window policy, diagnostics, WGPU, event loop, base-frame, and scene-texture dependencies"
    );
    assert!(
        !module.contains("BaseFramePresentState")
            && !module.contains("BaseFramePresentTarget")
            && !module.contains("present_base_frame")
            && !module.contains("render_scene_texture_if_needed"),
        "generic runtime root should not carry present-driver-only imports"
    );
}

#[test]
fn native_frame_preparation_stays_out_of_present_driver() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let frame_prepare =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/frame_prepare.rs");

    assert!(
        module.contains("mod frame_prepare;"),
        "generic runtime should expose frame preparation as a focused module"
    );
    assert!(
        present.contains("self.refresh_deferred_surface_if_needed(&mut profile);")
            && present.contains("self.paint_transient_overlays(&mut profile);"),
        "present driver should orchestrate frame preparation without owning its implementation"
    );
    assert!(
        !present.contains("self.core.refresh_surface()")
            && !present.contains("paint_transient_overlay(")
            && frame_prepare.contains("fn refresh_deferred_surface_if_needed")
            && frame_prepare.contains("fn paint_transient_overlays")
            && frame_prepare.contains("collect_gpu_surface_interaction_regions"),
        "deferred model refresh, paint-plan refresh, and transient overlay painting should stay in frame_prepare"
    );
    assert!(
        frame_prepare.contains("use super::{")
            && frame_prepare.contains("GenericNativeVelloRunner")
            && frame_prepare.contains("RenderFrameProfile")
            && frame_prepare.contains("collect_gpu_surface_interaction_regions")
            && frame_prepare.contains("use crate::runtime::RuntimeBridge;")
            && frame_prepare.contains("use std::time::Instant;")
            && !frame_prepare.starts_with("use super::*;"),
        "frame preparation should name runner, render profile, GPU-region collection, bridge, and timing dependencies"
    );
}

#[test]
fn native_frame_state_uses_explicit_imports() {
    let frame_state =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/frame_state.rs");

    assert!(
        frame_state.contains("use super::{")
            && frame_state.contains("CompositedBaseFrame")
            && frame_state.contains("GpuSurfaceInteractionRegion")
            && frame_state.contains("GpuSurfaceRenderer")
            && frame_state.contains("PostGpuOverlayRenderer")
            && frame_state.contains("RetainedSurfaceEncodeStats")
            && frame_state.contains("RetainedSurfaceFrameCache")
            && frame_state.contains("SceneTextRunBuffer")
            && frame_state.contains("gui_runtime::native_vello::NativeTextRenderer")
            && frame_state.contains(
                "runtime::{PaintPrimitive, RetainedSurfaceCachePolicy, SurfacePaintPlan}"
            )
            && frame_state.contains("theme::ThemeTokens")
            && frame_state.contains("use vello::Scene;")
            && !frame_state.starts_with("use super::*;"),
        "native frame state should name renderer, cache, runtime, theme, and scene dependencies"
    );
    assert!(
        frame_state.contains("struct NativeVelloFrameState")
            && frame_state.contains("transient_overlay_primitives: Vec<PaintPrimitive>")
            && frame_state.contains("fn mark_scene_texture_dirty")
            && frame_state.contains("fn mark_composited_base_dirty")
            && !frame_state.contains("Vec<crate::runtime::PaintPrimitive>"),
        "native frame state should own frame buffers, caches, and dirty flags without hidden runtime imports"
    );
}

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

    assert!(
        lifecycle.contains("use super::{")
            && lifecycle.contains("AuxiliaryWindowEventResult")
            && lifecycle.contains("GenericNativeVelloRunner")
            && lifecycle.contains("RuntimeUserEvent")
            && lifecycle.contains("TimedFrameCadence")
            && lifecycle.contains("maybe_log_route_profile")
            && lifecycle.contains("pointer_button_from_winit")
            && lifecycle.contains("scroll_delta_to_logical")
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

    assert!(
        module.contains("mod auxiliary;")
            && module
                .contains("use auxiliary::{AuxiliaryNativeWindow, AuxiliaryWindowEventResult};"),
        "generic runtime should expose auxiliary windows as a focused module"
    );
    assert!(
        auxiliary.contains("use super::{")
            && auxiliary.contains("GenericNativeVelloRunner")
            && auxiliary.contains("owner_window_handle")
            && auxiliary.contains("scroll_delta_to_logical")
            && auxiliary.contains("use crate::runtime::{AuxiliaryWindow, NativeRunOptions};")
            && auxiliary.contains("use winit::{")
            && !auxiliary.starts_with("use super::*;"),
        "auxiliary windows should name their runner, runtime helper, public model, and winit dependencies"
    );
}

#[test]
fn native_render_surface_target_size_stays_in_focused_module() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let composited =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/composited_base.rs");
    let surface_size =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface_size.rs");

    assert!(
        module.contains("mod surface_size;")
            && module.contains("use surface_size::RenderSurfacePixelSize;"),
        "generic runtime should own render-surface sizing through a focused module"
    );
    assert!(
        present.contains("RenderSurfacePixelSize::from_surface(surface)")
            && composited
                .matches("RenderSurfacePixelSize::from_surface(surface)")
                .count()
                == 2,
        "present and composited-base WGPU targets should use the shared render-surface size helper"
    );
    assert!(
        !present.contains("surface.config.width as f32")
            && !composited.contains("surface.config.width as f32")
            && surface_size.contains("pub(super) struct RenderSurfacePixelSize")
            && surface_size.contains("fn logical_size")
            && surface_size.contains("use crate::gui::types::Vector2;")
            && surface_size.contains("use vello::util::RenderSurface;")
            && !surface_size.starts_with("use super::*;"),
        "direct WGPU target size conversion should stay centralized with explicit geometry and surface dependencies"
    );
}

#[test]
fn native_surface_setup_uses_explicit_imports() {
    let surface = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface.rs");
    let backend =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface/backend.rs");
    let production_surface = surface
        .split("#[cfg(test)]")
        .next()
        .expect("surface production source should precede tests");

    assert!(
        surface.contains("use super::{")
            && surface.contains("GenericNativeVelloRunner")
            && surface.contains("generic_window_attributes")
            && surface.contains("reveal_window_after_surface_setup")
            && surface.contains("gui::types::Vector2")
            && surface.contains("select_present_mode")
            && surface.contains("startup_renderer_options")
            && surface.contains("runtime::RuntimeBridge")
            && surface.contains("use std::{sync::Arc, time::Instant};")
            && surface.contains("use tracing::{error, info, warn};")
            && surface.contains("use vello::{Renderer, wgpu};")
            && surface.contains("use winit::{")
            && !surface.starts_with("use super::*;"),
        "native surface setup should name runner, window policy, viewport, renderer config, bridge, timing, tracing, Vello/WGPU, and Winit dependencies"
    );
    assert!(
        production_surface.contains("fn initialize_runtime")
            && production_surface.contains("fn resize_surface")
            && production_surface.contains("fn acquire_present_surface_texture")
            && production_surface.contains("fn surface_size_changed")
            && !production_surface.contains("winit::dpi::PhysicalSize"),
        "native surface setup should keep initialization, resize, acquire, and physical-size comparison focused"
    );
    assert!(
        backend.contains("use crate::gui_runtime::{NativeGpuBackend, NativeRunOptions};")
            && backend.contains("use vello::{util::RenderContext, wgpu};")
            && backend.contains("fn render_context_for_options")
            && backend.contains("fn wgpu_backends")
            && !backend.starts_with("use super::*;")
            && !backend.starts_with("use super::super::*;"),
        "native surface backend setup should name run options, GPU backend policy, render context, and WGPU dependencies explicitly"
    );
}

#[test]
fn native_wgpu_device_target_helpers_use_explicit_imports() {
    let device = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/device.rs");

    assert!(
        device.contains("use vello::wgpu;")
            && device.contains("fn wgpu_device_id")
            && device.contains("fn wgpu_target_matches")
            && !device.starts_with("use super::*;"),
        "native WGPU target helpers should name their WGPU dependency instead of inheriting the runtime root"
    );
}

#[test]
fn native_render_profile_uses_explicit_imports() {
    let render_profile =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/render_profile.rs");

    assert!(
        render_profile.contains("use std::time::Duration;")
            && render_profile.contains("use tracing::info;")
            && render_profile.contains("RetainedSurfaceEncodeStats")
            && render_profile.contains("GpuSurfaceRenderStats")
            && render_profile.contains("TextLayoutProfileCounters")
            && !render_profile.contains("use super::*;"),
        "native render profile diagnostics should name their timing, tracing, scene, text, and GPU stats dependencies"
    );
}

#[test]
fn native_runtime_config_boundary_uses_explicit_imports_and_exports() {
    let facade = read_runtime_source("src/gui_runtime/native_vello.rs");
    let runtime_config = read_runtime_source("src/gui_runtime/native_vello/runtime_config.rs");

    assert!(
        facade.contains("pub(in crate::gui_runtime::native_vello) use runtime_config::{")
            && facade.contains("select_present_mode")
            && facade.contains("startup_renderer_options")
            && !facade.contains("pub(in crate::gui_runtime::native_vello) use runtime_config::*;"),
        "native Vello facade should name the runtime configuration helpers it shares internally"
    );
    assert!(
        runtime_config.contains("use vello::{AaSupport, RendererOptions, wgpu};")
            && !runtime_config.starts_with("use super::*;"),
        "runtime_config should import only renderer option types instead of inheriting the native Vello root"
    );
}

#[test]
fn native_startup_timing_uses_explicit_imports() {
    let startup = read_runtime_source("src/gui_runtime/native_vello/startup.rs");

    assert!(
        startup.contains("use std::time::Instant;")
            && startup.contains("mod artifact;")
            && startup.contains("mod logging;")
            && startup.contains("pub use artifact::NativeStartupTimingArtifact;")
            && !startup.starts_with("use super::*;"),
        "native startup timing should name its timing dependency and keep artifact/logging concerns local"
    );
    assert!(
        startup.contains("struct StartupTimingProfile")
            && startup.contains("fn maybe_emit_summary")
            && startup.contains("fn export_artifact")
            && startup.contains("fn failure_reason")
            && startup.contains("fn ms_between"),
        "native startup timing should keep profile state, summary export, failure reason, and duration math focused"
    );
}
