//! Machine-readable startup timing artifact export.

use super::{StartupTimingProfile, ms_between};
use serde::Serialize;

/// Machine-readable native startup timing payload exported by the runtime.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeStartupTimingArtifact {
    /// Whether startup reached the first-present summary path.
    pub status: String,
    /// Explicit startup failure reason when startup exits before first present.
    pub failure_reason: Option<String>,
    /// Milliseconds from startup init to native window creation.
    pub window_create_ms: Option<f64>,
    /// Milliseconds from startup init to window reveal.
    pub window_revealed_ms: Option<f64>,
    /// Milliseconds from window creation to wgpu surface creation.
    pub wgpu_surface_create_ms: Option<f64>,
    /// Milliseconds from window creation to wgpu device readiness.
    pub wgpu_device_ready_ms: Option<f64>,
    /// Milliseconds from startup init to render surface readiness.
    pub surface_ready_ms: Option<f64>,
    /// Milliseconds spent constructing the renderer.
    pub renderer_build_ms: Option<f64>,
    /// Milliseconds from startup init to renderer readiness.
    pub renderer_ready_ms: Option<f64>,
    /// Milliseconds from startup init to first scene readiness.
    pub first_scene_ready_ms: Option<f64>,
    /// Milliseconds from startup init to first redraw start.
    pub first_redraw_started_ms: Option<f64>,
    /// Milliseconds from first redraw start to first present.
    pub first_present_draw_ms: Option<f64>,
    /// Milliseconds from startup init to first present.
    pub first_present_ms: Option<f64>,
    /// Milliseconds between first present and deferred startup refresh completion.
    pub deferred_model_refresh_ms: Option<f64>,
    /// Milliseconds from startup init to deferred startup refresh completion.
    pub deferred_model_refresh_total_ms: Option<f64>,
}

pub(super) fn export_startup_timing_artifact(
    profile: &StartupTimingProfile,
) -> Option<NativeStartupTimingArtifact> {
    export_completed_startup_timing_artifact(profile)
        .or_else(|| export_incomplete_artifact(profile))
}

pub(super) fn export_completed_startup_timing_artifact(
    profile: &StartupTimingProfile,
) -> Option<NativeStartupTimingArtifact> {
    let (Some(init_started_at), Some(window_created_at), Some(first_presented_at)) = (
        profile.init_started_at,
        profile.window_created_at,
        profile.first_presented_at,
    ) else {
        return None;
    };
    let surface_ready_at = profile.surface_ready_at.unwrap_or(first_presented_at);
    let renderer_ready_at = profile.renderer_ready_at.unwrap_or(first_presented_at);
    let first_scene_ready_at = profile.first_scene_ready_at.unwrap_or(first_presented_at);
    let deferred_model_refresh_done_at = profile
        .deferred_model_refresh_done_at
        .unwrap_or(first_presented_at);
    let window_create_ms = Some(ms_between(init_started_at, window_created_at));
    let first_present_ms = Some(ms_between(init_started_at, first_presented_at));

    Some(NativeStartupTimingArtifact {
        status: String::from("complete"),
        failure_reason: None,
        window_create_ms,
        window_revealed_ms: Some(
            profile
                .window_revealed_at
                .map(|at| ms_between(init_started_at, at))
                .unwrap_or_else(|| ms_between(init_started_at, first_presented_at)),
        ),
        wgpu_surface_create_ms: Some(
            profile
                .wgpu_surface_created_at
                .map(|at| ms_between(window_created_at, at))
                .unwrap_or(0.0),
        ),
        wgpu_device_ready_ms: Some(
            profile
                .wgpu_device_ready_at
                .map(|at| ms_between(window_created_at, at))
                .unwrap_or(0.0),
        ),
        surface_ready_ms: Some(ms_between(init_started_at, surface_ready_at)),
        renderer_build_ms: Some(
            profile
                .renderer_started_at
                .map(|at| ms_between(at, renderer_ready_at))
                .unwrap_or(0.0),
        ),
        renderer_ready_ms: Some(ms_between(init_started_at, renderer_ready_at)),
        first_scene_ready_ms: Some(ms_between(init_started_at, first_scene_ready_at)),
        first_redraw_started_ms: Some(
            profile
                .first_redraw_started_at
                .map(|at| ms_between(init_started_at, at))
                .unwrap_or_else(|| ms_between(init_started_at, first_scene_ready_at)),
        ),
        first_present_draw_ms: Some(
            profile
                .first_redraw_started_at
                .map(|at| ms_between(at, first_presented_at))
                .unwrap_or(0.0),
        ),
        first_present_ms,
        deferred_model_refresh_ms: Some(ms_between(
            first_presented_at,
            deferred_model_refresh_done_at,
        )),
        deferred_model_refresh_total_ms: Some(ms_between(
            init_started_at,
            deferred_model_refresh_done_at,
        )),
    })
}

fn export_incomplete_artifact(
    profile: &StartupTimingProfile,
) -> Option<NativeStartupTimingArtifact> {
    let init_started_at = profile.init_started_at?;
    let status = profile.failure_reason()?;
    let window_created_at = profile.window_created_at;
    let renderer_ready_at = profile.renderer_ready_at;

    Some(NativeStartupTimingArtifact {
        status: String::from("incomplete"),
        failure_reason: Some(status.to_string()),
        window_create_ms: window_created_at.map(|at| ms_between(init_started_at, at)),
        window_revealed_ms: profile
            .window_revealed_at
            .map(|at| ms_between(init_started_at, at)),
        wgpu_surface_create_ms: window_created_at.and_then(|window_created_at| {
            profile
                .wgpu_surface_created_at
                .map(|at| ms_between(window_created_at, at))
        }),
        wgpu_device_ready_ms: window_created_at.and_then(|window_created_at| {
            profile
                .wgpu_device_ready_at
                .map(|at| ms_between(window_created_at, at))
        }),
        surface_ready_ms: profile
            .surface_ready_at
            .map(|at| ms_between(init_started_at, at)),
        renderer_build_ms: profile
            .renderer_started_at
            .zip(renderer_ready_at)
            .map(|(started_at, ready_at)| ms_between(started_at, ready_at)),
        renderer_ready_ms: renderer_ready_at.map(|at| ms_between(init_started_at, at)),
        first_scene_ready_ms: profile
            .first_scene_ready_at
            .map(|at| ms_between(init_started_at, at)),
        first_redraw_started_ms: profile
            .first_redraw_started_at
            .map(|at| ms_between(init_started_at, at)),
        first_present_draw_ms: None,
        first_present_ms: None,
        deferred_model_refresh_ms: None,
        deferred_model_refresh_total_ms: None,
    })
}
