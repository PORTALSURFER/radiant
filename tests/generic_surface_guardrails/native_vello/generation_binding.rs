//! Native adapter-generation binding guardrails.

use super::*;

#[test]
fn generic_native_window_resources_are_one_generation_bound_bundle() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");
    let adapter = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/adapter.rs"),
    )
    .expect("generic adapter source should be readable");

    for required in [
        "struct NativeWindowResourceBundle",
        "NativeAdapterGeneration",
        "RenderSurface<'static>",
        "Renderer",
        "native_resources: Option<NativeWindowResourceBundle>",
        "NativeResourceQuarantine<NativeWindowResourceBundle>",
        "quarantined_native_resources",
    ] {
        assert!(
            runner_state.contains(required),
            "generic window state should retain `{required}` in the atomic native bundle model"
        );
    }
    assert!(
        !runner_state.contains("render_surface: Option<RenderSurface<'static>>")
            && !runner_state.contains("renderer: Option<Renderer>"),
        "window state should not publish surface and renderer as parallel optionals"
    );
    assert!(
        runner_state.contains("MAX_QUARANTINED_NATIVE_RESOURCES")
            && runner_state.contains("fn try_push"),
        "quarantined native resources should have an explicit bounded admission boundary"
    );
    assert!(
        adapter.contains("fn capture_generation") && adapter.contains("fn admit_generation"),
        "the shared adapter owner should be the only production generation capture/admission boundary"
    );
}

#[test]
fn generic_native_resize_acquire_and_present_paths_admit_before_wgpu_work() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("generic surface source should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("generic present source should be readable");

    assert!(
        surface.matches("admit_native_resources(adapter)").count() >= 4,
        "resize, recovery resize, acquisition, and the shared admission boundary should all be fenced"
    );
    assert!(
        surface.contains("resources.render_surface.surface.get_current_texture()"),
        "surface acquisition should use only the admitted resource bundle"
    );
    assert!(
        present.matches("admit_native_resources(adapter)").count() >= 3
            && present.contains("resources.renderer")
            && present.contains("resources.render_surface"),
        "render and presentation should use the admitted atomic bundle"
    );
}

#[test]
fn auxiliary_windows_reuse_parent_adapter_initialization_without_selecting_generation() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("generic auxiliary source should be readable");
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("generic surface source should be readable");

    assert!(
        auxiliary.contains("initialize_runtime_with_adapter(event_loop, event_proxy, adapter)"),
        "auxiliary windows should use the shared adapter initialization path"
    );
    assert!(
        !auxiliary.contains("select_primary_device"),
        "auxiliary windows must not create or advance the shared adapter generation"
    );
    assert!(
        surface.contains("validate_auxiliary_surface") && surface.contains("capture_generation"),
        "the common initialization path should validate auxiliary compatibility and bind the current owner generation"
    );
}
