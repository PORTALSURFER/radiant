use super::super::read_runtime_source;

#[test]
fn native_generic_runtime_root_uses_explicit_facade_imports() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");

    assert!(
        module.contains("use super::{")
            && module.contains("NativeRunOptions")
            && module.contains("RuntimeUserEvent")
            && module.contains("gui::{repaint::RepaintSignal, types::Vector2}")
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
            && module.contains("mod run_report;")
            && module.contains("pub use run_report::{")
            && module.contains("NativeGenericRunError")
            && module.contains("NativeGenericRunReport")
            && module.contains("NativeGenericRuntimeArtifacts")
            && !module.starts_with("use super::*;"),
        "generic native runtime root should name startup, runtime, geometry, timing, event-loop, tracing, and report-boundary dependencies"
    );
}

#[test]
fn native_generic_runtime_report_types_stay_in_report_module() {
    let module = read_runtime_source("src/gui_runtime/native_vello/generic_runtime.rs");
    let run_report =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/run_report.rs");

    assert!(
        !module.contains("pub enum NativeGenericRunError")
            && !module.contains("pub struct NativeGenericRuntimeArtifacts")
            && run_report.contains("NativeRunOptionsError")
            && run_report.contains("NativeStartupTimingArtifact")
            && run_report.contains("RuntimeRunReport")
            && run_report.contains("pub struct NativeGenericRuntimeArtifacts")
            && run_report.contains("pub enum NativeGenericRunError")
            && run_report.contains("type NativeGenericRunReport =")
            && run_report
                .contains("RuntimeRunReport<NativeGenericRuntimeArtifacts, NativeGenericRunError>")
            && !run_report.starts_with("use super::*;"),
        "generic runtime report artifacts and typed errors should stay in the focused run_report module"
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
            && present.contains("render_scene_texture_if_needed")
            && present.contains("render_scene_to_surface_view"),
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
            && present.contains("use super::scene_texture::{")
            && present.contains("render_scene_texture_if_needed")
            && present.contains("render_scene_to_surface_view")
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
            && frame_prepare.contains("refresh_gpu_surface_interaction_regions"),
        "deferred model refresh, paint-plan refresh, GPU-region cache refresh, and transient overlay painting should stay in frame_prepare"
    );
    assert!(
        frame_prepare.contains("use super::{")
            && frame_prepare.contains("GenericNativeVelloRunner")
            && frame_prepare.contains("RenderFrameProfile")
            && frame_prepare.contains("use crate::runtime::RuntimeBridge;")
            && frame_prepare.contains("profile.measure(||")
            && !frame_prepare.starts_with("use super::*;"),
        "frame preparation should name runner, render profile, bridge, and profiled timing dependencies"
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
