//! Transaction-local access to bounded, worker-prepared custom pipelines.

use super::super::custom_shader_prepare::{
    CustomShaderPreparationRequest, CustomShaderTargetId, PreparedCustomShaderPipeline,
};
use super::gpu_surface_types::{CustomShaderPipeline, CustomShaderPipelineIdentity};
use super::resources::CustomShaderFrameRequest;
use super::{GpuSurfaceRenderer, custom_shader::pipeline::CustomShaderPreparationFailure};
use crate::runtime::{GpuSurfaceContent, PaintPrimitive};
use std::collections::HashMap;

pub(in crate::gui_runtime::native_vello::generic_runtime) type CustomShaderPreparationInstall = (
    CustomShaderTargetId,
    CustomShaderPreparationRequest,
    Option<PreparedCustomShaderPipeline>,
    Option<CustomShaderPreparationFailure>,
);

enum Candidate {
    Ready(PreparedCustomShaderPipeline),
    Failed(CustomShaderPreparationFailure),
}

#[derive(Default)]
pub(super) struct CustomShaderPreparationStaging {
    candidates: HashMap<CustomShaderPipelineIdentity, Candidate>,
    targets: HashMap<CustomShaderTargetId, CustomShaderPipelineIdentity>,
    committed: Vec<CustomShaderTargetId>,
}

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn replace_custom_shader_preparations(
        &mut self,
        installs: Vec<CustomShaderPreparationInstall>,
    ) {
        self.custom_shader_preparation_generation =
            self.custom_shader_preparation_generation.wrapping_add(1);
        let staging = &mut self.custom_shader_preparation;
        staging.candidates.clear();
        staging.targets.clear();
        staging.committed.clear();
        let mut text_bytes = 0usize;
        for (target, request, prepared, failure) in installs.into_iter().take(1024) {
            if target.adapter_generation() != request.adapter_generation() {
                continue;
            }
            let candidate = if let Some(prepared) = prepared {
                if !prepared.matches(&request) {
                    continue;
                }
                Candidate::Ready(prepared)
            } else if let Some(failure) = failure {
                Candidate::Failed(failure)
            } else {
                // Pending markers never retain a descriptor or device owner.
                continue;
            };
            let identity = request.identity();
            if !staging.candidates.contains_key(&identity) {
                let next = text_bytes.saturating_add(identity.key.text_bytes());
                if staging.candidates.len() >= 256 || next > 1024 * 1024 {
                    continue;
                }
                text_bytes = next;
                staging.candidates.insert(identity.clone(), candidate);
            }
            // Reuse canonical key text for equivalent target interests.
            if let Some((canonical, _)) = staging.candidates.get_key_value(&identity) {
                staging.targets.insert(target, canonical.clone());
            }
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn has_cached_custom_shader_preparation(
        &self,
        request: &CustomShaderPreparationRequest,
    ) -> bool {
        self.resources
            .has_custom_shader_pipeline_identity(&request.identity())
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn take_committed_custom_shader_targets(
        &mut self,
    ) -> Vec<CustomShaderTargetId> {
        std::mem::take(&mut self.custom_shader_preparation.committed)
    }

    pub(super) fn prepared_custom_shader_pipeline(
        &self,
        identity: &CustomShaderPipelineIdentity,
    ) -> Option<CustomShaderPipeline> {
        match self.custom_shader_preparation.candidates.get(identity)? {
            Candidate::Ready(prepared) => Some(prepared.pipeline().clone()),
            Candidate::Failed(_) => None,
        }
    }

    pub(super) fn custom_shader_preparation_failure(
        &self,
        identity: &CustomShaderPipelineIdentity,
    ) -> Option<CustomShaderPreparationFailure> {
        match self.custom_shader_preparation.candidates.get(identity)? {
            Candidate::Failed(failure) => Some(*failure),
            Candidate::Ready(_) => None,
        }
    }

    pub(super) fn custom_shader_preparation_available(
        &self,
        identity: &CustomShaderPipelineIdentity,
    ) -> bool {
        self.resources.has_custom_shader_pipeline_identity(identity)
            || matches!(
                self.custom_shader_preparation.candidates.get(identity),
                Some(Candidate::Ready(_))
            )
    }

    pub(super) fn custom_shader_frame_preparations_available(
        &self,
        requests: &[CustomShaderFrameRequest],
    ) -> bool {
        requests
            .iter()
            .all(|request| self.custom_shader_preparation_available(&request.identity))
    }

    pub(super) fn commit_custom_shader_preparations(&mut self, primitives: &[PaintPrimitive]) {
        let staging = &mut self.custom_shader_preparation;
        staging.committed.clear();
        for (target, identity) in &staging.targets {
            let Some(PaintPrimitive::GpuSurface(surface)) =
                primitives.get(target.primitive_index())
            else {
                continue;
            };
            if surface.key != target.surface_key()
                || !surface.rect.has_finite_positive_area()
                || !surface.content.is_renderable()
            {
                continue;
            }
            let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
                continue;
            };
            if super::custom_shader::pipeline::custom_shader_pipeline_key(descriptor).as_ref()
                != Some(&identity.key)
            {
                continue;
            }
            // Whole-frame commit proves each eligible ordered surface executed.
            // An earlier duplicate can have been replaced in the final cache;
            // its completed preparation still receives its own receipt.
            staging.committed.push(*target);
        }
        // The broker still retains these objects until the parent consumes the
        // receipts and runs maintenance. Installed caches own GPU handles only.
        for target in &staging.committed {
            staging.targets.remove(target);
        }
        staging
            .candidates
            .retain(|identity, _| staging.targets.values().any(|current| current == identity));
    }
}
