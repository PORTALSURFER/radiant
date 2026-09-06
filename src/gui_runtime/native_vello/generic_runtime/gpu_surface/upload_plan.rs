use super::super::adapter::NativeAdapterGeneration;
use super::super::native_encode_present::NativeEncodePresentPlanContext;
use super::gpu_surface_types::{
    CustomShaderBindingKey, CustomShaderPipelineKey, CustomShaderStaticPayloadKey,
    GpuSurfaceCompositeBindingKey, SignalBodyCacheKey, SignalBufferCacheKey,
};
use super::identity::RenderCanvasContentIdentity;
use vello::wgpu;

/// The target identity carried by one private upload-plan result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct GpuSurfaceRenderCanvasUploadTarget
{
    pub(super) device: usize,
    pub(super) format: wgpu::TextureFormat,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl GpuSurfaceRenderCanvasUploadTarget {
    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn new(
        device: usize,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            device,
            format,
            width,
            height,
        }
    }
}

/// Exact admission/resource/target context for one per-window plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct GpuSurfaceRenderCanvasUploadPlanContext
{
    pub(super) encode_present: NativeEncodePresentPlanContext,
    pub(super) resource_generation: NativeAdapterGeneration,
    pub(super) target: GpuSurfaceRenderCanvasUploadTarget,
}

impl GpuSurfaceRenderCanvasUploadPlanContext {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn new(
        encode_present: NativeEncodePresentPlanContext,
        resource_generation: NativeAdapterGeneration,
        target: GpuSurfaceRenderCanvasUploadTarget,
    ) -> Option<Self> {
        let context = Self {
            encode_present,
            resource_generation,
            target,
        };
        context.is_valid().then_some(context)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn is_valid(self) -> bool {
        self.encode_present.lifecycle.is_running()
            && self.encode_present.adapter_generation.is_known()
            && self.encode_present.target_generation.is_known()
            && self.resource_generation == self.encode_present.adapter_generation
            && self.target.width > 0
            && self.target.height > 0
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn accepts_candidate(self) -> bool {
        self.is_valid()
            && matches!(
                self.encode_present.path,
                super::super::native_encode_present::NativeEncodePresentPath::Composited
            )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadPlanEvidence {
    pub(super) operations: usize,
    pub(super) logical_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct GpuSurfaceRenderCanvasUploadPlanStats
{
    pub(super) immutable_payload: GpuSurfaceRenderCanvasUploadPlanEvidence,
    pub(super) volatile_payload: GpuSurfaceRenderCanvasUploadPlanEvidence,
    pub(super) renderer_parameter: GpuSurfaceRenderCanvasUploadPlanEvidence,
}

impl GpuSurfaceRenderCanvasUploadPlanStats {
    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn values(
        self,
    ) -> [(usize, u64); 3] {
        [
            (
                self.immutable_payload.operations,
                self.immutable_payload.logical_bytes,
            ),
            (
                self.volatile_payload.operations,
                self.volatile_payload.logical_bytes,
            ),
            (
                self.renderer_parameter.operations,
                self.renderer_parameter.logical_bytes,
            ),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum GpuSurfaceRenderCanvasUploadPlanUnavailableReason
{
    Invalid,
    Unsupported,
    Incomplete,
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadPlanResult {
    NoWork,
    Exact(GpuSurfaceRenderCanvasUploadPlanStats),
    Unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadPipeline {
    Composite,
    Signal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadSurface {
    Atlas,
    Signal,
    CustomShader,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadAtlasTextureOperation {
    Reuse,
    Upload {
        revision_mismatch: bool,
        content_mismatch: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadCompositeBindingOperation {
    Reuse,
    Rebuild {
        revision_mismatch: bool,
        content_mismatch: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadSignalValidationOperation {
    Pure,
    CacheHit,
    CacheUpdate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadSignalSummaryOperation {
    Reuse,
    Build,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadSignalBufferOperation {
    Reuse,
    Upload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadSignalBodyOperation {
    Reuse,
    Render,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadPipelineExecution {
    pub(super) pipeline: GpuSurfaceRenderCanvasUploadPipeline,
    pub(super) device: usize,
    pub(super) format: wgpu::TextureFormat,
    pub(super) generation: u64,
    pub(super) rebuild: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadAtlasTextureExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) device: usize,
    pub(super) revision: u64,
    pub(super) content_identity: RenderCanvasContentIdentity,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) extent_width: u32,
    pub(super) extent_height: u32,
    pub(super) bytes_per_row: u32,
    pub(super) byte_len: usize,
    pub(super) operation: GpuSurfaceRenderCanvasUploadAtlasTextureOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadCompositeBindingExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) cache_key: GpuSurfaceCompositeBindingKey,
    pub(super) uniform_byte_len: usize,
    pub(super) operation: GpuSurfaceRenderCanvasUploadCompositeBindingOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadSignalValidationExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) frames: usize,
    pub(super) band_count: usize,
    pub(super) summary: usize,
    pub(super) valid: bool,
    pub(super) operation: GpuSurfaceRenderCanvasUploadSignalValidationOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadSignalSummaryExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) revision: u64,
    pub(super) content_identity: RenderCanvasContentIdentity,
    pub(super) frames: usize,
    pub(super) band_count: usize,
    pub(super) sample_count: usize,
    pub(super) operation: GpuSurfaceRenderCanvasUploadSignalSummaryOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadSignalBufferExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) cache_key: SignalBufferCacheKey,
    pub(super) sample_count: usize,
    pub(super) immutable_byte_len: usize,
    pub(super) renderer_parameter_byte_len: usize,
    pub(super) operation: GpuSurfaceRenderCanvasUploadSignalBufferOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadSignalBodyExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) device: usize,
    pub(super) cache_key: SignalBodyCacheKey,
    pub(super) operation: GpuSurfaceRenderCanvasUploadSignalBodyOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadSurfaceDecision {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) surface: GpuSurfaceRenderCanvasUploadSurface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadCustomPipelineExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) device: usize,
    pub(super) format: wgpu::TextureFormat,
    pub(super) pipeline_key: CustomShaderPipelineKey,
    pub(super) rebuild: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadCustomBindingExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) cache_key: CustomShaderBindingKey,
    pub(super) rebuild: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadCustomStaticStateExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) payload: CustomShaderStaticPayloadKey,
    pub(super) write: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceRenderCanvasUploadCustomPresentationStateExecution {
    pub(super) surface_index: usize,
    pub(super) key: u64,
    pub(super) payload: CustomShaderStaticPayloadKey,
    pub(super) revision: u64,
    pub(super) byte_len: usize,
    pub(super) source: GpuSurfaceRenderCanvasUploadCustomPresentationSource,
    pub(super) write: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadClass {
    ImmutablePayload,
    VolatilePayload,
    RendererParameter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadCustomPresentationSource {
    Initial,
    Update,
}

/// One bounded operation in the complete ordered render-canvas stream.
/// Payloads stay in the immutable paint stream; the plan owns only identities,
/// derived facts, and checked byte counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceRenderCanvasUploadAction {
    BeginFrame,
    CustomShaderTransition {
        requests: Vec<super::resources::CustomShaderFrameRequest>,
    },
    Surface {
        surface_index: usize,
        key: u64,
        surface: GpuSurfaceRenderCanvasUploadSurface,
    },
    Skip {
        surface_index: usize,
        key: u64,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    },
    EnsurePipeline {
        pipeline: GpuSurfaceRenderCanvasUploadPipeline,
        device: usize,
        format: wgpu::TextureFormat,
        generation: u64,
        rebuild: bool,
    },
    AtlasTexture {
        surface_index: usize,
        key: u64,
        device: usize,
        revision: u64,
        content_identity: RenderCanvasContentIdentity,
        width: usize,
        height: usize,
        byte_len: usize,
        extent_width: u32,
        extent_height: u32,
        bytes_per_row: u32,
        operation: GpuSurfaceRenderCanvasUploadAtlasTextureOperation,
    },
    SignalValidation {
        surface_index: usize,
        key: u64,
        frames: usize,
        band_count: usize,
        summary: usize,
        valid: bool,
        operation: GpuSurfaceRenderCanvasUploadSignalValidationOperation,
    },
    SignalSummary {
        surface_index: usize,
        key: u64,
        revision: u64,
        content_identity: RenderCanvasContentIdentity,
        frames: usize,
        band_count: usize,
        sample_count: usize,
        operation: GpuSurfaceRenderCanvasUploadSignalSummaryOperation,
    },
    SignalBuffer {
        surface_index: usize,
        key: u64,
        cache_key: SignalBufferCacheKey,
        sample_count: usize,
        immutable_byte_len: usize,
        renderer_parameter_byte_len: usize,
        operation: GpuSurfaceRenderCanvasUploadSignalBufferOperation,
    },
    SignalBody {
        surface_index: usize,
        key: u64,
        device: usize,
        cache_key: SignalBodyCacheKey,
        operation: GpuSurfaceRenderCanvasUploadSignalBodyOperation,
    },
    CompositeBinding {
        surface_index: usize,
        key: u64,
        cache_key: GpuSurfaceCompositeBindingKey,
        uniform_byte_len: usize,
        operation: GpuSurfaceRenderCanvasUploadCompositeBindingOperation,
    },
    CustomPipeline {
        surface_index: usize,
        key: u64,
        device: usize,
        format: wgpu::TextureFormat,
        pipeline_key: CustomShaderPipelineKey,
        rebuild: bool,
    },
    CustomBinding {
        surface_index: usize,
        key: u64,
        cache_key: CustomShaderBindingKey,
        rebuild: bool,
    },
    Upload {
        surface_index: usize,
        class: GpuSurfaceRenderCanvasUploadClass,
        byte_len: usize,
    },
    CustomStaticState {
        surface_index: usize,
        key: u64,
        payload: CustomShaderStaticPayloadKey,
        write: bool,
    },
    CustomPresentationState {
        surface_index: usize,
        key: u64,
        payload: CustomShaderStaticPayloadKey,
        revision: u64,
        byte_len: usize,
        source: GpuSurfaceRenderCanvasUploadCustomPresentationSource,
        write: bool,
    },
    Activate {
        surface_index: usize,
        key: u64,
    },
    Prune {
        clear: bool,
    },
}

/// Renderer-wide exact preflight and execution witness. It is deliberately
/// non-`Clone`: the action stream can be consumed exactly once.
#[derive(Debug)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct GpuSurfaceRenderCanvasUploadPlan {
    pub(super) context: GpuSurfaceRenderCanvasUploadPlanContext,
    pub(super) result: GpuSurfaceRenderCanvasUploadPlanResult,
    pub(super) actions: Vec<GpuSurfaceRenderCanvasUploadAction>,
    stream_ptr: usize,
    stream_len: usize,
    state_fingerprint: u64,
    action_cursor: usize,
    consumed: bool,
    execution_vetoed: bool,
    execution_mutated: bool,
    atlas_executor_enabled: bool,
}

impl GpuSurfaceRenderCanvasUploadPlan {
    #[cfg(test)]
    pub(super) fn append_action_for_test(&mut self, action: GpuSurfaceRenderCanvasUploadAction) {
        self.actions.push(action);
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn new(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
    ) -> Self {
        Self {
            context,
            result: GpuSurfaceRenderCanvasUploadPlanResult::NoWork,
            actions: Vec::new(),
            stream_ptr: 0,
            stream_len: 0,
            state_fingerprint: 0,
            action_cursor: 0,
            consumed: false,
            execution_vetoed: false,
            execution_mutated: false,
            atlas_executor_enabled: false,
        }
    }

    pub(super) fn enable_atlas_executor(&mut self, enabled: bool) {
        self.atlas_executor_enabled = enabled;
    }

    pub(super) const fn atlas_executor_enabled(&self) -> bool {
        self.atlas_executor_enabled
    }

    pub(super) fn preflight(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        stream_ptr: usize,
        stream_len: usize,
        state_fingerprint: u64,
    ) -> Self {
        let mut plan = Self::new(context);
        plan.stream_ptr = stream_ptr;
        plan.stream_len = stream_len;
        plan.state_fingerprint = state_fingerprint;
        plan
    }

    pub(super) fn preflight_with_actions(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        stream_ptr: usize,
        stream_len: usize,
        state_fingerprint: u64,
        actions: Vec<GpuSurfaceRenderCanvasUploadAction>,
    ) -> Self {
        let mut plan = Self::preflight(context, stream_ptr, stream_len, state_fingerprint);
        plan.actions = actions;
        plan
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn from_observation(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        observation: GpuSurfaceRenderCanvasUploadPlanObservation,
    ) -> Self {
        let mut plan = Self::new(context);
        plan.result = match observation {
            GpuSurfaceRenderCanvasUploadPlanObservation::NoWork => {
                GpuSurfaceRenderCanvasUploadPlanResult::NoWork
            }
            GpuSurfaceRenderCanvasUploadPlanObservation::Exact(stats) => {
                GpuSurfaceRenderCanvasUploadPlanResult::Exact(stats)
            }
            GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason) => {
                GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(reason)
            }
        };
        plan
    }

    pub(super) fn record_observation(
        observation: &mut GpuSurfaceRenderCanvasUploadPlanObservation,
        class: GpuSurfaceRenderCanvasUploadClass,
        byte_len: usize,
    ) {
        if matches!(
            observation,
            GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(_)
        ) {
            return;
        }
        if matches!(
            observation,
            GpuSurfaceRenderCanvasUploadPlanObservation::NoWork
        ) {
            *observation = GpuSurfaceRenderCanvasUploadPlanObservation::Exact(Default::default());
        }
        let GpuSurfaceRenderCanvasUploadPlanObservation::Exact(stats) = observation else {
            return;
        };
        let evidence = match class {
            GpuSurfaceRenderCanvasUploadClass::ImmutablePayload => &mut stats.immutable_payload,
            GpuSurfaceRenderCanvasUploadClass::VolatilePayload => &mut stats.volatile_payload,
            GpuSurfaceRenderCanvasUploadClass::RendererParameter => &mut stats.renderer_parameter,
        };
        if let Err(reason) = update_evidence(evidence, byte_len) {
            *observation = GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason);
        }
    }

    pub(super) fn mark_observation_unavailable(
        observation: &mut GpuSurfaceRenderCanvasUploadPlanObservation,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    ) {
        if !matches!(
            observation,
            GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(_)
        ) {
            *observation = GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason);
        }
    }

    pub(super) fn push_action(&mut self, action: GpuSurfaceRenderCanvasUploadAction) {
        self.actions.push(action);
    }

    pub(super) fn consume_surface_decision(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<
        Result<
            GpuSurfaceRenderCanvasUploadSurfaceDecision,
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
        >,
    > {
        if self.execution_vetoed {
            return None;
        }
        let action = self.actions.get(self.action_cursor)?;
        let decision = match action {
            GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: action_index,
                key: action_key,
                surface,
            } if *action_index == surface_index && *action_key == key => {
                Ok(GpuSurfaceRenderCanvasUploadSurfaceDecision {
                    surface_index: *action_index,
                    key: *action_key,
                    surface: *surface,
                })
            }
            GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index: action_index,
                key: action_key,
                reason,
            } if *action_index == surface_index && *action_key == key => Err(*reason),
            _ => {
                self.mark_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                self.execution_vetoed = true;
                return Some(Err(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                ));
            }
        };
        self.action_cursor += 1;
        Some(decision)
    }

    pub(super) fn consume_pipeline(
        &mut self,
        pipeline: GpuSurfaceRenderCanvasUploadPipeline,
    ) -> Option<GpuSurfaceRenderCanvasUploadPipelineExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
            pipeline: action_pipeline,
            device,
            format,
            generation,
            rebuild,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_pipeline != pipeline {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadPipelineExecution {
            pipeline: *action_pipeline,
            device: *device,
            format: *format,
            generation: *generation,
            rebuild: *rebuild,
        })
    }

    pub(super) fn consume_atlas_texture(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadAtlasTextureExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::AtlasTexture {
            surface_index: action_index,
            key: action_key,
            device,
            revision,
            content_identity,
            width,
            height,
            byte_len,
            extent_width,
            extent_height,
            bytes_per_row,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadAtlasTextureExecution {
            surface_index: *action_index,
            key: *action_key,
            device: *device,
            revision: *revision,
            content_identity: *content_identity,
            width: *width,
            height: *height,
            extent_width: *extent_width,
            extent_height: *extent_height,
            bytes_per_row: *bytes_per_row,
            byte_len: *byte_len,
            operation: *operation,
        })
    }

    pub(super) fn consume_signal_validation(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadSignalValidationExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::SignalValidation {
            surface_index: action_index,
            key: action_key,
            frames,
            band_count,
            summary,
            valid,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadSignalValidationExecution {
            surface_index: *action_index,
            key: *action_key,
            frames: *frames,
            band_count: *band_count,
            summary: *summary,
            valid: *valid,
            operation: *operation,
        })
    }

    pub(super) fn consume_signal_summary(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadSignalSummaryExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::SignalSummary {
            surface_index: action_index,
            key: action_key,
            revision,
            content_identity,
            frames,
            band_count,
            sample_count,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadSignalSummaryExecution {
            surface_index: *action_index,
            key: *action_key,
            revision: *revision,
            content_identity: *content_identity,
            frames: *frames,
            band_count: *band_count,
            sample_count: *sample_count,
            operation: *operation,
        })
    }

    pub(super) fn consume_signal_buffer(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadSignalBufferExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::SignalBuffer {
            surface_index: action_index,
            key: action_key,
            cache_key,
            sample_count,
            immutable_byte_len,
            renderer_parameter_byte_len,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadSignalBufferExecution {
            surface_index: *action_index,
            key: *action_key,
            cache_key: *cache_key,
            sample_count: *sample_count,
            immutable_byte_len: *immutable_byte_len,
            renderer_parameter_byte_len: *renderer_parameter_byte_len,
            operation: *operation,
        })
    }

    pub(super) fn consume_signal_body(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadSignalBodyExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::SignalBody {
            surface_index: action_index,
            key: action_key,
            device,
            cache_key,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadSignalBodyExecution {
            surface_index: *action_index,
            key: *action_key,
            device: *device,
            cache_key: *cache_key,
            operation: *operation,
        })
    }

    pub(super) fn consume_composite_binding(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadCompositeBindingExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::CompositeBinding {
            surface_index: action_index,
            key: action_key,
            cache_key,
            uniform_byte_len,
            operation,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadCompositeBindingExecution {
            surface_index: *action_index,
            key: *action_key,
            cache_key: *cache_key,
            uniform_byte_len: *uniform_byte_len,
            operation: *operation,
        })
    }

    pub(super) fn consume_upload(
        &mut self,
        surface_index: usize,
        class: GpuSurfaceRenderCanvasUploadClass,
    ) -> Option<usize> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: action_index,
            class: action_class,
            byte_len,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_class != class {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(*byte_len)
    }

    pub(super) fn consume_custom_pipeline(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadCustomPipelineExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::CustomPipeline {
            surface_index: action_index,
            key: action_key,
            device,
            format,
            pipeline_key,
            rebuild,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadCustomPipelineExecution {
            surface_index: *action_index,
            key: *action_key,
            device: *device,
            format: *format,
            pipeline_key: pipeline_key.clone(),
            rebuild: *rebuild,
        })
    }

    pub(super) fn consume_custom_binding(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadCustomBindingExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::CustomBinding {
            surface_index: action_index,
            key: action_key,
            cache_key,
            rebuild,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadCustomBindingExecution {
            surface_index: *action_index,
            key: *action_key,
            cache_key: cache_key.clone(),
            rebuild: *rebuild,
        })
    }

    pub(super) fn consume_custom_static_state(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadCustomStaticStateExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::CustomStaticState {
            surface_index: action_index,
            key: action_key,
            payload,
            write,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(GpuSurfaceRenderCanvasUploadCustomStaticStateExecution {
            surface_index: *action_index,
            key: *action_key,
            payload: *payload,
            write: *write,
        })
    }

    pub(super) fn consume_custom_presentation_state(
        &mut self,
        surface_index: usize,
        key: u64,
    ) -> Option<GpuSurfaceRenderCanvasUploadCustomPresentationStateExecution> {
        let action = self.actions.get(self.action_cursor)?;
        let GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
            surface_index: action_index,
            key: action_key,
            payload,
            revision,
            byte_len,
            source,
            write,
        } = action
        else {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        };
        if *action_index != surface_index || *action_key != key {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return None;
        }
        self.action_cursor += 1;
        Some(
            GpuSurfaceRenderCanvasUploadCustomPresentationStateExecution {
                surface_index: *action_index,
                key: *action_key,
                payload: *payload,
                revision: *revision,
                byte_len: *byte_len,
                source: *source,
                write: *write,
            },
        )
    }

    pub(super) fn begin_execution(
        &mut self,
        current: GpuSurfaceRenderCanvasUploadPlanContext,
        stream_ptr: usize,
        stream_len: usize,
        state_fingerprint: u64,
    ) -> bool {
        if self.consumed {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return false;
        }
        self.consumed = true;
        if !self.matches_context(current)
            || self.stream_ptr != stream_ptr
            || self.stream_len != stream_len
            || self.state_fingerprint != state_fingerprint
        {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid);
            self.execution_vetoed = true;
            return false;
        }
        self.actions
            .first()
            .is_some_and(|action| matches!(action, GpuSurfaceRenderCanvasUploadAction::BeginFrame))
    }

    pub(super) fn consume_action(&mut self, expected: GpuSurfaceRenderCanvasUploadAction) -> bool {
        if self.execution_vetoed {
            return false;
        }
        let matches = self
            .actions
            .get(self.action_cursor)
            .is_some_and(|actual| actual == &expected);
        if !matches {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
            return false;
        }
        self.action_cursor += 1;
        true
    }

    pub(super) fn veto_execution(
        &mut self,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    ) {
        self.mark_unavailable(reason);
        self.execution_vetoed = true;
    }

    pub(super) fn mark_execution_mutated(&mut self) {
        self.execution_mutated = true;
    }

    pub(super) const fn execution_mutated(&self) -> bool {
        self.execution_mutated
    }

    pub(super) const fn execution_is_available(&self) -> bool {
        !self.execution_vetoed
    }

    pub(super) fn finish_execution(&mut self) -> bool {
        if !self.execution_vetoed && self.action_cursor != self.actions.len() {
            self.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            self.execution_vetoed = true;
        }
        self.actions.clear();
        !self.execution_vetoed
    }

    pub(super) fn into_recyclable_actions(mut self) -> Vec<GpuSurfaceRenderCanvasUploadAction> {
        self.actions.clear();
        self.actions
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn matches_context(
        &self,
        current: GpuSurfaceRenderCanvasUploadPlanContext,
    ) -> bool {
        self.context.accepts_candidate() && current.accepts_candidate() && self.context == current
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn observation(
        &self,
    ) -> GpuSurfaceRenderCanvasUploadPlanObservation {
        match self.result {
            GpuSurfaceRenderCanvasUploadPlanResult::NoWork => {
                GpuSurfaceRenderCanvasUploadPlanObservation::NoWork
            }
            GpuSurfaceRenderCanvasUploadPlanResult::Exact(stats) => {
                GpuSurfaceRenderCanvasUploadPlanObservation::Exact(stats)
            }
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(reason) => {
                GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason)
            }
        }
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_immutable_payload(
        &mut self,
        byte_len: usize,
    ) {
        self.record(
            GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            byte_len,
        );
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_volatile_payload(
        &mut self,
        byte_len: usize,
    ) {
        self.record(GpuSurfaceRenderCanvasUploadClass::VolatilePayload, byte_len);
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_renderer_parameter(
        &mut self,
        byte_len: usize,
    ) {
        self.record(
            GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len,
        );
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn mark_unavailable(
        &mut self,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    ) {
        if !matches!(
            self.result,
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(_)
        ) {
            self.result = GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(reason);
        }
    }

    #[cfg(test)]
    fn record(&mut self, class: GpuSurfaceRenderCanvasUploadClass, byte_len: usize) {
        if matches!(
            self.result,
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(_)
        ) {
            return;
        }
        if matches!(self.result, GpuSurfaceRenderCanvasUploadPlanResult::NoWork) {
            self.result = GpuSurfaceRenderCanvasUploadPlanResult::Exact(Default::default());
        }
        let outcome = match &mut self.result {
            GpuSurfaceRenderCanvasUploadPlanResult::Exact(stats) => update_evidence(
                match class {
                    GpuSurfaceRenderCanvasUploadClass::ImmutablePayload => {
                        &mut stats.immutable_payload
                    }
                    GpuSurfaceRenderCanvasUploadClass::VolatilePayload => {
                        &mut stats.volatile_payload
                    }
                    GpuSurfaceRenderCanvasUploadClass::RendererParameter => {
                        &mut stats.renderer_parameter
                    }
                },
                byte_len,
            ),
            GpuSurfaceRenderCanvasUploadPlanResult::NoWork
            | GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(_) => Ok(()),
        };
        if let Err(reason) = outcome {
            self.mark_unavailable(reason);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum GpuSurfaceRenderCanvasUploadPlanObservation
{
    NoWork,
    Exact(GpuSurfaceRenderCanvasUploadPlanStats),
    Unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason),
}

fn update_evidence(
    evidence: &mut GpuSurfaceRenderCanvasUploadPlanEvidence,
    byte_len: usize,
) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
    let byte_len = u64::try_from(byte_len)
        .map_err(|_| GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
    evidence.operations = evidence
        .operations
        .checked_add(1)
        .ok_or(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
    evidence.logical_bytes = evidence
        .logical_bytes
        .checked_add(byte_len)
        .ok_or(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::Arc;

    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::FrameWork;
    use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;
    use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
    use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::GpuSurfaceRenderCanvasUploadStats;
    use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::NativeEncodePresentPath;
    use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
        NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
    };
    use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;
    use winit::window::WindowId;

    fn encode_present_context() -> NativeEncodePresentPlanContext {
        let mut mailbox = NativeVisualRequestMailbox::new();
        let window_id = WindowId::dummy();
        assert!(mailbox.bind_window(window_id));
        let _ = mailbox.enqueue_for_test(FrameWork::None);
        let packet = match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet.identity(),
            other => panic!("unexpected packet begin: {other:?}"),
        };
        NativeEncodePresentPlanContext {
            packet,
            adapter_generation: NativeAdapterGeneration::from_test_serial(1),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            lifecycle: NativeLifecycle::default(),
            path: NativeEncodePresentPath::Composited,
            snapshot_revision: NonZeroU64::MIN,
        }
    }

    fn plan() -> GpuSurfaceRenderCanvasUploadPlan {
        let context = GpuSurfaceRenderCanvasUploadPlanContext::new(
            encode_present_context(),
            NativeAdapterGeneration::from_test_serial(1),
            GpuSurfaceRenderCanvasUploadTarget::new(1, wgpu::TextureFormat::Rgba8Unorm, 64, 32),
        )
        .expect("valid upload-plan context");
        GpuSurfaceRenderCanvasUploadPlan::new(context)
    }

    fn assert_exact_matches_actual(
        plan: GpuSurfaceRenderCanvasUploadPlan,
        actual: GpuSurfaceRenderCanvasUploadStats,
    ) {
        let GpuSurfaceRenderCanvasUploadPlanResult::Exact(expected) = plan.result else {
            panic!("expected exact plan, got {:?}", plan.result);
        };
        assert_eq!(
            (
                Some(expected.immutable_payload.operations),
                Some(expected.immutable_payload.logical_bytes)
            ),
            (
                actual.immutable_payload.operations,
                actual.immutable_payload.logical_bytes
            )
        );
        assert_eq!(
            (
                Some(expected.volatile_payload.operations),
                Some(expected.volatile_payload.logical_bytes)
            ),
            (
                actual.volatile_payload.operations,
                actual.volatile_payload.logical_bytes
            )
        );
        assert_eq!(
            (
                Some(expected.renderer_parameter.operations),
                Some(expected.renderer_parameter.logical_bytes)
            ),
            (
                actual.renderer_parameter.operations,
                actual.renderer_parameter.logical_bytes
            )
        );
    }

    #[test]
    fn cold_and_warm_atlas_fixtures_match_actual_classification() {
        let mut cold_plan = plan();
        let mut cold_actual = GpuSurfaceRenderCanvasUploadStats::default();
        cold_plan.record_immutable_payload(64);
        cold_actual.record_immutable_payload(64);
        cold_plan.record_renderer_parameter(240);
        cold_actual.record_renderer_parameter(240);
        assert_exact_matches_actual(cold_plan, cold_actual);

        let mut warm_plan = plan();
        let mut warm_actual = GpuSurfaceRenderCanvasUploadStats::default();
        warm_plan.record_renderer_parameter(240);
        warm_actual.record_renderer_parameter(240);
        assert_exact_matches_actual(warm_plan, warm_actual);
    }

    #[test]
    fn cold_and_warm_signal_fixtures_match_actual_classification() {
        let mut cold_plan = plan();
        let mut cold_actual = GpuSurfaceRenderCanvasUploadStats::default();
        cold_plan.record_immutable_payload(128);
        cold_actual.record_immutable_payload(128);
        cold_plan.record_renderer_parameter(144);
        cold_actual.record_renderer_parameter(144);
        cold_plan.record_renderer_parameter(240);
        cold_actual.record_renderer_parameter(240);
        assert_exact_matches_actual(cold_plan, cold_actual);

        let mut warm_plan = plan();
        let mut warm_actual = GpuSurfaceRenderCanvasUploadStats::default();
        warm_plan.record_renderer_parameter(144);
        warm_actual.record_renderer_parameter(144);
        warm_plan.record_renderer_parameter(240);
        warm_actual.record_renderer_parameter(240);
        assert_exact_matches_actual(warm_plan, warm_actual);
    }

    #[test]
    fn cold_and_warm_custom_shader_fixtures_match_actual_classification() {
        let mut cold_plan = plan();
        let mut cold_actual = GpuSurfaceRenderCanvasUploadStats::default();
        cold_plan.record_renderer_parameter(240);
        cold_actual.record_renderer_parameter(240);
        cold_plan.record_immutable_payload(16);
        cold_actual.record_immutable_payload(16);
        cold_plan.record_immutable_payload(32);
        cold_actual.record_immutable_payload(32);
        cold_plan.record_volatile_payload(12);
        cold_actual.record_volatile_payload(12);
        assert_exact_matches_actual(cold_plan, cold_actual);

        let mut warm_plan = plan();
        let mut warm_actual = GpuSurfaceRenderCanvasUploadStats::default();
        warm_plan.record_renderer_parameter(240);
        warm_actual.record_renderer_parameter(240);
        assert_exact_matches_actual(warm_plan, warm_actual);
    }

    #[test]
    fn mixed_surface_fixture_matches_actual_classification() {
        let mut candidate = plan();
        let mut actual = GpuSurfaceRenderCanvasUploadStats::default();
        for (class, byte_len) in [
            (0, 64),
            (1, 240),
            (2, 128),
            (1, 144),
            (1, 240),
            (0, 16),
            (2, 240),
        ] {
            match class {
                0 => {
                    candidate.record_immutable_payload(byte_len);
                    actual.record_immutable_payload(byte_len);
                }
                1 => {
                    candidate.record_renderer_parameter(byte_len);
                    actual.record_renderer_parameter(byte_len);
                }
                2 => {
                    candidate.record_volatile_payload(byte_len);
                    actual.record_volatile_payload(byte_len);
                }
                _ => unreachable!(),
            }
        }
        assert_exact_matches_actual(candidate, actual);
    }

    #[test]
    fn no_work_unavailable_and_overflow_are_typed_and_sticky() {
        let mut no_work = plan();
        assert_eq!(
            no_work.result,
            GpuSurfaceRenderCanvasUploadPlanResult::NoWork
        );
        no_work.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported);
        no_work.record_renderer_parameter(240);
        assert_eq!(
            no_work.result,
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported
            )
        );

        let mut overflow = plan();
        overflow.result =
            GpuSurfaceRenderCanvasUploadPlanResult::Exact(GpuSurfaceRenderCanvasUploadPlanStats {
                immutable_payload: GpuSurfaceRenderCanvasUploadPlanEvidence {
                    operations: usize::MAX,
                    logical_bytes: 0,
                },
                ..Default::default()
            });
        overflow.record_immutable_payload(1);
        assert_eq!(
            overflow.result,
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow
            )
        );
        overflow.record_renderer_parameter(1);
        assert_eq!(
            overflow.result,
            GpuSurfaceRenderCanvasUploadPlanResult::Unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow
            )
        );
    }

    #[test]
    fn invalid_context_publishes_no_plan() {
        let valid = encode_present_context();
        let target =
            GpuSurfaceRenderCanvasUploadTarget::new(1, wgpu::TextureFormat::Rgba8Unorm, 64, 32);
        assert!(
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                valid,
                NativeAdapterGeneration::from_test_serial(1),
                target,
            )
            .is_some()
        );
        assert!(
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                valid,
                NativeAdapterGeneration::from_test_serial(2),
                target,
            )
            .is_none()
        );
        assert!(
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                NativeEncodePresentPlanContext {
                    lifecycle: NativeLifecycle::Stopped,
                    ..valid
                },
                NativeAdapterGeneration::from_test_serial(1),
                target,
            )
            .is_none()
        );
        assert!(
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                NativeEncodePresentPlanContext {
                    adapter_generation: NativeAdapterGeneration::default(),
                    ..valid
                },
                NativeAdapterGeneration::default(),
                target,
            )
            .is_none()
        );
        assert!(
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                valid,
                NativeAdapterGeneration::from_test_serial(1),
                GpuSurfaceRenderCanvasUploadTarget::new(1, target.format, 0, target.height),
            )
            .is_none()
        );
    }

    #[test]
    fn preflight_plan_consumes_ordered_actions_once() {
        let initial = plan();
        let context = initial.context;
        let stream = [0_u8; 2];
        let mut preflight = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            17,
        );
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
            pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
            device: context.target.device,
            format: context.target.format,
            generation: 3,
            rebuild: true,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false });

        assert!(preflight.begin_execution(context, stream.as_ptr() as usize, 2, 17));
        assert!(preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        assert!(
            preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
                pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
                device: context.target.device,
                format: context.target.format,
                generation: 3,
                rebuild: true,
            })
        );
        assert!(
            preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false })
        );
        preflight.finish_execution();
        assert!(preflight.actions.is_empty());

        assert!(!preflight.begin_execution(context, stream.as_ptr() as usize, 2, 17));
        assert_eq!(
            preflight.observation(),
            GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete
            )
        );
    }

    #[test]
    fn preflight_context_or_stream_drift_vetoes_before_action_consumption() {
        let initial = plan();
        let context = initial.context;
        let stream = [0_u8; 1];
        let mut preflight = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            23,
        );
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);

        assert!(!preflight.begin_execution(context, stream.as_ptr() as usize, 2, 23));
        assert_eq!(preflight.action_cursor, 0);
        assert_eq!(preflight.actions.len(), 1);
        assert_eq!(
            preflight.observation(),
            GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid
            )
        );
        assert!(!preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
    }

    #[test]
    fn custom_shader_actions_are_consumed_once_in_mutation_order() {
        let initial = plan();
        let context = initial.context;
        let stream = [0_u8; 1];
        let pipeline_key = CustomShaderPipelineKey {
            shader_key: Arc::from("test/custom-shader"),
            wgsl_source: Arc::<str>::from("shader"),
            vertex_entry_point: Arc::from("vertex_main"),
            fragment_entry_point: Arc::from("fragment_main"),
            has_uniform_payload: true,
            has_storage_payload: false,
            has_presentation_uniform_payload: true,
        };
        let binding_key = CustomShaderBindingKey {
            pipeline_key: pipeline_key.clone(),
            uniform_bytes_len: 4,
            storage_bytes_len: 0,
            presentation_uniform_bytes_len: 4,
        };
        let payload = CustomShaderStaticPayloadKey::new(7, 11, 4, 0);
        let mut preflight = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            17,
        );
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index: 0,
            key: 41,
            surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::CustomPipeline {
            surface_index: 0,
            key: 41,
            device: context.target.device,
            format: context.target.format,
            pipeline_key: pipeline_key.clone(),
            rebuild: true,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::CustomBinding {
            surface_index: 0,
            key: 41,
            cache_key: binding_key,
            rebuild: true,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: 240,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::CustomStaticState {
            surface_index: 0,
            key: 41,
            payload,
            write: true,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            byte_len: 4,
        });
        preflight.push_action(
            GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                surface_index: 0,
                key: 41,
                payload,
                revision: 3,
                byte_len: 4,
                source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial,
                write: true,
            },
        );
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
            byte_len: 4,
        });
        preflight.push_action(
            GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                surface_index: 0,
                key: 41,
                payload,
                revision: 4,
                byte_len: 4,
                source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update,
                write: true,
            },
        );
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
            byte_len: 4,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
            surface_index: 0,
            key: 41,
        });
        preflight.push_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false });

        assert!(preflight.begin_execution(context, stream.as_ptr() as usize, 1, 17));
        assert!(preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        assert_eq!(
            preflight
                .consume_surface_decision(0, 41)
                .expect("surface decision")
                .expect("custom surface should be admitted")
                .surface,
            GpuSurfaceRenderCanvasUploadSurface::CustomShader
        );
        assert!(
            preflight
                .consume_custom_pipeline(0, 41)
                .expect("pipeline")
                .rebuild
        );
        assert!(
            preflight
                .consume_custom_binding(0, 41)
                .expect("binding")
                .rebuild
        );
        assert_eq!(
            preflight.consume_upload(0, GpuSurfaceRenderCanvasUploadClass::RendererParameter),
            Some(240)
        );
        assert!(
            preflight
                .consume_custom_static_state(0, 41)
                .expect("static")
                .write
        );
        assert_eq!(
            preflight.consume_upload(0, GpuSurfaceRenderCanvasUploadClass::ImmutablePayload),
            Some(4)
        );
        assert_eq!(
            preflight
                .consume_custom_presentation_state(0, 41)
                .expect("initial presentation")
                .source,
            GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial
        );
        assert_eq!(
            preflight.consume_upload(0, GpuSurfaceRenderCanvasUploadClass::VolatilePayload),
            Some(4)
        );
        assert_eq!(
            preflight
                .consume_custom_presentation_state(0, 41)
                .expect("update presentation")
                .source,
            GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update
        );
        assert_eq!(
            preflight.consume_upload(0, GpuSurfaceRenderCanvasUploadClass::VolatilePayload),
            Some(4)
        );
        assert!(
            preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: 0,
                key: 41,
            })
        );
        assert!(
            preflight.consume_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false })
        );
        assert!(preflight.finish_execution());
        assert!(preflight.actions.is_empty());
        assert!(!preflight.begin_execution(context, stream.as_ptr() as usize, 1, 17));
    }
}
