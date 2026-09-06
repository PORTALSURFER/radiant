use super::super::frame_scheduler_policy::{
    DiscreteInputCompletion, ImmediateTransientCompletion, NativeInputStageDisposition,
    discrete_input_completion_disposition,
};
use super::super::frame_stage_admission::FrameStageBudgetStatus;
use super::super::native_discrete_input_stage::NativeDiscreteInputKind;
use super::super::native_visual_packet::{
    NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestEnqueue,
};
use super::super::{
    FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleRedrawEvidence,
    assess_cpu_frame_fairness,
};
use super::super::{
    GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCustomShaderResidencySnapshot,
    GpuSurfaceSignalResidencySnapshot,
};
use super::{
    AuxiliaryNativeWindow, DeviceLossRegistration, FrameScheduleKey, FrameWork, FrameWorkReason,
    GenericNativeAdapterOwner, GenericNativeVelloRunner, GenericRouteOutcome,
    NativeAdapterAtlasResidencyProfile, NativeAdapterCustomShaderResidencyProfile,
    NativeAdapterGeneration, NativeAdapterSignalResidencyProfile,
    NativeAtlasResidencyWindowIdentity, NativeGenericRunError, NativeLifecycle,
    NativeLifecycleStageEvidence, NativeLifecycleTransitionKind, NativeResourceMaintenanceTurn,
    NativeTargetGeneration, NativeWindowAtlasResidencySnapshots,
    NativeWindowCustomShaderResidencySnapshots, NativeWindowSignalResidencySnapshots,
    TimedFrameCadence, recovery_completion_is_admissible, select_due_admitted_auxiliary_index,
};
use crate::{
    application::empty,
    gui::{
        input::InputTimestamp,
        types::{Point, Vector2},
    },
    gui_runtime::NativeRunOptions,
    prelude::IntoView,
    runtime::{
        AuxiliaryWindow, FrameGpuTimingSample, FrameProfile, NativeCpuFrameCompletionOutcome,
        NativeCpuFrameFairnessDiagnostics, NativeCpuFrameFairnessDisposition,
        NativeCpuFrameObservationDiagnostics, NativeFrameDiagnostics, NativeImeAdapterObservation,
        NativeWindowDiagnosticIdentity, ProfilingOptions, RuntimeAnimationActivity,
        RuntimeAnimationHost, RuntimeBridge, RuntimeFrameDiagnosticsHost,
        RuntimeFrameGpuTimingHost, RuntimeFrameProfileHost, RuntimeHostCapabilities,
        RuntimeNativeImeAdapterObserver, UiSurface,
    },
    widgets::PointerModifiers,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use winit::window::WindowId;

#[test]
fn due_admitted_auxiliary_selection_round_robins() {
    let due = [true, true];

    assert_eq!(select_due_admitted_auxiliary_index(0, &due), Some(0));
    assert_eq!(select_due_admitted_auxiliary_index(1, &due), Some(1));
    assert_eq!(select_due_admitted_auxiliary_index(2, &due), Some(0));
}

struct EmptyBridge;

impl RuntimeBridge<()> for EmptyBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }
}

#[derive(Default)]
struct CountingAnimationActivityBridge {
    animation_activity_polls: usize,
}

impl RuntimeBridge<()> for CountingAnimationActivityBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_animation()
    }
}

impl RuntimeAnimationHost for CountingAnimationActivityBridge {
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.animation_activity_polls += 1;
        RuntimeAnimationActivity::idle()
    }
}

type PublishedFrameEvents = Arc<Mutex<Vec<NativeFrameDiagnostics>>>;

struct RecordingFrameDiagnosticsBridge {
    published: PublishedFrameEvents,
}

impl RuntimeBridge<()> for RecordingFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for RecordingFrameDiagnosticsBridge {
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        self.published
            .lock()
            .expect("publication test events should not be poisoned")
            .push(diagnostics);
    }
}

type PublishedNativeImeAdapterObservations = Arc<Mutex<Vec<NativeImeAdapterObservation>>>;

struct RecordingNativeImeAdapterBridge {
    published: PublishedNativeImeAdapterObservations,
}

impl RuntimeBridge<()> for RecordingNativeImeAdapterBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_native_ime_adapter_observer()
    }
}

impl RuntimeNativeImeAdapterObserver for RecordingNativeImeAdapterBridge {
    fn observe_native_ime_adapter(&mut self, observation: NativeImeAdapterObservation) {
        self.published
            .lock()
            .expect("IME adapter publication test events should not be poisoned")
            .push(observation);
    }
}

type PublishedFrameProfiles = Arc<Mutex<Vec<FrameProfile>>>;

struct RecordingFrameProfileBridge {
    published: PublishedFrameProfiles,
}

impl RuntimeBridge<()> for RecordingFrameProfileBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_frame_profile()
    }
}

impl RuntimeFrameProfileHost for RecordingFrameProfileBridge {
    fn observe_frame_profile(&mut self, profile: FrameProfile) {
        self.published
            .lock()
            .expect("profile publication test events should not be poisoned")
            .push(profile);
    }
}

#[derive(Clone)]
struct RecordingFrameGpuTimingBridge {
    published: Arc<Mutex<Vec<FrameGpuTimingSample>>>,
}

impl RuntimeBridge<()> for RecordingFrameGpuTimingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(empty::<()>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_frame_gpu_timing()
    }
}

impl RuntimeFrameGpuTimingHost for RecordingFrameGpuTimingBridge {
    fn observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample) {
        self.published
            .lock()
            .expect("GPU timing publication test events should not be poisoned")
            .push(sample);
    }
}

fn staged_diagnostics() -> NativeFrameDiagnostics {
    NativeFrameDiagnostics {
        window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(1)),
        frame_sequence: Some(7),
        ..NativeFrameDiagnostics::default()
    }
}

#[test]
fn primary_ime_adapter_observation_publishes_once_at_admission_boundary() {
    // Native runner fixtures retain recursive surface/runtime state. Keep
    // this lifecycle boundary test on the established large test stack.
    std::thread::Builder::new()
        .name("primary-ime-observation".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let published = Arc::new(Mutex::new(Vec::new()));
            let mut runner = GenericNativeVelloRunner::new(
                NativeRunOptions::default(),
                RecordingNativeImeAdapterBridge {
                    published: Arc::clone(&published),
                },
                Vector2::new(320.0, 240.0),
            );
            let observation = NativeImeAdapterObservation {
                window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(1)),
                ..NativeImeAdapterObservation::default()
            };

            runner.native_ime_adapter_observation = Some(observation);
            runner.publish_native_ime_adapter_observation();
            runner.publish_native_ime_adapter_observation();

            assert_eq!(
                *published
                    .lock()
                    .expect("IME adapter publication test events should not be poisoned"),
                vec![observation]
            );
        })
        .expect("IME observation primary thread should spawn")
        .join()
        .expect("IME observation primary lifecycle should complete");
}

fn primary_publication_for_boundary(scheduled: bool) {
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingFrameDiagnosticsBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    let diagnostics = staged_diagnostics();

    if scheduled {
        runner.require_primary_frame_diagnostics_schedule_admission();
    }
    runner.stage_frame_diagnostics(diagnostics);
    assert!(
        published
            .lock()
            .expect("publication test events should not be poisoned")
            .is_empty()
    );
    runner
        .frame_diagnostics_publication
        .mark_observation_finalized();
    if scheduled {
        runner.publish_staged_frame_diagnostics();
        assert!(
            published
                .lock()
                .expect("publication test events should not be poisoned")
                .is_empty()
        );
    }
    let now = Instant::now();
    let primary_key = FrameScheduleKey::Primary;
    let demand = FrameScheduleDemand::from_cadence_with_requested_target_fps(
        primary_key.clone(),
        TimedFrameCadence::DrainNow {
            due_at: now - std::time::Duration::from_millis(5),
            next_wake: now + std::time::Duration::from_millis(16),
        },
        120,
        24,
        RuntimeAnimationActivity::paint_only_at(24),
        false,
        FrameScheduleRedrawEvidence::default(),
    );
    let demands = [demand];
    let plan = runner
        .frame_scheduler
        .observe(now, &demands, FrameScheduleDeadlines::default());
    assert_eq!(plan.selected, Some(primary_key.clone()));
    assess_cpu_frame_fairness(now, &demands, None)
        .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
    if scheduled {
        runner.record_frame_schedule_admission(primary_key);
    }
    runner.publish_staged_frame_diagnostics();
    runner.publish_staged_frame_diagnostics();

    let fairness = NativeCpuFrameFairnessDiagnostics {
        available: true,
        latest_disposition: NativeCpuFrameFairnessDisposition::Selected,
        requested_target_fps: 120,
        effective_target_fps: 24,
        latest_due_lateness_us: Some(5_000),
        selected_turns: 1,
        cursor_admissions: u64::from(scheduled),
        latest_selected_was_admitted: scheduled,
        ..NativeCpuFrameFairnessDiagnostics::default()
    };
    let expected = NativeFrameDiagnostics {
        cpu_fairness: fairness,
        ..diagnostics
    };
    assert_eq!(
        *published
            .lock()
            .expect("publication test events should not be poisoned"),
        vec![expected]
    );
}

#[test]
fn primary_direct_redraw_publishes_once_after_staging() {
    primary_publication_for_boundary(false);
}

#[test]
fn primary_route_time_flush_publishes_once_after_staging() {
    primary_publication_for_boundary(false);
}

#[test]
fn primary_scheduled_route_time_flush_publishes_after_admission_record() {
    primary_publication_for_boundary(true);
}

#[test]
fn diagnostics_disabled_staging_does_not_create_publication_state() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        EmptyBridge,
        Vector2::new(320.0, 240.0),
    );

    runner.stage_frame_diagnostics(staged_diagnostics());
    runner.publish_staged_frame_diagnostics();

    assert!(!runner.frame_diagnostics_enabled);
    assert!(runner.cpu_frame_observation.is_none());
    assert_eq!(runner.frame_diagnostics_publication.take(), None);
}

#[test]
fn disabled_input_binds_current_input_budget_and_records_missing_completion_clock() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingAnimationActivityBridge::default(),
        Vector2::new(320.0, 240.0),
    );

    assert!(!runner.frame_observation_enabled);
    assert_eq!(runner.core.runtime.bridge().animation_activity_polls, 0);
    let first_binding = runner.discrete_input_budget_binding();
    let second_binding = runner.discrete_input_budget_binding();
    assert_eq!(runner.core.runtime.bridge().animation_activity_polls, 2);
    assert!(first_binding.budget().is_some());
    assert_eq!(first_binding.started_at(), None);
    assert!(second_binding.budget().is_some());
    assert_eq!(second_binding.started_at(), None);

    let ticket = runner
        .frame_stage_owner
        .admit_discrete_input_with_budget(
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            first_binding,
        )
        .expect("disabled observation should still admit input");
    assert!(
        runner
            .frame_stage_owner
            .complete_discrete_input_at(ticket, None)
            .is_success()
    );
    let evidence = runner
        .frame_stage_owner
        .discrete_input_budget_evidence()
        .expect("unbudgeted completion evidence");
    assert!(evidence.budget().is_some());
    assert_eq!(evidence.elapsed(), Duration::ZERO);
    assert_eq!(evidence.status(), FrameStageBudgetStatus::NotBudgeted);
    assert_eq!(
        runner
            .frame_stage_owner
            .discrete_input_budget_breach_count(),
        0
    );
}

#[test]
fn disabled_immediate_transient_budget_binds_authoritative_input_budget() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingAnimationActivityBridge::default(),
        Vector2::new(320.0, 240.0),
    );

    let binding = runner.immediate_transient_budget_binding();

    assert!(binding.budget().is_some());
    assert_eq!(binding.started_at(), None);
    assert_eq!(runner.core.runtime.bridge().animation_activity_polls, 1);
    let ticket = runner
        .frame_stage_owner
        .admit_immediate_transient_with_budget(
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            binding,
        )
        .expect("disabled observation should still admit transient input");
    assert!(ticket.budget().budget().is_some());
    assert_eq!(
        runner
            .frame_stage_owner
            .complete_immediate_transient_at(ticket, None),
        ImmediateTransientCompletion::Completed(FrameStageBudgetStatus::NotBudgeted)
    );
}

#[test]
fn diagnostics_and_profiling_do_not_change_input_policy_mapping() {
    let mut diagnostics_off = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        EmptyBridge,
        Vector2::new(320.0, 240.0),
    );
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut diagnostics_on = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingFrameDiagnosticsBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    let profile_published = Arc::new(Mutex::new(Vec::new()));
    let mut profiling_on = GenericNativeVelloRunner::new(
        {
            let mut options = NativeRunOptions::default();
            options.frame.profiling = ProfilingOptions::frame();
            options
        },
        RecordingFrameProfileBridge {
            published: profile_published,
        },
        Vector2::new(320.0, 240.0),
    );

    assert!(!diagnostics_off.frame_observation_enabled);
    assert!(diagnostics_on.frame_observation_enabled);
    assert!(profiling_on.frame_observation_enabled);
    assert_eq!(
        diagnostics_off.discrete_input_budget_binding().budget(),
        diagnostics_on.discrete_input_budget_binding().budget()
    );
    assert_eq!(
        diagnostics_off.discrete_input_budget_binding().budget(),
        profiling_on.discrete_input_budget_binding().budget()
    );

    for status in [
        FrameStageBudgetStatus::Within,
        FrameStageBudgetStatus::Exceeded,
    ] {
        let off = discrete_input_completion_disposition(DiscreteInputCompletion::Completed(status));
        let on = discrete_input_completion_disposition(DiscreteInputCompletion::Completed(status));
        assert_eq!(off, on);
    }
    assert_eq!(
        discrete_input_completion_disposition(DiscreteInputCompletion::Completed(
            FrameStageBudgetStatus::Exceeded,
        )),
        Some(NativeInputStageDisposition::DeferLowerPriority)
    );
}

#[test]
fn gpu_timing_is_opt_in_to_frame_profiling_and_observer_for_primary_and_auxiliary() {
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut off_options = NativeRunOptions::default();
    off_options.frame.profiling = ProfilingOptions::off();
    let off = GenericNativeVelloRunner::new(
        off_options,
        RecordingFrameGpuTimingBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    assert!(!off.frame_gpu_timing_enabled);

    let mut frame_options = NativeRunOptions::default();
    frame_options.frame.profiling = ProfilingOptions::frame();
    let primary = GenericNativeVelloRunner::new(
        frame_options.clone(),
        RecordingFrameGpuTimingBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    assert!(primary.frame_gpu_timing_enabled);

    let auxiliary_without_observer = GenericNativeVelloRunner::new_auxiliary(
        frame_options.clone(),
        EmptyBridge,
        Vector2::new(320.0, 240.0),
        String::from("inspector"),
    );
    assert!(!auxiliary_without_observer.frame_gpu_timing_enabled);

    let auxiliary = GenericNativeVelloRunner::new_auxiliary(
        frame_options,
        RecordingFrameGpuTimingBridge { published },
        Vector2::new(320.0, 240.0),
        String::from("settings"),
    );
    assert!(auxiliary.frame_gpu_timing_enabled);
}

#[test]
fn profiling_off_suppresses_profile_publication() {
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingFrameProfileBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );

    runner.stage_frame_diagnostics(staged_diagnostics());
    runner
        .frame_diagnostics_publication
        .mark_observation_finalized();
    runner.publish_staged_frame_diagnostics();

    assert!(!runner.frame_profile_enabled);
    assert!(!runner.frame_observation_enabled);
    assert!(
        published
            .lock()
            .expect("profile publication test events should not be poisoned")
            .is_empty()
    );
}

#[test]
fn frame_profiling_delivers_successful_present_profiles_even_without_sequence() {
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut options = NativeRunOptions::default();
    options.frame.profiling = ProfilingOptions::frame();
    let mut runner = GenericNativeVelloRunner::new(
        options,
        RecordingFrameProfileBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    let diagnostics = staged_diagnostics();

    runner.stage_frame_diagnostics(diagnostics);
    runner
        .frame_diagnostics_publication
        .mark_observation_finalized();
    runner.publish_staged_frame_diagnostics();

    assert!(runner.frame_profile_enabled);
    assert_eq!(
        *published
            .lock()
            .expect("profile publication test events should not be poisoned"),
        vec![FrameProfile::from(diagnostics)]
    );

    let mut runner = GenericNativeVelloRunner::new(
        {
            let mut options = NativeRunOptions::default();
            options.frame.profiling = ProfilingOptions::frame();
            options
        },
        RecordingFrameProfileBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    runner.stage_frame_diagnostics(NativeFrameDiagnostics::default());
    runner
        .frame_diagnostics_publication
        .mark_observation_finalized();
    runner.publish_staged_frame_diagnostics();

    let published = published
        .lock()
        .expect("profile publication test events should not be poisoned");
    assert_eq!(published.len(), 2);
    assert_eq!(published[1].frame_sequence, None);
}

#[test]
fn primary_publication_projects_finalized_cpu_observation() {
    let published = Arc::new(Mutex::new(Vec::new()));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingFrameDiagnosticsBridge {
            published: Arc::clone(&published),
        },
        Vector2::new(320.0, 240.0),
    );
    let admission = runner
        .begin_cpu_frame_observation(FrameScheduleKey::Primary, Instant::now())
        .expect("enabled diagnostics should retain the primary observation ledger");
    runner
        .cpu_frame_observation_capture
        .record_frame_work(FrameWork::PaintOnly {
            reason: FrameWorkReason::RoutedInput,
        });
    runner
        .cpu_frame_observation_capture
        .mark_successful_presentation();
    let diagnostics = staged_diagnostics();

    runner.stage_frame_diagnostics(diagnostics);
    runner.finish_cpu_frame_observation(Some(admission), false);
    runner.publish_staged_frame_diagnostics();

    assert_eq!(
        *published
            .lock()
            .expect("publication test events should not be poisoned"),
        vec![NativeFrameDiagnostics {
            cpu_observation: NativeCpuFrameObservationDiagnostics {
                available: true,
                latest_outcome: NativeCpuFrameCompletionOutcome::SuccessfulPresentation,
                latest_exact_interaction: true,
                admitted_redraws: 1,
                successful_presentations: 1,
                ..NativeCpuFrameObservationDiagnostics::default()
            },
            ..diagnostics
        }]
    );
}

fn runner() -> GenericNativeVelloRunner<EmptyBridge, ()> {
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        EmptyBridge,
        Vector2::new(320.0, 240.0),
    )
}

fn atlas_snapshot(
    generation: NativeAdapterGeneration,
    resident_count: usize,
    logical_rgba_texel_bytes: Option<u64>,
) -> GpuSurfaceAtlasResidencySnapshot {
    let mut snapshot = GpuSurfaceAtlasResidencySnapshot::default().with_generation(generation);
    snapshot.resident_count = resident_count;
    snapshot.logical_rgba_texel_bytes = logical_rgba_texel_bytes;
    snapshot
}

fn signal_snapshot(
    generation: NativeAdapterGeneration,
    signal_buffer_resident_count: usize,
    signal_buffer_logical_bytes: Option<u64>,
    signal_body_texture_resident_count: usize,
    signal_body_texture_logical_rgba_bytes: Option<u64>,
) -> GpuSurfaceSignalResidencySnapshot {
    let mut snapshot = GpuSurfaceSignalResidencySnapshot::default().with_generation(generation);
    snapshot.signal_buffer_resident_count = signal_buffer_resident_count;
    snapshot.signal_buffer_logical_bytes = signal_buffer_logical_bytes;
    snapshot.signal_body_texture_resident_count = signal_body_texture_resident_count;
    snapshot.signal_body_texture_logical_rgba_bytes = signal_body_texture_logical_rgba_bytes;
    snapshot
}

fn custom_shader_snapshot(
    generation: NativeAdapterGeneration,
    pipeline_resident_count: usize,
    binding_resident_count: usize,
    surface_uniform_logical_bytes: Option<u64>,
    app_uniform_logical_bytes: Option<u64>,
    storage_logical_bytes: Option<u64>,
    presentation_uniform_logical_bytes: Option<u64>,
) -> GpuSurfaceCustomShaderResidencySnapshot {
    let mut snapshot =
        GpuSurfaceCustomShaderResidencySnapshot::default().with_generation(generation);
    snapshot.pipeline_resident_count = pipeline_resident_count;
    snapshot.binding_resident_count = binding_resident_count;
    snapshot.surface_uniform_logical_bytes = surface_uniform_logical_bytes;
    snapshot.app_uniform_logical_bytes = app_uniform_logical_bytes;
    snapshot.storage_logical_bytes = storage_logical_bytes;
    snapshot.presentation_uniform_logical_bytes = presentation_uniform_logical_bytes;
    snapshot
}

#[test]
fn atlas_residency_refresh_reregisters_a_rejected_live_token() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut adapter = GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    );
    let mut runner = runner();
    let mut old_active = GpuSurfaceAtlasResidencySnapshot::default().with_generation(generation);
    old_active.resident_count = 1;
    old_active.logical_rgba_texel_bytes = Some(4);
    let old_snapshots = NativeWindowAtlasResidencySnapshots {
        active: Some(old_active),
        ..NativeWindowAtlasResidencySnapshots::default()
    };
    let token = adapter
        .register_atlas_residency_account(
            NativeAtlasResidencyWindowIdentity::Primary,
            generation,
            old_snapshots,
        )
        .expect("the test account should register");
    runner.atlas_residency_account = Some(token.clone());
    assert!(adapter.remove_atlas_residency_account(&token));

    let mut current_active =
        GpuSurfaceAtlasResidencySnapshot::default().with_generation(generation);
    current_active.resident_count = 3;
    current_active.logical_rgba_texel_bytes = Some(12);
    runner.synchronize_atlas_residency_account(
        &mut adapter,
        generation,
        NativeWindowAtlasResidencySnapshots {
            active: Some(current_active),
            ..NativeWindowAtlasResidencySnapshots::default()
        },
    );

    assert!(runner.atlas_residency_account.is_some());
    let profile = adapter.capture_atlas_residency_profile();
    assert_eq!(profile.active_resident_count, Some(3));
    assert_eq!(profile.active_logical_rgba_texel_bytes, Some(12));
}

#[test]
fn signal_residency_refresh_reregisters_a_rejected_live_token() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut adapter = GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    );
    let mut runner = runner();
    let old_snapshots = NativeWindowSignalResidencySnapshots {
        active: Some(signal_snapshot(generation, 1, Some(4), 2, Some(8))),
        ..NativeWindowSignalResidencySnapshots::default()
    };
    let token = adapter
        .register_signal_residency_account(
            NativeAtlasResidencyWindowIdentity::Primary,
            generation,
            old_snapshots,
        )
        .expect("the signal test account should register");
    runner.signal_residency_account = Some(token.clone());
    assert!(adapter.remove_signal_residency_account(&token));

    let current_snapshots = NativeWindowSignalResidencySnapshots {
        active: Some(signal_snapshot(generation, 3, Some(12), 4, Some(16))),
        ..NativeWindowSignalResidencySnapshots::default()
    };
    runner.synchronize_signal_residency_account(&mut adapter, generation, current_snapshots);

    assert!(runner.signal_residency_account.is_some());
    let profile = adapter.capture_signal_residency_profile();
    assert_eq!(profile.active_signal_buffer_resident_count, Some(3));
    assert_eq!(profile.active_signal_buffer_logical_bytes, Some(12));
    assert_eq!(profile.active_signal_body_texture_resident_count, Some(4));
    assert_eq!(
        profile.active_signal_body_texture_logical_rgba_bytes,
        Some(16)
    );
    assert_eq!(
        runner.capture_signal_residency_profile(&mut adapter, false),
        NativeAdapterSignalResidencyProfile::default()
    );
    assert_eq!(
        adapter
            .capture_signal_residency_profile()
            .active_signal_buffer_resident_count,
        Some(3)
    );
    runner.refresh_signal_residency_account(&mut adapter);
    assert!(runner.signal_residency_account.is_none());
    assert_eq!(
        adapter
            .capture_signal_residency_profile()
            .active_signal_buffer_resident_count,
        Some(0)
    );
}

#[test]
fn custom_shader_residency_refresh_reregisters_a_rejected_live_token() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut adapter = GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    );
    let mut runner = runner();
    let old_snapshots = NativeWindowCustomShaderResidencySnapshots {
        active: Some(custom_shader_snapshot(
            generation,
            1,
            2,
            Some(4),
            Some(8),
            Some(12),
            Some(16),
        )),
        ..NativeWindowCustomShaderResidencySnapshots::default()
    };
    let token = adapter
        .register_custom_shader_residency_account(
            NativeAtlasResidencyWindowIdentity::Primary,
            generation,
            old_snapshots,
        )
        .expect("the custom-shader test account should register");
    runner.custom_shader_residency_account = Some(token.clone());
    assert!(adapter.remove_custom_shader_residency_account(&token));

    let current_snapshots = NativeWindowCustomShaderResidencySnapshots {
        active: Some(custom_shader_snapshot(
            generation,
            3,
            4,
            Some(12),
            Some(16),
            Some(20),
            Some(24),
        )),
        ..NativeWindowCustomShaderResidencySnapshots::default()
    };
    runner.synchronize_custom_shader_residency_account(&mut adapter, generation, current_snapshots);

    assert!(runner.custom_shader_residency_account.is_some());
    assert_ne!(
        runner.custom_shader_residency_account.as_ref(),
        Some(&token)
    );
    let profile = adapter.capture_custom_shader_residency_profile();
    assert_eq!(profile.adapter_generation, Some(generation));
    assert_eq!(profile.active_pipeline_resident_count, Some(3));
    assert_eq!(profile.active_binding_resident_count, Some(4));
    assert_eq!(profile.active_surface_uniform_logical_bytes, Some(12));
    assert_eq!(profile.active_app_uniform_logical_bytes, Some(16));
    assert_eq!(profile.active_storage_logical_bytes, Some(20));
    assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(24));
    assert_eq!(profile.quarantined_pipeline_resident_count, Some(0));
    assert_eq!(profile.quarantined_binding_resident_count, Some(0));
    assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_storage_logical_bytes, Some(0));
    assert_eq!(
        profile.quarantined_presentation_uniform_logical_bytes,
        Some(0)
    );

    assert_eq!(
        runner.capture_custom_shader_residency_profile(&mut adapter, false),
        NativeAdapterCustomShaderResidencyProfile::default()
    );

    runner.refresh_custom_shader_residency_account(&mut adapter);
    assert!(runner.custom_shader_residency_account.is_none());
    let profile = adapter.capture_custom_shader_residency_profile();
    assert_eq!(profile.adapter_generation, Some(generation));
    assert_eq!(profile.active_pipeline_resident_count, Some(0));
    assert_eq!(profile.active_binding_resident_count, Some(0));
    assert_eq!(profile.active_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_storage_logical_bytes, Some(0));
    assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_pipeline_resident_count, Some(0));
    assert_eq!(profile.quarantined_binding_resident_count, Some(0));
    assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_storage_logical_bytes, Some(0));
    assert_eq!(
        profile.quarantined_presentation_uniform_logical_bytes,
        Some(0)
    );
}

#[test]
fn atlas_ledger_syncs_post_cache_mutation_and_clear_at_profile_boundary() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut adapter = GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    );
    let mut runner = runner();
    let empty_snapshots = NativeWindowAtlasResidencySnapshots {
        active: Some(GpuSurfaceAtlasResidencySnapshot::default().with_generation(generation)),
        ..NativeWindowAtlasResidencySnapshots::default()
    };

    // This is the publication-time account state before the first atlas
    // upload. The test calls the same private synchronization boundary
    // used immediately after present_base_frame's cache mutation; the
    // unit harness has no live RenderSurface/device for a WGPU present.
    runner.synchronize_atlas_residency_account(&mut adapter, generation, empty_snapshots);
    assert_eq!(
        adapter
            .capture_atlas_residency_profile()
            .active_resident_count,
        Some(0)
    );

    let uploaded_snapshots = NativeWindowAtlasResidencySnapshots {
        active: Some(atlas_snapshot(generation, 3, Some(12))),
        quarantine_0: Some(atlas_snapshot(generation, 2, Some(8))),
        ..NativeWindowAtlasResidencySnapshots::default()
    };
    runner.synchronize_atlas_residency_account(&mut adapter, generation, uploaded_snapshots);
    let profile = adapter.capture_atlas_residency_profile();
    assert_eq!(profile.active_resident_count, Some(3));
    assert_eq!(profile.active_logical_rgba_texel_bytes, Some(12));
    assert_eq!(profile.quarantined_resident_count, Some(2));
    assert_eq!(profile.quarantined_logical_rgba_texel_bytes, Some(8));

    assert_eq!(
        runner.capture_atlas_residency_profile(&mut adapter, false),
        NativeAdapterAtlasResidencyProfile::default()
    );
    assert_eq!(
        adapter
            .capture_atlas_residency_profile()
            .active_resident_count,
        Some(3)
    );

    let cleared_snapshots = NativeWindowAtlasResidencySnapshots {
        active: Some(atlas_snapshot(generation, 0, Some(0))),
        ..NativeWindowAtlasResidencySnapshots::default()
    };
    runner.synchronize_atlas_residency_account(&mut adapter, generation, cleared_snapshots);
    let profile = adapter.capture_atlas_residency_profile();
    assert_eq!(profile.active_resident_count, Some(0));
    assert_eq!(profile.active_logical_rgba_texel_bytes, Some(0));
    assert_eq!(profile.quarantined_resident_count, Some(0));
    assert_eq!(profile.quarantined_logical_rgba_texel_bytes, Some(0));
}

#[test]
fn custom_shader_ledger_syncs_post_cache_mutation_and_clear_at_profile_boundary() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut adapter = GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    );
    let mut runner = runner();
    let empty_snapshots = NativeWindowCustomShaderResidencySnapshots {
        active: Some(custom_shader_snapshot(
            generation,
            0,
            0,
            Some(0),
            Some(0),
            Some(0),
            Some(0),
        )),
        ..NativeWindowCustomShaderResidencySnapshots::default()
    };

    runner.synchronize_custom_shader_residency_account(&mut adapter, generation, empty_snapshots);
    let profile = adapter.capture_custom_shader_residency_profile();
    assert_eq!(profile.active_pipeline_resident_count, Some(0));
    assert_eq!(profile.active_binding_resident_count, Some(0));
    assert_eq!(profile.active_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_storage_logical_bytes, Some(0));
    assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_pipeline_resident_count, Some(0));
    assert_eq!(profile.quarantined_binding_resident_count, Some(0));
    assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_storage_logical_bytes, Some(0));
    assert_eq!(
        profile.quarantined_presentation_uniform_logical_bytes,
        Some(0)
    );

    let populated_snapshots = NativeWindowCustomShaderResidencySnapshots {
        active: Some(custom_shader_snapshot(
            generation,
            3,
            4,
            Some(12),
            Some(16),
            Some(20),
            Some(24),
        )),
        quarantine_0: Some(custom_shader_snapshot(
            generation,
            2,
            3,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
        )),
        quarantine_1: Some(custom_shader_snapshot(
            generation,
            5,
            6,
            Some(7),
            Some(8),
            Some(9),
            Some(10),
        )),
    };
    runner.synchronize_custom_shader_residency_account(
        &mut adapter,
        generation,
        populated_snapshots,
    );
    let profile = adapter.capture_custom_shader_residency_profile();
    assert_eq!(profile.active_pipeline_resident_count, Some(3));
    assert_eq!(profile.active_binding_resident_count, Some(4));
    assert_eq!(profile.active_surface_uniform_logical_bytes, Some(12));
    assert_eq!(profile.active_app_uniform_logical_bytes, Some(16));
    assert_eq!(profile.active_storage_logical_bytes, Some(20));
    assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(24));
    assert_eq!(profile.quarantined_pipeline_resident_count, Some(7));
    assert_eq!(profile.quarantined_binding_resident_count, Some(9));
    assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(11));
    assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(13));
    assert_eq!(profile.quarantined_storage_logical_bytes, Some(15));
    assert_eq!(
        profile.quarantined_presentation_uniform_logical_bytes,
        Some(17)
    );

    assert_eq!(
        runner.capture_custom_shader_residency_profile(&mut adapter, false),
        NativeAdapterCustomShaderResidencyProfile::default()
    );

    let cleared_snapshots = NativeWindowCustomShaderResidencySnapshots {
        active: Some(custom_shader_snapshot(
            generation,
            0,
            0,
            Some(0),
            Some(0),
            Some(0),
            Some(0),
        )),
        ..NativeWindowCustomShaderResidencySnapshots::default()
    };
    runner.synchronize_custom_shader_residency_account(&mut adapter, generation, cleared_snapshots);
    let profile = adapter.capture_custom_shader_residency_profile();
    assert_eq!(profile.active_pipeline_resident_count, Some(0));
    assert_eq!(profile.active_binding_resident_count, Some(0));
    assert_eq!(profile.active_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.active_storage_logical_bytes, Some(0));
    assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_pipeline_resident_count, Some(0));
    assert_eq!(profile.quarantined_binding_resident_count, Some(0));
    assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(0));
    assert_eq!(profile.quarantined_storage_logical_bytes, Some(0));
    assert_eq!(
        profile.quarantined_presentation_uniform_logical_bytes,
        Some(0)
    );
}

#[test]
fn deferred_redraw_markers_respect_existing_redraw_ownership() {
    let mut runner = runner();
    assert!(
        runner.deferred_frame_work_needs_redraw_marker(FrameWork::PaintOnly {
            reason: FrameWorkReason::PointerHover,
        })
    );

    runner.queue_scroll_container_wheel_with_metadata_for_immediate_transient(
        Point::new(8.0, 8.0),
        Vector2::new(0.0, -4.0),
        PointerModifiers::default(),
        None,
        None,
    );
    runner.defer_lower_priority_route_outcome(
        GenericRouteOutcome::default()
            .with_native_input_stage_disposition(NativeInputStageDisposition::DeferLowerPriority),
    );

    assert!(runner.pending_coalesced_input_needs_redraw_marker());

    runner.timing.redraw_requested = true;
    assert!(!runner.pending_coalesced_input_needs_redraw_marker());

    runner.timing.redraw_requested = false;
    assert!(
        runner
            .window
            .native_visual_requests
            .bind_window(WindowId::from(19))
    );
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .enqueue_for_test(FrameWork::None),
        NativeVisualRequestEnqueue::Issued
    );
    assert!(!runner.pending_coalesced_input_needs_redraw_marker());
}

fn retiring_auxiliary_window_with_key(key: &str) -> AuxiliaryNativeWindow<()> {
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let options = NativeRunOptions::default();
    let mut window = AuxiliaryNativeWindow::new(
        AuxiliaryWindow::new(key, options.clone(), surface),
        &options,
        None,
        false,
        false,
    );
    let close = window.stage_destructive_close_for_test();
    let ticket = close.close_admission.expect("retiring close ticket").ticket;
    assert!(window.prepare_destructive_close(&ticket));
    assert!(window.complete_native_lifecycle(ticket));
    assert!(window.is_retiring());
    assert_eq!(window.take_close_message(), None);
    window
}

fn retiring_auxiliary_window() -> AuxiliaryNativeWindow<()> {
    retiring_auxiliary_window_with_key("settings")
}

fn retiring_auxiliary_window_with_pending_resource(key: &str) -> AuxiliaryNativeWindow<()> {
    let mut window = retiring_auxiliary_window_with_key(key);
    window.install_retiring_resource_test();
    window
}

#[test]
fn retiring_auxiliary_deadline_is_due_now_rearmed_and_cleared_by_one_turn() {
    let mut runner = runner();
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_key("retiring"));
    let now = Instant::now();

    runner.timing.retiring_auxiliary_maintenance_deadline = Some(now + Duration::from_millis(16));
    assert!(!runner.retiring_auxiliary_maintenance_is_due(now));
    assert_eq!(
        runner.retiring_auxiliary_maintenance_deadline(),
        Some(now + Duration::from_millis(16))
    );

    runner.arm_retiring_auxiliary_maintenance_due_now();
    assert!(runner.retiring_auxiliary_maintenance_is_due(Instant::now()));

    let mut turn = NativeResourceMaintenanceTurn::new();
    assert!(runner.maintain_retiring_auxiliary_resources_with_turn(&mut turn));
    runner.rearm_retiring_auxiliary_maintenance(now);
    assert!(runner.auxiliary_windows.is_empty());
    assert_eq!(runner.retiring_auxiliary_maintenance_deadline(), None);
    assert!(runner.timing.deferred_auxiliary_window_sync);
}

#[test]
fn retiring_auxiliary_late_wake_does_not_rearm_without_a_retiring_child() {
    let mut runner = runner();
    let deadline = Instant::now() + Duration::from_millis(16);
    runner.timing.retiring_auxiliary_maintenance_deadline = Some(deadline);

    runner.arm_retiring_auxiliary_maintenance_due_now();

    assert_eq!(runner.retiring_auxiliary_maintenance_deadline(), None);
}

#[test]
fn retiring_auxiliary_pending_deadline_does_not_fire_early() {
    let mut runner = runner();
    runner.auxiliary_windows.push(retiring_auxiliary_window());
    let now = Instant::now();
    let deadline = now + Duration::from_millis(16);
    runner.timing.retiring_auxiliary_maintenance_deadline = Some(deadline);

    assert!(!runner.retiring_auxiliary_maintenance_is_due(now));
    assert_eq!(
        runner.retiring_auxiliary_maintenance_deadline(),
        Some(deadline)
    );
    assert!(runner.retiring_auxiliary_maintenance_is_due(deadline));
}

#[test]
fn retiring_auxiliary_deadline_tracks_pending_completion_then_one_drop() {
    let mut runner = runner();
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_pending_resource("pending"));
    let now = Instant::now();

    runner.arm_retiring_auxiliary_maintenance_due_now();
    assert!(runner.retiring_auxiliary_maintenance_is_due(Instant::now()));
    assert!(runner.auxiliary_windows[0].retiring_resource_test_is_pending());

    // The wake/callback arm is inert with respect to resource ownership;
    // the child remains pending until the due AboutToWait turn.
    runner.arm_retiring_auxiliary_maintenance_due_now();
    assert!(runner.auxiliary_windows[0].retiring_resource_test_is_pending());

    let mut pending_turn = NativeResourceMaintenanceTurn::new();
    assert!(!runner.maintain_retiring_auxiliary_resources_with_turn(&mut pending_turn));
    assert_eq!(runner.auxiliary_windows.len(), 1);
    assert!(runner.auxiliary_windows[0].retiring_resource_test_is_completed());
    assert!(pending_turn.has_pending());

    runner.rearm_retiring_auxiliary_maintenance(now);
    let rearmed = runner
        .retiring_auxiliary_maintenance_deadline()
        .expect("pending completion should rearm retirement");
    assert!(rearmed > now);

    // A later due turn consumes the single drop budget and removes the
    // now-completed child, then the parent clears its deadline.
    runner.timing.retiring_auxiliary_maintenance_deadline =
        Some(Instant::now() - Duration::from_millis(1));
    let mut completed_turn = NativeResourceMaintenanceTurn::new();
    assert!(runner.maintain_retiring_auxiliary_resources_with_turn(&mut completed_turn));
    assert!(runner.auxiliary_windows.is_empty());
    runner.rearm_retiring_auxiliary_maintenance(Instant::now());
    assert_eq!(runner.retiring_auxiliary_maintenance_deadline(), None);
    assert!(runner.timing.deferred_auxiliary_window_sync);
}

#[test]
fn retiring_auxiliary_opportunity_has_no_window_demand_but_keeps_exact_wait_deadline() {
    let mut runner = runner();
    let now = Instant::now();
    let deadline = now + Duration::from_millis(16);
    runner.timing.retiring_auxiliary_maintenance_deadline = Some(deadline);

    let plan = runner.frame_scheduler.observe(
        now,
        &[],
        FrameScheduleDeadlines {
            maintenance: runner.retiring_auxiliary_maintenance_deadline(),
            ..FrameScheduleDeadlines::default()
        },
    );

    assert_eq!(plan.selected, None);
    assert_eq!(plan.deadlines.earliest(), Some(deadline));
}

#[test]
fn retiring_auxiliary_opportunity_shares_one_turn_across_multiple_children() {
    let mut runner = runner();
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_pending_resource("first"));
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_pending_resource("second"));

    let mut pending_turn = NativeResourceMaintenanceTurn::new();
    assert!(!runner.maintain_retiring_auxiliary_resources_with_turn(&mut pending_turn));
    assert_eq!(runner.auxiliary_windows.len(), 2);
    assert!(
        runner
            .auxiliary_windows
            .iter()
            .all(AuxiliaryNativeWindow::retiring_resource_test_is_completed)
    );
    assert!(pending_turn.has_pending());

    let mut one_drop_turn = NativeResourceMaintenanceTurn::new();
    assert!(runner.maintain_retiring_auxiliary_resources_with_turn(&mut one_drop_turn));
    assert_eq!(runner.auxiliary_windows.len(), 1);
    assert!(runner.auxiliary_windows[0].retiring_resource_test_is_completed());
    assert!(one_drop_turn.has_pending());
    assert!(runner.timing.deferred_auxiliary_window_sync);
}

#[test]
fn due_retiring_auxiliary_turn_leaves_normal_maintenance_due() {
    let mut runner = runner();
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_pending_resource("retiring"));
    let normal_deadline = Instant::now() - Duration::from_millis(1);
    runner.timing.native_resource_maintenance_deadline = Some(normal_deadline);
    runner.arm_retiring_auxiliary_maintenance_due_now();

    let mut turn = NativeResourceMaintenanceTurn::new();
    assert!(runner.retiring_auxiliary_maintenance_is_due(Instant::now()));
    assert!(!runner.maintain_retiring_auxiliary_resources_with_turn(&mut turn));
    runner.rearm_retiring_auxiliary_maintenance(Instant::now());

    // AboutToWait spends this turn exclusively on the retiring-child
    // opportunity; the separate normal MaintenanceStage ticket remains
    // due for the next scheduler opportunity.
    assert_eq!(
        runner.timing.native_resource_maintenance_deadline,
        Some(normal_deadline)
    );
}

fn finish_evidence(
    key: FrameScheduleKey,
    adapter_generation: NativeAdapterGeneration,
    evidence_window: Option<WindowId>,
    active_resource_generation: Option<NativeAdapterGeneration>,
    target_generation: NativeTargetGeneration,
    target_fenced: bool,
) -> NativeLifecycleStageEvidence {
    let mut source_phase = NativeLifecycle::default();
    assert!(source_phase.admit_recovery());
    NativeLifecycleStageEvidence {
        key,
        transition: NativeLifecycleTransitionKind::FinishDeviceRecovery,
        source_phase,
        window_id: evidence_window,
        adapter_generation: Some(adapter_generation),
        active_resource_generation,
        target_generation,
        target_fenced,
    }
}

#[test]
fn parent_admission_boundary_marks_fairness_before_cursor_progresses() {
    let mut runner = runner();
    let now = Instant::now();
    let primary_key = FrameScheduleKey::Primary;
    let auxiliary_key = FrameScheduleKey::Auxiliary("settings".to_owned());
    let demands = [
        FrameScheduleDemand::from_cadence(
            primary_key.clone(),
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + std::time::Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence::default(),
        ),
        FrameScheduleDemand::from_cadence(
            auxiliary_key.clone(),
            TimedFrameCadence::DrainNow {
                due_at: now,
                next_wake: now + std::time::Duration::from_millis(16),
            },
            60,
            RuntimeAnimationActivity::paint_only(),
            false,
            FrameScheduleRedrawEvidence::default(),
        ),
    ];
    let plan = runner
        .frame_scheduler
        .observe(now, &demands, FrameScheduleDeadlines::default());
    assess_cpu_frame_fairness(now, &demands, None)
        .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);

    runner.record_frame_schedule_admission(primary_key.clone());

    let primary_sample = runner
        .cpu_frame_fairness
        .as_ref()
        .unwrap()
        .projection()
        .window(&primary_key)
        .unwrap()
        .latest_sample()
        .unwrap();
    assert!(primary_sample.cursor_admitted);
    assert_eq!(
        runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default())
            .selected,
        Some(auxiliary_key)
    );
}

#[test]
fn parent_fairness_history_uses_existing_removal_and_recovery_fences() {
    let mut runner = runner();
    let now = Instant::now();
    let key = FrameScheduleKey::Auxiliary("settings".to_owned());
    let demands = [FrameScheduleDemand::from_cadence(
        key.clone(),
        TimedFrameCadence::Idle,
        60,
        RuntimeAnimationActivity::idle(),
        false,
        FrameScheduleRedrawEvidence::default(),
    )];
    let plan = runner
        .frame_scheduler
        .observe(now, &demands, FrameScheduleDeadlines::default());
    assess_cpu_frame_fairness(now, &demands, None)
        .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
    assert!(
        runner
            .cpu_frame_fairness
            .as_ref()
            .unwrap()
            .projection()
            .window(&key)
            .is_some()
    );

    runner.remove_cpu_frame_observation(&key);
    assert!(
        runner
            .cpu_frame_fairness
            .as_ref()
            .unwrap()
            .projection()
            .window(&key)
            .is_none()
    );

    assess_cpu_frame_fairness(now, &demands, None)
        .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
    runner.clear_cpu_frame_observation();
    assert!(
        runner
            .cpu_frame_fairness
            .as_ref()
            .unwrap()
            .projection()
            .window(&key)
            .is_none()
    );
}

#[test]
fn auxiliary_runner_omits_parent_fairness_ledger() {
    let runner = GenericNativeVelloRunner::new_auxiliary(
        NativeRunOptions::default(),
        EmptyBridge,
        Vector2::new(320.0, 240.0),
        String::from("settings"),
    );
    assert!(runner.cpu_frame_fairness.is_none());
}

#[test]
fn native_closing_fences_runner_admission_predicates() {
    let mut runner = runner();
    assert!(runner.is_running());
    assert!(runner.should_initialize_runtime());
    assert!(runner.should_admit_auxiliary_sync());

    assert!(runner.native_lifecycle.admit_closing(Instant::now()));

    assert!(!runner.is_running());
    assert!(runner.is_closing());
    assert!(!runner.should_initialize_runtime());
    assert!(!runner.should_admit_auxiliary_sync());
    assert!(runner.native_shutdown_requested());
}

#[test]
fn explicit_occlusion_overrides_acquisition_latch_for_activation() {
    let mut runner = runner();
    runner.window.surface_occluded = true;
    runner.window.surface_occluded_by_acquire = true;

    runner.handle_surface_occlusion(true);

    assert!(runner.window.surface_occluded);
    assert!(!runner.window.surface_occluded_by_acquire);
}

#[test]
fn activation_cannot_clear_acquisition_occlusion_during_recovery() {
    let mut runner = runner();
    runner.window.surface_occluded = true;
    runner.window.surface_occluded_by_acquire = true;
    assert!(runner.admit_device_recovery());

    assert!(!runner.clear_stale_acquisition_occlusion_for_activation());
    assert!(runner.window.surface_occluded);
    assert!(runner.window.surface_occluded_by_acquire);
}

#[test]
fn primary_discrete_input_requires_live_materialized_native_window() {
    let mut runner = runner();
    let generation = NativeAdapterGeneration::from_test_serial(1);
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    ));

    assert!(!runner.native_discrete_input_native_window_is_eligible(generation));
    let owner_generation = runner.frame_stage_owner.owner_generation();
    assert!(
        runner
            .admit_native_discrete_input_with_generation(
                NativeDiscreteInputKind::MouseInput,
                InputTimestamp::capture(),
                generation,
                true,
            )
            .is_none()
    );
    assert_eq!(
        runner.frame_stage_owner.owner_generation(),
        owner_generation
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn exhausted_other_fence_has_no_scheduler_retry_until_target_rearm() {
    let mut runner = runner();
    let now = Instant::now();
    runner.window.native_surface_target_fenced = true;
    runner.window.requested_recovery_redraw = true;
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(now - Duration::from_secs(1));

    assert!(!runner.native_visual_request_schedule_is_eligible());
    assert!(!runner.native_visual_request_schedule_is_ordinary());
    assert_eq!(runner.pending_redraw_retry_deadline(), None);
    let scheduled = now + Duration::from_secs(1);
    assert_eq!(runner.frame_wait_deadline(scheduled), scheduled);

    runner.prepare_successful_surface_acquisition();
    assert!(!runner.window.native_surface_target_fenced);
    assert!(runner.window.target_generation.is_known());
    // Target rearm alone cannot recreate scheduler demand while the
    // primary has no stored generation-bound adapter/resource bundle.
    assert_eq!(runner.pending_redraw_retry_deadline(), None);
}

#[test]
fn missing_primary_adapter_vetoes_packet_and_clears_recovery_wake() {
    let mut runner = runner();
    let window_id = WindowId::from(17);
    assert!(runner.window.native_visual_requests.bind_window(window_id));
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .enqueue_for_test(FrameWork::None),
        NativeVisualRequestEnqueue::Issued
    );
    let _consuming = match NativeVisualRequestAdapter::begin(
        &mut runner.window.native_visual_requests,
        window_id,
        true,
    ) {
        NativeVisualRequestBegin::Requested(packet) => packet,
        other => panic!("unexpected seeded packet state: {other:?}"),
    };
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .enqueue_for_test(FrameWork::None),
        NativeVisualRequestEnqueue::Queued
    );
    let owner = runner
        .window
        .native_visual_requests
        .owner_generation_for_test();
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    runner.window.requested_recovery_redraw = true;

    assert_eq!(
        runner.veto_native_visual_request_at_callback_boundary(),
        NativeVisualRequestBegin::RequestedVetoed
    );
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .owner_generation_for_test(),
        owner + 1
    );
    assert!(!runner.window.native_visual_requests.has_work());
    assert!(!runner.timing.redraw_requested);
    assert!(runner.timing.redraw_requested_at.is_none());
    assert!(!runner.window.requested_recovery_redraw);

    // A stray callback with no packet still clears stale wake state, but
    // does not advance ownership or create fallback work.
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    runner.window.requested_recovery_redraw = true;
    assert_eq!(
        runner.veto_native_visual_request_at_callback_boundary(),
        NativeVisualRequestBegin::Ineligible
    );
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .owner_generation_for_test(),
        owner + 1
    );
    assert!(!runner.timing.redraw_requested);
    assert!(runner.timing.redraw_requested_at.is_none());
    assert!(!runner.window.requested_recovery_redraw);
}

#[test]
fn primary_scheduler_quiesces_without_current_stored_adapter_generation() {
    let mut runner = runner();
    assert!(!runner.native_visual_request_scheduler_adapter_is_current());
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        NativeAdapterGeneration::from_test_serial(1),
        Arc::new(DeviceLossRegistration::new()),
    ));
    // A stored adapter is insufficient until the active resource bundle
    // proves the same exact generation.
    assert!(!runner.native_visual_request_scheduler_adapter_is_current());
    assert_eq!(
        runner.pending_redraw_retry_deadline(),
        None,
        "primary retry cadence remains quiescent without a current bundle"
    );
}

#[test]
fn unknown_callback_adapter_generation_uses_the_same_requested_veto() {
    let mut runner = runner();
    let window_id = WindowId::from(18);
    runner.window.id = Some(window_id);
    assert!(runner.window.native_visual_requests.bind_window(window_id));
    assert_eq!(
        runner
            .window
            .native_visual_requests
            .enqueue_for_test(FrameWork::None),
        NativeVisualRequestEnqueue::Issued
    );
    runner.window.requested_recovery_redraw = true;
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    let adapter = GenericNativeAdapterOwner::with_test_registration(
        NativeAdapterGeneration::unknown(),
        Arc::new(DeviceLossRegistration::new()),
    );

    assert_eq!(
        runner.begin_native_visual_request(&adapter),
        NativeVisualRequestBegin::RequestedVetoed
    );
    assert!(!runner.window.native_visual_requests.has_work());
    assert!(!runner.timing.redraw_requested);
    assert!(!runner.window.requested_recovery_redraw);
}

#[test]
fn visibility_intent_survives_recovery_concealment_and_reapplies_after_success() {
    let mut runner = runner();
    assert!(!runner.window.logical_window_visible);
    runner.set_native_window_visibility(true);
    assert!(runner.window.logical_window_visible);

    assert!(runner.admit_device_recovery());
    // Physical concealment must not erase the latest desired state.
    assert!(runner.window.logical_window_visible);
    assert!(runner.finish_device_recovery());
    runner.apply_native_window_visibility(runner.window.logical_window_visible);
    assert!(runner.window.logical_window_visible);

    // An explicit hidden intent remains hidden through the same boundary.
    assert!(runner.admit_device_recovery());
    runner.set_native_window_visibility(false);
    assert!(!runner.window.logical_window_visible);
    assert!(runner.finish_device_recovery());
    runner.apply_native_window_visibility(runner.window.logical_window_visible);
    assert!(!runner.window.logical_window_visible);
}

#[test]
fn native_recovery_round_trip_fences_without_terminal_cause() {
    let mut runner = runner();

    assert!(runner.admit_device_recovery());
    assert!(runner.is_recovering());
    assert!(!runner.is_running());
    assert!(!runner.is_closing());
    assert!(!runner.has_terminal_cause());
    assert!(!runner.should_admit_auxiliary_sync());
    let diagnostics = runner.core.runtime.runtime_diagnostics();
    assert_eq!(
        diagnostics.lifecycle.phase,
        crate::runtime::RuntimeLifecyclePhase::Recovering
    );
    assert_eq!(diagnostics.lifecycle.transition_count, 2);

    assert!(runner.finish_device_recovery());
    assert!(runner.is_running());
    assert!(!runner.has_terminal_cause());
    let diagnostics = runner.core.runtime.runtime_diagnostics();
    assert_eq!(
        diagnostics.lifecycle.phase,
        crate::runtime::RuntimeLifecyclePhase::Running
    );
    assert_eq!(diagnostics.lifecycle.transition_count, 3);
    assert_eq!(
        diagnostics.lifecycle.history,
        vec![
            crate::runtime::RuntimeLifecycleTransition {
                sequence: 1,
                from: crate::runtime::RuntimeLifecyclePhase::Starting,
                to: crate::runtime::RuntimeLifecyclePhase::Running,
            },
            crate::runtime::RuntimeLifecycleTransition {
                sequence: 2,
                from: crate::runtime::RuntimeLifecyclePhase::Running,
                to: crate::runtime::RuntimeLifecyclePhase::Recovering,
            },
            crate::runtime::RuntimeLifecycleTransition {
                sequence: 3,
                from: crate::runtime::RuntimeLifecyclePhase::Recovering,
                to: crate::runtime::RuntimeLifecyclePhase::Running,
            },
        ]
    );
}

#[test]
fn native_lifecycle_ticket_binds_shared_generation_and_exact_window_state() {
    let mut runner = runner();
    let generation = NativeAdapterGeneration::from_test_serial(1);
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    ));

    let ticket = runner
        .admit_native_lifecycle(Some(generation))
        .expect("primary lifecycle ticket");
    let current_generation = runner
        .adapter
        .as_ref()
        .and_then(GenericNativeAdapterOwner::capture_generation);
    assert!(runner.native_lifecycle_ticket_is_current(&ticket, current_generation));
    assert!(!runner.native_lifecycle_ticket_is_current(
        &ticket,
        Some(NativeAdapterGeneration::from_test_serial(2))
    ));
    assert!(runner.native_lifecycle_stage_ticket_is_current(&ticket));
    assert!(runner.complete_native_lifecycle(ticket));
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn native_closing_ticket_binds_absent_adapter_and_unknown_target() {
    let mut runner = runner();
    runner.window.native_surface_target_fenced = true;

    let ticket = runner
        .admit_native_closing(None)
        .expect("terminal closing ticket");
    let evidence = ticket.evidence();
    assert_eq!(evidence.key, FrameScheduleKey::Primary);
    assert_eq!(evidence.source_phase, NativeLifecycle::Running);
    assert_eq!(evidence.window_id, None);
    assert_eq!(evidence.adapter_generation, None);
    assert_eq!(evidence.active_resource_generation, None);
    assert_eq!(
        evidence.target_generation,
        NativeTargetGeneration::unknown()
    );
    assert!(evidence.target_fenced);
    assert!(runner.native_lifecycle_ticket_is_current(&ticket, None));
    assert!(runner.complete_native_lifecycle(ticket));
    assert!(!runner.frame_stage_owner.has_in_flight());

    let unknown = runner.admit_native_closing(Some(NativeAdapterGeneration::unknown()));
    assert!(unknown.is_none());
}

#[test]
fn native_closing_ticket_accepts_recovering_without_requiring_adapter_or_target() {
    let mut runner = runner();
    assert!(runner.admit_device_recovery());

    let ticket = runner
        .admit_native_closing(None)
        .expect("recovering terminal closing ticket");
    assert!(ticket.evidence().source_phase.is_recovering());
    assert!(runner.native_lifecycle_ticket_is_current(&ticket, None));
    assert!(runner.veto_native_lifecycle(ticket));
    assert!(runner.is_recovering());
}

#[test]
fn terminal_convergence_invalidates_primary_lifecycle_owner() {
    let mut runner = runner();
    let ticket = runner
        .admit_native_closing(None)
        .expect("primary terminal lifecycle ticket");
    let identity = ticket.stage_ticket().identity().clone();
    let owner_generation = runner.frame_stage_owner.owner_generation();
    assert!(runner.frame_stage_owner.has_in_flight());
    assert!(runner.prepare_native_shutdown(None).is_some());
    assert!(runner.is_closing());

    runner.invalidate_terminal_convergence_stage_owners();

    assert!(!runner.frame_stage_owner.has_in_flight());
    assert!(runner.frame_stage_owner.owner_generation() > owner_generation);
    assert!(runner.frame_stage_owner.stale(&identity));
    assert!(!runner.native_lifecycle_stage_ticket_is_current(&ticket));
    assert!(!runner.veto_native_lifecycle(ticket));
}

#[test]
fn preterminal_primary_admission_failure_is_inert_without_retry() {
    let mut runner = runner();
    let blocker = runner
        .admit_native_closing(None)
        .expect("blocking lifecycle ticket");
    let owner_generation = runner.frame_stage_owner.owner_generation();
    let cause = NativeGenericRunError::FrameRender(String::from("must remain pending"));

    assert!(
        runner
            .admit_native_shutdown_preterminal(Some(cause))
            .is_none()
    );
    assert!(runner.is_running());
    assert!(!runner.has_terminal_cause());
    assert!(runner.recovery_cause.is_none());
    assert_eq!(
        runner.frame_stage_owner.owner_generation(),
        owner_generation
    );
    assert!(runner.frame_stage_owner.has_in_flight());
    assert!(runner.native_lifecycle_stage_ticket_is_current(&blocker));

    assert!(runner.veto_native_lifecycle(blocker));
    assert!(runner.admit_native_closing(None).is_some());
}

#[test]
fn preterminal_recovery_veto_preserves_cause_for_fresh_admission() {
    let mut runner = runner();
    let original = NativeGenericRunError::RenderDeviceLost(String::from("original loss"));
    let secondary = NativeGenericRunError::FrameRender(String::from("secondary failure"));

    assert!(runner.admit_device_recovery());
    runner.recovery_cause = Some(original.clone());
    let blocker = runner
        .admit_native_closing(None)
        .expect("blocking recovering lifecycle ticket");

    assert!(
        runner
            .admit_native_shutdown_preterminal(Some(secondary))
            .is_none()
    );
    assert!(runner.is_recovering());
    assert_eq!(runner.recovery_cause, Some(original));
    assert!(!runner.has_terminal_cause());
    assert!(runner.native_lifecycle_stage_ticket_is_current(&blocker));

    assert!(runner.veto_native_lifecycle(blocker));
    let fresh = runner
        .admit_native_closing(None)
        .expect("fresh independent recovering admission");
    assert!(runner.veto_native_lifecycle(fresh));
    assert!(runner.is_recovering());
}

#[test]
fn preterminal_auxiliary_admission_failure_vetoes_primary_and_stays_inert() {
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let projection = AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface);
    let auxiliary =
        AuxiliaryNativeWindow::new(projection, &NativeRunOptions::default(), None, false, false);
    let mut runner = runner();
    runner.auxiliary_windows.push(auxiliary);
    let blocker = runner.auxiliary_windows[0]
        .admit_native_closing(None)
        .expect("blocking auxiliary lifecycle ticket");
    let primary_generation = runner.frame_stage_owner.owner_generation();
    let cause = NativeGenericRunError::FrameRender(String::from("must remain pending"));

    assert!(
        runner
            .admit_native_shutdown_preterminal(Some(cause))
            .is_none()
    );
    assert!(runner.is_running());
    assert!(!runner.has_terminal_cause());
    assert!(runner.recovery_cause.is_none());
    assert!(runner.frame_stage_owner.owner_generation() > primary_generation);
    assert!(!runner.frame_stage_owner.has_in_flight());
    assert!(runner.auxiliary_windows[0].is_admitted());
    assert!(runner.auxiliary_windows[0].frame_stage_owner_has_in_flight());

    assert!(runner.auxiliary_windows[0].veto_native_lifecycle(blocker));
    assert!(runner.admit_native_closing(None).is_some());
}

#[test]
fn preterminal_complete_set_currentness_vetoes_without_terminal_mutation() {
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let projection = AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface);
    let auxiliary =
        AuxiliaryNativeWindow::new(projection, &NativeRunOptions::default(), None, false, false);
    let mut runner = runner();
    runner.auxiliary_windows.push(auxiliary);
    let (primary_ticket, auxiliary_tickets) = runner
        .stage_native_closing_set(None)
        .expect("complete staged closing set");
    runner.window.native_surface_target_fenced = !primary_ticket.evidence().target_fenced;

    assert!(
        !runner.native_closing_stage_set_is_current(&primary_ticket, &auxiliary_tickets, None,)
    );
    assert!(runner.is_running());
    assert!(!runner.has_terminal_cause());
    assert!(runner.frame_stage_owner.has_in_flight());
    assert!(runner.auxiliary_windows[0].frame_stage_owner_has_in_flight());
    assert!(
        runner.auxiliary_windows[0]
            .native_lifecycle_stage_ticket_is_current(&auxiliary_tickets[0].1)
    );

    runner.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);
    assert!(!runner.frame_stage_owner.has_in_flight());
    assert!(!runner.auxiliary_windows[0].frame_stage_owner_has_in_flight());
}

#[test]
fn preterminal_prepare_rejection_vetoes_staged_attempt_without_mutation() {
    let mut runner = runner();
    let (primary_ticket, auxiliary_tickets) = runner
        .stage_native_closing_set(None)
        .expect("staged primary closing attempt");
    runner.native_lifecycle = NativeLifecycle::Stopped;
    let cause = NativeGenericRunError::FrameRender(String::from("must remain pending"));

    assert!(runner.prepare_native_shutdown(Some(cause)).is_none());
    runner.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);

    assert!(runner.native_lifecycle.is_stopped());
    assert!(!runner.has_terminal_cause());
    assert!(runner.recovery_cause.is_none());
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn running_shutdown_records_supplied_failure_cause_once() {
    let mut runner = runner();
    let cause = NativeGenericRunError::FrameRender(String::from("primary failure"));

    assert!(
        runner
            .prepare_native_shutdown(Some(cause.clone()))
            .is_some()
    );
    assert!(runner.is_closing());
    assert_eq!(runner.terminal_cause, Some(cause));
}

#[test]
fn recovering_shutdown_preserves_original_render_device_loss_cause() {
    let mut runner = runner();
    let original = NativeGenericRunError::RenderDeviceLost(String::from("device lost"));
    let secondary = NativeGenericRunError::FrameRender(String::from("secondary failure"));

    assert!(runner.admit_device_recovery());
    runner.recovery_cause = Some(original.clone());
    assert!(runner.prepare_native_shutdown(Some(secondary)).is_some());

    assert!(runner.is_closing());
    assert_eq!(runner.terminal_cause, Some(original));
    assert!(runner.recovery_cause.is_none());
}

#[test]
fn repeated_closing_preparation_is_inert_for_owner_budget_and_cause() {
    let mut runner = runner();
    let first = NativeGenericRunError::FrameRender(String::from("first failure"));
    let second = NativeGenericRunError::RenderDeviceLost(String::from("second failure"));

    assert!(
        runner
            .prepare_native_shutdown(Some(first.clone()))
            .is_some()
    );
    let owner_generation = runner.frame_stage_owner.owner_generation();
    assert!(runner.prepare_native_shutdown(Some(second)).is_none());

    assert_eq!(
        runner.frame_stage_owner.owner_generation(),
        owner_generation
    );
    assert_eq!(runner.terminal_cause, Some(first));
    assert!(runner.is_closing());
}

#[test]
fn closing_set_is_staged_before_any_window_phase_or_wrapper_mutation() {
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let auxiliary_projection =
        AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface);
    let auxiliary = AuxiliaryNativeWindow::new(
        auxiliary_projection,
        &NativeRunOptions::default(),
        None,
        false,
        false,
    );
    let mut runner = runner();
    runner.auxiliary_windows.push(auxiliary);

    let primary_ticket = runner
        .admit_native_closing(None)
        .expect("primary closing ticket");
    let auxiliary_ticket = runner.auxiliary_windows[0]
        .admit_native_closing(None)
        .expect("auxiliary closing ticket");

    assert!(runner.is_running());
    assert!(runner.auxiliary_windows[0].is_admitted());
    assert!(runner.native_lifecycle_ticket_is_current(&primary_ticket, None));
    assert!(
        runner.auxiliary_windows[0].native_lifecycle_ticket_is_current(&auxiliary_ticket, None,)
    );

    assert!(runner.prepare_native_shutdown(None).is_some());
    assert!(runner.auxiliary_windows[0].prepare_whole_run_closing());
    assert!(runner.is_closing());

    assert!(runner.complete_native_lifecycle(primary_ticket));
    assert!(runner.auxiliary_windows[0].complete_native_lifecycle(auxiliary_ticket));
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn finish_stages_primary_and_unmaterialized_auxiliary_before_phase_mutation() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut runner = runner();
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    ));
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let projection = AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface);
    let mut auxiliary =
        AuxiliaryNativeWindow::new(projection, &NativeRunOptions::default(), None, false, false);

    assert!(runner.admit_device_recovery());
    assert!(auxiliary.admit_device_recovery());
    assert!(!auxiliary.recovery_rebuild_pending());
    runner.auxiliary_windows.push(auxiliary);
    // A resident child that is already retiring is not part of the finish
    // ticket set, but its bounded cleanup must be handed back to Running.
    runner
        .auxiliary_windows
        .push(retiring_auxiliary_window_with_key("retiring"));

    let primary_evidence = finish_evidence(
        FrameScheduleKey::Primary,
        generation,
        Some(WindowId::dummy()),
        Some(generation),
        NativeTargetGeneration::from_test_serial(2),
        false,
    );
    let auxiliary_evidence = finish_evidence(
        FrameScheduleKey::Auxiliary(String::from("settings")),
        generation,
        None,
        None,
        NativeTargetGeneration::unknown(),
        true,
    );
    let primary_ticket = runner
        .admit_native_lifecycle_finish_with_evidence(primary_evidence.clone())
        .expect("primary finish ticket");
    let auxiliary_ticket = runner.auxiliary_windows[0]
        .admit_native_lifecycle_finish_with_evidence(auxiliary_evidence.clone())
        .expect("unmaterialized auxiliary finish ticket");
    assert!(runner.is_recovering());
    assert!(
        runner.native_lifecycle_ticket_is_current_with_evidence(&primary_ticket, &primary_evidence)
    );
    assert!(
        runner.auxiliary_windows[0].native_lifecycle_ticket_is_current_with_evidence(
            &auxiliary_ticket,
            &auxiliary_evidence
        )
    );
    assert!(
        runner
            .finish_staged_native_lifecycle_with_evidence(
                generation,
                primary_ticket,
                primary_evidence,
                vec![(0, auxiliary_ticket, auxiliary_evidence)]
            )
            .is_ok()
    );
    assert!(runner.is_running());
    assert!(!runner.frame_stage_owner.has_in_flight());
    assert!(runner.auxiliary_windows[0].can_prepare_device_recovery(generation));
    assert!(runner.retiring_auxiliary_maintenance_is_due(Instant::now()));
}

#[test]
fn primary_finish_admission_rejects_unmaterialized_evidence() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut runner = runner();
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    ));

    assert!(runner.admit_device_recovery());
    assert!(
        runner
            .admit_native_lifecycle_finish(Some(generation))
            .is_none()
    );
    assert!(runner.is_recovering());
}

#[test]
fn finish_auxiliary_failure_preserves_original_recovery_cause_without_replay() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut runner = runner();
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::new(DeviceLossRegistration::new()),
    ));
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let projection = AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface);
    let mut auxiliary =
        AuxiliaryNativeWindow::new(projection, &NativeRunOptions::default(), None, false, false);
    let cause =
        crate::gui_runtime::NativeGenericRunError::RenderDeviceLost(String::from("driver reset"));

    assert!(runner.admit_device_recovery());
    assert!(auxiliary.admit_device_recovery());
    runner.recovery_cause = Some(cause.clone());
    runner.auxiliary_windows.push(auxiliary);

    let primary_evidence = finish_evidence(
        FrameScheduleKey::Primary,
        generation,
        Some(WindowId::dummy()),
        Some(generation),
        NativeTargetGeneration::from_test_serial(2),
        false,
    );
    let auxiliary_evidence = finish_evidence(
        FrameScheduleKey::Auxiliary(String::from("settings")),
        generation,
        None,
        None,
        NativeTargetGeneration::unknown(),
        true,
    );
    let primary_ticket = runner
        .admit_native_lifecycle_finish_with_evidence(primary_evidence.clone())
        .expect("primary finish ticket");
    let auxiliary_ticket = runner.auxiliary_windows[0]
        .admit_native_lifecycle_finish_with_evidence(auxiliary_evidence.clone())
        .expect("auxiliary finish ticket");
    assert!(runner.auxiliary_windows[0].begin_controller_closing_for_test());
    assert!(
        runner
            .finish_staged_native_lifecycle_with_evidence(
                generation,
                primary_ticket,
                primary_evidence,
                vec![(0, auxiliary_ticket, auxiliary_evidence.clone())],
            )
            .is_err()
    );
    assert!(runner.is_running());
    assert!(!runner.frame_stage_owner.has_in_flight());
    let shutdown_cause = runner.recovery_cause.take();
    assert_eq!(shutdown_cause, Some(cause.clone()));
    assert!(runner.prepare_native_shutdown(shutdown_cause).is_some());
    assert!(runner.is_closing());
    assert_eq!(runner.take_terminal_cause(), Some(cause));
    let retry = runner.auxiliary_windows[0]
        .admit_native_lifecycle_finish_with_evidence(auxiliary_evidence)
        .expect("failed staged auxiliary ticket was vetoed");
    assert!(runner.auxiliary_windows[0].veto_native_lifecycle(retry));
}

#[test]
fn native_recovery_completion_preserves_controller_closing_veto() {
    let mut runner = runner();

    assert!(runner.admit_device_recovery());
    assert!(runner.core.runtime.begin_closing());
    let diagnostics = runner.core.runtime.runtime_diagnostics();
    assert_eq!(
        diagnostics.lifecycle.phase,
        crate::runtime::RuntimeLifecyclePhase::Closing
    );
    assert!(runner.is_recovering());
    assert!(!runner.finish_device_recovery());
    assert!(runner.is_recovering());
}

#[test]
fn overdue_recovery_completion_is_not_admissible() {
    assert!(!recovery_completion_is_admissible(true));
    assert!(recovery_completion_is_admissible(false));
}

#[test]
fn stopped_runner_cannot_resume_normal_admission() {
    let mut runner = runner();
    assert!(runner.native_lifecycle.admit_closing(Instant::now()));
    assert!(runner.native_lifecycle.finish_closing());
    assert!(!runner.is_running());
    assert!(!runner.is_closing());
    assert!(runner.native_shutdown_requested());
    assert!(!runner.native_lifecycle.admit_closing(Instant::now()));
    assert!(matches!(runner.native_lifecycle, NativeLifecycle::Stopped));
}
