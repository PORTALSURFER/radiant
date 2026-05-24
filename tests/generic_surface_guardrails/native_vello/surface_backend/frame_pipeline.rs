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
            && state.contains("struct NativeRunnerTimingState"),
        "native runner state groups should stay in runner_state.rs"
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
            && surface_size.contains("fn logical_size"),
        "direct WGPU target size conversion should stay centralized instead of repeating raw config casts"
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
