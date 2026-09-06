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
fn retained_window_gpu_state_is_generation_owned_and_admitted() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");
    let frame_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/frame_state.rs"),
    )
    .expect("generic frame state source should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("generic present source should be readable");
    for required in [
        "struct NativeWindowGpuResources",
        "gpu_surface_renderer: GpuSurfaceRenderer",
        "post_gpu_overlay_renderer: PostGpuOverlayRenderer",
        "composited_base_frame: Option<CompositedBaseFrame>",
        "composited_base_frame_retirement: Option<CompositedBaseFrameRetirement>",
        "gpu_timing: NativeGpuTimingResources",
        "gpu_resources: NativeWindowGpuResources",
        "NativeWindowGpuResources::new_with_timing(",
    ] {
        assert!(
            runner_state.contains(required),
            "generation-bound native resources should contain `{required}`"
        );
    }
    for escaped_owner in [
        "gpu_surface_renderer:",
        "post_gpu_overlay_renderer:",
        "composited_base_frame:",
    ] {
        assert!(
            !frame_state.contains(escaped_owner),
            "CPU-only native frame state must not retain `{escaped_owner}`"
        );
    }
    assert!(
        frame_state.contains("renderer: &mut PostGpuOverlayRenderer")
            && !frame_state.contains("post_gpu_overlay_renderer,"),
        "post-GPU overlay rendering should borrow its renderer from the admitted bundle"
    );
    assert!(
        present.contains("let gpu_resources = &mut resources.gpu_resources")
            && present.contains("gpu_resources.composited_base_frame")
            && present.contains("gpu_resources.composited_base_frame_retirement")
            && present.contains("gpu_resources.gpu_surface_renderer")
            && present.contains("gpu_resources.post_gpu_overlay_renderer")
            && !present.contains("self.frame.gpu_surface_renderer")
            && !present.contains("self.frame.post_gpu_overlay_renderer")
            && !present.contains("self.frame.composited_base_frame"),
        "presentation should access retained GPU state through the active resource bundle"
    );
    assert!(
        present.matches("admit_native_resources(adapter)").count() >= 2
            && present.contains("native_encode_present_ticket_is_current(")
            && present.contains("let mut ticket = Some(ticket);")
            && !present[present
                .find("let mut ticket = Some(ticket);")
                .expect("presentation should retain its admitted ticket")..]
                .contains("self.admit_native_resources(adapter)"),
        "presentation should admit the active generation before ticket creation and use pure ticket currentness checks for every later GPU phase"
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
        present.matches("admit_native_resources(adapter)").count() >= 2
            && present.contains("native_encode_present_ticket_is_current(")
            && present.contains("resources.renderer")
            && present.contains("resources.render_surface")
            && !present[present
                .find("let mut ticket = Some(ticket);")
                .expect("presentation should retain its admitted ticket")..]
                .contains("self.admit_native_resources(adapter)"),
        "render and presentation should use the admitted atomic bundle without recovery-capable re-admission after ticket creation"
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

#[test]
fn native_submission_completion_is_exact_generation_and_covers_both_present_paths() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let completion = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/submission_completion.rs"),
    )
    .expect("native submission completion source should be readable");
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("generic present source should be readable");
    let scene_texture = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene_texture.rs"),
    )
    .expect("generic scene texture source should be readable");

    for required in [
        "NativeSubmissionCompletionCapability",
        "generation: NativeAdapterGeneration",
        "device: wgpu::Device",
        "queue: wgpu::Queue",
        "on_submitted_work_done",
        "PollType::Poll",
        "RuntimeUserEvent::NativeResourceMaintenanceRequested",
    ] {
        assert!(
            completion.contains(required),
            "completion witnessing should retain exact-generation capability `{required}`"
        );
    }
    assert!(
        !completion.contains("PollType::Wait"),
        "completion maintenance must never use a blocking WGPU poll"
    );
    assert!(
        runner_state.contains("completion_witness")
            && runner_state.contains("&wgpu::Device")
            && runner_state.contains("&wgpu::Queue"),
        "each native resource bundle should own its exact queue progress witness"
    );
    assert!(
        scene_texture.contains("context.renderer.render_to_texture(")
            && scene_texture.contains("context.completion_witness")
            && present.contains("render_scene_to_surface_view("),
        "direct-resize rendering should use Vello's internal render_to_texture submission and its exact-generation witness"
    );

    let direct_render = present
        .find("render_scene_to_surface_view(")
        .expect("direct-resize path should render through Vello");
    let direct_present = present
        .find("surface_texture.present()")
        .expect("direct-resize path should present its surface texture");
    assert!(
        direct_render < direct_present,
        "direct-resize completion witnessing should remain inside the shared Vello boundary before presentation"
    );
    assert!(
        !present.contains("resources.record_successful_native_submission()"),
        "direct-resize must not double-record the Vello submission after the shared boundary witnesses it"
    );
    let scene_render = scene_texture
        .find("context.renderer.render_to_texture(")
        .expect("the shared scene-texture boundary should invoke Vello");
    let scene_success_witness = scene_texture
        .find("context.completion_witness.record_successful_submission()")
        .expect("the shared scene-texture boundary should witness successful Vello work");
    assert!(
        scene_render < scene_success_witness,
        "successful Vello work should be witnessed immediately after the shared render call"
    );

    let ordinary_submit = present
        .find("dev_handle.queue.submit(std::iter::once(encoder.finish()))")
        .expect("ordinary presentation should submit its final explicit command buffer");
    let ordinary_witness = present
        .find("self.record_successful_native_submission()")
        .expect("ordinary presentation should record its final explicit submission");
    let ordinary_present = present
        .rfind("surface_texture.present()")
        .expect("ordinary presentation should present after its final queue submission");
    assert!(
        ordinary_submit < ordinary_witness && ordinary_witness < ordinary_present,
        "ordinary completion witnessing should follow Radiant's final explicit queue submit"
    );
}

#[test]
fn native_resource_maintenance_is_shared_bounded_and_nonblocking() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("generic lifecycle source should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("generic runner source should be readable");
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("generic auxiliary source should be readable");
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("generic surface source should be readable");

    assert!(
        runner.contains("self.window.maintain_native_resources(turn)")
            && runner.contains("retain_mut(|window|")
            && runner.contains(
                "!window.maintain_native_resources_with_turn(turn, adapter.as_deref_mut())",
            )
            && runner.contains("pub(super) fn retire_native_resources_with_turn("),
        "primary and auxiliary bundles should share one maintenance turn and retire children only after GPU ownership is empty"
    );
    assert!(
        auxiliary.contains("sync_auxiliary_windows_with_adapter_in_turn")
            && auxiliary.contains("NativeResourceMaintenanceTurn"),
        "auxiliary replacement should retain the shared event-loop maintenance boundary"
    );
    let maintenance = lifecycle
        .find("begin_native_resource_maintenance()")
        .expect("lifecycle should maintain native resources before initialization checks");
    let initialize = lifecycle
        .find("initialize_runtime(event_loop")
        .expect("lifecycle should initialize through the maintenance-aware boundary");
    assert!(
        maintenance < initialize,
        "completed quarantine capacity should be reclaimed before runtime preflight"
    );
    assert!(
        lifecycle.contains("ControlFlow::WaitUntil")
            && lifecycle.contains("ControlFlow::Poll => {}")
            && lifecycle.contains("NATIVE_RESOURCE_MAINTENANCE_INTERVAL"),
        "pending maintenance should use a bounded future deadline without forcing Poll"
    );
    let completion_wake_start = lifecycle
        .find("RuntimeUserEvent::NativeResourceMaintenanceRequested => {")
        .expect("completion wake should be handled at the native event-loop boundary");
    let completion_wake = lifecycle[completion_wake_start..]
        .split("} else if self.is_running() {")
        .nth(1)
        .and_then(|branch| branch.split("#[cfg(target_os = \"macos\")]\n").next())
        .expect("running completion wake branch should be present");
    assert!(
        completion_wake.contains("self.wake_normal_native_resource_maintenance();")
            && completion_wake
                .contains("window.wake_normal_native_resource_maintenance(generation);")
            && !completion_wake.contains("begin_native_resource_maintenance")
            && !completion_wake.contains("request_redraw")
            && !completion_wake.contains("FrameWork"),
        "completion callbacks should wake primary and auxiliary maintenance only"
    );
    assert!(
        lifecycle.contains("selected_lane == FrameScheduleLane::Maintenance")
            && lifecycle.contains("self.admit_native_resource_maintenance(")
            && runner.contains("pub(super) fn admit_native_resource_maintenance(")
            && runner.contains(".maintain_native_resource_slot(binding, turn)")
            && runner.contains("advance_native_resource_maintenance_cursor")
            && !lifecycle.contains("begin_native_resource_maintenance_and_wake_primary")
            && !runner.contains("begin_native_resource_maintenance_and_wake_primary"),
        "normal Running maintenance should use the selected exact Maintenance lane and slot ticket"
    );
    let runner_admission_start = runner
        .find("pub(super) fn admit_native_resource_maintenance(")
        .expect("runner should expose the normal maintenance admission boundary");
    let runner_admission_end = runner[runner_admission_start..]
        .find("\n    pub(super) fn maintain_native_resources_with_turn(")
        .expect("runner maintenance admission should end before broad maintenance")
        + runner_admission_start;
    let runner_admission = &runner[runner_admission_start..runner_admission_end];
    assert!(
        runner_admission.contains("turn: &mut NativeResourceMaintenanceTurn")
            && runner_admission.contains("maintain_native_resource_slot(binding, turn)")
            && !runner_admission.contains("NativeResourceMaintenanceTurn::new()"),
        "ordinary runner admission must use the caller-owned maintenance turn"
    );
    let auxiliary_admission_start = auxiliary
        .find("pub(super) fn admit_native_resource_maintenance(")
        .expect("auxiliary should expose the normal maintenance admission boundary");
    let auxiliary_admission_end = auxiliary[auxiliary_admission_start..]
        .find("\n    pub(super) fn admit_frame_schedule_work(")
        .expect("auxiliary maintenance admission should end before scheduled frame work")
        + auxiliary_admission_start;
    let auxiliary_admission = &auxiliary[auxiliary_admission_start..auxiliary_admission_end];
    assert!(
        auxiliary_admission.contains("turn: &mut NativeResourceMaintenanceTurn")
            && auxiliary_admission.contains("self.runner.admit_native_resource_maintenance(")
            && auxiliary_admission.contains("turn,")
            && !auxiliary_admission.contains("NativeResourceMaintenanceTurn::new()"),
        "auxiliary admission must forward the caller-owned maintenance turn"
    );
    let about_to_wait_start = lifecycle
        .find("    fn about_to_wait(")
        .expect("lifecycle should retain one AboutToWait maintenance boundary");
    let about_to_wait_end = lifecycle[about_to_wait_start..]
        .find("\nfn native_resource_maintenance_deadline(")
        .expect("AboutToWait should end before standalone deadline helpers")
        + about_to_wait_start;
    let about_to_wait = &lifecycle[about_to_wait_start..about_to_wait_end];
    assert_eq!(
        about_to_wait
            .matches("NativeResourceMaintenanceTurn::new()")
            .count(),
        1,
        "AboutToWait should construct exactly one parent-owned maintenance turn"
    );
    assert!(
        about_to_wait.matches("&mut maintenance").count() >= 4,
        "target retirement, retiring cleanup, and selected primary/auxiliary maintenance should share the AboutToWait turn"
    );
    let primary_maintenance_start = about_to_wait
        .find("                FrameScheduleKey::Primary => {")
        .expect("AboutToWait should retain the primary ordinary maintenance branch");
    let primary_maintenance_end = about_to_wait[primary_maintenance_start..]
        .find("\n                FrameScheduleKey::Auxiliary(key) => {")
        .map(|end| primary_maintenance_start + end)
        .unwrap_or(about_to_wait.len());
    assert!(
        about_to_wait[primary_maintenance_start..primary_maintenance_end]
            .contains("!retiring_auxiliary_maintenance_due"),
        "primary ordinary maintenance should be gated when retiring cleanup is due"
    );
    let auxiliary_maintenance_start = about_to_wait
        .find("                FrameScheduleKey::Auxiliary(key) => {")
        .expect("AboutToWait should retain the auxiliary ordinary maintenance branch");
    assert!(
        about_to_wait[auxiliary_maintenance_start..]
            .contains("!retiring_auxiliary_maintenance_due"),
        "auxiliary ordinary maintenance should be gated when retiring cleanup is due"
    );
    let composited_pending_start = runner_state
        .find("    fn composited_base_frame_maintenance_pending(")
        .expect("runner state should retain composited-base pending detection");
    let composited_pending_end = runner_state[composited_pending_start..]
        .find("\n    fn maintain_composited_base_frame(")
        .expect("composited-base pending detection should end before maintenance")
        + composited_pending_start;
    let composited_pending = &runner_state[composited_pending_start..composited_pending_end];
    assert!(
        composited_pending.contains("completed_through(retirement.identity().completion())")
            && composited_pending.contains("let historical_completion")
            && composited_pending.contains("let current_witness_progress")
            && composited_pending.contains("historical_completion || current_witness_progress"),
        "composited-base pending detection should retain stored completion evidence while preserving current-witness progress"
    );
    assert!(
        surface.contains("maintenance: &mut NativeResourceMaintenanceTurn")
            && surface.contains("sync_auxiliary_windows_with_adapter_in_turn"),
        "startup should carry one maintenance turn through primary and auxiliary setup"
    );
}

#[test]
fn destructive_auxiliary_close_is_retiring_and_projection_vetoed() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("generic auxiliary source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("generic lifecycle source should be readable");
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");

    for required in [
        "enum AuxiliaryNativeWindowLifecycle",
        "Retiring",
        "self.lifecycle = AuxiliaryNativeWindowLifecycle::Retiring",
        "self.runner.core.runtime.begin_closing()",
        "self.close_message.take()",
        "if self.is_retiring()",
        "fn auxiliary_key_is_retiring",
        "window.is_admitted() && window.key() == projection.key",
        "window.update_projection_with_commands(projection, command_service.clone())",
        "if !self.is_admitted() || self.recovery_rebuild_pending",
        "self.show();",
    ] {
        assert!(
            auxiliary.contains(required),
            "destructive auxiliary lifecycle should retain `{required}`"
        );
    }
    assert!(
        auxiliary.contains("return AuxiliaryWindowEventResult::ignored()")
            && auxiliary.contains("then_some(self.runner.window.id)"),
        "retiring children should be non-routable before event handling"
    );
    assert!(
        lifecycle.contains("let AuxiliaryWindowEventResult {")
            && !lifecycle.contains("auxiliary_windows.remove(index)"),
        "close handling should retain the child for maintenance retirement"
    );
    for required in [
        "fn retire_native_resource_entries",
        "if quarantine.is_full()",
        "turn.record_pending()",
        "active.is_none() && quarantine.is_empty()",
        "NativeWindowResourceBundle::maintain_completion",
        "NativeWindowResourceBundle::retirement_eligible",
    ] {
        assert!(
            runner_state.contains(required),
            "native retirement should retain bounded witness logic `{required}`"
        );
    }
}

#[test]
fn native_whole_run_closing_is_central_bounded_and_nonblocking() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generic_runtime =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic runtime source should be readable");
    let closing = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/closing.rs"),
    )
    .expect("native closing policy source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("generic lifecycle source should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("generic runner source should be readable");
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("generic auxiliary source should be readable");
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("generic surface source should be readable");
    let sources = [
        &generic_runtime,
        &closing,
        &lifecycle,
        &runner,
        &auxiliary,
        &surface,
    ];

    for required in [
        "mod closing;",
        "NativeLifecycle",
        "Running",
        "Closing",
        "Stopped",
        "NATIVE_CLOSING_MAX_MAINTENANCE_OPPORTUNITIES",
        "NATIVE_CLOSING_MAX_DURATION",
        "observe_closing_opportunity",
    ] {
        assert!(
            closing.contains(required) || generic_runtime.contains(required),
            "native closing should retain `{required}`"
        );
    }
    assert!(
        runner.contains("pub(super) fn admit_native_shutdown")
            && runner.contains("self.core.runtime.begin_closing()")
            && runner.contains("window.begin_whole_run_retiring(event_loop)")
            && runner.contains("self.fence_native_presentation()"),
        "all whole-run terminal paths should converge on one admission fence"
    );
    assert!(
        lifecycle.contains("if self.is_closing()")
            && lifecycle.contains("self.advance_native_closing(event_loop, Instant::now())")
            && lifecycle.contains("ControlFlow::WaitUntil"),
        "closing should continue only through bounded maintenance wakeups"
    );
    let closing_schedule = runner.find("fn schedule_native_closing").and_then(|start| {
        runner[start..]
            .find("\n    }")
            .map(|end| &runner[start..start + end])
    });
    assert!(
        closing_schedule.is_some_and(|schedule| {
            schedule.contains("ControlFlow::WaitUntil") && !schedule.contains("ControlFlow::Poll")
        }) && !closing.contains("ControlFlow::Poll"),
        "closing policy must not select a polling control flow"
    );
    assert!(
        runner.contains("NativeResourceMaintenanceTurn::new()")
            && runner.contains("retire_all_native_resources_with_turn")
            && runner
                .contains(
                    "retain_mut(|window| {\n            !window.maintain_native_resources_with_turn(turn, adapter.as_mut())",
                )
            && runner.contains("self.auxiliary_windows.is_empty()"),
        "primary and auxiliary retirement should share one turn and retain unresolved ownership"
    );
    assert!(
        surface.contains("pub(super) fn fence_native_presentation")
            && runner.contains("if !self.is_running()")
            && lifecycle.contains("RuntimeUserEvent::NativeResourceMaintenanceRequested"),
        "presentation and normal native admission should be fenced while completion wakes remain allowed"
    );
    let whole_run_retirement = auxiliary
        .find("pub(super) fn begin_whole_run_retiring")
        .expect("auxiliary whole-run retirement should be explicit");
    let whole_run_retirement_end = auxiliary[whole_run_retirement..]
        .find("\n    fn handle_close_requested")
        .map_or(auxiliary.len(), |offset| whole_run_retirement + offset);
    assert!(
        !auxiliary[whole_run_retirement..whole_run_retirement_end].contains("close_message.take()"),
        "whole-run auxiliary retirement must not dispatch configured close messages"
    );
    assert_eq!(
        sources
            .iter()
            .map(|source| source.matches("event_loop.exit()").count())
            .sum::<usize>(),
        1,
        "native event-loop exit should remain inside the central shutdown stop helper"
    );
    for forbidden in [
        "PollType::Wait",
        "std::thread::sleep",
        "thread::sleep",
        ".join(",
        "spin_loop",
        "auxiliary_windows.clear()",
    ] {
        assert!(
            !sources.iter().any(|source| source.contains(forbidden)),
            "native closing must not use `{forbidden}`"
        );
    }
}

#[test]
fn device_loss_recovery_is_private_async_and_never_reuses_old_generation() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let recovery = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/recovery.rs"),
    )
    .expect("native recovery source should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("generic runner source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("generic lifecycle source should be readable");
    let adapter = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/adapter.rs"),
    )
    .expect("generic adapter source should be readable");
    let device = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/device.rs"),
    )
    .expect("generic device source should be readable");
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("generic auxiliary source should be readable");
    let runner_state = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner_state.rs"),
    )
    .expect("generic runner state source should be readable");

    for required in [
        "NativeRecoveryEpisodeToken",
        "sync_channel(1)",
        "try_recv()",
        "RadiantWgpuContext",
        "previous_device_identity",
        "candidate_starts",
        "candidate_completions",
        "max_in_flight",
        "AtomicBool",
        "OnceLock",
        "RecoveryFutureError::Cancelled",
        "acknowledge",
        "has_in_flight_candidate",
    ] {
        assert!(
            recovery.contains(required),
            "recovery should retain bounded async evidence `{required}`"
        );
    }
    for required in [
        "RadiantWgpuContext",
        "RadiantWgpuDevice",
        "devices: Vec::new()",
        "DeviceFeatureSelection::for_adapter",
    ] {
        assert!(
            adapter.contains(required),
            "adapter should retain private Radiant device ownership and feature selection `{required}`"
        );
    }
    assert!(
        device.contains("TIMESTAMP_QUERY")
            && device.contains("CLEAR_TEXTURE")
            && device.contains("PIPELINE_CACHE"),
        "device policy should own timestamp and Vello optional feature selection"
    );
    let request_policy = adapter
        .split("async fn request_device_with_fallback")
        .nth(1)
        .and_then(|source| source.split("fn device_descriptor").next())
        .expect("adapter should retain one centralized device request policy");
    assert!(
        request_policy.contains("drop(adapter)")
            && request_policy.contains("initialize_adapter_from_env_or_default")
            && request_policy.contains("compatible_surface")
            && request_policy.contains("fallback_adapter")
            && request_policy.contains(
                "DeviceFeatureSelection::for_adapter(\n                fallback_adapter.get_info().backend,\n                fallback_adapter.features(),\n            )",
            )
            && request_policy.contains("baseline_request()")
            && request_policy.contains("adapter: fallback_adapter"),
        "timestamp fallback should recompute its baseline from the fresh adapter and retain it using the same instance and surface policy"
    );
    assert_eq!(
        request_policy.matches(".request_device(").count(),
        2,
        "the initial request and one fresh-adapter baseline retry are the only device requests"
    );
    for forbidden in ["RenderContext", "DeviceHandle", "wgpu-profiler"] {
        assert!(
            !adapter.contains(forbidden) && !recovery.contains(forbidden),
            "generic adapter/recovery must not retain Vello convenience ownership or profiling dependency `{forbidden}`"
        );
    }
    for forbidden in [
        "pollster::block_on",
        "PollType::Wait",
        ".recv(",
        ".join(",
        "thread::sleep",
        "std::thread::sleep",
        "create_window(",
        "auxiliary_windows.clear()",
        "pub fn",
    ] {
        assert!(
            !recovery.contains(forbidden),
            "recovery must not introduce `{forbidden}`"
        );
    }
    assert!(
        !runner_state.contains("quarantined_native_resources.clear")
            && !recovery.contains("quarantined_native_resources.clear"),
        "recovery must preserve bounded quarantine ownership"
    );
    assert!(
        recovery.contains("cancelled: AtomicBool")
            && recovery.contains("fn cancel(&self)")
            && recovery.contains("thread.unpark()")
            && recovery.contains("fn is_cancelled(&self)")
            && recovery.contains("thread::park()")
            && recovery.contains("if cancellation.is_cancelled()"),
        "recovery worker parking must remain cancellation-wakeable and cancellation-fenced"
    );
    assert!(
        lifecycle.contains("recovery_deadline")
            && lifecycle.contains("recovery_expired")
            && lifecycle.contains("ControlFlow::WaitUntil")
            && runner.contains("self.recovery.acknowledge")
            && runner.contains("!self.recovery.has_in_flight_candidate()"),
        "recovery must have a deadline, retain the cancellation episode, and acknowledge late completion"
    );
    assert!(
        adapter.contains("is_strictly_newer_than")
            && adapter.contains("from_fresh_recovery_context")
            && recovery.contains("context.device(Some(&surface))"),
        "recovery must select through a fresh context and require a newer generation"
    );
    assert!(
        lifecycle.contains("DeviceRecoveryReady")
            && lifecycle.contains("self.is_recovering()")
            && runner.contains("handle_device_recovery_ready"),
        "the event loop should admit only the private recovery completion event"
    );
    let ready_handler = runner
        .find("pub(super) fn handle_device_recovery_ready")
        .expect("device-recovery-ready handler should remain explicit");
    let ready_handler_end = runner[ready_handler..]
        .find("\n    fn commit_device_recovery_candidate")
        .map_or(runner.len(), |offset| ready_handler + offset);
    let ready_handler_source = &runner[ready_handler..ready_handler_end];
    let expiry_check = ready_handler_source
        .find("self.recovery_expired(Instant::now())")
        .expect("recovery-ready admission should check the fixed deadline");
    let expiry_guard_source = &ready_handler_source[expiry_check..];
    let acknowledge = expiry_guard_source
        .find("self.recovery.acknowledge(episode)")
        .expect("an overdue matching recovery episode should be acknowledged");
    let shutdown = expiry_guard_source
        .find("self.admit_native_shutdown(event_loop, None);")
        .expect("an overdue matching recovery episode should enter central shutdown");
    let take_ready = expiry_guard_source
        .find("self.recovery.take_ready(episode)")
        .expect("normal recovery completion should still take a ready candidate");
    let candidate_commit = expiry_guard_source
        .find("self.commit_device_recovery_candidate(candidate)")
        .expect("normal recovery completion should still commit the candidate");
    assert!(
        ready_handler_source.contains("recovery_completion_is_admissible")
            && ready_handler_source.contains("if self.recovery.acknowledge(episode)")
            && acknowledge < shutdown
            && shutdown < take_ready
            && take_ready < candidate_commit
            && expiry_guard_source.contains("return;"),
        "overdue recovery completion must be acknowledged and shut down before candidate extraction or publication"
    );
    assert!(
        auxiliary.contains("recovery_rebuild_pending")
            && auxiliary.contains("recovery_opportunity")
            && auxiliary.contains("quarantine_device_recovery_resources"),
        "auxiliary recovery should remain lazy, bounded, and retirement-aware"
    );
    let recovery_rebuild_start = auxiliary
        .find("let mut recovery_opportunity")
        .expect("auxiliary recovery rebuild admission should remain explicit");
    let recovery_rebuild_end = auxiliary[recovery_rebuild_start..]
        .find("let projections")
        .map_or(auxiliary.len(), |offset| recovery_rebuild_start + offset);
    let recovery_rebuild_source = &auxiliary[recovery_rebuild_start..recovery_rebuild_end];
    assert!(
        recovery_rebuild_source.contains("let rebuild_result")
            && recovery_rebuild_source.contains("if let Err(error) = rebuild_result")
            && recovery_rebuild_source.contains("take_deferred_auxiliary_recovery_failure_cause")
            && recovery_rebuild_source
                .contains("self.admit_native_shutdown(event_loop, Some(cause));")
            && recovery_rebuild_source
                .contains("self.request_redraw_for_frame_work(FrameWork::None);")
            && !recovery_rebuild_source
                .contains("rebuild_after_device_recovery(adapter, event_proxy.clone())?"),
        "deferred auxiliary recovery failures must enter central bounded shutdown with the retained cause rather than escape into a discarded wrapper result"
    );

    let loss_handler = runner
        .find("pub(super) fn handle_device_lost_event")
        .expect("device-loss handler should remain explicit");
    let loss_handler_end = runner[loss_handler..]
        .find("\n    fn can_prepare_device_recovery")
        .map_or(runner.len(), |offset| loss_handler + offset);
    let loss_handler_source = &runner[loss_handler..loss_handler_end];
    assert!(
        loss_handler_source.contains("begin_device_recovery")
            && !loss_handler_source.contains("admit_native_shutdown")
            && !loss_handler_source.contains("record_render_device_lost_and_exit"),
        "an accepted current DeviceLost event must enter recovery rather than direct Closing"
    );
}
