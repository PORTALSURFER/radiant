use super::super::read_runtime_source;

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
