//! Runner state and redraw coordination for the generic native Vello runtime.

use super::device::wgpu_device_id;
use super::frame_scheduler_policy::{
    DiscreteInputCompletion, ImmediateTransientCompletion, NativeInputStageDisposition,
    SchedulerSoftBudgets, discrete_input_completion_disposition,
    immediate_transient_completion_disposition,
};
use super::frame_stage_admission::{FrameStageBudgetBinding, WindowStageOwner};
use super::gpu_surface::{
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanContext,
    GpuSurfaceRenderCanvasUploadTarget,
};
use super::gpu_timing::GpuTimingAdmission;
use super::native_discrete_input_stage::{
    NativeDiscreteInputKind, NativeDiscreteInputStageEvidence, NativeDiscreteInputStageTicket,
    admit_native_discrete_input_with_budget as admit_native_discrete_input_stage_with_budget,
    complete_native_discrete_input as complete_native_discrete_input_stage,
    veto_native_discrete_input as veto_native_discrete_input_stage,
};
use super::native_encode_present::{
    NativeEncodePresentAdmission, NativeEncodePresentCurrentEvidence, NativeEncodePresentPath,
    NativeEncodePresentPlanContext, NativeEncodePresentTicket, NativeFrameSnapshotRevision,
    complete_native_encode_present, veto_native_encode_present,
};
use super::native_immediate_transient_stage::{
    NativeImmediateTransientKind, NativeImmediateTransientStageEvidence,
    NativeImmediateTransientStageTicket,
    admit_native_immediate_transient_with_budget as admit_native_immediate_transient_stage_with_budget,
    complete_native_immediate_transient as complete_native_immediate_transient_stage,
    veto_native_immediate_transient as veto_native_immediate_transient_stage,
};
use super::native_lifecycle_stage::{
    NativeLifecycleStageEvidence, NativeLifecycleStageTicket, NativeLifecycleTransitionKind,
    admit_native_lifecycle as admit_native_lifecycle_stage,
    complete_native_lifecycle as complete_native_lifecycle_stage,
    veto_native_lifecycle as veto_native_lifecycle_stage,
};
#[cfg(target_os = "macos")]
use super::native_semantic_accessibility::NativeSemanticAccessibilityAdapter;
use super::native_visual_packet::{
    NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestDisposition,
    NativeVisualRequestEligibility, NativeVisualRequestEnqueue, NativeVisualRequestFinish,
    NativeVisualRequestPacket,
};
use super::recovery::{
    NativeRecoveryCandidate, NativeRecoveryCoordinator, NativeRecoveryEpisodeToken,
    NativeRecoveryRequest,
};
use super::renderer_recovery::{
    NativeRendererRecoveryCommitFacts, NativeRendererRecoveryPolicy,
    NativeRendererRecoveryWindowKind, construct_renderer_recovery_candidate,
    preflight_renderer_recovery, renderer_recovery_commit_is_valid,
};
use super::{
    ActivationRevealController, ApplicationReopenRegistration, AuxiliaryNativeWindow,
    CpuFrameFairnessLedger, CpuFrameObservationAdmission, CpuFrameObservationCapture,
    CpuFrameObservationLedger, CpuFrameObservationOwner, CpuFramePendingRedrawAge,
    DeviceLossRegistration, FrameScheduleKey, FrameScheduleLane, FrameWork, FrameWorkReason,
    GenericNativeAdapterOwner, GenericNativeRuntimeCore, GenericRouteOutcome,
    GpuSurfaceRenderCanvasUploadStats, NativeAdapterAtlasResidencyAccountToken,
    NativeAdapterAtlasResidencyProfile, NativeAdapterCustomShaderResidencyAccountToken,
    NativeAdapterCustomShaderResidencyProfile, NativeAdapterGeneration,
    NativeAdapterRenderCanvasUploadAccountToken, NativeAdapterRenderCanvasUploadProfile,
    NativeAdapterSignalResidencyAccountToken, NativeAdapterSignalResidencyProfile,
    NativeAtlasResidencyWindowIdentity, NativeAutomationTargetExporter, NativeClosingProgress,
    NativeFrameDiagnosticsPublication, NativeFrameScheduler, NativeGenericRunError,
    NativeGpuTimingRoute, NativeLifecycle, NativeRenderDeviceErrorKind,
    NativeResourceMaintenanceTurn, NativeRunnerInputState, NativeRunnerTimingState,
    NativeRunnerWindowState, NativeVelloFrameState, PaintPlanCacheDecision, RuntimeWakeup,
    SceneRebuildMode, SceneTextRunBuffer, SurfaceSceneEncodeContext, TimedFrameCadence,
    animation_frame_interval, animation_frame_interval_for_normalized_fps,
    encode_native_paint_segment_payloads, encode_surface_paint_plan_to_scene,
    slow_render_profile_enabled, timed_frame_cadence, timed_frame_target_fps,
};
use super::{
    frame_state::{
        NativeSceneAdmissionCandidate, NativeSceneAdmissionKind, NativeSceneAdmissionWitness,
        NativeSceneValidityFingerprint,
    },
    retained_paint_segments::NativePaintSegmentEligibilityPlan,
    runner_state::{
        NativeTargetGeneration, NativeWindowAtlasResidencySnapshots,
        NativeWindowCustomShaderResidencySnapshots, NativeWindowDiagnosticIdentityAllocator,
        NativeWindowSignalResidencySnapshots,
    },
    scene::{
        ArtifactFeasibilityObservation, NativePaintSegmentPayload,
        materialize_native_paint_segment_artifacts,
    },
    scene_texture::NativeFrameRenderFailure,
};
use crate::gui::input::InputTimestamp;
use crate::{
    gui::types::Vector2,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{
        FrameProfile, NativeCpuFrameFairnessDiagnostics, NativeCpuFrameObservationDiagnostics,
        NativeFrameDiagnostics, NativeRunOptions, NativeWindowDiagnosticIdentity,
        RuntimeAnimationActivity, RuntimeBridge,
    },
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use vello::Scene;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
};

pub(super) struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) options: NativeRunOptions,
    pub(super) core: GenericNativeRuntimeCore<Bridge, Message>,
    pub(super) runtime_wakeup: RuntimeWakeup,
    pub(super) activation_reveal: ActivationRevealController,
    pub(super) application_reopen_proxy: Option<EventLoopProxy<super::RuntimeUserEvent>>,
    pub(super) application_reopen_events: Option<ApplicationReopenRegistration>,
    /// One application-level adapter shared by the primary and auxiliary
    /// generic-native windows. Auxiliary runners borrow it at event boundaries.
    pub(super) adapter: Option<GenericNativeAdapterOwner>,
    pub(super) atlas_residency_account: Option<NativeAdapterAtlasResidencyAccountToken>,
    pub(super) signal_residency_account: Option<NativeAdapterSignalResidencyAccountToken>,
    pub(super) custom_shader_residency_account:
        Option<NativeAdapterCustomShaderResidencyAccountToken>,
    pub(super) render_canvas_upload_account: Option<NativeAdapterRenderCanvasUploadAccountToken>,
    atlas_residency_window_identity: NativeAtlasResidencyWindowIdentity,
    #[cfg(target_os = "macos")]
    pub(super) native_semantic_accessibility: Option<NativeSemanticAccessibilityAdapter>,
    pub(super) window: NativeRunnerWindowState,
    pub(super) frame: NativeVelloFrameState,
    pub(super) input: NativeRunnerInputState,
    pub(super) timing: NativeRunnerTimingState,
    pub(super) native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
    pub(super) frame_scheduler: NativeFrameScheduler,
    pub(super) frame_stage_owner: WindowStageOwner,
    pub(super) cpu_frame_fairness: Option<CpuFrameFairnessLedger>,
    pub(super) cpu_frame_observation: Option<CpuFrameObservationLedger>,
    pub(super) cpu_frame_observation_capture: CpuFrameObservationCapture,
    pub(super) frame_diagnostics_enabled: bool,
    pub(super) frame_profile_enabled: bool,
    pub(super) frame_gpu_timing_enabled: bool,
    pub(super) gpu_timing_route: NativeGpuTimingRoute,
    pub(super) frame_observation_enabled: bool,
    pub(super) frame_diagnostics_publication: NativeFrameDiagnosticsPublication,
    pub(super) native_ime_adapter_observer_enabled: bool,
    pub(super) native_ime_adapter_observation: Option<crate::runtime::NativeImeAdapterObservation>,
    pub(super) automation_targets: NativeAutomationTargetExporter,
    pub(super) auxiliary_windows: Vec<AuxiliaryNativeWindow<Message>>,
    native_lifecycle: NativeLifecycle,
    auxiliary_owner: bool,
    terminal_cause: Option<NativeGenericRunError>,
    pub(super) recovery: NativeRecoveryCoordinator,
    pub(super) renderer_recovery: NativeRendererRecoveryPolicy,
    pub(super) recovery_cause: Option<NativeGenericRunError>,
    pub(super) recovery_auxiliary_followup_pending: bool,
}

pub(super) fn select_due_admitted_auxiliary_index(
    cursor: usize,
    due_admitted: &[bool],
) -> Option<usize> {
    if due_admitted.is_empty() {
        return None;
    }
    (0..due_admitted.len())
        .map(|offset| (cursor + offset) % due_admitted.len())
        .find(|&index| due_admitted[index])
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AppliedRouteOutcome {
    pub(super) exit_requested: bool,
    pub(super) sync_auxiliary_windows_now: bool,
}

type NativeClosingAuxiliaryTickets = Vec<(usize, NativeLifecycleStageTicket)>;
type NativeClosingStageSet = (NativeLifecycleStageTicket, NativeClosingAuxiliaryTickets);
type NativeClosingAdmission = (NativeClosingStageSet, Instant);

const fn recovery_completion_is_admissible(recovery_expired: bool) -> bool {
    !recovery_expired
}

/// One-shot admission for materializing artifacts from one completed scene encode.
///
/// The runner is the only production owner that can construct this token. Its
/// private fields keep the authoritative scene, evidence, eligibility plan,
/// payloads, and target generation together until the materializer consumes it.
pub(super) struct NativePaintSegmentArtifactAdmission<'a> {
    scene: &'a Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: NativePaintSegmentEligibilityPlan,
    payloads: Vec<NativePaintSegmentPayload>,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
}

impl<'a> NativePaintSegmentArtifactAdmission<'a> {
    pub(super) fn into_parts(
        self,
    ) -> (
        &'a Scene,
        ArtifactFeasibilityObservation,
        NativePaintSegmentEligibilityPlan,
        Vec<NativePaintSegmentPayload>,
        NativeSceneValidityFingerprint,
        NativeTargetGeneration,
    ) {
        let Self {
            scene,
            feasibility,
            plan,
            payloads,
            scene_validity,
            target_generation,
        } = self;
        (
            scene,
            feasibility,
            plan,
            payloads,
            scene_validity,
            target_generation,
        )
    }
}

#[cfg(test)]
pub(super) fn materialize_native_paint_segment_artifacts_for_test(
    scene: &Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: NativePaintSegmentEligibilityPlan,
    payloads: Vec<NativePaintSegmentPayload>,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
) -> super::scene::NativePaintSegmentArtifactMaterialization {
    materialize_native_paint_segment_artifacts(NativePaintSegmentArtifactAdmission {
        scene,
        feasibility,
        plan,
        payloads,
        scene_validity,
        target_generation,
    })
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    const REDRAW_REISSUE_AFTER: Duration = Duration::from_millis(16);
    const REDRAW_REISSUE_LOG_AFTER: Duration = Duration::from_millis(32);

    pub(super) fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        let (native_window_diagnostic_identity_allocator, native_window_diagnostic_identity) =
            NativeWindowDiagnosticIdentityAllocator::for_primary();
        Self::new_with_diagnostic_identity(
            options,
            bridge,
            viewport,
            native_window_diagnostic_identity,
            native_window_diagnostic_identity_allocator,
        )
    }

    pub(super) fn new_with_diagnostic_identity(
        options: NativeRunOptions,
        bridge: Bridge,
        viewport: Vector2,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
    ) -> Self {
        Self::new_with_diagnostic_identity_and_schedule_key(
            options,
            bridge,
            viewport,
            native_window_diagnostic_identity,
            native_window_diagnostic_identity_allocator,
            FrameScheduleKey::Primary,
            false,
        )
    }

    pub(super) fn new_auxiliary_with_diagnostic_identity(
        options: NativeRunOptions,
        bridge: Bridge,
        viewport: Vector2,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
        key: String,
    ) -> Self {
        Self::new_with_diagnostic_identity_and_schedule_key(
            options,
            bridge,
            viewport,
            native_window_diagnostic_identity,
            native_window_diagnostic_identity_allocator,
            FrameScheduleKey::Auxiliary(key),
            true,
        )
    }

    #[cfg(test)]
    pub(super) fn new_auxiliary(
        options: NativeRunOptions,
        bridge: Bridge,
        viewport: Vector2,
        key: String,
    ) -> Self {
        Self::new_auxiliary_with_diagnostic_identity(
            options,
            bridge,
            viewport,
            None,
            NativeWindowDiagnosticIdentityAllocator::exhausted(),
            key,
        )
    }

    fn new_with_diagnostic_identity_and_schedule_key(
        options: NativeRunOptions,
        bridge: Bridge,
        viewport: Vector2,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
        frame_schedule_key: FrameScheduleKey,
        auxiliary_owner: bool,
    ) -> Self {
        let activation_reveal = ActivationRevealController::new(&options);
        let atlas_residency_window_identity = match &frame_schedule_key {
            FrameScheduleKey::Primary => NativeAtlasResidencyWindowIdentity::Primary,
            FrameScheduleKey::Auxiliary(key) => {
                NativeAtlasResidencyWindowIdentity::Auxiliary(key.clone())
            }
        };
        let text_renderer = NativeTextRenderer::with_options(&options.text);
        let debug_layout = options.frame.debug_layout;
        let devtools_overlay = options.frame.devtools;
        let retained_surface_cache = options.frame.retained_surface_cache;
        let core = GenericNativeRuntimeCore::new_with_frame_options(
            bridge,
            viewport,
            debug_layout,
            devtools_overlay,
        );
        let frame_diagnostics_enabled = core.has_frame_diagnostics_observer();
        let native_ime_adapter_observer_enabled = core.has_native_ime_adapter_observer();
        let frame_profile_enabled =
            options.frame.profiling.is_frame() && core.has_frame_profile_observer();
        let frame_gpu_timing_enabled =
            options.frame.profiling.is_frame() && core.has_frame_gpu_timing_observer();
        let gpu_timing_route = match &frame_schedule_key {
            FrameScheduleKey::Primary => NativeGpuTimingRoute::Primary,
            FrameScheduleKey::Auxiliary(key) => NativeGpuTimingRoute::Auxiliary(key.clone()),
        };
        let frame_observation_enabled = frame_diagnostics_enabled || frame_profile_enabled;
        Self {
            options,
            core,
            runtime_wakeup: RuntimeWakeup::default(),
            activation_reveal,
            application_reopen_proxy: None,
            application_reopen_events: None,
            adapter: None,
            atlas_residency_account: None,
            signal_residency_account: None,
            custom_shader_residency_account: None,
            render_canvas_upload_account: None,
            atlas_residency_window_identity,
            #[cfg(target_os = "macos")]
            native_semantic_accessibility: None,
            window: NativeRunnerWindowState::default(),
            frame: NativeVelloFrameState::new(text_renderer, retained_surface_cache),
            input: NativeRunnerInputState::default(),
            timing: NativeRunnerTimingState::new(native_window_diagnostic_identity),
            native_window_diagnostic_identity_allocator,
            frame_scheduler: NativeFrameScheduler::default(),
            frame_stage_owner: WindowStageOwner::new(frame_schedule_key.clone()),
            cpu_frame_fairness: (!auxiliary_owner).then(CpuFrameFairnessLedger::default),
            cpu_frame_observation: frame_diagnostics_enabled
                .then(CpuFrameObservationLedger::default)
                .or_else(|| frame_profile_enabled.then(CpuFrameObservationLedger::default))
                .filter(|_| !auxiliary_owner),
            cpu_frame_observation_capture: CpuFrameObservationCapture::default(),
            frame_diagnostics_enabled,
            frame_profile_enabled,
            frame_gpu_timing_enabled,
            gpu_timing_route,
            frame_observation_enabled,
            frame_diagnostics_publication: NativeFrameDiagnosticsPublication::default(),
            native_ime_adapter_observer_enabled,
            native_ime_adapter_observation: None,
            automation_targets: NativeAutomationTargetExporter::from_env(),
            auxiliary_windows: Vec::new(),
            native_lifecycle: NativeLifecycle::default(),
            auxiliary_owner,
            terminal_cause: None,
            recovery: NativeRecoveryCoordinator::default(),
            renderer_recovery: NativeRendererRecoveryPolicy::default(),
            recovery_cause: None,
            recovery_auxiliary_followup_pending: false,
        }
    }

    /// Synchronize the adapter-owned atlas-residency account with the physical
    /// active/quarantine bundles currently retained by this runner. The atlas
    /// snapshot is fixed active/Q0/Q1 bookkeeping rather than a resource-map
    /// traversal. Profile-enabled presentation calls this after cache
    /// mutations; lifecycle transitions call it unconditionally.
    pub(super) fn refresh_atlas_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        let resources_empty = self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty();
        if resources_empty {
            if let Some(token) = self.atlas_residency_account.as_ref()
                && adapter.remove_atlas_residency_account(token)
            {
                self.atlas_residency_account = None;
            }
            return;
        }

        let Some(adapter_generation) = adapter.capture_generation() else {
            return;
        };
        let snapshots: NativeWindowAtlasResidencySnapshots =
            self.window.atlas_residency_snapshots();
        self.synchronize_atlas_residency_account(adapter, adapter_generation, snapshots);
    }

    /// Synchronize the adapter-owned signal-residency account with the
    /// physical active/quarantine bundles currently retained by this runner.
    /// Signal snapshots use fixed active/Q0/Q1 bookkeeping rather than a
    /// resource-map traversal. Lifecycle transitions call this unconditionally;
    /// profile-enabled presentation captures the cached aggregate after signal
    /// cache mutation.
    pub(super) fn refresh_signal_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        let resources_empty = self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty();
        if resources_empty {
            if let Some(token) = self.signal_residency_account.as_ref()
                && adapter.remove_signal_residency_account(token)
            {
                self.signal_residency_account = None;
            }
            return;
        }

        let Some(adapter_generation) = adapter.capture_generation() else {
            return;
        };
        let snapshots: NativeWindowSignalResidencySnapshots =
            self.window.signal_residency_snapshots();
        self.synchronize_signal_residency_account(adapter, adapter_generation, snapshots);
    }

    /// Synchronize the adapter-owned custom-shader residency account with the
    /// physical active/quarantine bundles currently retained by this runner.
    /// Custom-shader snapshots use fixed active/Q0/Q1 bookkeeping rather than
    /// a resource-map traversal. Lifecycle transitions call this unconditionally;
    /// profile-enabled presentation captures the cached aggregate after cache
    /// mutation.
    pub(super) fn refresh_custom_shader_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        let resources_empty = self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty();
        if resources_empty {
            if let Some(token) = self.custom_shader_residency_account.as_ref()
                && adapter.remove_custom_shader_residency_account(token)
            {
                self.custom_shader_residency_account = None;
            }
            return;
        }

        let Some(adapter_generation) = adapter.capture_generation() else {
            return;
        };
        let snapshots: NativeWindowCustomShaderResidencySnapshots =
            self.window.custom_shader_residency_snapshots();
        self.synchronize_custom_shader_residency_account(adapter, adapter_generation, snapshots);
    }

    /// Synchronize the adapter-owned render-canvas upload account at resource
    /// and lifecycle boundaries. Successful presentation only contributes to
    /// the already-bound account.
    pub(super) fn refresh_render_canvas_upload_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        let resources_empty = self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty();
        if resources_empty {
            if let Some(token) = self.render_canvas_upload_account.as_ref()
                && adapter.remove_render_canvas_upload_account(token)
            {
                self.render_canvas_upload_account = None;
            }
            return;
        }

        let Some(adapter_generation) = adapter.capture_generation() else {
            return;
        };
        self.synchronize_render_canvas_upload_account(adapter, adapter_generation);
    }

    pub(super) fn capture_atlas_residency_profile(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        profile_enabled: bool,
    ) -> NativeAdapterAtlasResidencyProfile {
        if !profile_enabled {
            return NativeAdapterAtlasResidencyProfile::default();
        }
        self.refresh_atlas_residency_account(adapter);
        adapter.capture_atlas_residency_profile()
    }

    pub(super) fn capture_signal_residency_profile(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        profile_enabled: bool,
    ) -> NativeAdapterSignalResidencyProfile {
        if !profile_enabled {
            return NativeAdapterSignalResidencyProfile::default();
        }
        self.refresh_signal_residency_account(adapter);
        adapter.capture_signal_residency_profile()
    }

    pub(super) fn capture_custom_shader_residency_profile(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        profile_enabled: bool,
    ) -> NativeAdapterCustomShaderResidencyProfile {
        if !profile_enabled {
            return NativeAdapterCustomShaderResidencyProfile::default();
        }
        self.refresh_custom_shader_residency_account(adapter);
        adapter.capture_custom_shader_residency_profile()
    }

    pub(super) fn capture_render_canvas_upload_profile(
        &self,
        adapter: &GenericNativeAdapterOwner,
        profile_enabled: bool,
    ) -> NativeAdapterRenderCanvasUploadProfile {
        if !profile_enabled {
            return NativeAdapterRenderCanvasUploadProfile::default();
        }
        adapter.capture_render_canvas_upload_profile()
    }

    pub(super) fn current_render_canvas_upload_plan_context(
        &self,
        adapter: &GenericNativeAdapterOwner,
        encode_present: NativeEncodePresentPlanContext,
    ) -> Option<GpuSurfaceRenderCanvasUploadPlanContext> {
        let resources = self.window.native_resources.as_ref()?;
        let device = adapter.device_handle_for_surface(&resources.render_surface)?;
        GpuSurfaceRenderCanvasUploadPlanContext::new(
            encode_present,
            resources.generation,
            GpuSurfaceRenderCanvasUploadTarget::new(
                wgpu_device_id(&device.device),
                resources.render_surface.config.format,
                resources.render_surface.config.width,
                resources.render_surface.config.height,
            ),
        )
    }

    pub(super) fn contribute_render_canvas_uploads(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        frame_sequence: Option<u64>,
        stats: GpuSurfaceRenderCanvasUploadStats,
        candidate_plan: Option<GpuSurfaceRenderCanvasUploadPlan>,
        current_plan_context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    ) -> bool {
        let Some(frame_sequence) = frame_sequence else {
            return false;
        };
        let Some(token) = self.render_canvas_upload_account.as_ref() else {
            return false;
        };
        adapter.contribute_render_canvas_uploads(
            token,
            frame_sequence,
            stats,
            candidate_plan,
            current_plan_context,
        )
    }

    fn synchronize_atlas_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) {
        if let Some(token) = self.atlas_residency_account.as_mut() {
            let accepted = if token.adapter_generation == adapter_generation {
                adapter.update_atlas_residency_account(token, snapshots)
            } else if let Some(next) =
                adapter.rebind_atlas_residency_account(token, adapter_generation, snapshots)
            {
                *token = next;
                true
            } else {
                false
            };
            if !accepted {
                // A live token can outlast its ledger account. Re-register only
                // after rejection; a refused same-key registration preserves
                // the stale-token fence instead of overwriting its owner.
                if let Some(next) = adapter.register_atlas_residency_account(
                    token.window_identity.clone(),
                    adapter_generation,
                    snapshots,
                ) {
                    *token = next;
                }
            }
        } else if let Some(token) = adapter.register_atlas_residency_account(
            self.atlas_residency_window_identity.clone(),
            adapter_generation,
            snapshots,
        ) {
            self.atlas_residency_account = Some(token);
        }
    }

    fn synchronize_signal_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) {
        if let Some(token) = self.signal_residency_account.as_mut() {
            let accepted = if token.adapter_generation == adapter_generation {
                adapter.update_signal_residency_account(token, snapshots)
            } else if let Some(next) =
                adapter.rebind_signal_residency_account(token, adapter_generation, snapshots)
            {
                *token = next;
                true
            } else {
                false
            };
            if !accepted {
                // A live token can outlast its ledger account. Re-register only
                // after rejection; a refused same-key registration preserves
                // the stale-token fence instead of overwriting its owner.
                if let Some(next) = adapter.register_signal_residency_account(
                    token.window_identity.clone(),
                    adapter_generation,
                    snapshots,
                ) {
                    *token = next;
                }
            }
        } else if let Some(token) = adapter.register_signal_residency_account(
            self.atlas_residency_window_identity.clone(),
            adapter_generation,
            snapshots,
        ) {
            self.signal_residency_account = Some(token);
        }
    }

    fn synchronize_custom_shader_residency_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) {
        if let Some(token) = self.custom_shader_residency_account.as_mut() {
            let accepted = if token.adapter_generation == adapter_generation {
                adapter.update_custom_shader_residency_account(token, snapshots)
            } else if let Some(next) =
                adapter.rebind_custom_shader_residency_account(token, adapter_generation, snapshots)
            {
                *token = next;
                true
            } else {
                false
            };
            if !accepted {
                // A live token can outlast its ledger account. Re-register only
                // after rejection; a refused same-key registration preserves
                // the stale-token fence instead of overwriting its owner.
                if let Some(next) = adapter.register_custom_shader_residency_account(
                    token.window_identity.clone(),
                    adapter_generation,
                    snapshots,
                ) {
                    *token = next;
                }
            }
        } else if let Some(token) = adapter.register_custom_shader_residency_account(
            self.atlas_residency_window_identity.clone(),
            adapter_generation,
            snapshots,
        ) {
            self.custom_shader_residency_account = Some(token);
        }
    }

    fn synchronize_render_canvas_upload_account(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        adapter_generation: NativeAdapterGeneration,
    ) {
        if let Some(token) = self.render_canvas_upload_account.as_mut() {
            let accepted = if token.adapter_generation == adapter_generation {
                adapter.update_render_canvas_upload_account(token)
            } else if let Some(next) =
                adapter.rebind_render_canvas_upload_account(token, adapter_generation)
            {
                *token = next;
                true
            } else {
                false
            };
            if !accepted
                && let Some(next) = adapter.register_render_canvas_upload_account(
                    token.window_identity.clone(),
                    adapter_generation,
                )
            {
                *token = next;
            }
        } else if let Some(token) = adapter.register_render_canvas_upload_account(
            self.atlas_residency_window_identity.clone(),
            adapter_generation,
        ) {
            self.render_canvas_upload_account = Some(token);
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn republish_native_semantic_accessibility_passively(&mut self) {
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        match adapter.publish_passive(&self.core.runtime, self.window.native_window_focused) {
            Ok(()) => self.native_semantic_accessibility = Some(adapter),
            Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
        }
    }

    #[cfg(target_os = "macos")]
    fn discard_failed_native_semantic_accessibility(
        &mut self,
        mut adapter: NativeSemanticAccessibilityAdapter,
        error: String,
    ) {
        adapter.close_lease(&mut self.core.runtime);
        adapter.retire();
        warn!(
            error = %error,
            "radiant native semantic accessibility adapter retired after host publication failure"
        );
    }

    pub(super) fn require_primary_frame_diagnostics_schedule_admission(&mut self) {
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .require_schedule_admission();
        }
    }

    pub(super) fn stage_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        if !self.frame_observation_enabled {
            return;
        }
        if self.auxiliary_owner {
            // Auxiliary runners retain the existing one-slot child-to-parent
            // bridge handoff. This is not the application callback; the
            // parent drains and publishes it at the event boundary.
            self.core
                .runtime
                .host_observe_frame_diagnostics(diagnostics);
        } else {
            self.frame_diagnostics_publication.stage(diagnostics);
        }
    }

    pub(super) fn publish_staged_frame_diagnostics(&mut self) {
        if !self.frame_observation_enabled {
            return;
        }
        if let Some(mut diagnostics) = self.frame_diagnostics_publication.take_ready() {
            diagnostics.cpu_fairness = self
                .cpu_frame_fairness
                .as_ref()
                .map_or_else(NativeCpuFrameFairnessDiagnostics::default, |ledger| {
                    ledger.project_frame_diagnostics(&FrameScheduleKey::Primary)
                });
            diagnostics.cpu_observation = self
                .cpu_frame_observation
                .as_ref()
                .map_or_else(NativeCpuFrameObservationDiagnostics::default, |ledger| {
                    ledger.project_frame_diagnostics(&FrameScheduleKey::Primary)
                });
            if self.frame_diagnostics_enabled {
                self.core
                    .runtime
                    .host_observe_frame_diagnostics(diagnostics);
            }
            if self.frame_profile_enabled {
                self.core
                    .runtime
                    .host_observe_frame_profile(FrameProfile::from(diagnostics));
            }
        }
    }

    pub(super) fn allocate_auxiliary_window_diagnostic_identity(
        &mut self,
    ) -> Option<NativeWindowDiagnosticIdentity> {
        self.native_window_diagnostic_identity_allocator.allocate()
    }

    #[cfg(test)]
    pub(super) fn record_frame_schedule_admission(&mut self, key: FrameScheduleKey) {
        self.record_frame_schedule_admission_with_lane(
            key,
            FrameScheduleLane::Visual,
            false,
            false,
        );
    }

    pub(super) fn record_frame_schedule_admission_with_lane(
        &mut self,
        key: FrameScheduleKey,
        lane: FrameScheduleLane,
        visual_deadline_completed: bool,
        maintenance_due: bool,
    ) {
        let is_primary = matches!(&key, FrameScheduleKey::Primary);
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.mark_admitted(&key);
        }
        self.frame_scheduler.record_selected_admission(
            key,
            lane,
            visual_deadline_completed,
            maintenance_due,
        );
        if self.frame_observation_enabled && !self.auxiliary_owner && is_primary {
            self.frame_diagnostics_publication
                .mark_schedule_admission_recorded();
        }
    }

    pub(super) fn begin_cpu_frame_observation(
        &mut self,
        key: FrameScheduleKey,
        now: Instant,
    ) -> Option<CpuFrameObservationAdmission> {
        let (frame_work, cadence_target_fps, pending_redraw_age) =
            self.cpu_frame_observation_snapshot(now);
        self.cpu_frame_observation
            .as_mut()
            .map(|ledger| ledger.begin(key, frame_work, cadence_target_fps, pending_redraw_age))
    }

    fn cpu_frame_observation_snapshot(
        &mut self,
        now: Instant,
    ) -> (FrameWork, Option<u32>, CpuFramePendingRedrawAge) {
        let frame_work = self.timing.pending_frame_work;
        let cadence_target_fps = Some(timed_frame_target_fps(
            self.options.normalized_target_fps(),
            self.core.animation_activity(),
            self.core.has_focused_text_input(),
        ));
        let pending_redraw_age = self.pending_redraw_age(now);
        (frame_work, cadence_target_fps, pending_redraw_age)
    }

    pub(super) fn begin_cpu_frame_observation_with_owner(
        &mut self,
        owner: &mut CpuFrameObservationOwner<'_>,
        now: Instant,
    ) -> CpuFrameObservationAdmission {
        let (frame_work, cadence_target_fps, pending_redraw_age) =
            self.cpu_frame_observation_snapshot(now);
        owner.begin(frame_work, cadence_target_fps, pending_redraw_age)
    }

    pub(super) fn finish_cpu_frame_observation(
        &mut self,
        admission: Option<CpuFrameObservationAdmission>,
        redraw_failed: bool,
    ) {
        let capture = std::mem::take(&mut self.cpu_frame_observation_capture);
        self.finish_cpu_frame_observation_with_capture(admission, capture, redraw_failed);
    }

    pub(super) fn finish_cpu_frame_observation_with_capture(
        &mut self,
        admission: Option<CpuFrameObservationAdmission>,
        capture: CpuFrameObservationCapture,
        redraw_failed: bool,
    ) {
        let (Some(ledger), Some(admission)) = (self.cpu_frame_observation.as_mut(), admission)
        else {
            return;
        };
        ledger.finish(admission, capture, redraw_failed);
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn finish_cpu_frame_observation_with_owner(
        &mut self,
        owner: &mut CpuFrameObservationOwner<'_>,
        admission: CpuFrameObservationAdmission,
        redraw_failed: bool,
    ) {
        let capture = std::mem::take(&mut self.cpu_frame_observation_capture);
        owner.finish(admission, capture, redraw_failed);
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn take_cpu_frame_observation_capture(&mut self) -> CpuFrameObservationCapture {
        std::mem::take(&mut self.cpu_frame_observation_capture)
    }

    pub(super) fn mark_cpu_frame_observation_recovery(&mut self) {
        self.cpu_frame_observation_capture.mark_recovery_triggered();
    }

    pub(super) fn remove_cpu_frame_observation(&mut self, key: &FrameScheduleKey) {
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.remove(key);
        }
        if let Some(ledger) = self.cpu_frame_observation.as_mut() {
            ledger.remove(key);
        }
    }

    pub(super) const fn is_running(&self) -> bool {
        self.native_lifecycle.is_running()
    }

    pub(super) const fn is_auxiliary_owner(&self) -> bool {
        self.auxiliary_owner
    }

    pub(super) const fn is_closing(&self) -> bool {
        self.native_lifecycle.is_closing()
    }

    pub(super) const fn is_recovering(&self) -> bool {
        self.native_lifecycle.is_recovering()
    }

    pub(super) const fn native_lifecycle_snapshot(&self) -> NativeLifecycle {
        self.native_lifecycle
    }

    #[cfg(target_os = "macos")]
    pub(super) fn attach_native_semantic_accessibility(
        &mut self,
        proxy: EventLoopProxy<super::RuntimeUserEvent>,
    ) {
        if self.auxiliary_owner || self.native_semantic_accessibility.is_some() {
            return;
        }
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        match NativeSemanticAccessibilityAdapter::attach(&window, proxy) {
            Ok(mut adapter) => match adapter
                .publish_passive(&self.core.runtime, self.window.native_window_focused)
            {
                Ok(()) => self.native_semantic_accessibility = Some(adapter),
                Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
            },
            Err(error) => {
                warn!(error = %error, "radiant native semantic accessibility attachment withheld");
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_native_semantic_accessibility_query(
        &mut self,
        query: super::super::runtime_event::NativeSemanticAccessibilityQuery,
    ) {
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        adapter.handle_query(&mut self.core.runtime, query);
        if adapter.is_attached() {
            self.native_semantic_accessibility = Some(adapter);
        } else {
            self.discard_failed_native_semantic_accessibility(
                adapter,
                String::from("native semantic query publication failed"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_native_numeric_accessibility_action(
        &mut self,
        token: u64,
        target: crate::gui::automation::AutomationTarget,
        action: super::super::runtime_event::NativeNumericAccessibilityAction,
    ) {
        let request = {
            let Some(adapter) = self.native_semantic_accessibility.as_mut() else {
                return;
            };
            adapter.finish_numeric_action();
            adapter.numeric_accessibility_request(token, target, action)
        };
        let Some(request) = request else {
            return;
        };
        let _ = self
            .core
            .runtime
            .dispatch_numeric_accessibility_action(request);
        self.republish_native_semantic_accessibility_passively();
    }

    #[cfg(target_os = "macos")]
    pub(super) fn invalidate_native_semantic_accessibility_geometry(&mut self) {
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        {
            adapter.invalidate_window_generation(&window);
        }
        match adapter.publish_passive(&self.core.runtime, self.window.native_window_focused) {
            Ok(()) => self.native_semantic_accessibility = Some(adapter),
            Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn close_native_semantic_accessibility(&mut self) {
        if let Some(mut adapter) = self.native_semantic_accessibility.take() {
            adapter.close_lease(&mut self.core.runtime);
            adapter.retire();
        }
    }

    pub(super) const fn is_stopped(&self) -> bool {
        self.native_lifecycle.is_stopped()
    }

    pub(super) fn recovery_deadline(&self) -> Option<Instant> {
        self.native_lifecycle.recovery_deadline()
    }

    pub(super) fn recovery_expired(&self, now: Instant) -> bool {
        self.native_lifecycle.recovery_expired(now)
    }

    fn native_lifecycle_stage_evidence(
        &self,
        transition: NativeLifecycleTransitionKind,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> NativeLifecycleStageEvidence {
        NativeLifecycleStageEvidence {
            key: self.frame_stage_owner.schedule_key().clone(),
            transition,
            source_phase: self.native_lifecycle,
            window_id: self.window.id,
            adapter_generation,
            active_resource_generation: self
                .window
                .native_resources
                .as_ref()
                .map(|resources| resources.generation),
            target_generation: self.window.target_generation,
            target_fenced: self.window.native_surface_target_fenced,
        }
    }

    /// Stage one exact lifecycle transition before any native/controller
    /// lifecycle phase mutates.  Target and resource absence remain part of
    /// the exact evidence rather than being rejected as unusable post-
    /// transition state.
    pub(super) fn admit_native_lifecycle(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        let evidence = self.native_lifecycle_stage_evidence(
            NativeLifecycleTransitionKind::BeginDeviceRecovery,
            adapter_generation,
        );
        admit_native_lifecycle_stage(&mut self.frame_stage_owner, evidence)
    }

    pub(super) fn admit_native_lifecycle_finish(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        let evidence = self.native_lifecycle_stage_evidence(
            NativeLifecycleTransitionKind::FinishDeviceRecovery,
            adapter_generation,
        );
        admit_native_lifecycle_stage(&mut self.frame_stage_owner, evidence)
    }

    /// Stage one exact terminal Closing transition for whole-run shutdown or
    /// an independent child-local close. `None` is preserved as exact
    /// absent-adapter evidence; recovery admission above continues to require
    /// a known shared generation.
    pub(super) fn admit_native_closing(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        let evidence = self.native_lifecycle_stage_evidence(
            NativeLifecycleTransitionKind::BeginClosing,
            adapter_generation,
        );
        admit_native_lifecycle_stage(&mut self.frame_stage_owner, evidence)
    }

    #[cfg(test)]
    pub(super) fn admit_native_lifecycle_finish_with_evidence(
        &mut self,
        evidence: NativeLifecycleStageEvidence,
    ) -> Option<NativeLifecycleStageTicket> {
        admit_native_lifecycle_stage(&mut self.frame_stage_owner, evidence)
    }

    pub(super) fn native_lifecycle_stage_ticket_is_current(
        &self,
        ticket: &NativeLifecycleStageTicket,
    ) -> bool {
        self.frame_stage_owner.lifecycle_ticket_is_current(
            // The owner check does not consume or clone the ticket.  Native
            // evidence currentness is checked separately before transition.
            ticket.stage_ticket(),
        )
    }

    pub(super) fn native_lifecycle_ticket_is_current(
        &self,
        ticket: &NativeLifecycleStageTicket,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> bool {
        let evidence =
            self.native_lifecycle_stage_evidence(ticket.transition(), adapter_generation);
        ticket.is_current(&self.frame_stage_owner, &evidence)
    }

    #[cfg(test)]
    pub(super) fn native_lifecycle_ticket_is_current_with_evidence(
        &self,
        ticket: &NativeLifecycleStageTicket,
        evidence: &NativeLifecycleStageEvidence,
    ) -> bool {
        ticket.is_current(&self.frame_stage_owner, evidence)
    }

    pub(super) fn complete_native_lifecycle(&mut self, ticket: NativeLifecycleStageTicket) -> bool {
        complete_native_lifecycle_stage(&mut self.frame_stage_owner, ticket)
    }

    pub(super) fn veto_native_lifecycle(&mut self, ticket: NativeLifecycleStageTicket) -> bool {
        veto_native_lifecycle_stage(&mut self.frame_stage_owner, ticket)
    }

    /// Bind the current input/transient soft budget for an exact DiscreteInput
    /// attempt. This path is authoritative for its private policy consumer,
    /// so it does not depend on diagnostics or frame observation being enabled.
    fn discrete_input_budget_binding(&mut self) -> FrameStageBudgetBinding {
        let effective_fps = timed_frame_target_fps(
            self.options.normalized_target_fps(),
            self.core.animation_activity(),
            self.core.has_focused_text_input(),
        );
        let budget = SchedulerSoftBudgets::for_effective_fps(effective_fps).input_transient;
        FrameStageBudgetBinding::input_transient(budget)
    }

    /// Bind the current input/transient soft budget for every exact
    /// ImmediateTransient attempt. The budget is authoritative and therefore
    /// independent of diagnostics and frame observation availability.
    fn immediate_transient_budget_binding(&mut self) -> FrameStageBudgetBinding {
        self.discrete_input_budget_binding()
    }

    fn native_discrete_input_stage_evidence(
        &self,
        kind: NativeDiscreteInputKind,
        timestamp: InputTimestamp,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> NativeDiscreteInputStageEvidence {
        NativeDiscreteInputStageEvidence {
            key: self.frame_stage_owner.schedule_key().clone(),
            kind,
            timestamp,
            window_id: self.window.id,
            adapter_generation,
            active_resource_generation: self
                .window
                .native_resources
                .as_ref()
                .map(|resources| resources.generation),
            target_generation: self.window.target_generation,
            native_surface_target_fenced: self.window.native_surface_target_fenced,
            lifecycle: self.native_lifecycle,
            native_window_eligible: self
                .native_discrete_input_native_window_is_eligible(adapter_generation),
            wrapper_eligible,
        }
    }

    fn native_discrete_input_native_window_is_eligible(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && self.window.id.is_some()
            && self.window.window.is_some()
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.generation == adapter_generation)
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
    }

    /// Stage one exact native DiscreteInput event for an auxiliary runner that
    /// borrows the parent adapter at its event boundary.
    pub(super) fn admit_native_discrete_input_with_generation(
        &mut self,
        kind: NativeDiscreteInputKind,
        timestamp: InputTimestamp,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> Option<NativeDiscreteInputStageTicket> {
        let evidence = self.native_discrete_input_stage_evidence(
            kind,
            timestamp,
            adapter_generation,
            wrapper_eligible,
        );
        let budget = self.discrete_input_budget_binding();
        admit_native_discrete_input_stage_with_budget(&mut self.frame_stage_owner, evidence, budget)
    }

    /// Capture the exact safe-boundary admission for one native input event.
    /// Deferred Deadline work is completed first, after the caller has already
    /// captured `timestamp`; a failed pre-route currentness check is inert.
    pub(super) fn begin_native_discrete_input_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        kind: NativeDiscreteInputKind,
        timestamp: InputTimestamp,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> Option<NativeDiscreteInputStageTicket> {
        if !wrapper_eligible
            || !self.native_discrete_input_native_window_is_eligible(adapter_generation)
        {
            return None;
        }
        if !self.resume_deferred_deadline_before_discrete_input(event_loop, adapter_generation) {
            return None;
        }
        let ticket = self.admit_native_discrete_input_with_generation(
            kind,
            timestamp,
            adapter_generation,
            wrapper_eligible,
        )?;
        if !self.native_discrete_input_ticket_is_current(
            &ticket,
            adapter_generation,
            wrapper_eligible,
        ) {
            let _ = self.veto_native_discrete_input(ticket);
            return None;
        }
        Some(ticket)
    }

    pub(super) fn native_discrete_input_ticket_is_current(
        &self,
        ticket: &NativeDiscreteInputStageTicket,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> bool {
        let captured = ticket.evidence();
        let evidence = self.native_discrete_input_stage_evidence(
            captured.kind,
            captured.timestamp,
            adapter_generation,
            wrapper_eligible,
        );
        ticket.is_current(&self.frame_stage_owner, evidence)
    }

    pub(super) fn complete_native_discrete_input(
        &mut self,
        ticket: NativeDiscreteInputStageTicket,
    ) -> DiscreteInputCompletion {
        complete_native_discrete_input_stage(&mut self.frame_stage_owner, ticket)
    }

    /// Attach the exact completion's policy disposition to its already-routed
    /// outcome. A mismatch is terminal for the route and cannot authorize a
    /// fallback or recover policy from mutable owner evidence.
    pub(super) fn complete_native_discrete_input_route(
        &mut self,
        ticket: NativeDiscreteInputStageTicket,
        outcome: GenericRouteOutcome,
    ) -> Option<GenericRouteOutcome> {
        let disposition =
            discrete_input_completion_disposition(self.complete_native_discrete_input(ticket))?;
        Some(outcome.with_native_input_stage_disposition(disposition))
    }

    pub(super) fn veto_native_discrete_input(
        &mut self,
        ticket: NativeDiscreteInputStageTicket,
    ) -> bool {
        veto_native_discrete_input_stage(&mut self.frame_stage_owner, ticket)
    }

    fn native_immediate_transient_stage_evidence(
        &self,
        kind: NativeImmediateTransientKind,
        timestamp: InputTimestamp,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> NativeImmediateTransientStageEvidence {
        NativeImmediateTransientStageEvidence {
            key: self.frame_stage_owner.schedule_key().clone(),
            kind,
            timestamp,
            window_id: self.window.id,
            adapter_generation,
            active_resource_generation: self
                .window
                .native_resources
                .as_ref()
                .map(|resources| resources.generation),
            target_generation: self.window.target_generation,
            native_surface_target_fenced: self.window.native_surface_target_fenced,
            lifecycle: self.native_lifecycle,
            native_window_eligible: self
                .native_immediate_transient_native_window_is_eligible(adapter_generation),
            wrapper_eligible,
        }
    }

    fn native_immediate_transient_native_window_is_eligible(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && self.window.id.is_some()
            && self.window.window.is_some()
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.generation == adapter_generation)
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
    }

    /// Stage one exact native ImmediateTransient event for a primary or
    /// auxiliary runner.  The caller must capture `timestamp` before this
    /// boundary and keep the returned ticket live through synchronous local
    /// routing and message reduction.
    pub(super) fn begin_native_immediate_transient_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        kind: NativeImmediateTransientKind,
        timestamp: InputTimestamp,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> Option<NativeImmediateTransientStageTicket> {
        if !wrapper_eligible
            || !self.native_immediate_transient_native_window_is_eligible(adapter_generation)
        {
            return None;
        }
        if !self.resume_deferred_deadline_before_immediate_transient(event_loop, adapter_generation)
        {
            return None;
        }
        let evidence = self.native_immediate_transient_stage_evidence(
            kind,
            timestamp,
            adapter_generation,
            wrapper_eligible,
        );
        let budget = self.immediate_transient_budget_binding();
        let ticket = admit_native_immediate_transient_stage_with_budget(
            &mut self.frame_stage_owner,
            evidence,
            budget,
        )?;
        if !self.native_immediate_transient_ticket_is_current(
            &ticket,
            adapter_generation,
            wrapper_eligible,
        ) {
            let _ = self.veto_native_immediate_transient(ticket);
            return None;
        }
        Some(ticket)
    }

    pub(super) fn native_immediate_transient_ticket_is_current(
        &self,
        ticket: &NativeImmediateTransientStageTicket,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> bool {
        let captured = ticket.evidence();
        let evidence = self.native_immediate_transient_stage_evidence(
            captured.kind,
            captured.timestamp,
            adapter_generation,
            wrapper_eligible,
        );
        ticket.is_current(&self.frame_stage_owner, evidence)
    }

    /// Revalidate the exact transient witness immediately before its caller
    /// mutates runtime-local state. A failed revalidation consumes only the
    /// incoming witness and leaves the event inert.
    pub(super) fn revalidate_native_immediate_transient(
        &mut self,
        ticket: NativeImmediateTransientStageTicket,
        adapter_generation: NativeAdapterGeneration,
        wrapper_eligible: bool,
    ) -> Option<NativeImmediateTransientStageTicket> {
        if self.native_immediate_transient_ticket_is_current(
            &ticket,
            adapter_generation,
            wrapper_eligible,
        ) {
            Some(ticket)
        } else {
            let _ = self.veto_native_immediate_transient(ticket);
            None
        }
    }

    pub(super) fn complete_native_immediate_transient(
        &mut self,
        ticket: NativeImmediateTransientStageTicket,
    ) -> ImmediateTransientCompletion {
        complete_native_immediate_transient_stage(&mut self.frame_stage_owner, ticket)
    }

    /// Attach the exact completion's policy disposition to its already-routed
    /// outcome. A mismatch is terminal for the route and cannot authorize a
    /// fallback or recover policy from mutable owner evidence.
    pub(super) fn complete_native_immediate_transient_route(
        &mut self,
        ticket: NativeImmediateTransientStageTicket,
        outcome: GenericRouteOutcome,
    ) -> Option<GenericRouteOutcome> {
        let disposition = immediate_transient_completion_disposition(
            self.complete_native_immediate_transient(ticket),
        )?;
        Some(outcome.with_native_input_stage_disposition(disposition))
    }

    pub(super) fn veto_native_immediate_transient(
        &mut self,
        ticket: NativeImmediateTransientStageTicket,
    ) -> bool {
        veto_native_immediate_transient_stage(&mut self.frame_stage_owner, ticket)
    }

    pub(super) fn admit_device_recovery(&mut self) -> bool {
        if !self.native_lifecycle.admit_recovery() {
            return false;
        }
        if !self.core.begin_native_recovery() {
            let _ = self.native_lifecycle.finish_recovery();
            return false;
        }
        self.clear_cpu_frame_observation();
        self.fence_native_presentation();
        true
    }

    pub(super) fn clear_cpu_frame_observation(&mut self) {
        self.clear_cpu_frame_fairness();
        if let Some(ledger) = self.cpu_frame_observation.as_mut() {
            ledger.clear();
        }
    }

    pub(super) fn clear_cpu_frame_fairness(&mut self) {
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.clear();
        }
    }

    /// Apply the existing controller/native finish hooks after the caller has
    /// staged and revalidated an exact FinishDeviceRecovery Lifecycle ticket.
    /// Production callers must not use this hook as an admission path.
    pub(super) fn finish_device_recovery(&mut self) -> bool {
        if !self.native_lifecycle.is_recovering() {
            return false;
        }
        if !self.core.finish_native_recovery() {
            return false;
        }
        if self.native_lifecycle.finish_recovery() {
            return true;
        }
        let _ = self.core.begin_native_recovery();
        false
    }

    pub(super) const fn native_shutdown_requested(&self) -> bool {
        !self.native_lifecycle.is_running()
    }

    pub(super) fn record_terminal_cause(&mut self, cause: NativeGenericRunError) -> bool {
        if self.terminal_cause.is_some() {
            return false;
        }
        self.terminal_cause = Some(cause);
        true
    }

    pub(super) fn has_terminal_cause(&self) -> bool {
        self.terminal_cause.is_some()
    }

    pub(super) fn should_initialize_runtime(&self) -> bool {
        self.is_running() && self.window.window.is_none() && !self.has_terminal_cause()
    }

    pub(super) fn admit_native_shutdown(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: Option<NativeGenericRunError>,
    ) {
        // Repeated primary Closing/Stopped notifications are deliberately
        // inert.  A first terminal intent owns the only staging attempt.
        if self.is_closing() || self.native_lifecycle.is_stopped() {
            return;
        }

        let Some(((primary_ticket, auxiliary_tickets), now)) =
            self.admit_native_shutdown_preterminal(cause)
        else {
            return;
        };
        let mut auxiliary_preparation_failed = false;
        for window in &mut self.auxiliary_windows {
            let prepared = window.prepare_whole_run_closing();
            if !prepared {
                auxiliary_preparation_failed = true;
            } else {
                window.begin_whole_run_retiring(event_loop);
            }
        }
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        if auxiliary_preparation_failed {
            self.converge_post_terminal_native_shutdown(event_loop);
            return;
        }

        // Complete each exact ticket after its window's Closing fences.  A
        // completion fault consumes no replacement ticket; the remaining
        // witnesses are vetoed once and the bounded Closing authority takes
        // over without replay or redraw.
        let mut completion_failed = false;
        if !self.native_lifecycle_stage_ticket_is_current(&primary_ticket) {
            let _ = self.veto_native_lifecycle(primary_ticket);
            completion_failed = true;
        } else if !self.complete_native_lifecycle(primary_ticket) {
            completion_failed = true;
        }
        for (index, ticket) in auxiliary_tickets {
            if completion_failed {
                let _ = self.auxiliary_windows[index].veto_native_lifecycle(ticket);
            } else if !self.auxiliary_windows[index]
                .native_lifecycle_stage_ticket_is_current(&ticket)
            {
                let _ = self.auxiliary_windows[index].veto_native_lifecycle(ticket);
                completion_failed = true;
            } else if !self.auxiliary_windows[index].complete_native_lifecycle(ticket) {
                completion_failed = true;
            }
        }
        if completion_failed {
            self.converge_post_terminal_native_shutdown(event_loop);
            return;
        }

        self.finish_native_shutdown(event_loop, now);
    }

    fn admit_native_shutdown_preterminal(
        &mut self,
        cause: Option<NativeGenericRunError>,
    ) -> Option<NativeClosingAdmission> {
        let adapter_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let (primary_ticket, auxiliary_tickets) =
            self.stage_native_closing_set(adapter_generation)?;

        // Revalidate the complete no-yield set before any native, controller,
        // presentation, wrapper, cause, recovery, mailbox, or resource state
        // is changed.  Each ticket rechecks its full captured evidence.
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        if !self.native_closing_stage_set_is_current(
            &primary_ticket,
            &auxiliary_tickets,
            current_generation,
        ) {
            self.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);
            return None;
        }

        // Reuse the existing per-window Closing preparation only after the
        // complete staged set is current.  The primary owns recovery-cause
        // precedence and the first terminal cause; children retain the same
        // bounded preparation but never dispatch their close messages.
        let Some(now) = self.prepare_native_shutdown(cause) else {
            self.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);
            return None;
        };
        Some(((primary_ticket, auxiliary_tickets), now))
    }

    fn stage_native_closing_set(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeClosingStageSet> {
        let primary_ticket = self.admit_native_closing(adapter_generation)?;

        // The vector order is the resident auxiliary order and is therefore
        // the stable ticket order for this whole-run transition.  Retiring
        // wrappers remain eligible: their child runner can still be Running
        // or Recovering even though the wrapper no longer accepts messages.
        let mut auxiliary_tickets = Vec::with_capacity(self.auxiliary_windows.len());
        for index in 0..self.auxiliary_windows.len() {
            if !self.auxiliary_windows[index].should_stage_native_closing() {
                continue;
            }
            let Some(ticket) =
                self.auxiliary_windows[index].admit_native_closing(adapter_generation)
            else {
                self.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);
                return None;
            };
            auxiliary_tickets.push((index, ticket));
        }
        Some((primary_ticket, auxiliary_tickets))
    }

    fn native_closing_stage_set_is_current(
        &self,
        primary_ticket: &NativeLifecycleStageTicket,
        auxiliary_tickets: &NativeClosingAuxiliaryTickets,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> bool {
        let mut staged_current =
            self.native_lifecycle_ticket_is_current(primary_ticket, adapter_generation);
        for (index, ticket) in auxiliary_tickets {
            if !self.auxiliary_windows[*index]
                .native_lifecycle_ticket_is_current(ticket, adapter_generation)
            {
                staged_current = false;
            }
        }
        staged_current
    }

    fn converge_post_terminal_native_shutdown(&mut self, event_loop: &ActiveEventLoop) {
        // This path is intentionally ticket-free.  It is the one-way bounded
        // Closing fallback after primary terminal intent.  It never retries
        // or replays a lifecycle witness and cannot admit primary Closing.
        if !self.is_closing() {
            return;
        }
        self.invalidate_terminal_convergence_stage_owners();
        for window in &mut self.auxiliary_windows {
            if window.prepare_whole_run_closing() {
                window.begin_whole_run_retiring(event_loop);
            }
        }
        self.finish_native_shutdown(event_loop, Instant::now());
    }

    fn invalidate_terminal_convergence_stage_owners(&mut self) {
        self.frame_stage_owner.invalidate();
        for window in &mut self.auxiliary_windows {
            window.invalidate_terminal_convergence_stage_owner();
        }
    }

    fn finish_native_shutdown(&mut self, event_loop: &ActiveEventLoop, now: Instant) {
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        if self.native_resource_ownership_is_empty() {
            self.stop_native_event_loop(event_loop);
        } else {
            self.schedule_native_closing(event_loop, now);
        }
    }

    pub(super) fn prepare_native_shutdown(
        &mut self,
        cause: Option<NativeGenericRunError>,
    ) -> Option<Instant> {
        if self.is_closing() || self.native_lifecycle.is_stopped() {
            return None;
        }
        let was_recovering = self.is_recovering();
        let now = Instant::now();
        if !self.native_lifecycle.admit_closing(now) {
            return None;
        }
        let cause = if was_recovering {
            self.recovery_cause.take().or(cause)
        } else {
            self.recovery_cause.take();
            self.recovery_auxiliary_followup_pending = false;
            cause
        };
        self.recovery.cancel();
        self.recovery_auxiliary_followup_pending = false;
        if let Some(cause) = cause
            && self.record_terminal_cause(cause.clone())
        {
            error!(
                error = %cause,
                "radiant generic native vello: native shutdown admitted after terminal failure"
            );
        }
        #[cfg(target_os = "macos")]
        self.close_native_semantic_accessibility();
        let _ = self.core.runtime.begin_closing();
        self.fence_native_presentation();
        self.window.native_visual_requests.retire();
        self.clear_cpu_frame_fairness();
        self.application_reopen_events.take();
        self.application_reopen_proxy.take();
        self.runtime_wakeup.clear_pending();
        Some(now)
    }

    pub(super) fn advance_native_closing(&mut self, event_loop: &ActiveEventLoop, now: Instant) {
        if !self.is_closing() {
            return;
        }
        let mut turn = NativeResourceMaintenanceTurn::new();
        let native_ownership_empty = self.retire_all_native_resources_with_turn(&mut turn)
            && !self.recovery.has_in_flight_candidate();
        let Some(progress) = self
            .native_lifecycle
            .observe_closing_opportunity(now, native_ownership_empty)
        else {
            return;
        };
        match progress {
            NativeClosingProgress::Complete | NativeClosingProgress::Cutoff => {
                self.stop_native_event_loop(event_loop);
            }
            NativeClosingProgress::Continue => self.schedule_native_closing(event_loop, now),
        }
    }

    fn schedule_native_closing(&self, event_loop: &ActiveEventLoop, now: Instant) {
        if let Some(deadline) = self.native_lifecycle.closing_deadline(now) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }

    fn stop_native_event_loop(&mut self, event_loop: &ActiveEventLoop) {
        let _ = self.native_lifecycle.finish_closing();
        self.clear_retiring_auxiliary_maintenance();
        if !self.auxiliary_owner {
            event_loop.exit();
        }
    }

    pub(super) fn native_resource_ownership_is_empty(&self) -> bool {
        self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty()
            && self
                .auxiliary_windows
                .iter()
                .all(AuxiliaryNativeWindow::native_resource_ownership_is_empty)
            && !self.recovery.has_in_flight_candidate()
    }

    pub(super) fn has_retiring_auxiliary_windows(&self) -> bool {
        self.auxiliary_windows
            .iter()
            .any(AuxiliaryNativeWindow::is_retiring)
    }

    pub(super) fn retiring_auxiliary_maintenance_deadline(&self) -> Option<Instant> {
        self.timing.retiring_auxiliary_maintenance_deadline
    }

    pub(super) fn retiring_auxiliary_maintenance_is_due(&self, now: Instant) -> bool {
        self.timing
            .retiring_auxiliary_maintenance_deadline
            .is_some_and(|deadline| deadline <= now)
    }

    /// Arm the parent-owned retirement opportunity without doing any work.
    /// Completion callbacks and accepted close events use this wake-only hook;
    /// the actual bounded cleanup remains an `AboutToWait` authority.
    pub(super) fn arm_retiring_auxiliary_maintenance_due_now(&mut self) {
        if self.is_running() {
            self.timing.retiring_auxiliary_maintenance_deadline = self
                .has_retiring_auxiliary_windows()
                .then_some(Instant::now());
        }
    }

    pub(super) fn rearm_retiring_auxiliary_maintenance(&mut self, now: Instant) {
        self.timing.retiring_auxiliary_maintenance_deadline =
            self.has_retiring_auxiliary_windows().then_some(
                now + super::native_resource_maintenance::NATIVE_RESOURCE_MAINTENANCE_INTERVAL,
            );
    }

    fn clear_retiring_auxiliary_maintenance(&mut self) {
        self.timing.retiring_auxiliary_maintenance_deadline = None;
    }

    pub(super) fn begin_native_resource_maintenance(&mut self) -> NativeResourceMaintenanceTurn {
        let mut turn = NativeResourceMaintenanceTurn::new();
        let mut adapter = self.adapter.take();
        self.maintain_native_resources_with_turn_and_adapter(&mut turn, adapter.as_mut());
        self.adapter = adapter;
        turn
    }

    pub(super) fn native_surface_target_retirement_deadline(&self) -> Option<Instant> {
        self.timing.native_surface_target_retirement_deadline
    }

    pub(super) fn native_surface_target_retirement_is_due(&self, now: Instant) -> bool {
        self.timing
            .native_surface_target_retirement_deadline
            .is_some_and(|deadline| deadline <= now)
    }

    pub(super) fn arm_native_surface_target_retirement_maintenance(&mut self) {
        if self.is_running()
            && self.window.native_surface_target_fenced
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.render_target_retirement.is_some())
        {
            self.timing
                .native_surface_target_retirement_deadline
                .get_or_insert_with(Instant::now);
        }
    }

    pub(super) fn wake_native_surface_target_retirement_maintenance(&mut self) {
        if self.is_running()
            && self.window.native_surface_target_fenced
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.render_target_retirement.is_some())
        {
            self.timing.native_surface_target_retirement_deadline = Some(Instant::now());
        }
    }

    pub(super) fn maintain_native_surface_target_retirement_if_due_with_turn(
        &mut self,
        now: Instant,
        current_adapter_generation: NativeAdapterGeneration,
        turn: &mut NativeResourceMaintenanceTurn,
    ) {
        if !self.native_surface_target_retirement_is_due(now) {
            return;
        }
        if !self.is_running()
            || self.has_terminal_cause()
            || !self.window.native_surface_target_fenced
            || self.validated_pending_resize().is_none()
            || !self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| {
                    resources.generation == current_adapter_generation
                        && resources.render_target_retirement.is_some()
                })
        {
            self.timing.native_surface_target_retirement_deadline = None;
            return;
        }
        let target_retirement_removed = self.window.maintain_native_surface_target_retirement(turn);
        let target_retirement_pending = self
            .window
            .native_resources
            .as_ref()
            .is_some_and(|resources| resources.render_target_retirement.is_some());
        let pending_resize = self.validated_pending_resize().is_some();
        if target_retirement_removed && pending_resize {
            self.timing.native_surface_target_retirement_deadline = None;
            if self.window.native_surface_target_fenced {
                self.request_redraw_for_recovery();
            } else {
                self.request_pending_surface_resize_retry();
            }
        } else if self.window.native_surface_target_fenced
            && target_retirement_pending
            && pending_resize
        {
            self.timing.native_surface_target_retirement_deadline = Some(
                now + super::native_resource_maintenance::NATIVE_RESOURCE_MAINTENANCE_INTERVAL,
            );
        } else {
            self.timing.native_surface_target_retirement_deadline = None;
        }
    }

    fn normal_native_resource_maintenance_eligible(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && self.window.id.is_some()
            && self.window.window.is_some()
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.generation == adapter_generation)
    }

    pub(super) fn normal_native_resource_maintenance_deadline(
        &mut self,
        now: Instant,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<Instant> {
        let Some(adapter_generation) = adapter_generation else {
            self.timing.native_resource_maintenance_deadline = None;
            return None;
        };
        if !self.normal_native_resource_maintenance_eligible(adapter_generation)
            || self
                .window
                .native_resource_maintenance_candidate()
                .is_none()
        {
            self.timing.native_resource_maintenance_deadline = None;
            return None;
        }
        Some(
            *self
                .timing
                .native_resource_maintenance_deadline
                .get_or_insert(now),
        )
    }

    /// A completion callback only wakes the event loop.  The next scheduler
    /// observation may make one exact window due; no redraw or frame work is
    /// requested here.
    pub(super) fn wake_normal_native_resource_maintenance(&mut self) {
        let Some(adapter_generation) = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
        else {
            return;
        };
        self.wake_normal_native_resource_maintenance_with_generation(adapter_generation);
    }

    pub(super) fn wake_normal_native_resource_maintenance_with_generation(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
    ) {
        if !self.is_running() || self.has_terminal_cause() {
            return;
        }
        if self.normal_native_resource_maintenance_eligible(adapter_generation)
            && self
                .window
                .native_resource_maintenance_candidate()
                .is_some()
        {
            self.timing.native_resource_maintenance_deadline = Some(Instant::now());
        }
    }

    fn defer_normal_native_resource_maintenance(&mut self, now: Instant) {
        self.timing.native_resource_maintenance_deadline = self
            .window
            .native_resource_maintenance_candidate()
            .map(|_| {
                now + super::native_resource_maintenance::NATIVE_RESOURCE_MAINTENANCE_INTERVAL
            });
    }

    /// Admit and execute one exact Maintenance-stage unit for this window.
    /// Every currentness failure is inert and leaves the real owner intact.
    pub(super) fn admit_native_resource_maintenance(
        &mut self,
        now: Instant,
        key: &FrameScheduleKey,
        adapter_generation: NativeAdapterGeneration,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        if !self.frame_stage_owner.owns_key(key)
            || !self.normal_native_resource_maintenance_eligible(adapter_generation)
        {
            return false;
        }
        let Some(binding) = self.window.native_resource_maintenance_candidate() else {
            self.timing.native_resource_maintenance_deadline = None;
            return false;
        };
        let Some(ticket) = self.frame_stage_owner.admit_maintenance(
            adapter_generation,
            self.window.target_generation,
            binding,
        ) else {
            self.defer_normal_native_resource_maintenance(now);
            return false;
        };
        let current = self
            .window
            .native_resource_maintenance_binding(binding.slot());
        if current != Some(binding)
            || !self
                .frame_stage_owner
                .maintenance_ticket_is_current(&ticket)
        {
            let _ = self.frame_stage_owner.veto_maintenance(ticket);
            self.defer_normal_native_resource_maintenance(now);
            return false;
        }
        let Some(quarantine_removed) = self.window.maintain_native_resource_slot(binding, turn)
        else {
            let _ = self.frame_stage_owner.veto_maintenance(ticket);
            self.defer_normal_native_resource_maintenance(now);
            return false;
        };
        let target_retirement_removed = binding.render_target_retirement().is_some()
            && self
                .window
                .native_resource_maintenance_binding(binding.slot())
                .is_none_or(|current| current.render_target_retirement().is_none());
        let mut adapter = self.adapter.take();
        if let Some(adapter) = adapter.as_mut() {
            self.refresh_atlas_residency_account(adapter);
            self.refresh_signal_residency_account(adapter);
            self.refresh_custom_shader_residency_account(adapter);
            self.refresh_render_canvas_upload_account(adapter);
        }
        self.adapter = adapter;
        if !self.frame_stage_owner.complete_maintenance(ticket) {
            // The bounded unit already ran.  Never retry it through a broad
            // maintenance fallback after a completion mismatch.
            self.defer_normal_native_resource_maintenance(now);
            return false;
        }
        self.window
            .advance_native_resource_maintenance_cursor(binding.slot(), quarantine_removed);
        self.defer_normal_native_resource_maintenance(now);
        if target_retirement_removed {
            self.request_pending_surface_resize_retry();
        }
        true
    }

    pub(super) fn maintain_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        let mut adapter = self.adapter.take();
        let removed = self.maintain_native_resources_with_turn_and_adapter(turn, adapter.as_mut());
        self.adapter = adapter;
        removed
    }

    fn maintain_native_resources_with_turn_and_adapter(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
        mut adapter: Option<&mut GenericNativeAdapterOwner>,
    ) -> bool {
        self.window.maintain_native_resources(turn);
        if let Some(adapter) = adapter.as_mut() {
            self.refresh_atlas_residency_account(adapter);
            self.refresh_signal_residency_account(adapter);
            self.refresh_custom_shader_residency_account(adapter);
            self.refresh_render_canvas_upload_account(adapter);
        }
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        let auxiliary_count = self.auxiliary_windows.len();
        self.auxiliary_windows.retain_mut(|window| {
            !window.maintain_native_resources_with_turn(turn, adapter.as_deref_mut())
        });
        let removed_auxiliary = self.auxiliary_windows.len() != auxiliary_count;
        if removed_auxiliary {
            self.timing.deferred_auxiliary_window_sync = true;
        }
        removed_auxiliary
    }

    /// Normal Running auxiliary projection sync may only advance cleanup for
    /// children that are already retiring.  Active resources are consumed by
    /// the exact per-window Maintenance ticket path in the parent scheduler.
    pub(super) fn maintain_retiring_auxiliary_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        let mut adapter = self.adapter.take();
        let removed =
            self.maintain_retiring_auxiliary_resources_with_adapter(turn, adapter.as_mut());
        self.adapter = adapter;
        removed
    }

    pub(super) fn maintain_retiring_auxiliary_resources_with_adapter(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
        mut adapter: Option<&mut GenericNativeAdapterOwner>,
    ) -> bool {
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        let auxiliary_count = self.auxiliary_windows.len();
        self.auxiliary_windows.retain_mut(|window| {
            !window.is_retiring()
                || !window.maintain_native_resources_with_turn(turn, adapter.as_deref_mut())
        });
        let removed_auxiliary = self.auxiliary_windows.len() != auxiliary_count;
        if removed_auxiliary
            || self
                .auxiliary_windows
                .iter()
                .any(AuxiliaryNativeWindow::is_retiring)
        {
            // Keep the deferred sync boundary armed while a retiring child
            // still owns a completion witness. Completion callbacks wake the
            // parent, which then advances this same bounded cleanup path.
            self.timing.deferred_auxiliary_window_sync = true;
        }
        removed_auxiliary
    }

    pub(super) fn retire_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        self.window.retire_native_resources(turn)
    }

    pub(super) fn retire_all_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        let mut adapter = self.adapter.take();
        let primary_empty = self.retire_native_resources_with_turn(turn);
        if let Some(adapter) = adapter.as_mut() {
            self.refresh_atlas_residency_account(adapter);
            self.refresh_signal_residency_account(adapter);
            self.refresh_custom_shader_residency_account(adapter);
            self.refresh_render_canvas_upload_account(adapter);
        }
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        let auxiliary_count = self.auxiliary_windows.len();
        self.auxiliary_windows.retain_mut(|window| {
            !window.maintain_native_resources_with_turn(turn, adapter.as_mut())
        });
        if self.auxiliary_windows.len() != auxiliary_count {
            self.timing.deferred_auxiliary_window_sync = true;
        }
        self.adapter = adapter;
        primary_empty && self.auxiliary_windows.is_empty()
    }

    pub(super) fn record_successful_native_submission(&mut self) {
        if let Some(resources) = self.window.native_resources.as_mut() {
            resources.record_successful_native_submission();
        }
    }

    pub(super) fn process_native_gpu_timing_ready(
        &mut self,
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    ) {
        let Some(delivery) = self.window.prepare_native_gpu_timing_delivery(
            generation,
            resource_identity,
            slot,
            token,
        ) else {
            let _ = self.window.discard_native_gpu_timing_delivery(
                generation,
                resource_identity,
                slot,
                token,
            );
            return;
        };
        self.core
            .runtime
            .host_observe_frame_gpu_timing(delivery.sample);
        let _ = self.window.finish_native_gpu_timing_delivery(delivery);
    }

    pub(super) fn discard_native_gpu_timing_ready(
        &mut self,
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    ) {
        let _ = self.window.discard_native_gpu_timing_delivery(
            generation,
            resource_identity,
            slot,
            token,
        );
    }

    pub(super) fn start_native_gpu_timing(&mut self, admission: &mut Option<GpuTimingAdmission>) {
        let Some(GpuTimingAdmission::Reserved(reservation)) = *admission else {
            return;
        };
        let started = self
            .window
            .native_resources
            .as_mut()
            .is_some_and(|resources| resources.submit_gpu_timing_start(reservation));
        if !started {
            self.cancel_native_gpu_timing(admission);
        }
    }

    pub(super) fn cancel_native_gpu_timing(&mut self, admission: &mut Option<GpuTimingAdmission>) {
        let Some(GpuTimingAdmission::Reserved(reservation)) = admission.take() else {
            return;
        };
        if let Some(resources) = self.window.native_resources.as_mut() {
            let _ = resources.cancel_gpu_timing(reservation);
        }
    }

    pub(super) fn finalize_native_gpu_timing(
        &mut self,
        admission: Option<GpuTimingAdmission>,
        frame_sequence: Option<u64>,
    ) {
        let Some(admission) = admission else {
            return;
        };
        let Some(frame_sequence) = frame_sequence else {
            let mut admission = Some(admission);
            self.cancel_native_gpu_timing(&mut admission);
            return;
        };
        let Some(window_identity) = self.timing.native_window_diagnostic_identity else {
            let mut admission = Some(admission);
            self.cancel_native_gpu_timing(&mut admission);
            return;
        };
        match admission {
            GpuTimingAdmission::Disabled => {}
            GpuTimingAdmission::Unsupported
            | GpuTimingAdmission::InvalidClock
            | GpuTimingAdmission::CapacityRefused => {
                let Some(reason) = admission.unavailable_reason() else {
                    return;
                };
                self.core.runtime.host_observe_frame_gpu_timing(
                    crate::runtime::FrameGpuTimingSample::new(
                        window_identity.get(),
                        frame_sequence,
                        crate::runtime::FrameGpuTimingOutcome::unavailable(reason),
                    ),
                );
            }
            GpuTimingAdmission::Reserved(reservation) => {
                let bound = self
                    .window
                    .native_resources
                    .as_mut()
                    .is_some_and(|resources| {
                        resources.bind_gpu_timing(
                            reservation,
                            window_identity.get(),
                            frame_sequence,
                        )
                    });
                if !bound {
                    let mut admission = Some(GpuTimingAdmission::Reserved(reservation));
                    self.cancel_native_gpu_timing(&mut admission);
                }
            }
        }
    }

    /// Recover one eligible FrameRender failure after the failed redraw has
    /// returned and dropped its acquired SurfaceTexture. A veto or candidate
    /// failure converges on the existing bounded whole-run Closing policy with
    /// the original FrameRender as the first cause.
    pub(super) fn recover_frame_render_failure(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
        failure: NativeFrameRenderFailure,
        kind: NativeRendererRecoveryWindowKind,
    ) -> Result<(), NativeGenericRunError> {
        let cause = failure.into_error();
        match self.try_recover_frame_render(adapter, kind) {
            Ok(()) => Ok(()),
            Err(reason) => {
                warn!(
                    reason,
                    "radiant generic native vello: renderer recovery was vetoed"
                );
                self.admit_native_shutdown(event_loop, Some(cause.clone()));
                Err(cause)
            }
        }
    }

    fn try_recover_frame_render(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        kind: NativeRendererRecoveryWindowKind,
    ) -> Result<(), String> {
        let active_generation = self
            .window
            .native_resources
            .as_ref()
            .map(|resources| resources.generation);
        let current_generation = adapter.capture_generation();
        let window_identity = self.window.window.as_ref().zip(self.window.id);
        let admission = preflight_renderer_recovery(
            &self.renderer_recovery,
            active_generation,
            current_generation,
            window_identity,
            self.window.can_publish_native_resources(),
            self.window.target_generation,
            self.is_running() && !self.has_terminal_cause(),
        )
        .map_err(|veto| format!("renderer recovery preflight vetoed: {veto:?}"))?;

        // Renderer recovery is a physical presentation concealment boundary,
        // but it must not overwrite Radiant's latest visibility intent.
        self.apply_native_window_visibility(false);

        // This is deliberately before event-proxy lookup and all candidate GPU
        // construction. Candidate failure therefore consumes the generation's
        // one bounded allowance just like a successful candidate.
        self.renderer_recovery.record_attempt(admission.generation);
        let event_proxy = self
            .runtime_wakeup
            .event_loop_proxy()
            .ok_or_else(|| String::from("native event-loop proxy was not installed"))?;
        let candidate = construct_renderer_recovery_candidate(
            &self.options,
            adapter,
            &admission,
            event_proxy,
            kind,
            self.gpu_timing_route.clone(),
            self.frame_gpu_timing_enabled,
        )
        .map_err(|error| error.to_string())?;

        if !renderer_recovery_commit_is_valid(
            &self.renderer_recovery,
            &admission,
            &candidate,
            NativeRendererRecoveryCommitFacts {
                active_generation: self
                    .window
                    .native_resources
                    .as_ref()
                    .map(|resources| resources.generation),
                current_generation: adapter.capture_generation(),
                current_window: self.window.window.as_ref().zip(self.window.id),
                publication_available: self.window.can_publish_native_resources(),
                target_generation: self.window.target_generation,
                run_admissible: self.is_running() && !self.has_terminal_cause(),
            },
        ) {
            return Err(String::from(
                "renderer recovery candidate failed final identity, generation, lifecycle, or publication validation",
            ));
        }
        if !self.invalidate_native_visual_requests() {
            return Err(String::from(
                "native renderer recovery could not advance the visual request owner",
            ));
        }
        let Some(publication) = self.window.reserve_native_resource_publication() else {
            return Err(String::from(
                "renderer recovery publication capacity changed before commit",
            ));
        };
        publication.publish(candidate.bundle);
        self.refresh_atlas_residency_account(adapter);
        self.refresh_signal_residency_account(adapter);
        self.refresh_custom_shader_residency_account(adapter);
        self.refresh_render_canvas_upload_account(adapter);
        self.window.target_generation = admission.next_target_generation;
        self.window.native_surface_target_fenced = false;
        self.frame.invalidate_native_resources_for_recovery();
        self.rebuild_scene();
        self.timing.last_redraw = Instant::now();
        self.apply_native_window_visibility(self.window.logical_window_visible);
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
        Ok(())
    }

    pub(super) fn should_admit_auxiliary_sync(&self) -> bool {
        self.is_running() && !self.has_terminal_cause()
    }

    pub(super) fn handle_device_lost_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
        message: String,
    ) {
        if !self.is_running() {
            return;
        }
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }
        let cause = NativeGenericRunError::RenderDeviceLost(message);
        self.begin_device_recovery(event_loop, generation, registration, cause);
    }

    fn can_prepare_device_recovery(&self, generation: NativeAdapterGeneration) -> bool {
        self.window
            .native_resources
            .as_ref()
            .is_none_or(|resources| resources.generation == generation)
            && self.window.can_publish_native_resources()
            && self
                .auxiliary_windows
                .iter()
                .all(|window| window.can_prepare_device_recovery(generation))
    }

    fn veto_staged_native_lifecycle(
        &mut self,
        primary: Option<NativeLifecycleStageTicket>,
        auxiliaries: Vec<(usize, NativeLifecycleStageTicket)>,
    ) {
        if let Some(ticket) = primary {
            let _ = self.veto_native_lifecycle(ticket);
        }
        for (index, ticket) in auxiliaries {
            if let Some(window) = self.auxiliary_windows.get_mut(index) {
                let _ = window.veto_native_lifecycle(ticket);
            }
        }
    }

    fn fail_staged_native_lifecycle(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
        primary: Option<NativeLifecycleStageTicket>,
        auxiliaries: Vec<(usize, NativeLifecycleStageTicket)>,
    ) {
        self.veto_staged_native_lifecycle(primary, auxiliaries);
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    fn finish_staged_native_lifecycle(
        &mut self,
        generation: NativeAdapterGeneration,
        primary: NativeLifecycleStageTicket,
        auxiliaries: Vec<(usize, NativeLifecycleStageTicket)>,
    ) -> Result<(), String> {
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let staged_current = current_generation == Some(generation)
            && self.native_lifecycle_ticket_is_current(&primary, current_generation)
            && auxiliaries.iter().all(|(index, ticket)| {
                self.auxiliary_windows[*index]
                    .native_lifecycle_ticket_is_current(ticket, current_generation)
            });
        self.finish_staged_native_lifecycle_after_revalidation(primary, auxiliaries, staged_current)
    }

    #[cfg(test)]
    fn finish_staged_native_lifecycle_with_evidence(
        &mut self,
        generation: NativeAdapterGeneration,
        primary: NativeLifecycleStageTicket,
        primary_evidence: NativeLifecycleStageEvidence,
        auxiliaries: Vec<(
            usize,
            NativeLifecycleStageTicket,
            NativeLifecycleStageEvidence,
        )>,
    ) -> Result<(), String> {
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let staged_current = current_generation == Some(generation)
            && primary.is_current(&self.frame_stage_owner, &primary_evidence)
            && auxiliaries.iter().all(|(index, ticket, evidence)| {
                self.auxiliary_windows[*index]
                    .native_lifecycle_ticket_is_current_with_evidence(ticket, evidence)
            });
        let auxiliaries = auxiliaries
            .into_iter()
            .map(|(index, ticket, _)| (index, ticket))
            .collect();
        self.finish_staged_native_lifecycle_after_revalidation(primary, auxiliaries, staged_current)
    }

    fn finish_staged_native_lifecycle_after_revalidation(
        &mut self,
        primary: NativeLifecycleStageTicket,
        auxiliaries: Vec<(usize, NativeLifecycleStageTicket)>,
        staged_current: bool,
    ) -> Result<(), String> {
        let mut primary = Some(primary);
        if !staged_current {
            self.veto_staged_native_lifecycle(primary, auxiliaries);
            return Err(String::from(
                "native recovery finish lifecycle ticket currentness was lost",
            ));
        }

        // Every finishing window is staged and revalidated before the first
        // Recovering-to-Running transition mutates a native or controller
        // phase. The set remains a single event-loop turn with no yield.
        if !self.finish_device_recovery() {
            self.veto_staged_native_lifecycle(primary, auxiliaries);
            return Err(String::from(
                "native recovery primary lifecycle finish transition was vetoed",
            ));
        }

        let Some(primary_ticket) = primary.take() else {
            self.veto_staged_native_lifecycle(None, auxiliaries);
            return Err(String::from(
                "native recovery primary lifecycle finish ticket disappeared",
            ));
        };
        if !self.native_lifecycle_stage_ticket_is_current(&primary_ticket) {
            self.veto_staged_native_lifecycle(None, auxiliaries);
            return Err(String::from(
                "native recovery primary lifecycle finish owner changed",
            ));
        }
        if !self.complete_native_lifecycle(primary_ticket) {
            self.veto_staged_native_lifecycle(None, auxiliaries);
            return Err(String::from(
                "native recovery primary lifecycle finish completion was vetoed",
            ));
        }

        let mut remaining_auxiliaries = auxiliaries.into_iter();
        while let Some((index, ticket)) = remaining_auxiliaries.next() {
            if !self.auxiliary_windows[index].finish_device_recovery_if_no_rebuild() {
                let _ = self.auxiliary_windows[index].veto_native_lifecycle(ticket);
                self.veto_staged_native_lifecycle(None, remaining_auxiliaries.collect::<Vec<_>>());
                return Err(String::from(
                    "native recovery auxiliary lifecycle finish transition was vetoed",
                ));
            }
            if !self.auxiliary_windows[index].native_lifecycle_stage_ticket_is_current(&ticket) {
                let _ = self.auxiliary_windows[index].veto_native_lifecycle(ticket);
                self.veto_staged_native_lifecycle(None, remaining_auxiliaries.collect::<Vec<_>>());
                return Err(String::from(
                    "native recovery auxiliary lifecycle finish owner changed",
                ));
            }
            if !self.auxiliary_windows[index].complete_native_lifecycle(ticket) {
                self.veto_staged_native_lifecycle(None, remaining_auxiliaries.collect::<Vec<_>>());
                return Err(String::from(
                    "native recovery auxiliary lifecycle finish completion was vetoed",
                ));
            }
        }
        self.arm_retiring_auxiliary_maintenance_due_now();
        Ok(())
    }

    fn begin_device_recovery(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
        cause: NativeGenericRunError,
    ) {
        // Recheck the callback witness before any fallback or staging.  A
        // late registration/generation remains inert rather than turning an
        // already superseded device-loss notification into Closing.
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }
        let Some(adapter) = self.adapter.as_ref() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        if !self.can_prepare_device_recovery(generation) {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        }
        let Some(previous_device_identity) = adapter.selected_device_identity() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let Some(next_generation) = adapter.next_recovery_generation() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        if !next_generation.is_strictly_newer_than(generation) {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        }
        let Some(window) = self.window.window.clone() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let Some(instance) = adapter.instance().cloned() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let size = window.inner_size();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                warn!(error = %error, "radiant generic native vello: recovery surface creation failed");
                self.admit_native_shutdown(event_loop, Some(cause));
                return;
            }
        };
        let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };

        // A callback that was current during the initial preflight must still
        // name the shared adapter registration immediately before staging.
        // Earlier stale callbacks remain inert and do not enter Closing.
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }

        let Some(primary_ticket) = self.admit_native_lifecycle(Some(generation)) else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let mut auxiliary_tickets = Vec::with_capacity(self.auxiliary_windows.len());
        for index in 0..self.auxiliary_windows.len() {
            if !self.auxiliary_windows[index].is_admitted() {
                continue;
            }
            let Some(ticket) =
                self.auxiliary_windows[index].admit_native_lifecycle(Some(generation))
            else {
                self.fail_staged_native_lifecycle(
                    event_loop,
                    cause,
                    Some(primary_ticket),
                    auxiliary_tickets,
                );
                return;
            };
            auxiliary_tickets.push((index, ticket));
        }

        // Revalidate the complete staged set synchronously.  No lifecycle or
        // controller phase has changed yet, and no scheduler yield is allowed
        // between this check and the transition hooks below.
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let staged_current = self.device_loss_event_is_current(generation, &registration)
            && self.can_prepare_device_recovery(generation)
            && self.native_lifecycle_ticket_is_current(&primary_ticket, current_generation)
            && auxiliary_tickets.iter().all(|(index, ticket)| {
                self.auxiliary_windows[*index]
                    .native_lifecycle_ticket_is_current(ticket, current_generation)
            });
        if !staged_current {
            self.fail_staged_native_lifecycle(
                event_loop,
                cause,
                Some(primary_ticket),
                auxiliary_tickets,
            );
            return;
        }

        // Only after every window's exact Lifecycle ticket is staged and
        // current may the existing native/controller recovery hooks mutate
        // phases and install their presentation/resource fences.
        if !self.admit_device_recovery()
            || auxiliary_tickets
                .iter()
                .any(|(index, _)| !self.auxiliary_windows[*index].admit_device_recovery())
        {
            self.fail_staged_native_lifecycle(
                event_loop,
                cause,
                Some(primary_ticket),
                auxiliary_tickets,
            );
            return;
        }

        #[cfg(target_os = "macos")]
        self.close_native_semantic_accessibility();

        let mut primary_ticket = Some(primary_ticket);
        let mut completion_failed = false;
        if let Some(ticket) = primary_ticket.take()
            && (!self.native_lifecycle_stage_ticket_is_current(&ticket)
                || !self.complete_native_lifecycle(ticket))
        {
            completion_failed = true;
        }
        let mut remaining_auxiliary_tickets = Vec::new();
        for (index, ticket) in auxiliary_tickets {
            if completion_failed {
                remaining_auxiliary_tickets.push((index, ticket));
            } else if !self.auxiliary_windows[index]
                .native_lifecycle_stage_ticket_is_current(&ticket)
            {
                completion_failed = true;
                remaining_auxiliary_tickets.push((index, ticket));
            } else if !self.auxiliary_windows[index].complete_native_lifecycle(ticket) {
                // The exact ticket has been consumed by the failed
                // completion attempt.  Keep the owner fenced and converge to
                // Closing below; there is deliberately no replay ticket.
                completion_failed = true;
            }
        }
        if completion_failed {
            self.fail_staged_native_lifecycle(
                event_loop,
                cause,
                primary_ticket,
                remaining_auxiliary_tickets,
            );
            return;
        }

        self.recovery_cause = Some(cause);
        let request = NativeRecoveryRequest {
            instance,
            surface,
            width: size.width.max(1),
            height: size.height.max(1),
            target_fps: self.options.normalized_target_fps(),
            generation: next_generation,
            previous_device_identity,
            event_proxy,
            gpu_timing_route: self.gpu_timing_route.clone(),
            gpu_timing_enabled: self.frame_gpu_timing_enabled,
        };
        if let Err(error) = self.recovery.start(request) {
            warn!(error = %error, "radiant generic native vello: recovery candidate could not start");
            self.admit_native_shutdown(event_loop, None);
        }
    }

    pub(super) fn handle_device_recovery_ready(
        &mut self,
        event_loop: &ActiveEventLoop,
        episode: NativeRecoveryEpisodeToken,
    ) {
        if !self.is_recovering() {
            let _ = self.recovery.acknowledge(episode);
            return;
        }
        if !recovery_completion_is_admissible(self.recovery_expired(Instant::now())) {
            if self.recovery.acknowledge(episode) {
                self.admit_native_shutdown(event_loop, None);
            }
            return;
        }
        let Some(result) = self.recovery.take_ready(episode) else {
            return;
        };
        match result {
            Ok(candidate) => {
                if let Err(error) = self.commit_device_recovery_candidate(candidate) {
                    warn!(error = %error, "radiant generic native vello: recovery candidate publication failed");
                    let cause = self.recovery_cause.take();
                    self.admit_native_shutdown(event_loop, cause);
                }
            }
            Err(error) => {
                warn!(error = %error, "radiant generic native vello: recovery candidate preparation failed");
                self.admit_native_shutdown(event_loop, None);
            }
        }
    }

    fn commit_device_recovery_candidate(
        &mut self,
        candidate: NativeRecoveryCandidate,
    ) -> Result<(), String> {
        if !self.is_recovering() {
            return Err(String::from(
                "native recovery lifecycle is no longer recovering",
            ));
        }
        let Some(previous_generation) = self
            .adapter
            .as_ref()
            .and_then(|adapter| adapter.capture_generation())
        else {
            return Err(String::from(
                "native recovery lost its previous adapter generation",
            ));
        };
        let NativeRecoveryCandidate {
            mut adapter,
            mut primary,
        } = candidate;
        if !primary
            .generation
            .is_strictly_newer_than(previous_generation)
            || adapter.capture_generation() != Some(primary.generation)
            || !self.can_prepare_device_recovery(previous_generation)
        {
            return Err(String::from(
                "native recovery candidate did not retain exact newer-generation evidence",
            ));
        }
        if let Some(window) = self.window.window.as_ref() {
            let size = window.inner_size();
            if !adapter.resize_unpublished_recovery_candidate_surface(
                &mut primary.render_surface,
                size.width.max(1),
                size.height.max(1),
            ) {
                return Err(String::from(
                    "native recovery candidate could not match the current primary geometry",
                ));
            }
        } else {
            return Err(String::from("native recovery primary window disappeared"));
        }
        let Some(mut previous_adapter) = self.adapter.take() else {
            return Err(String::from(
                "native recovery previous adapter owner disappeared during commit",
            ));
        };
        for window in &mut self.auxiliary_windows {
            if !window.quarantine_device_recovery_resources(&mut previous_adapter) {
                self.adapter = Some(previous_adapter);
                return Err(String::from(
                    "native recovery auxiliary quarantine capacity changed during commit",
                ));
            }
        }
        let Some(publication) = self.window.reserve_native_resource_publication() else {
            self.adapter = Some(previous_adapter);
            return Err(String::from(
                "native recovery primary quarantine capacity changed during commit",
            ));
        };
        publication.publish(primary);
        self.refresh_atlas_residency_account(&mut previous_adapter);
        self.refresh_signal_residency_account(&mut previous_adapter);
        self.refresh_custom_shader_residency_account(&mut previous_adapter);
        self.refresh_render_canvas_upload_account(&mut previous_adapter);
        adapter.adopt_atlas_residency_ledger(&mut previous_adapter);
        adapter.adopt_signal_residency_ledger(&mut previous_adapter);
        adapter.adopt_custom_shader_residency_ledger(&mut previous_adapter);
        adapter.adopt_render_canvas_upload_ledger(&mut previous_adapter);
        self.adapter = Some(adapter);
        let Some(mut adapter) = self.adapter.take() else {
            return Err(String::from(
                "native recovery adapter owner disappeared after publication",
            ));
        };
        self.refresh_atlas_residency_account(&mut adapter);
        self.refresh_signal_residency_account(&mut adapter);
        self.refresh_custom_shader_residency_account(&mut adapter);
        self.refresh_render_canvas_upload_account(&mut adapter);
        self.adapter = Some(adapter);
        self.complete_native_recovery_target_transition();
        self.frame.invalidate_native_resources_for_recovery();

        let generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
            .ok_or_else(|| String::from("native recovery finish adapter generation disappeared"))?;
        let Some(primary_ticket) = self.admit_native_lifecycle_finish(Some(generation)) else {
            return Err(String::from(
                "native recovery primary finish lifecycle ticket was not admitted",
            ));
        };
        let mut auxiliary_tickets = Vec::new();
        for index in 0..self.auxiliary_windows.len() {
            let window = &mut self.auxiliary_windows[index];
            if !window.is_admitted() || window.recovery_rebuild_pending() {
                continue;
            }
            let Some(ticket) = window.admit_native_lifecycle_finish(Some(generation)) else {
                self.veto_staged_native_lifecycle(Some(primary_ticket), auxiliary_tickets);
                return Err(String::from(
                    "native recovery auxiliary finish lifecycle ticket was not admitted",
                ));
            };
            auxiliary_tickets.push((index, ticket));
        }
        self.finish_staged_native_lifecycle(generation, primary_ticket, auxiliary_tickets)?;

        #[cfg(target_os = "macos")]
        if let Some(proxy) = self.runtime_wakeup.event_loop_proxy() {
            self.attach_native_semantic_accessibility(proxy);
        }
        self.rebuild_scene();
        self.clear_native_visual_request_wake();
        self.recovery_auxiliary_followup_pending = true;
        self.timing.deferred_auxiliary_window_sync = true;
        self.timing.last_redraw = Instant::now();
        self.apply_native_window_visibility(self.window.logical_window_visible);
        self.apply_pending_normal_window_activation("recovery-complete");
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
        Ok(())
    }

    pub(super) fn handle_render_device_error_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
        kind: NativeRenderDeviceErrorKind,
        message: String,
    ) {
        if !self.is_running() {
            return;
        }
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }
        let cause = NativeGenericRunError::RenderDeviceError { kind, message };
        self.record_render_device_error_and_exit(event_loop, cause);
    }

    pub(super) fn device_loss_event_is_current(
        &self,
        generation: NativeAdapterGeneration,
        registration: &Arc<DeviceLossRegistration>,
    ) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.accepts_device_loss(generation, registration))
    }

    pub(super) fn record_initialization_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn record_render_device_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn record_auxiliary_terminal_cause_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn take_terminal_cause(&mut self) -> Option<NativeGenericRunError> {
        self.terminal_cause.take()
    }

    pub(super) fn resolve_run_result(
        &mut self,
        run_result: Result<(), NativeGenericRunError>,
    ) -> Result<(), NativeGenericRunError> {
        let Some(terminal_cause) = self.take_terminal_cause() else {
            return run_result;
        };
        if let Err(event_loop_error) = &run_result {
            warn!(
                terminal_cause = %terminal_cause,
                event_loop_error = %event_loop_error,
                "native terminal cause superseded the event-loop error"
            );
        }
        Err(terminal_cause)
    }

    /// Apply an explicit Radiant visibility decision and retain it for
    /// lifecycle recovery.  Eligibility never reads host visibility back.
    pub(super) fn set_native_window_visibility(&mut self, visible: bool) {
        self.window.logical_window_visible = visible;
        if self.is_running() {
            self.apply_native_window_visibility(visible);
        }
    }

    /// Apply the current physical Winit visibility without changing Radiant's
    /// desired visibility. Recovery and closing use this concealment-only
    /// operation so a later success can restore the latest explicit intent.
    pub(super) fn apply_native_window_visibility(&self, visible: bool) {
        if let Some(window) = self.window.window.as_ref() {
            window.set_visible(visible);
        }
    }

    pub(super) fn clear_native_visual_request_wake(&mut self) {
        self.timing.redraw_requested = false;
        self.timing.redraw_requested_at = None;
        self.window.requested_recovery_redraw = false;
    }

    pub(super) fn invalidate_native_visual_requests(&mut self) -> bool {
        self.clear_native_visual_request_wake();
        self.window.native_visual_requests.invalidate()
    }

    pub(super) fn suspend_native_visual_requests(&mut self) -> bool {
        self.clear_native_visual_request_wake();
        self.window.native_visual_requests.suspend()
    }

    pub(super) fn resume_native_visual_requests(&mut self) -> bool {
        self.window.native_visual_requests.resume()
    }

    pub(super) fn handle_surface_occlusion(&mut self, occluded: bool) {
        if occluded {
            self.window.surface_occluded = true;
            self.window.surface_occluded_by_acquire = false;
        } else {
            self.window.surface_occluded = false;
            self.window.surface_occluded_by_acquire = false;
            self.request_redraw_after_surface_unoccluded();
        }
    }

    /// Recover only a stale occlusion latch produced by surface acquisition.
    /// The caller must already hold the normal native visual fences; the
    /// existing mailbox reissue path then supplies at most one bounded Winit
    /// wake for retained work.
    pub(super) fn clear_stale_acquisition_occlusion_for_activation(&mut self) -> bool {
        if !self.window.surface_occluded
            || !self.window.surface_occluded_by_acquire
            || !self.native_visual_request_offer_is_eligible()
        {
            return false;
        }
        self.window.surface_occluded = false;
        self.window.surface_occluded_by_acquire = false;
        self.request_redraw_after_surface_unoccluded();
        true
    }

    fn request_redraw_after_surface_unoccluded(&mut self) {
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        let Some(window_id) = self.window.id else {
            return;
        };
        if self.window.native_visual_requests.has_requested()
            && NativeVisualRequestAdapter::reissue(
                &mut self.window.native_visual_requests,
                &window,
                window_id,
            )
        {
            let now = Instant::now();
            self.timing.redraw_requested = true;
            self.timing.redraw_requested_at = Some(now);
            return;
        }
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    /// Veto the callback boundary when the primary adapter owner is absent or
    /// cannot provide a current generation. This is a packet-ownership
    /// transition, not a terminal lifecycle failure: any requested/pending
    /// work is discarded, and stale wake/recovery state is cleared without
    /// entering redraw, finish, fallback, or diagnostics.
    pub(super) fn veto_native_visual_request_at_callback_boundary(
        &mut self,
    ) -> NativeVisualRequestBegin {
        let begin = self.window.native_visual_requests.veto_requested();
        self.clear_native_visual_request_wake();
        begin
    }

    pub(super) fn arm_requested_recovery_redraw(&mut self) {
        self.window.requested_recovery_redraw = true;
    }

    pub(super) fn request_redraw_for_recovery(&mut self) {
        self.arm_requested_recovery_redraw();
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    fn request_pending_surface_resize_retry(&mut self) {
        if self.validated_pending_resize().is_some() {
            let reason = self
                .timing
                .pending_surface_resize_reason
                .unwrap_or(FrameWorkReason::NativeResize);
            self.request_redraw_for_frame_work(FrameWork::ResizeSurface { reason });
        }
    }

    fn redraw_marker_is_available(&self) -> bool {
        !self.timing.redraw_requested && !self.window.native_visual_requests.has_work()
    }

    fn pending_coalesced_input_needs_redraw_marker(&self) -> bool {
        (self.input.pending_gpu_surface_wheel.is_some()
            || self.input.pending_scroll_container_wheel.is_some()
            || self.input.pending_scrollbar_drag.is_some())
            && self.redraw_marker_is_available()
    }

    pub(super) fn request_redraw_for_pending_coalesced_input(&mut self) {
        if self.pending_coalesced_input_needs_redraw_marker() {
            self.request_redraw_for_frame_work(FrameWork::None);
        }
    }

    fn deferred_frame_work_needs_redraw_marker(&self, frame_work: FrameWork) -> bool {
        matches!(frame_work, FrameWork::None | FrameWork::PaintOnly { .. })
            && self.redraw_marker_is_available()
    }

    pub(super) fn request_redraw_for_deferred_frame_work(&mut self, frame_work: FrameWork) {
        if self.deferred_frame_work_needs_redraw_marker(frame_work) {
            self.request_redraw_for_frame_work(FrameWork::None);
        }
    }

    pub(super) fn request_redraw_for_frame_work(&mut self, frame_work: FrameWork) {
        if !self.is_running() {
            return;
        }
        self.record_frame_work(frame_work);
        let ordinary = self.native_visual_request_offer_is_eligible();
        let recovery = self.native_visual_request_recovery_offer_is_eligible();
        if !ordinary && !recovery {
            return;
        }
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        let Some(window_id) = self.window.id else {
            return;
        };
        let now = Instant::now();
        let stale_before_offer = !self.window.surface_occluded
            && self.timing.redraw_requested
            && self.pending_redraw_request_is_stale(now);
        // Allocate and enqueue the newest offer before reissuing a stale wake.
        // Otherwise a stale wake can win the race and the newer work is never
        // represented by a packet revision.
        let enqueue = if self.window.surface_occluded {
            NativeVisualRequestAdapter::enqueue_without_wakeup(
                &mut self.window.native_visual_requests,
                frame_work,
            )
        } else {
            NativeVisualRequestAdapter::enqueue(
                &mut self.window.native_visual_requests,
                &window,
                frame_work,
            )
        };
        match enqueue {
            NativeVisualRequestEnqueue::Issued => {
                if !self.window.surface_occluded {
                    self.timing.redraw_requested = true;
                    self.timing.redraw_requested_at = Some(now);
                }
            }
            NativeVisualRequestEnqueue::Replaced if stale_before_offer => {
                if NativeVisualRequestAdapter::reissue(
                    &mut self.window.native_visual_requests,
                    &window,
                    window_id,
                ) {
                    if let Some(requested_at) = self.timing.redraw_requested_at {
                        let pending = now.duration_since(requested_at);
                        if slow_render_profile_enabled()
                            && pending >= Self::REDRAW_REISSUE_LOG_AFTER
                        {
                            warn!(
                                target = "radiant::debug::frame_profile",
                                event = "radiant.redraw_request.reissued",
                                pending_us = pending.as_micros(),
                                stale_after_us = Self::REDRAW_REISSUE_AFTER.as_millis(),
                                "Reissued the newest native visual request packet"
                            );
                        }
                    }
                    self.timing.redraw_requested = true;
                    self.timing.redraw_requested_at = Some(now);
                }
            }
            NativeVisualRequestEnqueue::Replaced | NativeVisualRequestEnqueue::Queued => {
                // Replaced requests already own the outstanding Winit wakeup;
                // queued requests will publish one when consuming finishes.
            }
            NativeVisualRequestEnqueue::Rejected => {}
        }
    }

    pub(super) fn clear_native_visual_request_wake_timing(&mut self) {
        self.timing.redraw_requested = false;
        self.timing.redraw_requested_at = None;
    }

    fn native_visual_request_local_fences_hold(&self) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && !self.is_recovering()
            && !self.is_closing()
            && !self.is_stopped()
            && self
                .window
                .id
                .is_some_and(|window_id| self.window.native_visual_requests.is_bound_to(window_id))
            && self.window.window.is_some()
            && self.window.native_resources.is_some()
            && !self.window.native_visual_requests.is_suspended()
    }

    pub(super) fn native_visual_request_offer_is_eligible(&self) -> bool {
        self.native_visual_request_local_fences_hold()
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
    }

    fn native_visual_request_recovery_offer_is_eligible(&self) -> bool {
        self.native_visual_request_local_fences_hold()
            && self.window.requested_recovery_redraw
            && (self.validated_pending_resize().is_some()
                || self.window.native_surface_target_fenced
                || !self.window.target_generation.is_known())
    }

    pub(super) fn native_visual_request_schedule_is_eligible(&self) -> bool {
        !self.window.surface_occluded
            && self.native_visual_request_scheduler_adapter_is_current()
            && (self.native_visual_request_offer_is_eligible()
                || (self.window.native_visual_requests.has_requested()
                    && self.native_visual_request_recovery_offer_is_eligible()))
    }

    pub(super) fn native_visual_request_schedule_is_ordinary(&self) -> bool {
        !self.window.surface_occluded
            && self.native_visual_request_scheduler_adapter_is_current()
            && self.native_visual_request_offer_is_eligible()
    }

    pub(super) fn native_visual_request_recovery_schedule_is_eligible(&self) -> bool {
        !self.window.surface_occluded
            && self.native_visual_request_scheduler_adapter_is_current()
            && self.window.native_visual_requests.has_requested()
            && self.native_visual_request_recovery_offer_is_eligible()
    }

    fn native_visual_request_scheduler_adapter_is_current(&self) -> bool {
        // Auxiliary runners borrow the parent's adapter at each scheduler
        // boundary; their parent generation is validated by
        // `AuxiliaryScheduleEligibility` and must remain the authority.
        if self.auxiliary_owner {
            return true;
        }
        let Some(resource_generation) = self
            .window
            .native_resources
            .as_ref()
            .map(|resources| resources.generation)
        else {
            return false;
        };
        self.adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
            .is_some_and(|adapter_generation| adapter_generation == resource_generation)
    }

    pub(super) fn begin_native_visual_request(
        &mut self,
        adapter: &GenericNativeAdapterOwner,
    ) -> NativeVisualRequestBegin {
        let Some(window_id) = self.window.id else {
            return self.veto_native_visual_request_at_callback_boundary();
        };
        let Some(adapter_generation) = adapter.capture_generation() else {
            return self.veto_native_visual_request_at_callback_boundary();
        };
        let ordinary = self.native_visual_request_is_eligible(adapter_generation);
        let requested = ordinary
            || (self.window.requested_recovery_redraw
                && self.native_visual_request_recovery_fences_hold(adapter_generation));
        let begin = NativeVisualRequestAdapter::begin(
            &mut self.window.native_visual_requests,
            window_id,
            NativeVisualRequestEligibility {
                requested,
                fallback: ordinary,
            },
        );
        if matches!(begin, NativeVisualRequestBegin::Requested(_)) {
            self.window.requested_recovery_redraw = false;
        } else if matches!(begin, NativeVisualRequestBegin::RequestedVetoed) {
            self.clear_native_visual_request_wake();
        }
        begin
    }

    fn native_visual_request_is_eligible(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && !self.is_recovering()
            && !self.is_closing()
            && !self.is_stopped()
            && self.window.id.is_some()
            && self.window.window.is_some()
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.generation == adapter_generation)
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
    }

    fn native_visual_request_recovery_fences_hold(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && !self.has_terminal_cause()
            && !self.is_recovering()
            && !self.is_closing()
            && !self.is_stopped()
            && self.window.id.is_some()
            && self.window.window.is_some()
            && self
                .window
                .native_resources
                .as_ref()
                .is_some_and(|resources| resources.generation == adapter_generation)
            && (self.validated_pending_resize().is_some()
                || self.window.native_surface_target_fenced)
    }

    fn validated_pending_resize(&self) -> Option<winit::dpi::PhysicalSize<u32>> {
        self.timing
            .pending_surface_resize
            .filter(|size| size.width > 0 && size.height > 0)
    }

    pub(super) fn native_presentation_target_is_ready(
        &self,
        adapter: &GenericNativeAdapterOwner,
    ) -> bool {
        let Some(generation) = adapter.capture_generation() else {
            return false;
        };
        self.native_visual_request_is_eligible(generation)
    }

    /// Admit the irreversible native encode/submit/present boundary only after
    /// the caller has finished the CPU-side frame snapshot.  The exact packet
    /// identity is carried separately from its consuming packet so a pending
    /// visual successor cannot veto this complete snapshot.
    pub(super) fn admit_native_encode_present(
        &mut self,
        packet: super::native_visual_packet::NativeVisualRequestIdentity,
        adapter_generation: NativeAdapterGeneration,
        path: NativeEncodePresentPath,
        snapshot_revision: NativeFrameSnapshotRevision,
    ) -> Option<NativeEncodePresentTicket> {
        let lifecycle = self.native_lifecycle_snapshot();
        if !lifecycle.is_running()
            || !self.window.target_generation.is_known()
            || self.window.native_surface_target_fenced
            || self
                .window
                .native_resources
                .as_ref()
                .is_none_or(|resources| resources.generation != adapter_generation)
        {
            return None;
        }
        let target_generation = self.window.target_generation;
        let stage = self
            .frame_stage_owner
            .admit_encode_present(adapter_generation, target_generation)?;
        Some(NativeEncodePresentTicket::new(
            stage,
            NativeEncodePresentAdmission {
                packet,
                adapter_generation,
                target_generation,
                lifecycle,
                path,
                snapshot_revision,
            },
        ))
    }

    pub(super) fn native_encode_present_ticket_is_current(
        &self,
        ticket: &NativeEncodePresentTicket,
        packet: super::native_visual_packet::NativeVisualRequestIdentity,
        adapter: &GenericNativeAdapterOwner,
        path: NativeEncodePresentPath,
    ) -> bool {
        let Some(adapter_generation) = adapter.capture_generation() else {
            return false;
        };
        let resource_generation_current = self
            .window
            .native_resources
            .as_ref()
            .is_some_and(|resources| resources.generation == adapter_generation);
        ticket.is_current(
            &self.frame_stage_owner,
            NativeEncodePresentCurrentEvidence {
                packet,
                adapter_generation,
                target_generation: self.window.target_generation,
                lifecycle: self.native_lifecycle_snapshot(),
                path,
                snapshot_revision: ticket.snapshot_revision(),
            },
        ) && resource_generation_current
    }

    pub(super) fn complete_native_encode_present(
        &mut self,
        ticket: NativeEncodePresentTicket,
    ) -> bool {
        complete_native_encode_present(&mut self.frame_stage_owner, ticket)
    }

    pub(super) fn veto_native_encode_present(&mut self, ticket: NativeEncodePresentTicket) -> bool {
        veto_native_encode_present(&mut self.frame_stage_owner, ticket)
    }

    pub(super) fn allocate_native_frame_snapshot_revision(
        &mut self,
    ) -> Option<NativeFrameSnapshotRevision> {
        self.timing.native_frame_snapshot_revision.allocate()
    }

    pub(super) fn finish_native_visual_request(
        &mut self,
        packet: NativeVisualRequestPacket,
        disposition: NativeVisualRequestDisposition,
    ) -> NativeVisualRequestFinish {
        let Some(window) = self.window.window.as_ref().cloned() else {
            self.window.native_visual_requests.retire();
            self.clear_native_visual_request_wake();
            return NativeVisualRequestFinish::WrongWindow;
        };
        let Some(window_id) = self.window.id else {
            self.window.native_visual_requests.retire();
            self.clear_native_visual_request_wake();
            return NativeVisualRequestFinish::WrongWindow;
        };
        let result = NativeVisualRequestAdapter::finish(
            &mut self.window.native_visual_requests,
            &window,
            window_id,
            packet,
            disposition,
        );
        if matches!(result, NativeVisualRequestFinish::Reissued) {
            let now = Instant::now();
            self.timing.redraw_requested = true;
            self.timing.redraw_requested_at = Some(now);
        } else if matches!(result, NativeVisualRequestFinish::Retained)
            || !self.window.native_visual_requests.has_requested()
        {
            self.clear_native_visual_request_wake_timing();
        }
        result
    }

    pub(super) fn record_frame_work(&mut self, frame_work: FrameWork) {
        if !self.frame_observation_enabled {
            return;
        }
        self.timing.pending_frame_work = self.timing.pending_frame_work.merge(frame_work);
    }

    pub(super) fn record_native_interactive_arrival(&mut self, arrived_at: Instant) {
        self.timing.record_native_interactive_arrival_if_enabled(
            self.frame_observation_enabled,
            arrived_at,
        );
    }

    pub(super) fn take_pending_frame_work(&mut self) -> FrameWork {
        if !self.frame_observation_enabled {
            return FrameWork::None;
        }
        let frame_work = self.timing.pending_frame_work;
        self.timing.pending_frame_work = FrameWork::None;
        frame_work
    }

    pub(super) fn pending_redraw_request_is_stale(&self, now: Instant) -> bool {
        self.timing.redraw_requested_at.is_none_or(|requested_at| {
            now.duration_since(requested_at) >= Self::REDRAW_REISSUE_AFTER
        })
    }

    pub(super) fn should_defer_timed_frame_drain_for_pending_redraw(&self, now: Instant) -> bool {
        self.timing.redraw_requested && !self.pending_redraw_request_is_stale(now)
    }

    pub(super) fn pending_redraw_retry_deadline(&self) -> Option<Instant> {
        if !self.timing.redraw_requested
            || self.window.native_surface_target_fenced
            || self.window.native_visual_requests.is_suspended()
            || (!self.auxiliary_owner && !self.native_visual_request_scheduler_adapter_is_current())
            || (self.window.id.is_some() && !self.native_visual_request_schedule_is_eligible())
        {
            return None;
        }
        self.timing
            .redraw_requested_at
            .and_then(|requested_at| requested_at.checked_add(Self::REDRAW_REISSUE_AFTER))
    }

    pub(super) fn frame_wait_deadline(&self, scheduled: Instant) -> Instant {
        self.pending_redraw_retry_deadline()
            .map_or(scheduled, |deadline| scheduled.min(deadline))
    }

    pub(super) fn pending_redraw_elapsed(&self, now: Instant) -> Option<Duration> {
        if !self.timing.redraw_requested {
            return None;
        }
        let requested_at = self.timing.redraw_requested_at?;
        Some(now.duration_since(requested_at))
    }

    pub(super) fn pending_redraw_age(&self, now: Instant) -> CpuFramePendingRedrawAge {
        if !self.timing.redraw_requested {
            CpuFramePendingRedrawAge::NotRequested
        } else {
            self.timing
                .redraw_requested_at
                .map(|requested_at| {
                    CpuFramePendingRedrawAge::Known(now.saturating_duration_since(requested_at))
                })
                .unwrap_or(CpuFramePendingRedrawAge::Unknown)
        }
    }

    pub(super) fn pending_interactive_scroll_flush_is_due(&self, now: Instant) -> bool {
        self.timing.redraw_requested && self.pending_redraw_request_is_stale(now)
    }

    pub(super) fn should_flush_pending_redraw_after_route(
        &self,
        pending: Duration,
        _since_last_present: Duration,
    ) -> bool {
        pending >= Self::REDRAW_REISSUE_AFTER
    }

    pub(super) fn should_flush_pending_redraw_for_route_outcome(
        &self,
        outcome: GenericRouteOutcome,
        pending: Duration,
        since_last_present: Duration,
    ) -> bool {
        !matches!(
            outcome.native_input_stage_disposition(),
            Some(NativeInputStageDisposition::DeferLowerPriority)
        ) && self.should_flush_pending_redraw_after_route(pending, since_last_present)
    }

    fn should_log_pending_redraw_route_flush(
        &self,
        pending: Duration,
        since_last_present: Duration,
    ) -> bool {
        slow_render_profile_enabled()
            && (pending >= Self::REDRAW_REISSUE_LOG_AFTER
                || since_last_present >= Self::REDRAW_REISSUE_LOG_AFTER)
    }

    pub(super) fn drain_timed_frame_now(
        &mut self,
        now: Instant,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        if !self.is_running() {
            return GenericRouteOutcome::default();
        }
        self.timing.last_timed_frame_drain = now;
        self.core
            .drain_timed_frame(animation_activity, needs_text_caret_animation)
    }

    /// Resume a Deadline drain retained by the scheduler before this redraw
    /// performs resize or any CPU/native frame preparation. The scheduler owns
    /// the exact identity and payload; this boundary only dispatches its
    /// terminal route outcome and never selects or replays another frame.
    pub(super) fn resume_deferred_deadline_before_redraw(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &GenericNativeAdapterOwner,
    ) -> bool {
        if !self.is_running() || !self.frame_stage_owner.has_deferred_timed_frame() {
            return true;
        }
        let key = self.frame_stage_owner.schedule_key().clone();
        let admission = self.admit_deferred_timed_frame_deadline(
            Instant::now(),
            &key,
            adapter.capture_generation(),
            self.window.target_generation,
        );
        if !admission.route_outcome {
            return true;
        }
        self.handle_route_outcome_deferred_publication(event_loop, admission.outcome);
        self.is_running()
    }

    /// Finish an exact deferred Deadline operation before a discrete native
    /// input event is admitted. The caller captures the input timestamp before
    /// entering this boundary so synchronous drainage cannot change it.
    pub(super) fn resume_deferred_deadline_before_discrete_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        if !self.is_running() || !self.frame_stage_owner.has_deferred_timed_frame() {
            return self.is_running();
        }
        let admission = self.admit_deferred_timed_frame_before_discrete_input(
            adapter_generation,
            self.window.target_generation,
        );
        if admission.route_outcome {
            self.handle_route_outcome_deferred_publication(event_loop, admission.outcome);
        }
        self.is_running()
    }

    /// Finish an exact deferred Deadline operation before an
    /// ImmediateTransient event is admitted. The transient timestamp was
    /// captured by the caller before this synchronous drainage.
    pub(super) fn resume_deferred_deadline_before_immediate_transient(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        if !self.is_running() || !self.frame_stage_owner.has_deferred_timed_frame() {
            return self.is_running();
        }
        let admission = self.admit_deferred_timed_frame_before_native_input(
            adapter_generation,
            self.window.target_generation,
        );
        if admission.route_outcome {
            self.handle_route_outcome_deferred_publication(event_loop, admission.outcome);
        }
        self.is_running()
    }

    pub(super) fn merge_due_timed_frame_for_route(&mut self, outcome: &mut GenericRouteOutcome) {
        if !self.is_running() {
            return;
        }
        let now = Instant::now();
        let native_target_fps = self.options.normalized_target_fps();
        let native_frame_interval = animation_frame_interval_for_normalized_fps(native_target_fps);
        if now.duration_since(self.timing.last_timed_frame_drain) < native_frame_interval {
            return;
        }
        if self.should_defer_timed_frame_drain_for_pending_redraw(now) {
            return;
        }
        let animation_activity = self.core.animation_activity();
        let needs_text_caret_animation = self.core.has_focused_text_input();
        if !animation_activity.needs_animation() && !needs_text_caret_animation {
            return;
        }
        let frame_target_fps = timed_frame_target_fps(
            native_target_fps,
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            self.timing.last_timed_frame_drain,
            frame_target_fps,
            true,
        );
        if !matches!(cadence, TimedFrameCadence::DrainNow { .. }) {
            return;
        }
        outcome.merge(self.drain_timed_frame_now(
            now,
            animation_activity,
            needs_text_caret_animation,
        ));
    }

    pub(super) fn request_runtime_wakeup_if_needed(&self, outcome: GenericRouteOutcome) {
        if !self.is_running() {
            return;
        }
        if self.core.runtime.interactive_pointer_route_active() {
            return;
        }
        self.runtime_wakeup
            .request_if(outcome.runtime_work_remaining);
    }

    pub(super) fn rebuild_scene(&mut self) {
        self.rebuild_scene_with_refresh_evidence(false);
    }

    pub(super) fn rebuild_scene_after_surface_refresh(&mut self) {
        self.rebuild_scene_with_refresh_evidence(true);
    }

    fn rebuild_scene_with_refresh_evidence(&mut self, freshly_refreshed: bool) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.deferred_scene_rebuild_requires_encode = false;
        self.frame.reset_scene_build_outcome();
        let _ = self.apply_pending_viewport_resize_if_needed();
        let paint_plan_decision = self.core.paint_plan_into(&mut self.frame.last_paint_plan);
        self.frame.seed_text_input_snapshots_for_current_plan(
            matches!(paint_plan_decision, PaintPlanCacheDecision::Rebuilt),
            self.core.runtime.context().application_environment(),
        );
        self.publish_native_ime_cursor_area();
        self.admit_scene_from_current_plan(paint_plan_decision, freshly_refreshed, false);
    }

    /// Admit the exact plan installed by a successful prepared refresh. This
    /// path deliberately skips every plan-building and projection operation;
    /// it only runs the existing native scene decision chain and keeps full
    /// encoding detached until publication.
    pub(super) fn admit_prepared_scene_refresh(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.deferred_scene_rebuild_requires_encode = false;
        self.frame.reset_scene_build_outcome();
        self.admit_scene_from_current_plan(PaintPlanCacheDecision::Rebuilt, true, true);
    }

    pub(super) fn complete_prepared_surface_refresh(&mut self, terminal_messages: Vec<Message>) {
        // Keep the detached scene admission boundary ahead of terminal
        // dispatch for every prepared refresh consumer.
        self.admit_prepared_scene_refresh();
        self.core.finish_prepared_surface_refresh(terminal_messages);
    }

    fn admit_scene_from_current_plan(
        &mut self,
        paint_plan_decision: PaintPlanCacheDecision,
        freshly_refreshed: bool,
        prepared_refresh_admission: bool,
    ) {
        let viewport = self.core.runtime.viewport();
        let scene_validity = self.frame.native_scene_validity_fingerprint(
            self.core.base_paint_plan_context(),
            self.core.resolved_appearance(),
            self.window.dpi_scale,
        );
        if freshly_refreshed
            && matches!(paint_plan_decision, PaintPlanCacheDecision::Reused)
            && self.frame.can_reuse_native_scene(scene_validity)
            && !self.timing.surface_resize_applied_this_frame
        {
            // The scene remains valid, but this refresh still has visible work:
            // make the cached base texture/composited frame available for the
            // presentation pass and continue transient/native overlays below.
            self.frame.mark_scene_texture_dirty();
            self.frame.record_scene_reuse();
            self.restore_native_hover_cursor_overlay();
            self.export_automation_targets();
            return;
        }
        #[cfg(test)]
        self.frame.begin_test_phase_trace();
        let retained_surface = self.core.runtime.retained_surface_capability();
        let paint = self.core.paint_segment_observation();
        self.frame.observe_native_paint_segment_eligibility(
            paint,
            self.frame.last_scene_stats.artifact_feasibility,
            self.window.target_generation,
        );
        self.frame.derive_native_paint_segment_render_selection(
            scene_validity,
            self.window.target_generation,
        );
        let render_selection = self.frame.native_paint_segment_render_selection();
        let assembly_attempted = render_selection.should_attempt_mixed_assembly();
        let mut assembly_vetoed = false;
        if assembly_attempted {
            match self.frame.assemble_mixed_native_scene(
                viewport,
                paint,
                scene_validity,
                self.window.target_generation,
                render_selection.full_encode_plan(),
            ) {
                Ok(bundle) => {
                    if self
                        .frame
                        .commit_native_scene_assembly(bundle, scene_validity)
                        .is_err()
                    {
                        assembly_vetoed = true;
                    } else {
                        self.frame.refresh_gpu_surface_interaction_regions();
                        self.frame.refresh_post_gpu_overlay_cache();
                        self.restore_native_hover_cursor_overlay();
                        self.frame.mark_scene_content_dirty();
                        self.export_automation_targets();
                        return;
                    }
                }
                Err(_) => {
                    assembly_vetoed = true;
                }
            }
        }
        // Any attempted assembly that did not return above falls through to
        // the authoritative encoder for conservative repair. Prepared
        // refreshes use detached CPU-side state so the candidate can be
        // published atomically; ordinary rebuilds retain the original
        // in-place scene/cache path.
        #[cfg(test)]
        self.frame.record_scene_encode_boundary();
        if prepared_refresh_admission {
            let mut scene = Scene::new();
            let mut retained_surface_cache = self.frame.retained_surface_cache.clone();
            let mut scene_text_runs = SceneTextRunBuffer::new();
            let stats = encode_surface_paint_plan_to_scene(
                &self.frame.last_paint_plan,
                SurfaceSceneEncodeContext {
                    scene: &mut scene,
                    text_renderer: &mut self.frame.text_renderer,
                    bridge: self.core.runtime.bridge_mut(),
                    retained_surface,
                    viewport,
                    retained_cache: &mut retained_surface_cache,
                    text_runs: &mut scene_text_runs,
                    animation_time: self.timing.animation_origin.elapsed(),
                    text_input_snapshot_fence: self.frame.current_text_input_snapshot_fence,
                },
            );
            let eligibility = render_selection.full_encode_plan();
            let payloads = encode_native_paint_segment_payloads(
                &self.frame.last_paint_plan.primitives,
                viewport,
                paint,
                eligibility,
                scene_validity,
                self.window.target_generation,
                &self.frame.native_paint_segment_artifact_store,
            )
            .into_parts()
            .0;
            let materialization =
                materialize_native_paint_segment_artifacts(NativePaintSegmentArtifactAdmission {
                    scene: &scene,
                    feasibility: stats.artifact_feasibility,
                    plan: eligibility,
                    payloads,
                    scene_validity,
                    target_generation: self.window.target_generation,
                });
            let witness = NativeSceneAdmissionWitness {
                scene_validity,
                target_generation: self.window.target_generation,
                paint,
                eligibility: self.frame.last_native_paint_segment_eligibility,
                artifact_residency: render_selection.selected_artifact_residency(),
                render_selection,
            };
            self.frame
                .commit_native_scene_admission(NativeSceneAdmissionCandidate {
                    scene,
                    stats,
                    text_runs: Some(scene_text_runs),
                    retained_surface_cache: Some(retained_surface_cache),
                    materialization,
                    witness,
                    kind: NativeSceneAdmissionKind::FullEncode { assembly_vetoed },
                });
        } else {
            self.frame.last_scene_stats = encode_surface_paint_plan_to_scene(
                &self.frame.last_paint_plan,
                SurfaceSceneEncodeContext {
                    scene: &mut self.frame.scene,
                    text_renderer: &mut self.frame.text_renderer,
                    bridge: self.core.runtime.bridge_mut(),
                    retained_surface,
                    viewport,
                    retained_cache: &mut self.frame.retained_surface_cache,
                    text_runs: &mut self.frame.scene_text_runs,
                    animation_time: self.timing.animation_origin.elapsed(),
                    text_input_snapshot_fence: self.frame.current_text_input_snapshot_fence,
                },
            );
            let eligibility = render_selection.full_encode_plan();
            let payloads = encode_native_paint_segment_payloads(
                &self.frame.last_paint_plan.primitives,
                viewport,
                paint,
                eligibility,
                scene_validity,
                self.window.target_generation,
                &self.frame.native_paint_segment_artifact_store,
            )
            .into_parts()
            .0;
            let materialization =
                materialize_native_paint_segment_artifacts(NativePaintSegmentArtifactAdmission {
                    scene: &self.frame.scene,
                    feasibility: self.frame.last_scene_stats.artifact_feasibility,
                    plan: eligibility,
                    payloads,
                    scene_validity,
                    target_generation: self.window.target_generation,
                });
            self.frame
                .reconcile_native_paint_segment_artifacts(materialization);
            self.frame.reconcile_native_paint_segments(
                paint,
                self.frame.last_scene_stats.segment_encoding,
                self.window.target_generation,
            );
            if assembly_vetoed {
                self.frame
                    .record_scene_encode_after_assembly_veto(scene_validity);
            } else {
                self.frame.record_scene_encode(scene_validity);
            }
            self.frame.record_native_paint_segment_full_encode(
                paint,
                self.frame.last_scene_stats.segment_encoding,
                self.frame.last_scene_stats.artifact_feasibility,
                self.window.target_generation,
                assembly_vetoed,
            );
        }
        self.frame.refresh_gpu_surface_interaction_regions();
        self.frame.refresh_post_gpu_overlay_cache();
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_content_dirty();
        self.export_automation_targets();
    }

    pub(super) fn export_automation_targets(&mut self) {
        #[cfg(target_os = "macos")]
        self.republish_native_semantic_accessibility_passively();
        let snapshot = self.core.runtime.automation_target_snapshot();
        match self.automation_targets.export(&snapshot) {
            Ok(true) => {
                if let Some(path) = self.automation_targets.path() {
                    info!(
                        "radiant generic native vello: exported automation target snapshot to {}",
                        path.display()
                    );
                }
            }
            Ok(false) => {}
            Err(err) => {
                if self.automation_targets.has_warned_after_failure() {
                    return;
                }
                self.automation_targets.mark_warned_after_failure();
                if let Some(path) = err.path() {
                    warn!(
                        "radiant generic native vello: failed to export automation target snapshot to {}: {}",
                        path.display(),
                        err
                    );
                } else {
                    warn!(
                        "radiant generic native vello: failed to export automation target snapshot: {}",
                        err
                    );
                }
            }
        }
    }

    pub(super) fn rebuild_scene_for_interactive_route_now(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.last_interactive_scene_rebuild = Instant::now();
        self.rebuild_scene();
    }

    pub(super) fn rebuild_scene_for_interactive_route_now_after_surface_refresh(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.last_interactive_scene_rebuild = Instant::now();
        self.rebuild_scene_after_surface_refresh();
    }

    pub(super) fn refresh_and_rebuild_scene_now_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        let scope = self
            .take_deferred_surface_refresh_scope()
            .map_or(scope, |pending| pending.merge(scope));
        self.core.refresh_surface_with_scope(scope);
        self.rebuild_scene_after_surface_refresh();
    }

    pub(super) fn refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        let scope = self
            .take_deferred_surface_refresh_scope()
            .map_or(scope, |pending| pending.merge(scope));
        self.core.refresh_surface_with_scope(scope);
        self.rebuild_scene_for_interactive_route_now_after_surface_refresh();
    }

    pub(super) fn defer_surface_refresh_with_scope(&mut self, scope: crate::runtime::RepaintScope) {
        self.timing.deferred_surface_refresh = true;
        self.timing.deferred_surface_refresh_scope = Some(
            self.timing
                .deferred_surface_refresh_scope
                .map_or(scope, |pending| pending.merge(scope)),
        );
    }

    pub(super) fn take_deferred_surface_refresh_scope(
        &mut self,
    ) -> Option<crate::runtime::RepaintScope> {
        if !self.timing.deferred_surface_refresh {
            return None;
        }
        self.timing.deferred_surface_refresh = false;
        Some(
            self.timing
                .deferred_surface_refresh_scope
                .take()
                .unwrap_or(crate::runtime::RepaintScope::Surface),
        )
    }

    pub(super) fn should_rebuild_interactive_scene_now(&self, now: Instant) -> bool {
        let interval = animation_frame_interval(self.options.normalized_target_fps());
        now.duration_since(self.timing.last_interactive_scene_rebuild) >= interval
    }

    pub(super) fn defer_scene_rebuild(&mut self) {
        self.timing.deferred_scene_rebuild = true;
        self.timing.deferred_scene_rebuild_requires_encode = true;
    }

    #[cfg(test)]
    pub(super) fn defer_viewport_resize(&mut self, viewport: Vector2) {
        self.defer_viewport_resize_with_reason(viewport, FrameWorkReason::NativeResize);
    }

    pub(super) fn defer_viewport_resize_with_reason(
        &mut self,
        viewport: Vector2,
        reason: FrameWorkReason,
    ) {
        self.timing.pending_viewport_resize = Some(viewport);
        self.timing.pending_viewport_resize_reason = Some(reason);
        self.timing.deferred_scene_rebuild = true;
    }

    pub(super) fn apply_pending_viewport_resize_if_needed(&mut self) -> Option<bool> {
        let viewport = self.timing.pending_viewport_resize.take()?;
        let reason = self
            .timing
            .pending_viewport_resize_reason
            .take()
            .unwrap_or(FrameWorkReason::NativeResize);
        let relayout = self.core.set_viewport(viewport);
        if relayout {
            self.record_frame_work(FrameWork::ResizeAndRebuild { reason });
        }
        Some(relayout)
    }

    pub(super) fn defer_interactive_scene_rebuild(&mut self) {
        self.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::Surface);
        self.defer_scene_rebuild();
    }

    pub(super) fn defer_interactive_scene_rebuild_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        self.defer_surface_refresh_with_scope(scope);
        self.defer_scene_rebuild();
    }

    pub(super) fn queue_window_environment_change(
        &mut self,
        change: crate::runtime::WindowEnvironmentChange,
    ) {
        self.queue_window_environment_change_with_reason(
            change,
            FrameWorkReason::NativeWindowEnvironment,
        );
    }

    pub(super) fn queue_window_environment_change_with_reason(
        &mut self,
        change: crate::runtime::WindowEnvironmentChange,
        reason: FrameWorkReason,
    ) {
        self.defer_interactive_scene_rebuild_with_scope(change.repaint_scope());
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason,
            mode: SceneRebuildMode::Interactive,
        });
    }

    pub(super) fn update_window_environment(
        &mut self,
        environment: crate::runtime::WindowEnvironment,
    ) -> bool {
        if self.window.environment == environment {
            return false;
        }
        self.window.environment = environment;
        self.core.runtime.set_window_environment(environment)
    }

    pub(super) fn observe_monitor_move(&mut self) {
        let Some(window) = self.window.window.as_ref() else {
            return;
        };
        let Some(next) = super::window_environment::current_monitor_fingerprint(window) else {
            return;
        };
        if self.window.monitor_fingerprint.as_ref() == Some(&next) {
            return;
        }
        self.window.monitor_fingerprint = Some(next);
        self.queue_window_environment_change(
            crate::runtime::WindowEnvironmentChange::DisplayScaleOrMonitor,
        );
    }

    pub(super) fn observe_theme_change(&mut self, theme: Option<winit::window::Theme>) {
        let environment = super::window_environment::environment_for_native_state(
            self.window.dpi_scale,
            super::window_environment::window_color_scheme(theme),
            self.window.accessibility_display,
        );
        if self.update_window_environment(environment) {
            self.queue_window_environment_change(
                crate::runtime::WindowEnvironmentChange::ColorSchemeOrContrast,
            );
        }
    }

    fn restore_native_hover_cursor_overlay(&mut self) {
        let Some(position) = self.input.last_cursor else {
            return;
        };
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
        }
    }

    pub(super) fn handle_route_outcome(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, true, true);
    }

    pub(super) fn sync_native_ime_allowed(&self) {
        if let Some(window) = self.window.window.as_ref() {
            window.set_ime_allowed(self.core.has_focused_text_input());
        }
    }

    pub(super) fn publish_native_ime_cursor_area(&mut self) {
        let candidate = self.frame.native_ime_cursor_area();
        let Some(window) = self.window.window.as_ref().cloned() else {
            self.window.ime_cursor_area_cache.invalidate();
            return;
        };
        let window_id = window.id();
        let native_scale_generation = self.window.target_generation;
        let native_dpi_scale = self.window.native_dpi_scale;
        let Some(area) = self.window.ime_cursor_area_cache.candidate_to_publish(
            window_id,
            native_scale_generation,
            native_dpi_scale,
            candidate,
        ) else {
            return;
        };
        window.set_ime_cursor_area(
            LogicalPosition::new(area.min.x as f64, area.min.y as f64),
            LogicalSize::new(area.width() as f64, area.height() as f64),
        );
        self.window.ime_cursor_area_cache.record(
            window_id,
            native_scale_generation,
            native_dpi_scale,
            area,
        );
    }

    pub(super) fn handle_route_outcome_without_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, false, true);
    }

    pub(super) fn handle_route_outcome_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: &mut GenericNativeAdapterOwner,
        _observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        self.handle_route_outcome_inner(
            event_loop,
            outcome,
            Some(adapter),
            _observation,
            true,
            false,
        );
    }

    pub(super) fn handle_route_outcome_with_adapter_without_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        self.handle_route_outcome_inner(
            event_loop,
            outcome,
            Some(adapter),
            observation,
            false,
            false,
        );
    }

    pub(super) fn handle_route_outcome_deferred_publication(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, false, false);
    }

    fn handle_route_outcome_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: Option<&mut GenericNativeAdapterOwner>,
        _observation: Option<&mut CpuFrameObservationOwner<'_>>,
        merge_due_timed_frame: bool,
        publish_frame_diagnostics: bool,
    ) {
        if !self.is_running() {
            return;
        }
        let applied = if merge_due_timed_frame {
            self.apply_route_outcome(outcome)
        } else {
            self.apply_route_outcome_with_timed_frame(outcome, false)
        };
        if applied.exit_requested {
            self.admit_native_shutdown(event_loop, None);
            return;
        }
        self.sync_native_ime_allowed();
        if applied.sync_auxiliary_windows_now
            && adapter.is_none()
            && let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy()
            && self
                .sync_auxiliary_windows(event_loop, event_proxy)
                .is_err()
        {
            return;
        }
        let route_end_now = Instant::now();
        if self.timing.redraw_requested
            && self.pending_redraw_request_is_stale(route_end_now)
            && let Some(pending) = self.pending_redraw_elapsed(route_end_now)
        {
            let since_last_present = route_end_now.duration_since(self.timing.last_redraw);
            if self.should_flush_pending_redraw_for_route_outcome(
                outcome,
                pending,
                since_last_present,
            ) {
                if self.should_log_pending_redraw_route_flush(pending, since_last_present) {
                    warn!(
                        target: "radiant::debug::frame_profile",
                        event = "radiant.redraw_request.flushed_pending",
                        pending_us = pending.as_micros(),
                        since_last_present_us = since_last_present.as_micros(),
                        stale = pending >= Self::REDRAW_REISSUE_AFTER,
                        "Flushed current pending redraw request after route"
                    );
                }
                // Route-time work may reissue the exact native packet, but
                // presentation is owned exclusively by the later
                // `WindowEvent::RedrawRequested` boundary.
                self.request_redraw_for_frame_work(FrameWork::None);
            }
        }
        if publish_frame_diagnostics {
            self.publish_staged_frame_diagnostics();
        }
    }

    pub(super) fn apply_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
    ) -> AppliedRouteOutcome {
        self.apply_route_outcome_with_timed_frame(outcome, true)
    }

    pub(super) fn apply_route_outcome_with_timed_frame(
        &mut self,
        mut outcome: GenericRouteOutcome,
        merge_due_timed_frame: bool,
    ) -> AppliedRouteOutcome {
        if !self.is_running() {
            return AppliedRouteOutcome::default();
        }
        if outcome.exit_requested {
            return AppliedRouteOutcome {
                exit_requested: true,
                sync_auxiliary_windows_now: false,
            };
        }
        let defer_lower_priority = matches!(
            outcome.native_input_stage_disposition(),
            Some(NativeInputStageDisposition::DeferLowerPriority)
        );
        if merge_due_timed_frame && !defer_lower_priority {
            self.merge_due_timed_frame_for_route(&mut outcome);
        }
        if let Some(scale) = outcome.dpi_scale_override {
            self.set_dpi_scale_override(scale);
        }
        if let Some(size) = outcome.window_logical_size {
            self.set_window_logical_size(size);
        }
        if defer_lower_priority {
            return self.defer_lower_priority_route_outcome(outcome);
        }
        let mut sync_auxiliary_windows_now = false;
        match outcome.frame_work() {
            FrameWork::None
            | FrameWork::PaintOnly { .. }
            | FrameWork::ResizeSurface { .. }
            | FrameWork::Exit { .. } => {}
            FrameWork::RefreshSurface { .. } => {
                self.defer_surface_refresh_with_scope(outcome.surface_refresh_scope_or_surface());
            }
            FrameWork::ResizeAndRebuild { .. } => {
                self.rebuild_scene();
                sync_auxiliary_windows_now = true;
            }
            FrameWork::RebuildScene { mode, .. } => match mode {
                SceneRebuildMode::InteractiveWithSurfaceRefresh => {
                    self.refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    self.defer_auxiliary_window_sync();
                }
                SceneRebuildMode::ImmediateWithSurfaceRefresh => {
                    self.refresh_and_rebuild_scene_now_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    sync_auxiliary_windows_now = true;
                }
                SceneRebuildMode::Interactive => {
                    let now = Instant::now();
                    if self.should_rebuild_interactive_scene_now(now) {
                        self.rebuild_scene_for_interactive_route_now();
                        self.defer_auxiliary_window_sync();
                    } else {
                        self.defer_interactive_scene_rebuild();
                        self.defer_auxiliary_window_sync();
                    }
                }
                SceneRebuildMode::Immediate => {
                    self.rebuild_scene();
                    sync_auxiliary_windows_now = true;
                }
            },
        }
        if outcome.needs_redraw() {
            self.request_redraw_for_frame_work(outcome.frame_work());
        }
        self.request_runtime_wakeup_if_needed(outcome);
        AppliedRouteOutcome {
            exit_requested: false,
            sync_auxiliary_windows_now,
        }
    }

    /// Defer only the lower-priority work attached to an exact over-budget
    /// input route. Semantic input has already completed; the existing bounded
    /// deferred flags and visual mailbox retain the latest safe visual state.
    pub(super) fn defer_lower_priority_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
    ) -> AppliedRouteOutcome {
        let frame_work = outcome.frame_work();
        self.record_frame_work(frame_work);
        match frame_work {
            FrameWork::None | FrameWork::PaintOnly { .. } | FrameWork::ResizeSurface { .. } => {}
            FrameWork::RefreshSurface { .. } => {
                self.defer_surface_refresh_with_scope(outcome.surface_refresh_scope_or_surface());
            }
            FrameWork::ResizeAndRebuild { .. } => {
                self.defer_scene_rebuild();
                self.defer_auxiliary_window_sync();
            }
            FrameWork::RebuildScene { mode, .. } => match mode {
                SceneRebuildMode::InteractiveWithSurfaceRefresh => {
                    self.defer_interactive_scene_rebuild_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    self.defer_auxiliary_window_sync();
                }
                SceneRebuildMode::ImmediateWithSurfaceRefresh => {
                    self.defer_surface_refresh_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    self.defer_scene_rebuild();
                    self.defer_auxiliary_window_sync();
                }
                SceneRebuildMode::Interactive => {
                    self.defer_interactive_scene_rebuild();
                    self.defer_auxiliary_window_sync();
                }
                SceneRebuildMode::Immediate => {
                    self.defer_scene_rebuild();
                    self.defer_auxiliary_window_sync();
                }
            },
            FrameWork::Exit { .. } => {
                // Exit is handled before this helper and is never deferred.
            }
        }
        if !outcome.exit_requested {
            self.request_redraw_for_pending_coalesced_input();
        }
        AppliedRouteOutcome {
            exit_requested: false,
            sync_auxiliary_windows_now: false,
        }
    }
}

#[cfg(test)]
#[path = "runner/tests.rs"]
mod tests;
