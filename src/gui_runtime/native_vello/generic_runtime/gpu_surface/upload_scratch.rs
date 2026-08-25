use super::atlas::AtlasUploadPreflightState;
use super::custom_shader::CustomShaderUploadPreflightState;
use super::resources::GpuSurfaceResourceFingerprintScratch;
use super::signal::SignalUploadPreflightState;
use super::upload_plan::{GpuSurfaceRenderCanvasUploadAction, GpuSurfaceRenderCanvasUploadPlan};

/// Event-loop-confined storage for one render-canvas upload transaction.
///
/// The action stream is moved into the one-shot plan only while a transaction
/// is in flight. All scaffolding is cleared and returned here before the next
/// transaction starts, retaining its capacity for the renderer's steady state.
#[derive(Default)]
pub(super) struct GpuSurfaceRenderCanvasUploadScratch {
    pub(super) actions: Vec<GpuSurfaceRenderCanvasUploadAction>,
    pub(super) atlas: AtlasUploadPreflightState,
    pub(super) signal: SignalUploadPreflightState,
    pub(super) custom_shader: CustomShaderUploadPreflightState,
    pub(super) fingerprint: GpuSurfaceResourceFingerprintScratch,
}

impl GpuSurfaceRenderCanvasUploadScratch {
    pub(super) fn take_action_stream(&mut self) -> Vec<GpuSurfaceRenderCanvasUploadAction> {
        std::mem::take(&mut self.actions)
    }

    pub(super) fn recycle_plan(&mut self, plan: GpuSurfaceRenderCanvasUploadPlan) {
        debug_assert!(self.actions.is_empty());
        self.actions = plan.into_recyclable_actions();
    }

    #[cfg(test)]
    pub(super) fn capacities(&self) -> UploadScratchCapacities {
        UploadScratchCapacities {
            actions: self.actions.capacity(),
            atlas_textures: self.atlas.textures_capacity(),
            atlas_bindings: self.atlas.composite_bindings_capacity(),
            signal_validations: self.signal.validations_capacity(),
            signal_summaries: self.signal.summaries_capacity(),
            signal_buffers: self.signal.buffers_capacity(),
            signal_bodies: self.signal.bodies_capacity(),
            custom_pipelines: self.custom_shader.pipelines_capacity(),
            custom_bindings: self.custom_shader.bindings_capacity(),
            fingerprint_atlas: self.fingerprint.capacity().0,
            fingerprint_bindings: self.fingerprint.capacity().1,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct UploadScratchCapacities {
    pub(super) actions: usize,
    pub(super) atlas_textures: usize,
    pub(super) atlas_bindings: usize,
    pub(super) signal_validations: usize,
    pub(super) signal_summaries: usize,
    pub(super) signal_buffers: usize,
    pub(super) signal_bodies: usize,
    pub(super) custom_pipelines: usize,
    pub(super) custom_bindings: usize,
    pub(super) fingerprint_atlas: usize,
    pub(super) fingerprint_bindings: usize,
}
