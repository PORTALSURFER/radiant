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
        scene_texture.contains("NativeVelloFrameState")
            && scene_texture.contains("use crate::gui_runtime::native_vello::color_from_rgba;")
            && scene_texture.contains("time::{Duration, Instant}")
            && scene_texture.contains("use tracing::error;")
            && scene_texture.contains("AaConfig")
            && scene_texture.contains("RenderParams")
            && scene_texture.contains("Renderer")
            && scene_texture.contains("util::RenderSurface")
            && scene_texture.contains("wgpu")
            && scene_texture.contains("NativeGenericRunError")
            && scene_texture.contains("Result<Duration, NativeGenericRunError>")
            && !scene_texture.contains("ActiveEventLoop")
            && !scene_texture.contains("event_loop.exit()")
            && !scene_texture.starts_with("use super::*;"),
        "scene texture rendering should name its frame, color, timing, tracing, Vello, WGPU, and typed error dependencies"
    );
}

#[test]
fn native_renderer_unwind_containment_stays_local_to_scene_texture() {
    let scene_texture =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/scene_texture.rs");
    let broad_runtime_modules = [
        "src/gui_runtime/native_vello/generic_runtime.rs",
        "src/gui_runtime/native_vello/generic_runtime/present.rs",
        "src/gui_runtime/native_vello/generic_runtime/runner.rs",
        "src/gui_runtime/native_vello/generic_runtime/lifecycle.rs",
        "src/gui_runtime/native_vello/generic_runtime/frame_prepare.rs",
        "src/gui_runtime/native_vello/generic_runtime/scene.rs",
        "src/gui_runtime/native_vello/generic_runtime/auxiliary.rs",
    ];

    assert_eq!(
        scene_texture.matches("catch_unwind").count(),
        1,
        "the native renderer unwind boundary should have one local catcher"
    );
    assert!(
        scene_texture.contains("std::panic::AssertUnwindSafe"),
        "the native renderer unwind boundary should use the narrowly justified assertion"
    );
    assert!(
        scene_texture.contains("render_to_texture(")
            && scene_texture.contains("catch_renderer_unwind"),
        "the local catcher should surround the sole production render invocation"
    );

    for relative in broad_runtime_modules {
        let source = read_runtime_source(relative);
        assert!(
            !source.contains("catch_unwind") && !source.contains("AssertUnwindSafe"),
            "broad native runtime module {relative} must not catch renderer or application panics"
        );
    }
}

#[test]
fn native_present_propagates_direct_and_cached_scene_failures_before_completion() {
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");
    let redraw_start = present
        .find("pub(super) fn redraw(")
        .expect("present driver should define redraw");
    let redraw_end = present
        .find("pub(super) fn redraw_and_exit_on_error")
        .expect("present driver should define redraw error handoff");
    let redraw = &present[redraw_start..redraw_end];
    let direct_render = redraw
        .find("render_scene_to_surface_view(")
        .expect("direct resize should render through the focused scene-texture adapter");
    let cached_render = redraw
        .find("render_scene_texture_if_needed(")
        .expect("normal redraw should render through the focused scene-texture adapter");
    let direct_completion = redraw
        .find("self.finish_direct_resize_present(")
        .expect("direct resize should retain its success completion path");
    let cached_completion = redraw
        .find("present_base_frame(")
        .expect("normal redraw should retain its success completion path");

    assert!(
        redraw.contains("Result<(), NativeGenericRunError>")
            && redraw.matches(")?").count() >= 2
            && direct_render < direct_completion
            && cached_render < cached_completion,
        "both scene render paths should return before their success-only presentation bookkeeping"
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
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");
    let scene = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/scene.rs");

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
        runner.contains("self.frame.refresh_gpu_surface_interaction_regions();")
            && !scene.contains("gpu_surface_interaction_regions")
            && !scene.contains("GpuSurfaceInteractionRegion::from_gpu_surface"),
        "normal scene rebuilds should refresh shared clipped GPU interaction regions after encoding"
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
            && frame_state.contains("GpuSurfaceInteractionRegion")
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
            && !frame_state.contains("gpu_surface_renderer: GpuSurfaceRenderer")
            && !frame_state.contains("composited_base_frame: Option<CompositedBaseFrame>")
            && !frame_state.starts_with("use super::*;"),
        "native frame state should name CPU renderer, cache, runtime, theme, and scene dependencies without retaining generation-bound GPU owners"
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
fn native_device_callbacks_use_shared_callback_and_user_event_boundaries() {
    let device = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/device.rs");
    let adapter = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/adapter.rs");
    let surface = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/surface.rs");
    let auxiliary =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs");
    let runtime_wakeup =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runtime_wakeup.rs");
    let lifecycle =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs");
    let runtime_event = read_runtime_source("src/gui_runtime/native_vello/runtime_event.rs");

    let initialization_sources =
        [adapter.as_str(), surface.as_str(), auxiliary.as_str()].join("\n");
    let callback = initialization_sources
        .find("install_device_loss_callback(")
        .expect("adapter initialization should install device-loss callbacks");
    let renderer = initialization_sources
        .find("Renderer::new")
        .expect("native initialization should retain renderer creation");

    assert_eq!(
        adapter.matches("install_device_loss_callback(").count(),
        1,
        "the adapter owner should install one shared callback pair"
    );
    assert!(
        !surface.contains("install_device_loss_callback(")
            && !auxiliary.contains("install_device_loss_callback(")
    );
    assert!(callback < renderer);
    assert!(device.contains("set_device_lost_callback"));
    assert_eq!(device.matches("on_uncaptured_error").count(), 1);
    assert!(surface.contains("event_proxy.clone()"));
    assert!(
        auxiliary.contains(
            "initialize_runtime(event_loop, parent_window, event_proxy.clone(), adapter)"
        )
    );
    assert!(runtime_wakeup.contains("event_loop_proxy"));
    assert!(lifecycle.contains("RuntimeUserEvent::DeviceLost"));
    assert!(lifecycle.contains("RuntimeUserEvent::RenderDeviceError"));
    assert!(runtime_event.contains("Arc<DeviceLossRegistration>"));
    assert!(runtime_event.contains("NativeAdapterGeneration"));
    assert!(runtime_event.contains("NativeRenderDeviceErrorKind"));
    assert!(runtime_event.contains("Arc::ptr_eq"));
}

#[test]
fn native_device_callbacks_are_witness_scoped_without_broad_panic_handling() {
    let adapter = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/adapter.rs");
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");
    let runner_state =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner_state.rs");
    let device = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/device.rs");
    let runtime_modules = [
        "src/gui_runtime/native_vello/generic_runtime.rs",
        "src/gui_runtime/native_vello/generic_runtime/present.rs",
        "src/gui_runtime/native_vello/generic_runtime/lifecycle.rs",
        "src/gui_runtime/native_vello/generic_runtime/auxiliary.rs",
        "src/gui_runtime/native_vello/generic_runtime/adapter.rs",
    ];

    assert!(
        adapter.contains("pub(super) struct GenericNativeAdapterOwner")
            && adapter.contains("struct SelectedNativeAdapter")
            && adapter.contains("selected: Option<SelectedNativeAdapter>")
            && adapter.contains("generation: NativeAdapterGeneration")
            && adapter.contains("device_loss_registration: Arc<DeviceLossRegistration>")
            && adapter.contains("let selected =")
            && adapter.contains("self.selected = Some(selected)")
            && adapter.contains("fn accepts_device_loss")
            && adapter.contains("device_loss_registration_matches")
            && adapter.contains("Arc::ptr_eq")
            && !adapter.contains("selected_device_id: Option")
            && !adapter.contains("selected_backend: Option")
            && !adapter.contains("device_loss_registration: Option"),
        "the adapter owner should publish one complete generation-tagged selection record"
    );
    assert!(
        runner.contains("adapter: Option<GenericNativeAdapterOwner>")
            && runner.contains("device_loss_event_is_current")
            && runner.contains("adapter.accepts_device_loss(generation, registration)")
            && !runner.contains("Arc::ptr_eq")
            && !runner.contains("device_loss_registration")
            && !runner.contains("RenderContext"),
        "the runner should delegate callback admission to the current adapter owner"
    );
    assert!(
        !runner_state.contains("DeviceLossRegistration")
            && !runner_state.contains("device_loss_registration")
            && !runner_state.contains("RenderContext"),
        "window state should not retain callback witnesses or the adapter render context"
    );
    assert!(runner.contains("handle_render_device_error_event"));
    assert!(runner.contains("NativeGenericRunError::RenderDeviceError"));
    assert!(device.contains("DeviceLostReason::Destroyed"));
    assert!(device.contains("Arc<DeviceLossRegistration>"));
    assert!(device.contains("generation: NativeAdapterGeneration"));
    assert!(device.contains("classify_uncaptured_error"));
    assert!(device.contains("RENDER_DEVICE_ERROR_MESSAGE_FALLBACK"));
    assert!(!device.contains("catch_unwind") && !device.contains("AssertUnwindSafe"));
    for module in runtime_modules {
        let source = read_runtime_source(module);
        assert!(
            !source.contains("catch_unwind") && !source.contains("AssertUnwindSafe"),
            "device-loss admission must not broaden renderer panic containment in {module}"
        );
    }
}

#[test]
fn native_auxiliary_windows_borrow_the_owner_without_advancing_generation() {
    let auxiliary =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs");

    assert!(auxiliary.contains("adapter: &mut GenericNativeAdapterOwner"));
    assert!(auxiliary.contains("NativeAdapterGeneration"));
    assert!(auxiliary.contains("capture_generation"));
    for forbidden in ["select_primary_device", "next_generation", ".advance("] {
        assert!(
            !auxiliary.contains(forbidden),
            "auxiliary windows must not select or advance the shared adapter generation through `{forbidden}`"
        );
    }
}
