//! Bounded per-window recovery for a failed native Vello renderer call.

use super::{
    NativeGenericRunError, NativeInitializationStage, RuntimeUserEvent,
    adapter::{GenericNativeAdapterOwner, NativeAdapterGeneration},
    runner_state::{NativeTargetGeneration, NativeWindowResourceBundle},
};
use crate::gui_runtime::native_vello::{select_present_mode, startup_renderer_options};
use crate::gui_runtime::{NativeGpuBackend, NativeRunOptions};
use std::sync::Arc;
use vello::{Renderer, util::RenderSurface};
use winit::{
    event_loop::EventLoopProxy,
    window::{Window, WindowId},
};

/// One per-window renderer reconstruction allowance.
///
/// The allowance is keyed only by the shared adapter generation. Target
/// generation changes belong to the window and do not silently grant another
/// renderer attempt. A newer adapter generation does.
#[derive(Default)]
pub(super) struct NativeRendererRecoveryPolicy {
    attempted_generation: Option<NativeAdapterGeneration>,
}

impl NativeRendererRecoveryPolicy {
    pub(super) fn admits(&self, generation: NativeAdapterGeneration) -> bool {
        generation.is_known()
            && self
                .attempted_generation
                .is_none_or(|previous| generation.is_strictly_newer_than(previous))
    }

    pub(super) fn record_attempt(&mut self, generation: NativeAdapterGeneration) {
        self.attempted_generation = Some(generation);
    }

    pub(super) fn attempt_matches(&self, generation: NativeAdapterGeneration) -> bool {
        self.attempted_generation == Some(generation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeRendererRecoveryVeto {
    Lifecycle,
    MissingWindow,
    MissingActiveBundle,
    UnknownAdapterGeneration,
    ActiveGenerationMismatch,
    PublicationCapacity,
    AttemptAlreadyUsed,
    TargetGenerationExhausted,
}

/// The candidate construction mode shared by primary and auxiliary windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeRendererRecoveryWindowKind {
    Primary,
    Auxiliary { requested_backend: NativeGpuBackend },
}

/// Immutable evidence captured before any candidate GPU setup begins.
pub(super) struct NativeRendererRecoveryAdmission {
    pub(super) generation: NativeAdapterGeneration,
    pub(super) window_id: WindowId,
    pub(super) window: Arc<Window>,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) next_target_generation: NativeTargetGeneration,
}

/// A complete fresh native resource bundle plus the exact window identity it
/// was constructed for. The old active bundle is never copied into this type.
pub(super) struct NativeRendererRecoveryCandidate {
    pub(super) generation: NativeAdapterGeneration,
    pub(super) window_id: WindowId,
    pub(super) window: Arc<Window>,
    pub(super) bundle: NativeWindowResourceBundle,
}

/// Current event-loop-owned facts rechecked immediately before publication.
pub(super) struct NativeRendererRecoveryCommitFacts<'a> {
    pub(super) active_generation: Option<NativeAdapterGeneration>,
    pub(super) current_generation: Option<NativeAdapterGeneration>,
    pub(super) current_window: Option<(&'a Arc<Window>, WindowId)>,
    pub(super) publication_available: bool,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) run_admissible: bool,
}

pub(super) fn preflight_renderer_recovery(
    policy: &NativeRendererRecoveryPolicy,
    active_generation: Option<NativeAdapterGeneration>,
    current_generation: Option<NativeAdapterGeneration>,
    window: Option<(&Arc<Window>, WindowId)>,
    publication_available: bool,
    target_generation: NativeTargetGeneration,
    run_admissible: bool,
) -> Result<NativeRendererRecoveryAdmission, NativeRendererRecoveryVeto> {
    if !run_admissible {
        return Err(NativeRendererRecoveryVeto::Lifecycle);
    }
    let Some((window, window_id)) = window else {
        return Err(NativeRendererRecoveryVeto::MissingWindow);
    };
    let Some(current_generation) = current_generation else {
        return Err(NativeRendererRecoveryVeto::UnknownAdapterGeneration);
    };
    if !current_generation.is_known() {
        return Err(NativeRendererRecoveryVeto::UnknownAdapterGeneration);
    }
    if active_generation.is_none() {
        return Err(NativeRendererRecoveryVeto::MissingActiveBundle);
    }
    if active_generation != Some(current_generation) {
        return Err(NativeRendererRecoveryVeto::ActiveGenerationMismatch);
    }
    if !publication_available {
        return Err(NativeRendererRecoveryVeto::PublicationCapacity);
    }
    if !policy.admits(current_generation) {
        return Err(NativeRendererRecoveryVeto::AttemptAlreadyUsed);
    }
    let mut next_target_generation = target_generation;
    if !next_target_generation.advance() {
        return Err(NativeRendererRecoveryVeto::TargetGenerationExhausted);
    }
    Ok(NativeRendererRecoveryAdmission {
        generation: current_generation,
        window_id,
        window: Arc::clone(window),
        target_generation,
        next_target_generation,
    })
}

/// Recheck every identity and lifecycle fact immediately before publication.
pub(super) fn renderer_recovery_commit_is_valid(
    policy: &NativeRendererRecoveryPolicy,
    admission: &NativeRendererRecoveryAdmission,
    candidate: &NativeRendererRecoveryCandidate,
    facts: NativeRendererRecoveryCommitFacts<'_>,
) -> bool {
    facts.run_admissible
        && policy.attempt_matches(admission.generation)
        && facts.active_generation == Some(admission.generation)
        && facts.current_generation == Some(admission.generation)
        && facts.publication_available
        && facts.target_generation == admission.target_generation
        && candidate.generation == admission.generation
        && candidate.bundle.generation == admission.generation
        && candidate.window_id == admission.window_id
        && facts.current_window.is_some_and(|(window, window_id)| {
            window_id == candidate.window_id && Arc::ptr_eq(window, &candidate.window)
        })
}

/// Construct one complete candidate against the adapter's already selected
/// device and queue. This function performs no adapter/device selection and no
/// queue submission; all old renderer/surface/GPU/completion state stays owned
/// by the active bundle until the caller publishes this candidate.
pub(super) fn construct_renderer_recovery_candidate(
    options: &NativeRunOptions,
    adapter: &GenericNativeAdapterOwner,
    admission: &NativeRendererRecoveryAdmission,
    event_proxy: EventLoopProxy<RuntimeUserEvent>,
    kind: NativeRendererRecoveryWindowKind,
) -> Result<NativeRendererRecoveryCandidate, NativeGenericRunError> {
    let window = Arc::clone(&admission.window);
    let instance = adapter.instance().ok_or_else(|| {
        renderer_recovery_error(
            NativeInitializationStage::WgpuSurfaceCreation,
            "native adapter render context is unavailable",
        )
    })?;
    let surface = instance.create_surface(window.clone()).map_err(|error| {
        renderer_recovery_error(NativeInitializationStage::WgpuSurfaceCreation, error)
    })?;
    if let NativeRendererRecoveryWindowKind::Auxiliary { requested_backend } = kind {
        adapter
            .validate_auxiliary_surface(requested_backend, &surface)
            .map_err(|error| {
                renderer_recovery_error(NativeInitializationStage::DeviceAcquisition, error)
            })?;
    }
    let device = adapter.selected_device_handle().ok_or_else(|| {
        renderer_recovery_error(
            NativeInitializationStage::DeviceAcquisition,
            "native adapter did not retain its selected device",
        )
    })?;
    let capabilities = surface.get_capabilities(device.adapter());
    let present_mode =
        select_present_mode(options.normalized_target_fps(), &capabilities.present_modes);
    let size = window.inner_size();
    let render_surface: RenderSurface<'static> = adapter
        .create_render_surface_for_selected(
            surface,
            size.width.max(1),
            size.height.max(1),
            present_mode,
        )
        .map_err(|error| {
            renderer_recovery_error(NativeInitializationStage::RenderSurfaceCreation, error)
        })?;
    let renderer_options = startup_renderer_options();
    let renderer = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Renderer::new(&device.device, renderer_options)
    })) {
        Ok(Ok(renderer)) => renderer,
        Ok(Err(error)) => {
            return Err(renderer_recovery_error(
                NativeInitializationStage::RendererCreation,
                error,
            ));
        }
        Err(payload) => {
            return Err(renderer_recovery_error(
                NativeInitializationStage::RendererCreation,
                normalize_renderer_creation_panic(payload),
            ));
        }
    };
    let bundle = NativeWindowResourceBundle::new(
        admission.generation,
        render_surface,
        renderer,
        &device.device,
        &device.queue,
        event_proxy,
    )
    .ok_or_else(|| {
        renderer_recovery_error(
            NativeInitializationStage::DeviceAcquisition,
            "fresh renderer recovery bundle was not generation-bound",
        )
    })?;
    Ok(NativeRendererRecoveryCandidate {
        generation: admission.generation,
        window_id: admission.window_id,
        window,
        bundle,
    })
}

fn renderer_recovery_error(
    stage: NativeInitializationStage,
    error: impl std::fmt::Display,
) -> NativeGenericRunError {
    NativeGenericRunError::NativeInitialization {
        stage,
        message: error.to_string(),
    }
}

fn normalize_renderer_creation_panic(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(_) => String::from("renderer construction panic payload was not a string"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::{NativeAdapterGeneration, runner_state::NativeTargetGeneration};
    use super::{
        NativeRendererRecoveryPolicy, NativeRendererRecoveryVeto, preflight_renderer_recovery,
    };

    fn generation(serial: u64) -> NativeAdapterGeneration {
        NativeAdapterGeneration::from_test_serial(serial)
    }

    fn target_generation(serial: u64) -> NativeTargetGeneration {
        NativeTargetGeneration::from_test_serial(serial)
    }

    #[test]
    fn first_exact_generation_is_admitted() {
        let policy = NativeRendererRecoveryPolicy::default();

        assert!(policy.admits(generation(1)));
    }

    #[test]
    fn duplicate_same_generation_is_rejected_after_attempt_recording() {
        let mut policy = NativeRendererRecoveryPolicy::default();
        let current = generation(1);

        policy.record_attempt(current);

        assert!(!policy.admits(current));
    }

    #[test]
    fn newer_generation_restores_one_reconstruction_allowance() {
        let mut policy = NativeRendererRecoveryPolicy::default();
        policy.record_attempt(generation(1));

        assert!(policy.admits(generation(2)));
    }

    #[test]
    fn stale_unknown_and_exhausted_generations_are_rejected() {
        let mut policy = NativeRendererRecoveryPolicy::default();
        policy.record_attempt(generation(2));
        let mut exhausted = generation(u64::MAX);
        assert!(!exhausted.advance());

        assert!(!policy.admits(generation(1)));
        assert!(!policy.admits(NativeAdapterGeneration::unknown()));
        assert!(!policy.admits(exhausted));
    }

    #[test]
    fn failed_candidate_construction_consumes_the_recorded_attempt() {
        let mut policy = NativeRendererRecoveryPolicy::default();
        let current = generation(1);

        assert!(policy.admits(current));
        policy.record_attempt(current);
        // A failed candidate is intentionally not a rollback point.
        assert!(!policy.admits(current));
    }

    #[test]
    fn target_generation_only_changes_do_not_reset_the_adapter_attempt() {
        let mut policy = NativeRendererRecoveryPolicy::default();
        let current = generation(1);
        let mut target = target_generation(1);

        policy.record_attempt(current);
        assert!(target.advance());

        assert!(!policy.admits(current));
    }

    #[test]
    fn preflight_rejects_every_non_admissible_boundary_before_gpu_setup() {
        let policy = NativeRendererRecoveryPolicy::default();
        let current = generation(1);
        let target = target_generation(1);

        assert!(matches!(
            preflight_renderer_recovery(
                &policy,
                Some(current),
                Some(current),
                None,
                true,
                target,
                false,
            ),
            Err(NativeRendererRecoveryVeto::Lifecycle)
        ));
        assert!(matches!(
            preflight_renderer_recovery(&policy, None, Some(current), None, true, target, true,),
            Err(NativeRendererRecoveryVeto::MissingWindow)
        ));
    }
}
