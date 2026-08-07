use super::{
    NativeCpuFrameCompletionOutcome, NativeCpuFrameFairnessDiagnostics,
    NativeCpuFrameFairnessDisposition, NativeCpuFrameObservationDiagnostics,
    NativeFrameDiagnostics, NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, NativeSurfaceRecoveryDiagnostics,
    NativeTextDiagnostics,
};
use std::time::Duration;

/// Native runtime profiling mode.
///
/// `Frame` is the first bounded profiling path. More detailed, scope-selected
/// profiling remains a future contract and is intentionally not represented by
/// this enum yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProfilingMode {
    /// Do not collect or publish frame profiles.
    #[default]
    Off,
    /// Collect and publish one fixed-cost profile for each successful present.
    Frame,
}

impl ProfilingMode {
    /// Return whether profiling is disabled.
    pub const fn is_off(self) -> bool {
        matches!(self, Self::Off)
    }

    /// Return whether fixed-cost frame profiling is enabled.
    pub const fn is_frame(self) -> bool {
        matches!(self, Self::Frame)
    }
}

/// Native runtime profiling configuration.
///
/// The value is deliberately small and copyable so launch builders can carry
/// it without introducing runtime ownership or allocation policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfilingOptions {
    mode: ProfilingMode,
}

impl ProfilingOptions {
    /// Construct the default disabled profiling configuration.
    pub const fn off() -> Self {
        Self {
            mode: ProfilingMode::Off,
        }
    }

    /// Construct the fixed-cost per-frame profiling configuration.
    pub const fn frame() -> Self {
        Self {
            mode: ProfilingMode::Frame,
        }
    }

    /// Return the configured profiling mode.
    pub const fn mode(self) -> ProfilingMode {
        self.mode
    }

    /// Return whether profiling is disabled.
    pub const fn is_off(self) -> bool {
        self.mode.is_off()
    }

    /// Return whether fixed-cost frame profiling is enabled.
    pub const fn is_frame(self) -> bool {
        self.mode.is_frame()
    }
}

/// GPU timestamp data exposed by a frame profile.
///
/// The current native runtime has no backend GPU timestamp query path. It
/// therefore reports [`Self::Unavailable`] instead of presenting a CPU
/// duration as GPU work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameProfileGpuTimingStatus {
    /// The selected backend did not provide a GPU timestamp for this frame.
    #[default]
    Unavailable,
}

/// Backend-neutral projection of one successfully presented frame.
///
/// Profiles are delivered only at the same bounded publication boundary as
/// native frame diagnostics. The window identity and frame sequence are
/// stable correlation values; `None` means the source diagnostics did not have
/// a successful-presentation identity available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfile {
    /// Stable native-window identity for the presenting runner.
    pub window_identity: Option<u64>,
    /// Monotonic successful-presentation sequence for that window.
    pub frame_sequence: Option<u64>,
    /// Saturating microseconds from the latest tracked native interaction to
    /// this successful presentation, when one was available.
    pub input_to_present_latency_us: Option<u64>,
    /// Coarse frame-work kind selected by runtime routing.
    pub frame_work_kind: &'static str,
    /// Stable frame-work reason label.
    pub frame_work_reason: &'static str,
    /// Stable surface-invalidation label.
    pub surface_invalidation: &'static str,
    /// Whether the presented frame stayed on paint-only work.
    pub paint_only: bool,
    /// Whether the presented frame rebuilt the scene.
    pub scene_rebuild: bool,
    /// Fixed CPU stage timings for the presented frame.
    pub timings: FrameProfileTimings,
    /// Fixed work, cache, recovery, and observation counters for the frame.
    pub counters: FrameProfileCounters,
    /// Availability of backend GPU timestamp data.
    pub gpu_timing: FrameProfileGpuTimingStatus,
}

/// Fixed CPU timings projected into a backend-neutral frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileTimings {
    /// CPU work stages performed while preparing the frame.
    pub frame_work: FrameProfileWorkTimings,
    /// Composited-base refresh timing and cache reuse.
    pub composited_base: FrameProfileCompositedBaseTiming,
    /// Transient-overlay timing and primitive count.
    pub transient_overlay: FrameProfileTransientOverlayTiming,
    /// CPU-side command submission and presentation envelope.
    pub submit_present: Duration,
    /// Time since the previous successful presentation.
    pub since_last_present: Duration,
}

impl FrameProfileTimings {
    /// Return the tracked CPU-side work envelope for this profile.
    ///
    /// The cadence interval in [`Self::since_last_present`] is intentionally
    /// excluded. This is the same accounting boundary as
    /// [`NativeFrameTimingDiagnostics::cpu_envelope_total`].
    pub fn cpu_envelope_total(self) -> Duration {
        self.frame_work.total()
            + self.composited_base.refresh
            + self.transient_overlay.paint
            + self.submit_present
    }
}

/// CPU work stages projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileWorkTimings {
    /// Time spent routing a coalesced GPU-surface wheel event.
    pub coalesced_wheel_route: Duration,
    /// Aggregate time spent refreshing runtime surface snapshots.
    pub refresh_surface: Duration,
    /// Time spent pulling the host application projection.
    pub application_projection: Duration,
    /// Time spent rebuilding runtime projection and traversal.
    pub runtime_projection: Duration,
    /// Time spent synchronizing stateful widgets.
    pub widget_state_sync: Duration,
    /// Time spent recomputing layout.
    pub layout: Duration,
    /// Time spent building the backend-neutral paint plan.
    pub paint_plan: Duration,
    /// Time spent rendering the scene to the cached texture.
    pub render_to_texture: Duration,
    /// Time spent encoding the full-screen blit/composite pass.
    pub full_screen_blit: Duration,
}

impl FrameProfileWorkTimings {
    /// Return the tracked CPU-side frame preparation envelope.
    pub fn total(self) -> Duration {
        self.coalesced_wheel_route
            + self.surface_refresh_total()
            + self.paint_plan
            + self.render_to_texture
            + self.full_screen_blit
    }

    fn surface_refresh_total(self) -> Duration {
        if !self.refresh_surface.is_zero() {
            self.refresh_surface
        } else {
            self.application_projection
                + self.runtime_projection
                + self.widget_state_sync
                + self.layout
        }
    }
}

/// Composited-base timing projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileCompositedBaseTiming {
    /// Time spent refreshing the composited base frame.
    pub refresh: Duration,
    /// Whether the composited base frame was reused from cache.
    pub cache_hit: bool,
}

/// Transient-overlay timing projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileTransientOverlayTiming {
    /// Time spent collecting transient overlay primitives.
    pub paint: Duration,
    /// Number of transient overlay primitives.
    pub primitives: usize,
}

/// Fixed counters projected from native frame observations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileCounters {
    /// Scene encoding, traversal, media, text, and surface counters.
    pub scene: FrameProfileSceneCounters,
    /// Text-cache and text-quality counters.
    pub text: FrameProfileTextCounters,
    /// Retained custom-surface cache counters.
    pub retained_surfaces: FrameProfileRetainedSurfaceCounters,
    /// GPU-surface work and cache counters.
    pub gpu_surfaces: FrameProfileGpuSurfaceCounters,
    /// Native surface recovery counters.
    pub surface_recovery: FrameProfileSurfaceRecoveryCounters,
    /// CPU scheduler-turn observations available at publication.
    pub cpu_fairness: FrameProfileCpuFairnessCounters,
    /// CPU frame admission/completion observations available at publication.
    pub cpu_observation: FrameProfileCpuObservationCounters,
}

/// Scene and paint-plan counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSceneCounters {
    /// Number of scene encodes performed for this frame.
    pub scene_encode_count: u64,
    /// Number of scene encodes reused for this frame.
    pub scene_reuse_count: u64,
    /// Number of scene assemblies attempted for this frame.
    pub scene_assembly_count: u64,
    /// Number of scene assemblies rejected by an admission or safety fence.
    pub scene_assembly_veto_count: u64,
    /// Number of mixed resident/fresh scene assemblies.
    pub scene_mixed_assembly_count: u64,
    /// Number of fresh entries encoded during scene assembly.
    pub scene_assembly_fresh_count: u64,
    /// Number of retained entries reused during scene assembly.
    pub scene_assembly_reused_count: u64,
    /// Number of entries appended while assembling a scene.
    pub scene_assembly_append_count: u64,
    /// Stable scene-build outcome label.
    pub scene_build_outcome: &'static str,
    /// Traversal and paint-plan counters.
    pub traversal: FrameProfileSceneTraversalCounters,
    /// Text primitive and run counters.
    pub text: FrameProfileSceneTextCounters,
    /// Image and SVG counters.
    pub media: FrameProfileSceneMediaCounters,
    /// GPU/custom-surface counters.
    pub surfaces: FrameProfileSceneSurfaceCounters,
}

/// Scene traversal counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSceneTraversalCounters {
    /// Number of paint primitives visited or emitted.
    pub paint_plan_primitives: usize,
    /// Number of clip layers encountered.
    pub clip_layer_count: usize,
}

/// Scene text counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSceneTextCounters {
    /// Number of text primitives in the scene.
    pub text_primitive_count: usize,
    /// Number of text-input primitives in the scene.
    pub text_input_count: usize,
    /// Number of text runs in the scene.
    pub text_run_count: usize,
}

/// Scene media counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSceneMediaCounters {
    /// Number of raster image primitives in the scene.
    pub image_count: usize,
    /// Number of SVG documents in the scene.
    pub svg_document_count: usize,
}

/// Scene custom-surface counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSceneSurfaceCounters {
    /// Number of GPU-surface primitives in the scene.
    pub gpu_surface_count: usize,
    /// Number of retained custom-surface primitives in the scene.
    pub custom_surface_count: usize,
    /// Number of custom-surface fallback paths used by the scene.
    pub custom_surface_fallback_count: u32,
}

/// Text cache and quality counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileTextCounters {
    /// Text cache counters.
    pub cache: FrameProfileTextCacheCounters,
    /// Text shaping and glyph-quality counters.
    pub quality: FrameProfileTextQualityCounters,
}

/// Text-cache counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileTextCacheCounters {
    /// Layout-cache counters.
    pub layout: FrameProfileCacheCounters,
    /// Text-atom-cache counters.
    pub atom: FrameProfileCacheCounters,
}

/// Generic cache counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileCacheCounters {
    /// Cache hits observed while preparing the frame.
    pub hits: u64,
    /// Cache misses observed while preparing the frame.
    pub misses: u64,
    /// Cache evictions observed while preparing the frame.
    pub evictions: u64,
}

/// Text shaping and font-coverage counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileTextQualityCounters {
    /// Text runs that exceeded the current basic shaping path.
    pub unsupported_shaping_runs: u64,
    /// Scalars in runs that exceeded the current basic shaping path.
    pub unsupported_shaping_scalars: u64,
    /// Glyphs substituted with the renderer fallback glyph.
    pub fallback_glyphs: u64,
    /// Glyphs unresolved by the active font configuration.
    pub missing_glyphs: u64,
}

/// Retained custom-surface cache counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileRetainedSurfaceCounters {
    /// Configured retained-frame cache capacity.
    pub cache_capacity: usize,
    /// Number of retained frames currently held by the runtime.
    pub cache_entries: usize,
    /// Calls into the host retained-surface bridge.
    pub bridge_calls: u32,
    /// Retained frames reused from the runtime cache.
    pub cache_hits: u32,
    /// Retained surfaces the host bridge could not render.
    pub miss_count: u32,
    /// Primitives encoded from retained frames.
    pub retained_frame_primitive_count: usize,
    /// Text runs encoded from retained frames.
    pub retained_frame_text_run_count: usize,
}

/// GPU-surface work and cache counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceCounters {
    /// Retained atlas texture counters.
    pub atlas: FrameProfileGpuSurfaceAtlasCounters,
    /// Signal summary/body counters.
    pub signal: FrameProfileGpuSurfaceSignalCounters,
    /// Composite binding-group counters.
    pub composite: FrameProfileGpuSurfaceCompositeCounters,
    /// Native custom-shader counters.
    pub custom_shader: FrameProfileGpuSurfaceCustomShaderCounters,
}

/// GPU-surface atlas counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceAtlasCounters {
    /// Atlas texture uploads performed for the frame.
    pub texture_uploads: usize,
    /// Atlas texture cache hits for the frame.
    pub texture_cache_hits: usize,
    /// Atlas reuse rejected because the host revision changed.
    pub texture_revision_mismatches: usize,
    /// Atlas reuse rejected because exact content changed.
    pub texture_content_mismatches: usize,
}

/// GPU-surface signal counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceSignalCounters {
    /// Signal summary buffers built for the frame.
    pub summary_builds: usize,
    /// Signal summary cache hits for the frame.
    pub summary_cache_hits: usize,
    /// Signal summary reuse rejected because the host revision changed.
    pub summary_revision_mismatches: usize,
    /// Signal summary reuse rejected because exact content changed.
    pub summary_content_mismatches: usize,
    /// Signal body renders encoded for the frame.
    pub body_renders: usize,
    /// Signal body cache hits for the frame.
    pub body_cache_hits: usize,
    /// Signal body reuse rejected because the host revision changed.
    pub body_revision_mismatches: usize,
    /// Signal body reuse rejected because exact content changed.
    pub body_content_mismatches: usize,
}

/// GPU-surface composite counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceCompositeCounters {
    /// Composite binding groups rebuilt for the frame.
    pub binding_rebuilds: usize,
    /// Composite binding groups reused for the frame.
    pub binding_cache_hits: usize,
    /// Composite reuse rejected because the retained revision changed.
    pub binding_revision_mismatches: usize,
    /// Composite reuse rejected because exact content changed.
    pub binding_content_mismatches: usize,
}

/// GPU-surface custom-shader counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceCustomShaderCounters {
    /// Custom-shader surfaces encoded for the frame.
    pub surfaces_rendered: usize,
    /// Custom-shader pipelines rebuilt for the frame.
    pub pipeline_rebuilds: usize,
    /// Custom-shader bindings rebuilt for the frame.
    pub binding_rebuilds: usize,
    /// Custom-shader bindings reused for the frame.
    pub binding_cache_hits: usize,
    /// Custom-shader setup failures.
    pub failures: FrameProfileGpuSurfaceCustomShaderFailureCounters,
    /// Valid custom-shader surfaces skipped by this backend.
    pub unsupported: FrameProfileGpuSurfaceUnsupportedCustomShaderCounters,
}

/// GPU-surface custom-shader failures projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceCustomShaderFailureCounters {
    /// Custom-shader surfaces that could not be encoded.
    pub surfaces_failed: usize,
    /// WGSL module validation failures.
    pub shader_module_failures: usize,
    /// Render-pipeline validation failures.
    pub pipeline_failures: usize,
    /// Bind-group validation failures.
    pub binding_failures: usize,
}

/// Unsupported custom-shader work projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileGpuSurfaceUnsupportedCustomShaderCounters {
    /// Valid custom-shader surfaces skipped by this backend.
    pub surfaces: usize,
    /// Total vertices requested by skipped surfaces.
    pub vertices: usize,
    /// Total WGSL source bytes carried by skipped surfaces.
    pub source_bytes: usize,
    /// Total uniform payload bytes carried by skipped surfaces.
    pub uniform_bytes: usize,
    /// Total storage payload bytes carried by skipped surfaces.
    pub storage_bytes: usize,
}

/// Native surface-recovery counters projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileSurfaceRecoveryCounters {
    /// Surface acquisitions reported as lost.
    pub lost: u64,
    /// Surface acquisitions reported as outdated.
    pub outdated: u64,
    /// Surface acquisitions reported as timed out.
    pub timeouts: u64,
    /// Surface acquisitions reported with another error.
    pub others: u64,
    /// Forced surface reconfigurations that completed.
    pub completed_reconfigures: u64,
    /// Lost/outdated acquisitions deferred at zero size.
    pub zero_size_deferrals: u64,
    /// Redraw retries requested after reconfiguration.
    pub retry_requests: u64,
    /// One-shot retries requested after a timeout.
    pub timeout_retry_requests: u64,
    /// One-shot retries requested after another error.
    pub other_retry_requests: u64,
}

/// CPU scheduler observations projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileCpuFairnessCounters {
    /// Whether a bounded scheduler-turn state exists.
    pub available: bool,
    /// Disposition recorded by the latest scheduler turn.
    pub latest_disposition: FrameProfileCpuFairnessDisposition,
    /// Native target FPS before activity caps.
    pub requested_target_fps: u32,
    /// Effective target FPS used by cadence policy.
    pub effective_target_fps: u32,
    /// Saturating lateness of the latest scheduler turn in microseconds.
    pub latest_due_lateness_us: Option<u64>,
    /// Turns with no due work.
    pub not_due_turns: u64,
    /// Turns where this window was selected.
    pub selected_turns: u64,
    /// Turns where due work was deferred.
    pub due_but_deferred_turns: u64,
    /// Exact cursor admissions for this window.
    pub cursor_admissions: u64,
    /// Whether the latest selected turn reached admission.
    pub latest_selected_was_admitted: bool,
}

/// Scheduler-turn disposition projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameProfileCpuFairnessDisposition {
    /// No bounded scheduler-turn state is available.
    #[default]
    Unknown,
    /// The window had no due work in the latest turn.
    NotDue,
    /// The window was selected in the latest turn.
    Selected,
    /// Due work was deferred in favor of another key.
    DueButDeferred,
}

/// CPU frame admission/completion observations projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProfileCpuObservationCounters {
    /// Whether a completed bounded observation exists.
    pub available: bool,
    /// Completion outcome recorded by the latest observation.
    pub latest_outcome: FrameProfileCpuCompletionOutcome,
    /// Whether the latest frame carried exact routed interaction evidence.
    pub latest_exact_interaction: bool,
    /// Redraws admitted to the bounded ledger.
    pub admitted_redraws: u64,
    /// Redraws that reached successful presentation.
    pub successful_presentations: u64,
    /// Admitted redraws skipped or vetoed.
    pub skipped_or_vetoed_redraws: u64,
    /// Redraws that started but did not complete a frame.
    pub incomplete_frames: u64,
    /// Redraws that failed.
    pub failed_frames: u64,
    /// Redraws that triggered native recovery.
    pub recovery_triggered_frames: u64,
}

/// CPU frame completion outcome projected into a frame profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameProfileCpuCompletionOutcome {
    /// No completed bounded observation is available.
    #[default]
    Unknown,
    /// The observed redraw reached successful presentation.
    SuccessfulPresentation,
    /// The observed redraw was admitted but skipped or vetoed.
    SkippedOrVetoed,
    /// The observed redraw started but did not present.
    Incomplete,
    /// The observed redraw failed without recovery.
    Failed,
    /// The observed redraw triggered native recovery.
    RecoveryTriggered,
}

impl From<NativeFrameDiagnostics> for FrameProfile {
    fn from(diagnostics: NativeFrameDiagnostics) -> Self {
        Self {
            window_identity: diagnostics.window_identity.map(|identity| identity.get()),
            frame_sequence: diagnostics.frame_sequence,
            input_to_present_latency_us: diagnostics.input_to_present_latency_us,
            frame_work_kind: diagnostics.presentation.frame_work_kind,
            frame_work_reason: diagnostics.presentation.frame_work_reason,
            surface_invalidation: diagnostics.presentation.surface_invalidation,
            paint_only: diagnostics.presentation.paint_only,
            scene_rebuild: diagnostics.presentation.scene_rebuild,
            timings: FrameProfileTimings::from(diagnostics.timings),
            counters: FrameProfileCounters::from(diagnostics),
            gpu_timing: FrameProfileGpuTimingStatus::Unavailable,
        }
    }
}

impl FrameProfile {
    /// Project the compatibility diagnostics payload into the backend-neutral
    /// profiling payload.
    pub fn from_native_frame_diagnostics(diagnostics: NativeFrameDiagnostics) -> Self {
        Self::from(diagnostics)
    }
}

impl From<NativeFrameTimingDiagnostics> for FrameProfileTimings {
    fn from(timings: NativeFrameTimingDiagnostics) -> Self {
        Self {
            frame_work: FrameProfileWorkTimings {
                coalesced_wheel_route: timings.frame_work.coalesced_wheel_route,
                refresh_surface: timings.frame_work.refresh_surface,
                application_projection: timings.frame_work.application_projection,
                runtime_projection: timings.frame_work.runtime_projection,
                widget_state_sync: timings.frame_work.widget_state_sync,
                layout: timings.frame_work.layout,
                paint_plan: timings.frame_work.paint_plan,
                render_to_texture: timings.frame_work.render_to_texture,
                full_screen_blit: timings.frame_work.full_screen_blit,
            },
            composited_base: FrameProfileCompositedBaseTiming {
                refresh: timings.composited_base.refresh,
                cache_hit: timings.composited_base.cache_hit,
            },
            transient_overlay: FrameProfileTransientOverlayTiming {
                paint: timings.transient_overlay.paint,
                primitives: timings.transient_overlay.primitives,
            },
            submit_present: timings.submit_present,
            since_last_present: timings.since_last_present,
        }
    }
}

impl From<NativeFrameDiagnostics> for FrameProfileCounters {
    fn from(diagnostics: NativeFrameDiagnostics) -> Self {
        Self {
            scene: FrameProfileSceneCounters::from(diagnostics.scene),
            text: FrameProfileTextCounters::from(diagnostics.text),
            retained_surfaces: FrameProfileRetainedSurfaceCounters::from(
                diagnostics.retained_surfaces,
            ),
            gpu_surfaces: FrameProfileGpuSurfaceCounters::from(diagnostics.gpu_surfaces),
            surface_recovery: FrameProfileSurfaceRecoveryCounters::from(
                diagnostics.surface_recovery,
            ),
            cpu_fairness: FrameProfileCpuFairnessCounters::from(diagnostics.cpu_fairness),
            cpu_observation: FrameProfileCpuObservationCounters::from(diagnostics.cpu_observation),
        }
    }
}

impl From<NativeSceneDiagnostics> for FrameProfileSceneCounters {
    fn from(scene: NativeSceneDiagnostics) -> Self {
        Self {
            scene_encode_count: scene.scene_encode_count,
            scene_reuse_count: scene.scene_reuse_count,
            scene_assembly_count: scene.scene_assembly_count,
            scene_assembly_veto_count: scene.scene_assembly_veto_count,
            scene_mixed_assembly_count: scene.scene_mixed_assembly_count,
            scene_assembly_fresh_count: scene.scene_assembly_fresh_count,
            scene_assembly_reused_count: scene.scene_assembly_reused_count,
            scene_assembly_append_count: scene.scene_assembly_append_count,
            scene_build_outcome: scene.scene_build_outcome,
            traversal: FrameProfileSceneTraversalCounters {
                paint_plan_primitives: scene.traversal.paint_plan_primitives,
                clip_layer_count: scene.traversal.clip_layer_count,
            },
            text: FrameProfileSceneTextCounters {
                text_primitive_count: scene.text.text_primitive_count,
                text_input_count: scene.text.text_input_count,
                text_run_count: scene.text.text_run_count,
            },
            media: FrameProfileSceneMediaCounters {
                image_count: scene.media.image_count,
                svg_document_count: scene.media.svg_document_count,
            },
            surfaces: FrameProfileSceneSurfaceCounters {
                gpu_surface_count: scene.surfaces.gpu_surface_count,
                custom_surface_count: scene.surfaces.custom_surface_count,
                custom_surface_fallback_count: scene.surfaces.custom_surface_fallback_count,
            },
        }
    }
}

impl From<NativeTextDiagnostics> for FrameProfileTextCounters {
    fn from(text: NativeTextDiagnostics) -> Self {
        Self {
            cache: FrameProfileTextCacheCounters {
                layout: FrameProfileCacheCounters {
                    hits: text.cache.layout.hits,
                    misses: text.cache.layout.misses,
                    evictions: text.cache.layout.evictions,
                },
                atom: FrameProfileCacheCounters {
                    hits: text.cache.atom.hits,
                    misses: text.cache.atom.misses,
                    evictions: text.cache.atom.evictions,
                },
            },
            quality: FrameProfileTextQualityCounters {
                unsupported_shaping_runs: text.quality.unsupported_shaping_runs,
                unsupported_shaping_scalars: text.quality.unsupported_shaping_scalars,
                fallback_glyphs: text.quality.fallback_glyphs,
                missing_glyphs: text.quality.missing_glyphs,
            },
        }
    }
}

impl From<NativeRetainedSurfaceDiagnostics> for FrameProfileRetainedSurfaceCounters {
    fn from(retained: NativeRetainedSurfaceDiagnostics) -> Self {
        Self {
            cache_capacity: retained.cache_capacity,
            cache_entries: retained.cache_entries,
            bridge_calls: retained.bridge_calls,
            cache_hits: retained.cache_hits,
            miss_count: retained.miss_count,
            retained_frame_primitive_count: retained.retained_frame_primitive_count,
            retained_frame_text_run_count: retained.retained_frame_text_run_count,
        }
    }
}

impl From<NativeGpuSurfaceDiagnostics> for FrameProfileGpuSurfaceCounters {
    fn from(gpu: NativeGpuSurfaceDiagnostics) -> Self {
        Self {
            atlas: FrameProfileGpuSurfaceAtlasCounters {
                texture_uploads: gpu.atlas.texture_uploads,
                texture_cache_hits: gpu.atlas.texture_cache_hits,
                texture_revision_mismatches: gpu.atlas.texture_revision_mismatches,
                texture_content_mismatches: gpu.atlas.texture_content_mismatches,
            },
            signal: FrameProfileGpuSurfaceSignalCounters {
                summary_builds: gpu.signal.summary_builds,
                summary_cache_hits: gpu.signal.summary_cache_hits,
                summary_revision_mismatches: gpu.signal.summary_revision_mismatches,
                summary_content_mismatches: gpu.signal.summary_content_mismatches,
                body_renders: gpu.signal.body_renders,
                body_cache_hits: gpu.signal.body_cache_hits,
                body_revision_mismatches: gpu.signal.body_revision_mismatches,
                body_content_mismatches: gpu.signal.body_content_mismatches,
            },
            composite: FrameProfileGpuSurfaceCompositeCounters {
                binding_rebuilds: gpu.composite.binding_rebuilds,
                binding_cache_hits: gpu.composite.binding_cache_hits,
                binding_revision_mismatches: gpu.composite.binding_revision_mismatches,
                binding_content_mismatches: gpu.composite.binding_content_mismatches,
            },
            custom_shader: FrameProfileGpuSurfaceCustomShaderCounters {
                surfaces_rendered: gpu.custom_shader.surfaces_rendered,
                pipeline_rebuilds: gpu.custom_shader.pipeline_rebuilds,
                binding_rebuilds: gpu.custom_shader.binding_rebuilds,
                binding_cache_hits: gpu.custom_shader.binding_cache_hits,
                failures: FrameProfileGpuSurfaceCustomShaderFailureCounters {
                    surfaces_failed: gpu.custom_shader.failures.surfaces_failed,
                    shader_module_failures: gpu.custom_shader.failures.shader_module_failures,
                    pipeline_failures: gpu.custom_shader.failures.pipeline_failures,
                    binding_failures: gpu.custom_shader.failures.binding_failures,
                },
                unsupported: FrameProfileGpuSurfaceUnsupportedCustomShaderCounters {
                    surfaces: gpu.custom_shader.unsupported.surfaces,
                    vertices: gpu.custom_shader.unsupported.vertices,
                    source_bytes: gpu.custom_shader.unsupported.source_bytes,
                    uniform_bytes: gpu.custom_shader.unsupported.uniform_bytes,
                    storage_bytes: gpu.custom_shader.unsupported.storage_bytes,
                },
            },
        }
    }
}

impl From<NativeSurfaceRecoveryDiagnostics> for FrameProfileSurfaceRecoveryCounters {
    fn from(recovery: NativeSurfaceRecoveryDiagnostics) -> Self {
        Self {
            lost: recovery.lost,
            outdated: recovery.outdated,
            timeouts: recovery.timeouts,
            others: recovery.others,
            completed_reconfigures: recovery.completed_reconfigures,
            zero_size_deferrals: recovery.zero_size_deferrals,
            retry_requests: recovery.retry_requests,
            timeout_retry_requests: recovery.timeout_retry_requests,
            other_retry_requests: recovery.other_retry_requests,
        }
    }
}

impl From<NativeCpuFrameFairnessDiagnostics> for FrameProfileCpuFairnessCounters {
    fn from(fairness: NativeCpuFrameFairnessDiagnostics) -> Self {
        Self {
            available: fairness.available,
            latest_disposition: match fairness.latest_disposition {
                NativeCpuFrameFairnessDisposition::Unknown => {
                    FrameProfileCpuFairnessDisposition::Unknown
                }
                NativeCpuFrameFairnessDisposition::NotDue => {
                    FrameProfileCpuFairnessDisposition::NotDue
                }
                NativeCpuFrameFairnessDisposition::Selected => {
                    FrameProfileCpuFairnessDisposition::Selected
                }
                NativeCpuFrameFairnessDisposition::DueButDeferred => {
                    FrameProfileCpuFairnessDisposition::DueButDeferred
                }
            },
            requested_target_fps: fairness.requested_target_fps,
            effective_target_fps: fairness.effective_target_fps,
            latest_due_lateness_us: fairness.latest_due_lateness_us,
            not_due_turns: fairness.not_due_turns,
            selected_turns: fairness.selected_turns,
            due_but_deferred_turns: fairness.due_but_deferred_turns,
            cursor_admissions: fairness.cursor_admissions,
            latest_selected_was_admitted: fairness.latest_selected_was_admitted,
        }
    }
}

impl From<NativeCpuFrameObservationDiagnostics> for FrameProfileCpuObservationCounters {
    fn from(observation: NativeCpuFrameObservationDiagnostics) -> Self {
        Self {
            available: observation.available,
            latest_outcome: match observation.latest_outcome {
                NativeCpuFrameCompletionOutcome::Unknown => {
                    FrameProfileCpuCompletionOutcome::Unknown
                }
                NativeCpuFrameCompletionOutcome::SuccessfulPresentation => {
                    FrameProfileCpuCompletionOutcome::SuccessfulPresentation
                }
                NativeCpuFrameCompletionOutcome::SkippedOrVetoed => {
                    FrameProfileCpuCompletionOutcome::SkippedOrVetoed
                }
                NativeCpuFrameCompletionOutcome::Incomplete => {
                    FrameProfileCpuCompletionOutcome::Incomplete
                }
                NativeCpuFrameCompletionOutcome::Failed => FrameProfileCpuCompletionOutcome::Failed,
                NativeCpuFrameCompletionOutcome::RecoveryTriggered => {
                    FrameProfileCpuCompletionOutcome::RecoveryTriggered
                }
            },
            latest_exact_interaction: observation.latest_exact_interaction,
            admitted_redraws: observation.admitted_redraws,
            successful_presentations: observation.successful_presentations,
            skipped_or_vetoed_redraws: observation.skipped_or_vetoed_redraws,
            incomplete_frames: observation.incomplete_frames,
            failed_frames: observation.failed_frames,
            recovery_triggered_frames: observation.recovery_triggered_frames,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameProfile, FrameProfileGpuTimingStatus, ProfilingMode, ProfilingOptions};
    use crate::runtime::{
        NativeFrameDiagnostics, NativeFramePresentationDiagnostics, NativeFrameTimingDiagnostics,
        NativeSceneDiagnostics, NativeWindowDiagnosticIdentity,
    };
    use std::time::Duration;

    #[test]
    fn profiling_options_default_to_off_and_expose_frame_constructor() {
        assert_eq!(ProfilingOptions::default().mode(), ProfilingMode::Off);
        assert!(ProfilingOptions::off().is_off());
        assert!(ProfilingOptions::frame().is_frame());
    }

    #[test]
    fn frame_profile_projects_identity_labels_timings_and_counters() {
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(7)),
            frame_sequence: Some(41),
            presentation: NativeFramePresentationDiagnostics {
                frame_work_kind: "rebuild_scene",
                frame_work_reason: "routed_input",
                surface_invalidation: "layout",
                paint_only: false,
                scene_rebuild: true,
            },
            scene: NativeSceneDiagnostics {
                scene_encode_count: 11,
                ..NativeSceneDiagnostics::default()
            },
            timings: NativeFrameTimingDiagnostics {
                submit_present: Duration::from_micros(13),
                ..NativeFrameTimingDiagnostics::default()
            },
            ..NativeFrameDiagnostics::default()
        };

        let profile = FrameProfile::from_native_frame_diagnostics(diagnostics);

        assert_eq!(profile.window_identity, Some(7));
        assert_eq!(profile.frame_sequence, Some(41));
        assert_eq!(profile.surface_invalidation, "layout");
        assert!(profile.scene_rebuild);
        assert_eq!(profile.counters.scene.scene_encode_count, 11);
        assert_eq!(profile.timings.submit_present, Duration::from_micros(13));
        assert_eq!(profile.gpu_timing, FrameProfileGpuTimingStatus::Unavailable);
    }
}
