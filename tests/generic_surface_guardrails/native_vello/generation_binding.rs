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
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("generic surface source should be readable");

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
    let capacity_check = surface
        .find("can_publish_native_resources")
        .expect("surface initialization should preflight native resource capacity");
    let window_creation = surface
        .find("create_window")
        .expect("surface initialization should create its window");
    assert!(
        capacity_check < window_creation,
        "native resource capacity should be checked before window/GPU setup"
    );
    let publication_reservation = surface
        .find("reserve_native_resource_publication")
        .expect("surface initialization should reserve publication capacity");
    let surface_creation = surface
        .find(".create_surface(")
        .expect("surface initialization should create a WGPU surface");
    assert!(
        publication_reservation < surface_creation,
        "publication capacity should be reserved before WGPU surface creation"
    );
    for candidate in [
        "let candidate_native_dpi_scale =",
        "let candidate_dpi_scale =",
        "let candidate_monitor_fingerprint =",
        "let candidate_accessibility_display =",
        "let candidate_environment =",
        "let candidate_viewport =",
    ] {
        let candidate_position = surface
            .find(candidate)
            .unwrap_or_else(|| panic!("surface initialization should derive `{candidate}`"));
        assert!(
            candidate_position < publication_reservation,
            "candidate `{candidate}` should be derived before publication reservation"
        );
    }
    let publication_commit = surface
        .find("native_resource_publication.publish(native_resources)")
        .expect("surface initialization should commit the reserved resource publication");
    let window_metadata = surface
        .find("self.window.id = Some(window.id())")
        .expect("surface initialization should publish window metadata");
    assert!(
        publication_commit < window_metadata,
        "window metadata should follow successful native resource publication"
    );
    for metadata_commit in [
        "self.window.native_dpi_scale = candidate_native_dpi_scale",
        "self.window.dpi_scale = candidate_dpi_scale",
        "self.window.monitor_fingerprint = candidate_monitor_fingerprint",
        "self.window.accessibility_display = candidate_accessibility_display",
        "self.window.environment = candidate_environment",
    ] {
        let metadata_position = surface
            .find(metadata_commit)
            .unwrap_or_else(|| panic!("surface initialization should commit `{metadata_commit}`"));
        assert!(
            publication_commit < metadata_position,
            "window metadata `{metadata_commit}` should follow successful native resource publication"
        );
    }
    let environment_update = surface
        .find("set_window_environment(candidate_environment)")
        .expect("startup should commit the candidate environment to the core");
    let environment_refresh = surface
        .find("self.core.refresh_surface()")
        .expect("startup should refresh before rebuilding after an environment change");
    let viewport_commit = surface
        .find("self.core.set_viewport(candidate_viewport)")
        .expect("startup should commit the candidate viewport to the core");
    let scene_rebuild = surface
        .find("self.rebuild_scene()")
        .expect("startup should rebuild the first scene after candidate commit");
    assert!(
        publication_commit < environment_update
            && environment_update < environment_refresh
            && environment_refresh < viewport_commit
            && viewport_commit < scene_rebuild,
        "startup environment refresh and viewport commit should follow publication before the first scene rebuild"
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
