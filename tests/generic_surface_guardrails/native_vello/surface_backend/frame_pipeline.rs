use super::read_runtime_source;

#[test]
fn native_vello_scene_texture_rendering_stays_out_of_present_driver() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let scene_texture =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/scene_texture.rs");

    assert!(
        module.contains("mod scene_texture;")
            && module.contains("use scene_texture::render_scene_texture_if_needed;"),
        "generic runtime should expose the Vello scene texture renderer as a focused module"
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
