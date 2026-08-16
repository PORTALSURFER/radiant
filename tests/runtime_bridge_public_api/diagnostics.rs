use super::*;
use radiant::runtime::{
    NativeCompositedBaseTiming, NativeCpuFrameCompletionOutcome, NativeCpuFrameFairnessDiagnostics,
    NativeCpuFrameFairnessDisposition, NativeCpuFrameObservationDiagnostics,
    NativeFrameDiagnostics, NativeFramePresentationDiagnostics, NativeFrameTimingDiagnostics,
    NativeFrameWorkTimings, NativeGpuSurfaceCustomShaderDiagnostics,
    NativeGpuSurfaceCustomShaderFailureDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeGpuSurfaceSignalDiagnostics, NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
    NativeGpuTimingStatus, NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics,
    NativeSceneSurfaceDiagnostics, NativeSceneTraversalDiagnostics,
    NativeSurfaceRecoveryDiagnostics, NativeTextCacheCounters, NativeTextCacheDiagnostics,
    NativeTextDiagnostics, NativeTextQualityDiagnostics, NativeTextQualityStatus,
    NativeTransientOverlayTiming, NativeWindowDiagnosticIdentity, RuntimeBridge,
    RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
};
use std::time::Duration;

#[test]
fn runtime_bridge_can_observe_structured_frame_diagnostics() {
    let mut bridge = DiagnosticBridge::default();
    let diagnostics = NativeFrameDiagnostics {
        window_identity: None,
        frame_sequence: Some(41),
        input_to_present_latency_us: Some(1234),
        cpu_fairness: NativeCpuFrameFairnessDiagnostics {
            available: true,
            latest_disposition: NativeCpuFrameFairnessDisposition::Selected,
            requested_target_fps: 120,
            effective_target_fps: 24,
            latest_due_lateness_us: Some(17_000),
            not_due_turns: 3,
            selected_turns: 5,
            due_but_deferred_turns: 7,
            cursor_admissions: 5,
            latest_selected_was_admitted: true,
        },
        cpu_observation: NativeCpuFrameObservationDiagnostics {
            available: true,
            latest_outcome: NativeCpuFrameCompletionOutcome::SuccessfulPresentation,
            latest_exact_interaction: true,
            admitted_redraws: 29,
            successful_presentations: 23,
            skipped_or_vetoed_redraws: 2,
            incomplete_frames: 1,
            failed_frames: 3,
            recovery_triggered_frames: 1,
        },
        presentation: NativeFramePresentationDiagnostics::default(),
        surface_recovery: NativeSurfaceRecoveryDiagnostics {
            lost: 2,
            outdated: 3,
            timeouts: 5,
            others: 7,
            completed_reconfigures: 11,
            zero_size_deferrals: 13,
            retry_requests: 17,
            timeout_retry_requests: 19,
            other_retry_requests: 23,
        },
        scene: NativeSceneDiagnostics {
            traversal: NativeSceneTraversalDiagnostics {
                paint_plan_primitives: 12,
                ..NativeSceneTraversalDiagnostics::default()
            },
            surfaces: NativeSceneSurfaceDiagnostics {
                custom_surface_count: 2,
                ..NativeSceneSurfaceDiagnostics::default()
            },
            ..NativeSceneDiagnostics::default()
        },
        retained_surfaces: NativeRetainedSurfaceDiagnostics {
            cache_capacity: 8,
            cache_entries: 3,
            bridge_calls: 1,
            cache_hits: 2,
            ..NativeRetainedSurfaceDiagnostics::default()
        },
        gpu_surfaces: NativeGpuSurfaceDiagnostics {
            signal: NativeGpuSurfaceSignalDiagnostics {
                summary_cache_hits: 4,
                ..NativeGpuSurfaceSignalDiagnostics::default()
            },
            custom_shader: NativeGpuSurfaceCustomShaderDiagnostics {
                surfaces_rendered: 2,
                pipeline_rebuilds: 1,
                binding_rebuilds: 1,
                binding_cache_hits: 3,
                static_writes: 5,
                static_write_bytes: 320,
                presentation_writes: 7,
                presentation_write_bytes: 448,
                failures: NativeGpuSurfaceCustomShaderFailureDiagnostics {
                    surfaces_failed: 1,
                    shader_module_failures: 1,
                    pipeline_failures: 1,
                    binding_failures: 1,
                },
                unsupported: NativeGpuSurfaceUnsupportedCustomShaderDiagnostics {
                    surfaces: 1,
                    vertices: 6,
                    source_bytes: 64,
                    uniform_bytes: 16,
                    storage_bytes: 128,
                },
            },
            ..NativeGpuSurfaceDiagnostics::default()
        },
        text: NativeTextDiagnostics {
            cache: NativeTextCacheDiagnostics {
                layout: NativeTextCacheCounters {
                    hits: 6,
                    ..NativeTextCacheCounters::default()
                },
                atom: NativeTextCacheCounters {
                    misses: 2,
                    ..NativeTextCacheCounters::default()
                },
            },
            quality: NativeTextQualityDiagnostics {
                unsupported_shaping_runs: 1,
                unsupported_shaping_scalars: 4,
                fallback_glyphs: 3,
                missing_glyphs: 1,
            },
        },
        timings: NativeFrameTimingDiagnostics {
            gpu_timing_status: NativeGpuTimingStatus::CpuEnvelopeOnly,
            frame_work: NativeFrameWorkTimings {
                refresh_surface: Duration::from_micros(7),
                paint_plan: Duration::from_micros(11),
                render_to_texture: Duration::from_micros(13),
                full_screen_blit: Duration::from_micros(17),
                ..NativeFrameWorkTimings::default()
            },
            composited_base: NativeCompositedBaseTiming::default(),
            transient_overlay: NativeTransientOverlayTiming {
                primitives: 5,
                ..NativeTransientOverlayTiming::default()
            },
            submit_present: Duration::from_micros(19),
            since_last_present: Duration::from_micros(1000),
        },
    };

    assert!(bridge.host_capabilities().has_frame_diagnostics());
    fn assert_copy<T: Copy>() {}
    fn assert_public_diagnostic_traits<T: Copy + Default + std::fmt::Debug + Eq>() {}
    assert_copy::<NativeWindowDiagnosticIdentity>();
    assert_public_diagnostic_traits::<NativeCpuFrameFairnessDiagnostics>();
    assert_public_diagnostic_traits::<NativeCpuFrameFairnessDisposition>();
    assert_public_diagnostic_traits::<NativeCpuFrameCompletionOutcome>();
    assert_public_diagnostic_traits::<NativeCpuFrameObservationDiagnostics>();
    bridge.observe_frame_diagnostics(diagnostics);

    assert_eq!(bridge.last, Some(diagnostics));
    assert_eq!(diagnostics.window_identity, None);
    assert_eq!(diagnostics.frame_sequence, Some(41));
    assert_eq!(diagnostics.input_to_present_latency_us, Some(1234));
    assert_eq!(diagnostics.cpu_fairness.requested_target_fps, 120);
    assert_eq!(diagnostics.cpu_fairness.effective_target_fps, 24);
    assert_eq!(
        diagnostics.cpu_fairness.latest_due_lateness_us,
        Some(17_000)
    );
    assert_eq!(
        diagnostics.cpu_fairness.latest_disposition,
        NativeCpuFrameFairnessDisposition::Selected
    );
    assert!(diagnostics.cpu_fairness.latest_selected_was_admitted);
    assert_eq!(
        diagnostics.cpu_observation.latest_outcome,
        NativeCpuFrameCompletionOutcome::SuccessfulPresentation
    );
    assert!(diagnostics.cpu_observation.latest_exact_interaction);
    assert_eq!(diagnostics.cpu_observation.admitted_redraws, 29);
    assert_eq!(diagnostics.cpu_observation.successful_presentations, 23);
    assert_eq!(diagnostics.cpu_observation.skipped_or_vetoed_redraws, 2);
    assert_eq!(diagnostics.cpu_observation.incomplete_frames, 1);
    assert_eq!(diagnostics.cpu_observation.failed_frames, 3);
    assert_eq!(diagnostics.cpu_observation.recovery_triggered_frames, 1);
    assert_eq!(diagnostics.surface_recovery.lost, 2);
    assert_eq!(diagnostics.surface_recovery.outdated, 3);
    assert_eq!(diagnostics.surface_recovery.timeouts, 5);
    assert_eq!(diagnostics.surface_recovery.others, 7);
    assert_eq!(diagnostics.surface_recovery.completed_reconfigures, 11);
    assert_eq!(diagnostics.surface_recovery.zero_size_deferrals, 13);
    assert_eq!(diagnostics.surface_recovery.retry_requests, 17);
    assert_eq!(diagnostics.surface_recovery.timeout_retry_requests, 19);
    assert_eq!(diagnostics.surface_recovery.other_retry_requests, 23);
    assert_eq!(diagnostics.gpu_surfaces.custom_shader.static_writes, 5);
    assert_eq!(
        diagnostics.gpu_surfaces.custom_shader.static_write_bytes,
        320
    );
    assert_eq!(
        diagnostics.gpu_surfaces.custom_shader.presentation_writes,
        7
    );
    assert_eq!(
        diagnostics
            .gpu_surfaces
            .custom_shader
            .presentation_write_bytes,
        448
    );
    assert!(diagnostics.text.has_shaping_limits());
    assert!(diagnostics.text.has_font_coverage_gaps());
    assert!(diagnostics.text.has_text_quality_warnings());
    assert_eq!(
        diagnostics.text.quality_status(),
        NativeTextQualityStatus::ShapingAndFontCoverageLimited
    );
    assert_eq!(
        NativeTextDiagnostics {
            quality: NativeTextQualityDiagnostics {
                unsupported_shaping_runs: 1,
                ..NativeTextQualityDiagnostics::default()
            },
            ..NativeTextDiagnostics::default()
        }
        .quality_status(),
        NativeTextQualityStatus::ShapingLimited
    );
    assert_eq!(
        NativeTextDiagnostics {
            quality: NativeTextQualityDiagnostics {
                missing_glyphs: 1,
                ..NativeTextQualityDiagnostics::default()
            },
            ..NativeTextDiagnostics::default()
        }
        .quality_status(),
        NativeTextQualityStatus::FontCoverageLimited
    );
    assert!(!NativeTextDiagnostics::default().has_text_quality_warnings());
    assert_eq!(
        NativeTextDiagnostics::default().quality_status(),
        NativeTextQualityStatus::Clean
    );
    assert_eq!(
        diagnostics.timings.gpu_timing_status,
        NativeGpuTimingStatus::CpuEnvelopeOnly
    );
    assert_eq!(
        diagnostics.timings.cpu_envelope_total(),
        Duration::from_micros(67)
    );
}

#[test]
fn native_surface_recovery_diagnostics_default_to_zero() {
    let default_recovery = NativeSurfaceRecoveryDiagnostics::default();
    assert_eq!(
        default_recovery,
        NativeSurfaceRecoveryDiagnostics {
            lost: 0,
            outdated: 0,
            timeouts: 0,
            others: 0,
            completed_reconfigures: 0,
            zero_size_deferrals: 0,
            retry_requests: 0,
            timeout_retry_requests: 0,
            other_retry_requests: 0,
        }
    );
    assert_eq!(
        NativeFrameDiagnostics::default().surface_recovery,
        default_recovery
    );
    assert_eq!(
        NativeFrameDiagnostics::default().cpu_fairness,
        NativeCpuFrameFairnessDiagnostics::default()
    );
    assert_eq!(
        NativeFrameDiagnostics::default().cpu_observation,
        NativeCpuFrameObservationDiagnostics::default()
    );
    assert_eq!(
        NativeFrameDiagnostics::default().input_to_present_latency_us,
        None
    );
    assert_eq!(
        NativeCpuFrameFairnessDiagnostics::default().latest_due_lateness_us,
        None
    );
}

#[derive(Default)]
struct DiagnosticBridge {
    last: Option<NativeFrameDiagnostics>,
}

impl RuntimeBridge<()> for DiagnosticBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for DiagnosticBridge {
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        self.last = Some(diagnostics);
    }
}
