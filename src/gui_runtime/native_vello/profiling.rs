use super::*;

#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_INTERVAL_FRAMES: u64 = 240;
#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_ENV: &str = "SEMPAL_NATIVE_RENDER_PROFILE";

/// Interaction classes tracked by runtime performance profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InteractionProfileKind {
    Hover,
    Wheel,
    MapPanProxy,
    Waveform,
    Volume,
}

#[cfg(feature = "gui-performance")]
#[derive(Clone, Copy, Debug, Default)]
struct InteractionProfileStats {
    samples: u64,
    total_ns: u128,
    max_ns: u128,
}

#[cfg(feature = "gui-performance")]
impl InteractionProfileStats {
    fn record(&mut self, duration: Duration) {
        let nanos = duration.as_nanos();
        self.samples = self.samples.saturating_add(1);
        self.total_ns = self.total_ns.saturating_add(nanos);
        self.max_ns = self.max_ns.max(nanos);
    }

    fn avg_ms(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }
        (self.total_ns as f64 / self.samples as f64) / 1_000_000.0
    }

    fn max_ms(&self) -> f64 {
        self.max_ns as f64 / 1_000_000.0
    }
}

#[cfg(feature = "gui-performance")]
#[derive(Debug, Default)]
pub(super) struct NativeVelloProfiler {
    enabled: bool,
    frames: u64,
    rebuild_ns: u128,
    acquire_ns: u128,
    render_ns: u128,
    blit_ns: u128,
    present_ns: u128,
    total_ns: u128,
    scene_rebuilds: u64,
    state_overlay_rebuilds: u64,
    motion_overlay_rebuilds: u64,
    model_refreshes: u64,
    model_pull_ns: u128,
    motion_pull_ns: u128,
    bridge_model_pull_rebuilds: u64,
    bridge_motion_pull_rebuilds: u64,
    explicit_static_rebuilds: u64,
    dirty_mask_static_rebuilds: u64,
    tick_ns: u128,
    build_static_ns: u128,
    build_state_overlay_ns: u128,
    build_motion_overlay_ns: u128,
    encode_static_ns: u128,
    encode_state_overlay_ns: u128,
    encode_motion_overlay_ns: u128,
    motion_overlay_skips: u64,
    hover_latency: InteractionProfileStats,
    wheel_latency: InteractionProfileStats,
    map_pan_proxy_latency: InteractionProfileStats,
    waveform_latency: InteractionProfileStats,
    volume_latency: InteractionProfileStats,
}

#[cfg(feature = "gui-performance")]
impl NativeVelloProfiler {
    pub(super) fn new() -> Self {
        Self {
            enabled: crate::env_flags::env_var_truthy(REDRAW_PROFILE_ENV),
            ..Self::default()
        }
    }

    pub(super) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(super) fn now_if_enabled(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }

    pub(super) fn add_tick(&mut self, duration: Duration) {
        self.tick_ns = self.tick_ns.saturating_add(duration.as_nanos());
    }

    pub(super) fn record_scene_rebuilds(
        &mut self,
        scene: bool,
        state_overlay: bool,
        motion_overlay: bool,
    ) {
        if scene {
            self.scene_rebuilds = self.scene_rebuilds.saturating_add(1);
        }
        if state_overlay {
            self.state_overlay_rebuilds = self.state_overlay_rebuilds.saturating_add(1);
        }
        if motion_overlay {
            self.motion_overlay_rebuilds = self.motion_overlay_rebuilds.saturating_add(1);
        }
    }

    pub(super) fn add_model_refresh(&mut self) {
        self.model_refreshes = self.model_refreshes.saturating_add(1);
    }

    pub(super) fn add_model_pull(&mut self, duration: Duration) {
        self.model_pull_ns = self.model_pull_ns.saturating_add(duration.as_nanos());
    }

    pub(super) fn add_bridge_model_pull_rebuild(&mut self) {
        self.bridge_model_pull_rebuilds = self.bridge_model_pull_rebuilds.saturating_add(1);
    }

    pub(super) fn add_bridge_motion_pull_rebuild(&mut self) {
        self.bridge_motion_pull_rebuilds = self.bridge_motion_pull_rebuilds.saturating_add(1);
    }

    pub(super) fn add_explicit_static_rebuild(&mut self) {
        self.explicit_static_rebuilds = self.explicit_static_rebuilds.saturating_add(1);
    }

    pub(super) fn add_dirty_mask_static_rebuild(&mut self) {
        self.dirty_mask_static_rebuilds = self.dirty_mask_static_rebuilds.saturating_add(1);
    }

    pub(super) fn add_motion_pull(&mut self, duration: Duration) {
        self.motion_pull_ns = self.motion_pull_ns.saturating_add(duration.as_nanos());
    }

    pub(super) fn add_motion_overlay_skip(&mut self) {
        self.motion_overlay_skips = self.motion_overlay_skips.saturating_add(1);
    }

    pub(super) fn add_build_static(&mut self, duration: Duration) {
        self.build_static_ns = self.build_static_ns.saturating_add(duration.as_nanos());
    }

    pub(super) fn add_build_state_overlay(&mut self, duration: Duration) {
        self.build_state_overlay_ns = self
            .build_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(super) fn add_build_motion_overlay(&mut self, duration: Duration) {
        self.build_motion_overlay_ns = self
            .build_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(super) fn add_encode_static(&mut self, duration: Duration) {
        self.encode_static_ns = self.encode_static_ns.saturating_add(duration.as_nanos());
    }

    pub(super) fn add_encode_state_overlay(&mut self, duration: Duration) {
        self.encode_state_overlay_ns = self
            .encode_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(super) fn add_encode_motion_overlay(&mut self, duration: Duration) {
        self.encode_motion_overlay_ns = self
            .encode_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(super) fn add_interaction_latency(
        &mut self,
        kind: InteractionProfileKind,
        duration: Duration,
    ) {
        match kind {
            InteractionProfileKind::Hover => self.hover_latency.record(duration),
            InteractionProfileKind::Wheel => self.wheel_latency.record(duration),
            InteractionProfileKind::MapPanProxy => self.map_pan_proxy_latency.record(duration),
            InteractionProfileKind::Waveform => self.waveform_latency.record(duration),
            InteractionProfileKind::Volume => self.volume_latency.record(duration),
        }
    }

    pub(super) fn record_redraw(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
        text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
        if !self.enabled {
            return;
        }
        self.frames = self.frames.saturating_add(1);
        self.rebuild_ns = self.rebuild_ns.saturating_add(rebuild.as_nanos());
        self.acquire_ns = self.acquire_ns.saturating_add(acquire.as_nanos());
        self.render_ns = self.render_ns.saturating_add(render.as_nanos());
        self.blit_ns = self.blit_ns.saturating_add(blit.as_nanos());
        self.present_ns = self.present_ns.saturating_add(present.as_nanos());
        self.total_ns = self.total_ns.saturating_add(total.as_nanos());

        if self.frames < REDRAW_PROFILE_INTERVAL_FRAMES {
            return;
        }

        let frames = self.frames as f64;
        let total_ns = self.total_ns as f64;
        if total_ns <= 0.0 {
            self.reset();
            return;
        }

        let ms = |value_ns: u128| value_ns as f64 / 1_000_000.0;
        let avg_total_ms = ms(self.total_ns) / frames;
        let avg_rebuild_ms = ms(self.rebuild_ns) / frames;
        let avg_acquire_ms = ms(self.acquire_ns) / frames;
        let avg_render_ms = ms(self.render_ns) / frames;
        let avg_blit_ms = ms(self.blit_ns) / frames;
        let avg_present_ms = ms(self.present_ns) / frames;
        let avg_model_pull_ms = ms(self.model_pull_ns) / frames;
        let avg_motion_pull_ms = ms(self.motion_pull_ns) / frames;
        let avg_tick_ms = ms(self.tick_ns) / frames;
        let avg_build_static_ms = ms(self.build_static_ns) / frames;
        let avg_build_state_overlay_ms = ms(self.build_state_overlay_ns) / frames;
        let avg_build_motion_overlay_ms = ms(self.build_motion_overlay_ns) / frames;
        let avg_encode_static_ms = ms(self.encode_static_ns) / frames;
        let avg_encode_state_overlay_ms = ms(self.encode_state_overlay_ns) / frames;
        let avg_encode_motion_overlay_ms = ms(self.encode_motion_overlay_ns) / frames;
        let fps = 1000.0 / avg_total_ms.max(0.001);
        let rebuild_pct = (self.rebuild_ns as f64) * 100.0 / total_ns;
        let acquire_pct = (self.acquire_ns as f64) * 100.0 / total_ns;
        let render_pct = (self.render_ns as f64) * 100.0 / total_ns;
        let blit_pct = (self.blit_ns as f64) * 100.0 / total_ns;
        let present_pct = (self.present_ns as f64) * 100.0 / total_ns;
        let model_refresh_avg = self.model_refreshes as f64 / frames;
        let scene_rebuild_avg = self.scene_rebuilds as f64 / frames;
        let state_overlay_rebuild_avg = self.state_overlay_rebuilds as f64 / frames;
        let motion_overlay_rebuild_avg = self.motion_overlay_rebuilds as f64 / frames;
        let motion_overlay_skip_avg = self.motion_overlay_skips as f64 / frames;
        let bridge_model_pull_rebuild_avg = self.bridge_model_pull_rebuilds as f64 / frames;
        let bridge_motion_pull_rebuild_avg = self.bridge_motion_pull_rebuilds as f64 / frames;
        let explicit_static_rebuild_avg = self.explicit_static_rebuilds as f64 / frames;
        let dirty_mask_static_rebuild_avg = self.dirty_mask_static_rebuilds as f64 / frames;
        let (text_hits, text_misses, text_evictions, atom_hits, atom_misses, atom_evictions) =
            text_profile;
        let text_cache_hit_rate = if text_hits + text_misses == 0 {
            0.0
        } else {
            100.0 * (text_hits as f64) / (text_hits + text_misses) as f64
        };
        let text_cache_miss_rate = if text_hits + text_misses == 0 {
            0.0
        } else {
            100.0 * (text_misses as f64) / (text_hits + text_misses) as f64
        };
        let atom_cache_hit_rate = if atom_hits + atom_misses == 0 {
            0.0
        } else {
            100.0 * (atom_hits as f64) / (atom_hits + atom_misses) as f64
        };
        let atom_cache_miss_rate = if atom_hits + atom_misses == 0 {
            0.0
        } else {
            100.0 * (atom_misses as f64) / (atom_hits + atom_misses) as f64
        };
        eprintln!(
            "[native-vello] redraw avg over {REDRAW_PROFILE_INTERVAL_FRAMES} frames: \
             total={avg_total_ms:.2}ms ({fps:.1} fps) rebuild={avg_rebuild_ms:.2}ms ({rebuild_pct:.1}%) \
             acquire={avg_acquire_ms:.2}ms ({acquire_pct:.1}%) render={avg_render_ms:.2}ms ({render_pct:.1}%) \
             blit={avg_blit_ms:.2}ms ({blit_pct:.1}%) present={avg_present_ms:.2}ms ({present_pct:.1}%) \
             model_refresh_avg={model_refresh_avg:.2} scene_rebuild_avg={scene_rebuild_avg:.2} \
             state_overlay_rebuild_avg={state_overlay_rebuild_avg:.2} motion_overlay_rebuild_avg={motion_overlay_rebuild_avg:.2} motion_overlay_skip_avg={motion_overlay_skip_avg:.2} \
             bridge_model_pull_rebuild_avg={bridge_model_pull_rebuild_avg:.2} bridge_motion_pull_rebuild_avg={bridge_motion_pull_rebuild_avg:.2} \
             explicit_static_rebuild_avg={explicit_static_rebuild_avg:.2} dirty_mask_static_rebuild_avg={dirty_mask_static_rebuild_avg:.2} \
             model_pull_ms={avg_model_pull_ms:.3} motion_pull_ms={avg_motion_pull_ms:.3} tick_ms={avg_tick_ms:.3} \
             build_static_ms={avg_build_static_ms:.3} build_state_overlay_ms={avg_build_state_overlay_ms:.3} build_motion_overlay_ms={avg_build_motion_overlay_ms:.3} \
             encode_static_ms={avg_encode_static_ms:.3} encode_state_overlay_ms={avg_encode_state_overlay_ms:.3} encode_motion_overlay_ms={avg_encode_motion_overlay_ms:.3} \
             hover_samples={} hover_avg_ms={:.3} hover_max_ms={:.3} wheel_samples={} wheel_avg_ms={:.3} wheel_max_ms={:.3} \
             map_proxy_samples={} map_proxy_avg_ms={:.3} map_proxy_max_ms={:.3} waveform_samples={} waveform_avg_ms={:.3} waveform_max_ms={:.3} \
             volume_samples={} volume_avg_ms={:.3} volume_max_ms={:.3} \
             text_layout_hits={text_hits} text_layout_misses={text_misses} text_layout_evictions={text_evictions} text_hit_rate={text_cache_hit_rate:.1}% text_miss_rate={text_cache_miss_rate:.1}% \
             text_atom_hits={atom_hits} text_atom_misses={atom_misses} text_atom_evictions={atom_evictions} text_atom_hit_rate={atom_cache_hit_rate:.1}% text_atom_miss_rate={atom_cache_miss_rate:.1}%",
            self.hover_latency.samples,
            self.hover_latency.avg_ms(),
            self.hover_latency.max_ms(),
            self.wheel_latency.samples,
            self.wheel_latency.avg_ms(),
            self.wheel_latency.max_ms(),
            self.map_pan_proxy_latency.samples,
            self.map_pan_proxy_latency.avg_ms(),
            self.map_pan_proxy_latency.max_ms(),
            self.waveform_latency.samples,
            self.waveform_latency.avg_ms(),
            self.waveform_latency.max_ms(),
            self.volume_latency.samples,
            self.volume_latency.avg_ms(),
            self.volume_latency.max_ms(),
        );
        self.reset();
    }

    fn reset(&mut self) {
        self.frames = 0;
        self.rebuild_ns = 0;
        self.acquire_ns = 0;
        self.render_ns = 0;
        self.blit_ns = 0;
        self.present_ns = 0;
        self.total_ns = 0;
        self.scene_rebuilds = 0;
        self.state_overlay_rebuilds = 0;
        self.motion_overlay_rebuilds = 0;
        self.model_refreshes = 0;
        self.model_pull_ns = 0;
        self.motion_pull_ns = 0;
        self.bridge_model_pull_rebuilds = 0;
        self.bridge_motion_pull_rebuilds = 0;
        self.explicit_static_rebuilds = 0;
        self.dirty_mask_static_rebuilds = 0;
        self.tick_ns = 0;
        self.build_static_ns = 0;
        self.build_state_overlay_ns = 0;
        self.build_motion_overlay_ns = 0;
        self.encode_static_ns = 0;
        self.encode_state_overlay_ns = 0;
        self.encode_motion_overlay_ns = 0;
        self.motion_overlay_skips = 0;
        self.hover_latency = InteractionProfileStats::default();
        self.wheel_latency = InteractionProfileStats::default();
        self.map_pan_proxy_latency = InteractionProfileStats::default();
        self.waveform_latency = InteractionProfileStats::default();
        self.volume_latency = InteractionProfileStats::default();
    }
}

#[cfg(not(feature = "gui-performance"))]
#[derive(Debug, Default)]
pub(super) struct NativeVelloProfiler;

#[cfg(not(feature = "gui-performance"))]
impl NativeVelloProfiler {
    pub(super) fn new() -> Self {
        Self
    }
    pub(super) fn is_enabled(&self) -> bool {
        false
    }
    pub(super) fn now_if_enabled(&self) -> Option<Instant> {
        None
    }
    pub(super) fn add_tick(&mut self, _duration: Duration) {}
    pub(super) fn record_scene_rebuilds(
        &mut self,
        _scene: bool,
        _state_overlay: bool,
        _motion_overlay: bool,
    ) {
    }
    pub(super) fn add_model_refresh(&mut self) {}
    pub(super) fn add_model_pull(&mut self, _duration: Duration) {}
    pub(super) fn add_bridge_model_pull_rebuild(&mut self) {}
    pub(super) fn add_bridge_motion_pull_rebuild(&mut self) {}
    pub(super) fn add_explicit_static_rebuild(&mut self) {}
    pub(super) fn add_dirty_mask_static_rebuild(&mut self) {}
    pub(super) fn add_motion_pull(&mut self, _duration: Duration) {}
    pub(super) fn add_motion_overlay_skip(&mut self) {}
    pub(super) fn add_build_static(&mut self, _duration: Duration) {}
    pub(super) fn add_build_state_overlay(&mut self, _duration: Duration) {}
    pub(super) fn add_build_motion_overlay(&mut self, _duration: Duration) {}
    pub(super) fn add_encode_static(&mut self, _duration: Duration) {}
    pub(super) fn add_encode_state_overlay(&mut self, _duration: Duration) {}
    pub(super) fn add_encode_motion_overlay(&mut self, _duration: Duration) {}
    pub(super) fn add_interaction_latency(
        &mut self,
        _kind: InteractionProfileKind,
        _duration: Duration,
    ) {
    }
    pub(super) fn record_redraw(
        &mut self,
        _rebuild: Duration,
        _acquire: Duration,
        _render: Duration,
        _blit: Duration,
        _present: Duration,
        _total: Duration,
        _text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
    }
}
