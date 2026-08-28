//! Private ownership and admission evidence for native render-target retirement.

use super::{
    adapter::NativeAdapterGeneration, runner_state::NativeTargetGeneration,
    submission_completion::NativeSubmissionCompletionIdentity,
};
use vello::wgpu;

/// Exact evidence captured immediately before replacing a published target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeRenderTargetReplacementEvidence {
    resource_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    completion: NativeSubmissionCompletionIdentity,
}

impl NativeRenderTargetReplacementEvidence {
    pub(super) const fn new(
        resource_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        completion: NativeSubmissionCompletionIdentity,
    ) -> Self {
        Self {
            resource_generation,
            target_generation,
            completion,
        }
    }

    pub(super) const fn resource_generation(self) -> NativeAdapterGeneration {
        self.resource_generation
    }

    pub(super) const fn target_generation(self) -> NativeTargetGeneration {
        self.target_generation
    }

    pub(super) const fn completion(self) -> NativeSubmissionCompletionIdentity {
        self.completion
    }
}

/// Exact identity of the predecessor retained after a target replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeRenderTargetRetirementIdentity {
    resource_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    completion: NativeSubmissionCompletionIdentity,
}

/// One old target pair retained until its bundle completion witness is ready.
pub(super) struct NativeRenderTargetRetirement {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    identity: NativeRenderTargetRetirementIdentity,
}

impl NativeRenderTargetRetirement {
    pub(super) const fn new(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        identity: NativeRenderTargetRetirementIdentity,
    ) -> Self {
        Self {
            texture,
            view,
            identity,
        }
    }

    pub(super) const fn identity(&self) -> NativeRenderTargetRetirementIdentity {
        self.identity
    }

    pub(super) fn drop_owned_targets(self) {
        let Self { texture, view, .. } = self;
        drop((texture, view));
    }

    pub(super) fn requested_rgba8_bytes(&self) -> Option<u64> {
        requested_rgba8_bytes(self.identity.width, self.identity.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeRenderTargetReplacementMode {
    Ordinary,
    Recovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeRenderTargetReplacementOutcome {
    Noop,
    Deferred,
    Committed {
        next_target_generation: NativeTargetGeneration,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeRenderTargetReplacementRequest {
    pub(super) mode: NativeRenderTargetReplacementMode,
    pub(super) current_width: u32,
    pub(super) current_height: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) predecessor_occupied: bool,
    pub(super) evidence: Option<NativeRenderTargetReplacementEvidence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeRenderTargetReplacementContext {
    pub(super) surface_device_id: usize,
    pub(super) selected_device_id: Option<usize>,
    pub(super) selected_generation: Option<NativeAdapterGeneration>,
}

pub(super) const NATIVE_RENDER_TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub(super) fn replacement_preflight(
    request: NativeRenderTargetReplacementRequest,
    context: NativeRenderTargetReplacementContext,
) -> NativeRenderTargetReplacementOutcome {
    let NativeRenderTargetReplacementRequest {
        mode,
        current_width,
        current_height,
        width,
        height,
        predecessor_occupied,
        evidence,
    } = request;
    let NativeRenderTargetReplacementContext {
        surface_device_id,
        selected_device_id,
        selected_generation,
    } = context;

    if mode == NativeRenderTargetReplacementMode::Ordinary
        && current_width == width
        && current_height == height
    {
        return NativeRenderTargetReplacementOutcome::Noop;
    }
    if width == 0 || height == 0 || predecessor_occupied {
        return NativeRenderTargetReplacementOutcome::Deferred;
    }
    let Some(selected_device_id) = selected_device_id else {
        return NativeRenderTargetReplacementOutcome::Deferred;
    };
    if surface_device_id != selected_device_id {
        return NativeRenderTargetReplacementOutcome::Deferred;
    }
    let Some(selected_generation) = selected_generation else {
        return NativeRenderTargetReplacementOutcome::Deferred;
    };
    let Some(evidence) = evidence else {
        return NativeRenderTargetReplacementOutcome::Deferred;
    };
    if evidence.resource_generation() != selected_generation
        || !evidence.resource_generation().is_known()
        || evidence.completion().generation() != evidence.resource_generation()
        || !evidence.completion().is_valid_for_retirement()
    {
        return NativeRenderTargetReplacementOutcome::Deferred;
    }
    let mut next_target_generation = evidence.target_generation();
    if !next_target_generation.is_known() || !next_target_generation.advance() {
        return NativeRenderTargetReplacementOutcome::Deferred;
    }
    NativeRenderTargetReplacementOutcome::Committed {
        next_target_generation,
    }
}

pub(super) fn requested_rgba8_bytes(width: u32, height: u32) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes > 0)
}

pub(super) fn retirement_identity(
    evidence: NativeRenderTargetReplacementEvidence,
    width: u32,
    height: u32,
) -> NativeRenderTargetRetirementIdentity {
    NativeRenderTargetRetirementIdentity {
        resource_generation: evidence.resource_generation(),
        target_generation: evidence.target_generation(),
        width,
        height,
        format: NATIVE_RENDER_TARGET_FORMAT,
        completion: evidence.completion(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::submission_completion::NativeSubmissionCompletionIdentity;

    fn evidence(resource_serial: u64, target_serial: u64) -> NativeRenderTargetReplacementEvidence {
        let resource_generation = NativeAdapterGeneration::from_test_serial(resource_serial);
        NativeRenderTargetReplacementEvidence::new(
            resource_generation,
            NativeTargetGeneration::from_test_serial(target_serial),
            NativeSubmissionCompletionIdentity::never_submitted(resource_generation),
        )
    }

    fn request(
        mode: NativeRenderTargetReplacementMode,
        current_size: (u32, u32),
        requested_size: (u32, u32),
        predecessor_occupied: bool,
        evidence: Option<NativeRenderTargetReplacementEvidence>,
    ) -> NativeRenderTargetReplacementRequest {
        NativeRenderTargetReplacementRequest {
            mode,
            current_width: current_size.0,
            current_height: current_size.1,
            width: requested_size.0,
            height: requested_size.1,
            predecessor_occupied,
            evidence,
        }
    }

    fn context(
        surface_device_id: usize,
        selected_device_id: Option<usize>,
        selected_generation: Option<NativeAdapterGeneration>,
    ) -> NativeRenderTargetReplacementContext {
        NativeRenderTargetReplacementContext {
            surface_device_id,
            selected_device_id,
            selected_generation,
        }
    }

    #[test]
    fn ordinary_same_size_is_a_noop_before_target_creation_evidence() {
        assert_eq!(
            replacement_preflight(
                request(
                    NativeRenderTargetReplacementMode::Ordinary,
                    (640, 360),
                    (640, 360),
                    true,
                    None,
                ),
                context(
                    7,
                    Some(7),
                    Some(NativeAdapterGeneration::from_test_serial(1)),
                ),
            ),
            NativeRenderTargetReplacementOutcome::Noop
        );
    }

    #[test]
    fn valid_replacement_advances_only_the_successor_generation() {
        let result = replacement_preflight(
            request(
                NativeRenderTargetReplacementMode::Ordinary,
                (640, 360),
                (800, 450),
                false,
                Some(evidence(1, 4)),
            ),
            context(
                7,
                Some(7),
                Some(NativeAdapterGeneration::from_test_serial(1)),
            ),
        );
        assert_eq!(
            result,
            NativeRenderTargetReplacementOutcome::Committed {
                next_target_generation: NativeTargetGeneration::from_test_serial(5),
            }
        );
    }

    #[test]
    fn recovery_replacement_advances_generation_even_for_same_size() {
        assert_eq!(
            replacement_preflight(
                request(
                    NativeRenderTargetReplacementMode::Recovery,
                    (640, 360),
                    (640, 360),
                    false,
                    Some(evidence(1, 4)),
                ),
                context(
                    7,
                    Some(7),
                    Some(NativeAdapterGeneration::from_test_serial(1)),
                ),
            ),
            NativeRenderTargetReplacementOutcome::Committed {
                next_target_generation: NativeTargetGeneration::from_test_serial(5),
            }
        );
    }

    #[test]
    fn occupied_predecessor_defers_without_consuming_latest_evidence() {
        let result = replacement_preflight(
            request(
                NativeRenderTargetReplacementMode::Ordinary,
                (640, 360),
                (800, 450),
                true,
                Some(evidence(1, 4)),
            ),
            context(
                7,
                Some(7),
                Some(NativeAdapterGeneration::from_test_serial(1)),
            ),
        );
        assert_eq!(result, NativeRenderTargetReplacementOutcome::Deferred);
    }

    #[test]
    fn missing_stale_unknown_and_exhausted_evidence_vetoes() {
        let resource = NativeAdapterGeneration::from_test_serial(1);
        let cases = [
            None,
            Some(NativeRenderTargetReplacementEvidence::new(
                NativeAdapterGeneration::from_test_serial(2),
                NativeTargetGeneration::from_test_serial(4),
                NativeSubmissionCompletionIdentity::never_submitted(resource),
            )),
            Some(NativeRenderTargetReplacementEvidence::new(
                resource,
                NativeTargetGeneration::unknown(),
                NativeSubmissionCompletionIdentity::never_submitted(resource),
            )),
        ];
        for evidence in cases {
            assert_eq!(
                replacement_preflight(
                    request(
                        NativeRenderTargetReplacementMode::Recovery,
                        (640, 360),
                        (800, 450),
                        false,
                        evidence,
                    ),
                    context(7, Some(7), Some(resource)),
                ),
                NativeRenderTargetReplacementOutcome::Deferred
            );
        }
        assert_eq!(
            replacement_preflight(
                request(
                    NativeRenderTargetReplacementMode::Recovery,
                    (640, 360),
                    (800, 450),
                    false,
                    Some(evidence(1, u64::MAX)),
                ),
                context(7, Some(7), Some(resource)),
            ),
            NativeRenderTargetReplacementOutcome::Deferred
        );
    }

    #[test]
    fn selected_device_and_completion_generation_mismatches_veto() {
        let resource = NativeAdapterGeneration::from_test_serial(1);
        let mismatched_completion = NativeRenderTargetReplacementEvidence::new(
            resource,
            NativeTargetGeneration::from_test_serial(4),
            NativeSubmissionCompletionIdentity::never_submitted(
                NativeAdapterGeneration::from_test_serial(2),
            ),
        );
        for (surface_device_id, selected_device_id, evidence) in [
            (8, Some(7), Some(evidence(1, 4))),
            (7, Some(7), Some(mismatched_completion)),
        ] {
            assert_eq!(
                replacement_preflight(
                    request(
                        NativeRenderTargetReplacementMode::Recovery,
                        (640, 360),
                        (800, 450),
                        false,
                        evidence,
                    ),
                    context(surface_device_id, selected_device_id, Some(resource)),
                ),
                NativeRenderTargetReplacementOutcome::Deferred
            );
        }
    }

    #[test]
    fn requested_rgba8_bytes_are_checked() {
        assert_eq!(requested_rgba8_bytes(640, 360), Some(921_600));
        assert_eq!(requested_rgba8_bytes(0, 360), None);
        assert_eq!(requested_rgba8_bytes(u32::MAX, u32::MAX), None);
    }
}
