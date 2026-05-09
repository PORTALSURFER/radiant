use super::*;

mod artifact;
mod logging;

pub use artifact::NativeStartupTimingArtifact;

/// Startup lifecycle timing breakdown for first paint and deferred refresh.
#[derive(Debug, Default)]
pub(super) struct StartupTimingProfile {
    enabled: bool,
    init_started_at: Option<Instant>,
    window_created_at: Option<Instant>,
    window_revealed_at: Option<Instant>,
    wgpu_surface_created_at: Option<Instant>,
    wgpu_device_ready_at: Option<Instant>,
    surface_ready_at: Option<Instant>,
    renderer_started_at: Option<Instant>,
    renderer_ready_at: Option<Instant>,
    first_scene_ready_at: Option<Instant>,
    first_redraw_started_at: Option<Instant>,
    first_presented_at: Option<Instant>,
    deferred_model_refresh_done_at: Option<Instant>,
    summary_emitted: bool,
}

impl StartupTimingProfile {
    pub(super) fn new() -> Self {
        let enabled = logging::startup_profile_enabled();
        Self {
            enabled,
            ..Self::default()
        }
    }

    pub(super) fn mark_init_started(&mut self) {
        self.init_started_at = Some(Instant::now());
    }
    pub(super) fn mark_window_created(&mut self) {
        self.window_created_at = Some(Instant::now());
    }
    pub(super) fn mark_window_revealed(&mut self) {
        self.window_revealed_at.get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_wgpu_surface_created(&mut self) {
        self.wgpu_surface_created_at = Some(Instant::now());
    }
    pub(super) fn mark_wgpu_device_ready(&mut self) {
        self.wgpu_device_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_surface_ready(&mut self) {
        self.surface_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_renderer_started(&mut self) {
        self.renderer_started_at.get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_renderer_ready(&mut self) {
        self.renderer_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_first_scene_ready(&mut self) {
        self.first_scene_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_first_redraw_started(&mut self) {
        self.first_redraw_started_at
            .get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_first_presented(&mut self) {
        self.first_presented_at = Some(Instant::now());
    }
    pub(super) fn mark_deferred_model_refresh_done(&mut self) {
        self.deferred_model_refresh_done_at = Some(Instant::now());
    }

    pub(super) fn maybe_emit_summary(&mut self) {
        logging::emit_summary_if_ready(self);
    }

    pub(super) fn export_artifact(&self) -> Option<NativeStartupTimingArtifact> {
        artifact::export_startup_timing_artifact(self)
    }

    /// Return the explicit startup-profile failure reason for a run that exited
    /// before first present, if startup had already begun.
    fn failure_reason(&self) -> Option<&'static str> {
        if self.summary_emitted
            || self.first_presented_at.is_some()
            || self.init_started_at.is_none()
        {
            return None;
        }
        Some("startup_exited_before_first_present")
    }

    #[cfg(test)]
    pub(super) fn did_emit_summary(&self) -> bool {
        self.summary_emitted
    }

    #[cfg(test)]
    pub(super) fn failure_reason_for_test(&self) -> Option<&'static str> {
        self.failure_reason()
    }
}

impl Drop for StartupTimingProfile {
    fn drop(&mut self) {
        logging::emit_failure_reason_if_needed(self);
    }
}

fn ms_between(start: Instant, end: Instant) -> f64 {
    (end - start).as_secs_f64() * 1000.0
}
