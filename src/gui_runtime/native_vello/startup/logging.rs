//! Native startup timing diagnostic emission.

use super::{StartupTimingProfile, artifact};

const STARTUP_PROFILE_ENV: &str = "RADIANT_NATIVE_STARTUP_PROFILE";
const STARTUP_PROFILE_LOG_PREFIX: &str = "[native-vello-startup]";

pub(super) fn startup_profile_enabled() -> bool {
    crate::env_flags::env_var_truthy(STARTUP_PROFILE_ENV)
}

pub(super) fn emit_summary_if_ready(profile: &mut StartupTimingProfile) {
    if profile.summary_emitted {
        return;
    }
    let Some(artifact) = artifact::export_completed_startup_timing_artifact(profile) else {
        return;
    };
    tracing::info!(
        window_create_ms = artifact.window_create_ms.unwrap_or_default(),
        window_revealed_ms = artifact.window_revealed_ms.unwrap_or_default(),
        wgpu_surface_create_ms = artifact.wgpu_surface_create_ms.unwrap_or_default(),
        wgpu_device_ready_ms = artifact.wgpu_device_ready_ms.unwrap_or_default(),
        surface_ready_ms = artifact.surface_ready_ms.unwrap_or_default(),
        renderer_build_ms = artifact.renderer_build_ms.unwrap_or_default(),
        renderer_ready_ms = artifact.renderer_ready_ms.unwrap_or_default(),
        first_scene_ready_ms = artifact.first_scene_ready_ms.unwrap_or_default(),
        first_redraw_started_ms = artifact.first_redraw_started_ms.unwrap_or_default(),
        first_present_draw_ms = artifact.first_present_draw_ms.unwrap_or_default(),
        first_present_ms = artifact.first_present_ms.unwrap_or_default(),
        deferred_model_refresh_ms = artifact.deferred_model_refresh_ms.unwrap_or_default(),
        deferred_model_refresh_total_ms =
            artifact.deferred_model_refresh_total_ms.unwrap_or_default(),
        "native vello startup timing summary"
    );
    if profile.enabled {
        eprintln!(
            "{STARTUP_PROFILE_LOG_PREFIX} window_create_ms={:.3} \
window_revealed_ms={:.3} \
wgpu_surface_create_ms={:.3} \
wgpu_device_ready_ms={:.3} \
surface_ready_ms={:.3} renderer_ready_ms={:.3} \
renderer_build_ms={:.3} first_scene_ready_ms={:.3} \
first_redraw_started_ms={:.3} \
first_present_draw_ms={:.3} first_present_ms={:.3} \
deferred_model_refresh_ms={:.3} \
deferred_model_refresh_total_ms={:.3}",
            artifact.window_create_ms.unwrap_or_default(),
            artifact.window_revealed_ms.unwrap_or_default(),
            artifact.wgpu_surface_create_ms.unwrap_or_default(),
            artifact.wgpu_device_ready_ms.unwrap_or_default(),
            artifact.surface_ready_ms.unwrap_or_default(),
            artifact.renderer_ready_ms.unwrap_or_default(),
            artifact.renderer_build_ms.unwrap_or_default(),
            artifact.first_scene_ready_ms.unwrap_or_default(),
            artifact.first_redraw_started_ms.unwrap_or_default(),
            artifact.first_present_draw_ms.unwrap_or_default(),
            artifact.first_present_ms.unwrap_or_default(),
            artifact.deferred_model_refresh_ms.unwrap_or_default(),
            artifact.deferred_model_refresh_total_ms.unwrap_or_default(),
        );
    }
    profile.summary_emitted = true;
}

pub(super) fn emit_failure_reason_if_needed(profile: &StartupTimingProfile) {
    let Some(reason) = profile.failure_reason() else {
        return;
    };
    if profile.enabled {
        eprintln!("{STARTUP_PROFILE_LOG_PREFIX} status=failed reason={reason}");
    }
}
