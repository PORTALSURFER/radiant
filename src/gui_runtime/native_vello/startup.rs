use super::*;

const STARTUP_PROFILE_ENV: &str = "SEMPAL_NATIVE_STARTUP_PROFILE";

/// Startup lifecycle timing breakdown for first paint and deferred refresh.
#[derive(Debug, Default)]
pub(super) struct StartupTimingProfile {
    enabled: bool,
    init_started_at: Option<Instant>,
    window_created_at: Option<Instant>,
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
        let enabled = crate::env_flags::env_var_truthy(STARTUP_PROFILE_ENV);
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
        if self.summary_emitted {
            return;
        }
        let (
            Some(init_started_at),
            Some(window_created_at),
            Some(surface_ready_at),
            Some(renderer_ready_at),
            Some(first_scene_ready_at),
            Some(first_presented_at),
        ) = (
            self.init_started_at,
            self.window_created_at,
            self.surface_ready_at,
            self.renderer_ready_at,
            self.first_scene_ready_at,
            self.first_presented_at,
        )
        else {
            return;
        };
        let deferred_model_refresh_done_at = self
            .deferred_model_refresh_done_at
            .unwrap_or(first_presented_at);
        let ms = |start: Instant, end: Instant| (end - start).as_secs_f64() * 1000.0;
        let window_create_ms = ms(init_started_at, window_created_at);
        let wgpu_surface_create_ms = self
            .wgpu_surface_created_at
            .map(|at| ms(window_created_at, at))
            .unwrap_or(0.0);
        let wgpu_device_ready_ms = self
            .wgpu_device_ready_at
            .map(|at| ms(window_created_at, at))
            .unwrap_or(0.0);
        let surface_ready_ms = ms(init_started_at, surface_ready_at);
        let renderer_build_ms = self
            .renderer_started_at
            .map(|at| ms(at, renderer_ready_at))
            .unwrap_or(0.0);
        let renderer_ready_ms = ms(init_started_at, renderer_ready_at);
        let first_scene_ready_ms = ms(init_started_at, first_scene_ready_at);
        let first_redraw_started_ms = self
            .first_redraw_started_at
            .map(|at| ms(init_started_at, at))
            .unwrap_or(first_scene_ready_ms);
        let first_present_draw_ms = self
            .first_redraw_started_at
            .map(|at| ms(at, first_presented_at))
            .unwrap_or(0.0);
        let first_present_ms = ms(init_started_at, first_presented_at);
        let deferred_model_refresh_ms = ms(first_presented_at, deferred_model_refresh_done_at);
        let deferred_model_refresh_total_ms = ms(init_started_at, deferred_model_refresh_done_at);
        info!(
            window_create_ms,
            wgpu_surface_create_ms,
            wgpu_device_ready_ms,
            surface_ready_ms,
            renderer_build_ms,
            renderer_ready_ms,
            first_scene_ready_ms,
            first_redraw_started_ms,
            first_present_draw_ms,
            first_present_ms,
            deferred_model_refresh_ms,
            deferred_model_refresh_total_ms,
            "native vello startup timing summary"
        );
        if self.enabled {
            eprintln!(
                "[native-vello-startup] window_create_ms={window_create_ms:.3} \
wgpu_surface_create_ms={wgpu_surface_create_ms:.3} \
wgpu_device_ready_ms={wgpu_device_ready_ms:.3} \
surface_ready_ms={surface_ready_ms:.3} renderer_ready_ms={renderer_ready_ms:.3} \
renderer_build_ms={renderer_build_ms:.3} first_scene_ready_ms={first_scene_ready_ms:.3} \
first_redraw_started_ms={first_redraw_started_ms:.3} \
first_present_draw_ms={first_present_draw_ms:.3} first_present_ms={first_present_ms:.3} \
deferred_model_refresh_ms={deferred_model_refresh_ms:.3} \
deferred_model_refresh_total_ms={deferred_model_refresh_total_ms:.3}"
            );
        }
        self.summary_emitted = true;
    }

    #[cfg(test)]
    pub(super) fn did_emit_summary(&self) -> bool {
        self.summary_emitted
    }
}
