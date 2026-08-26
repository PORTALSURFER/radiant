use super::super::adapter::NativeAdapterGeneration;
use super::super::device::wgpu_device_id;
use super::super::runner_state::NativeTargetGeneration;
use super::super::submission_completion::NativeSubmissionCompletionIdentity;
use vello::wgpu;

pub(in crate::gui_runtime::native_vello::generic_runtime) struct CompositedBaseFrame {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    device: usize,
    adapter_generation: NativeAdapterGeneration,
    resource_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
}

/// The exact predecessor identity retained until its bundle's completion
/// witness proves that the old sampled texture is no longer in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct CompositedBaseFrameRetirementIdentity
{
    descriptor: CompositedBaseFrameDescriptor,
    completion: NativeSubmissionCompletionIdentity,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) struct CompositedBaseFrameRetirement {
    frame: CompositedBaseFrame,
    identity: CompositedBaseFrameRetirementIdentity,
}

impl CompositedBaseFrameRetirement {
    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn identity(
        &self,
    ) -> CompositedBaseFrameRetirementIdentity {
        self.identity
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn requested_backing_bytes(
        &self,
    ) -> Option<u64> {
        self.frame.requested_backing_bytes()
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime) struct CompositedBaseFrameEnsureRequest<
    'a,
> {
    pub(super) device: &'a wgpu::Device,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) format: wgpu::TextureFormat,
    pub(super) adapter_generation: NativeAdapterGeneration,
    pub(super) resource_generation: NativeAdapterGeneration,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) target_fenced: bool,
    pub(super) completion_identity: Option<NativeSubmissionCompletionIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum CompositedBaseFrameEnsureOutcome {
    Reused,
    Created,
    Vetoed,
}

impl CompositedBaseFrameEnsureRequest<'_> {
    fn context(&self) -> CompositedBaseFrameEnsureContext {
        let descriptor = composited_base_frame_descriptor(CompositedBaseFrameDescriptorRequest {
            device: wgpu_device_id(self.device),
            width: self.width,
            height: self.height,
            format: self.format,
            adapter_generation: self.adapter_generation,
            resource_generation: self.resource_generation,
            target_generation: self.target_generation,
            target_fenced: self.target_fenced,
        });
        CompositedBaseFrameEnsureContext {
            descriptor,
            completion_identity: self.completion_identity,
        }
    }
}

impl CompositedBaseFrame {
    pub(super) fn ensure(
        frame: &mut Option<Self>,
        retired: &mut Option<CompositedBaseFrameRetirement>,
        request: CompositedBaseFrameEnsureRequest<'_>,
    ) -> CompositedBaseFrameEnsureOutcome {
        let context = request.context();
        let decision = composited_base_frame_ensure_decision(
            frame.as_ref().map(Self::descriptor),
            retired.is_some(),
            context,
        );
        match decision {
            CompositedBaseFrameEnsureDecision::Vetoed => CompositedBaseFrameEnsureOutcome::Vetoed,
            CompositedBaseFrameEnsureDecision::Reused => CompositedBaseFrameEnsureOutcome::Reused,
            CompositedBaseFrameEnsureDecision::Create => {
                let Some(descriptor) = context.descriptor else {
                    return CompositedBaseFrameEnsureOutcome::Vetoed;
                };
                *frame = Some(Self::new(request.device, descriptor));
                CompositedBaseFrameEnsureOutcome::Created
            }
            CompositedBaseFrameEnsureDecision::Replace => {
                let Some(descriptor) = context.descriptor else {
                    return CompositedBaseFrameEnsureOutcome::Vetoed;
                };
                let Some(completion) = context.completion_identity else {
                    return CompositedBaseFrameEnsureOutcome::Vetoed;
                };
                let successor = Self::new(request.device, descriptor);
                let Some(predecessor) = frame.take() else {
                    return CompositedBaseFrameEnsureOutcome::Vetoed;
                };
                let predecessor_descriptor = predecessor.descriptor();
                *retired = Some(CompositedBaseFrameRetirement {
                    frame: predecessor,
                    identity: CompositedBaseFrameRetirementIdentity {
                        descriptor: predecessor_descriptor,
                        completion,
                    },
                });
                *frame = Some(successor);
                CompositedBaseFrameEnsureOutcome::Created
            }
        }
    }

    fn new(device: &wgpu::Device, descriptor: CompositedBaseFrameDescriptor) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_composited_base_frame"),
            size: wgpu::Extent3d {
                width: descriptor.width,
                height: descriptor.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: descriptor.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            width: descriptor.width,
            height: descriptor.height,
            format: descriptor.format,
            device: descriptor.device,
            adapter_generation: descriptor.adapter_generation,
            resource_generation: descriptor.resource_generation,
            target_generation: descriptor.target_generation,
        }
    }

    fn descriptor(&self) -> CompositedBaseFrameDescriptor {
        CompositedBaseFrameDescriptor {
            device: self.device,
            width: self.width,
            height: self.height,
            format: self.format,
            adapter_generation: self.adapter_generation,
            resource_generation: self.resource_generation,
            target_generation: self.target_generation,
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn requested_backing_bytes(
        &self,
    ) -> Option<u64> {
        self.descriptor().requested_backing_bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompositedBaseFrameDescriptor {
    device: usize,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    adapter_generation: NativeAdapterGeneration,
    resource_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
}

impl CompositedBaseFrameDescriptor {
    fn requested_backing_bytes(self) -> Option<u64> {
        let (block_width, block_height) = self.format.block_dimensions();
        let block_size = self.format.block_copy_size(None)?;
        let width_blocks = checked_block_count(self.width, block_width)?;
        let height_blocks = checked_block_count(self.height, block_height)?;
        let bytes = width_blocks
            .checked_mul(height_blocks)?
            .checked_mul(u64::from(block_size))?;
        (bytes > 0).then_some(bytes)
    }
}

struct CompositedBaseFrameDescriptorRequest {
    device: usize,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    adapter_generation: NativeAdapterGeneration,
    resource_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    target_fenced: bool,
}

fn composited_base_frame_descriptor(
    request: CompositedBaseFrameDescriptorRequest,
) -> Option<CompositedBaseFrameDescriptor> {
    if request.target_fenced
        || !request.adapter_generation.is_known()
        || request.adapter_generation != request.resource_generation
        || !request.resource_generation.is_known()
        || !request.target_generation.is_known()
    {
        return None;
    }
    Some(CompositedBaseFrameDescriptor {
        device: request.device,
        width: request.width.max(1),
        height: request.height.max(1),
        format: request.format,
        adapter_generation: request.adapter_generation,
        resource_generation: request.resource_generation,
        target_generation: request.target_generation,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompositedBaseFrameEnsureContext {
    descriptor: Option<CompositedBaseFrameDescriptor>,
    completion_identity: Option<NativeSubmissionCompletionIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositedBaseFrameEnsureDecision {
    Reused,
    Create,
    Replace,
    Vetoed,
}

fn composited_base_frame_ensure_decision(
    current: Option<CompositedBaseFrameDescriptor>,
    retired_occupied: bool,
    context: CompositedBaseFrameEnsureContext,
) -> CompositedBaseFrameEnsureDecision {
    let Some(target) = context.descriptor else {
        return CompositedBaseFrameEnsureDecision::Vetoed;
    };
    if current == Some(target) {
        return CompositedBaseFrameEnsureDecision::Reused;
    }
    if retired_occupied
        || current.is_some_and(|current| {
            current.adapter_generation != target.adapter_generation
                || current.resource_generation != target.resource_generation
        })
    {
        return CompositedBaseFrameEnsureDecision::Vetoed;
    }
    let Some(completion) = context.completion_identity else {
        return CompositedBaseFrameEnsureDecision::Vetoed;
    };
    if !completion.is_valid_for_retirement()
        || completion.generation() != target.resource_generation
    {
        return CompositedBaseFrameEnsureDecision::Vetoed;
    }
    if current.is_some() {
        CompositedBaseFrameEnsureDecision::Replace
    } else {
        CompositedBaseFrameEnsureDecision::Create
    }
}

fn checked_block_count(size: u32, block_size: u32) -> Option<u64> {
    if block_size == 0 {
        return None;
    }
    let size = u64::from(size.max(1));
    let block_size = u64::from(block_size);
    size.checked_add(block_size.checked_sub(1)?)?
        .checked_div(block_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;

    fn descriptor(
        device: usize,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        generation: u64,
        target: u64,
    ) -> CompositedBaseFrameDescriptor {
        CompositedBaseFrameDescriptor {
            device,
            width,
            height,
            format,
            adapter_generation: NativeAdapterGeneration::from_test_serial(generation),
            resource_generation: NativeAdapterGeneration::from_test_serial(generation),
            target_generation: NativeTargetGeneration::from_test_serial(target),
        }
    }

    fn context(
        descriptor: Option<CompositedBaseFrameDescriptor>,
        completion_generation: Option<u64>,
    ) -> CompositedBaseFrameEnsureContext {
        CompositedBaseFrameEnsureContext {
            descriptor,
            completion_identity: completion_generation.map(|generation| {
                NativeSubmissionCompletionIdentity::never_submitted(
                    NativeAdapterGeneration::from_test_serial(generation),
                )
            }),
        }
    }

    #[test]
    fn composited_base_frame_matches_full_surface_descriptor() {
        let descriptor = descriptor(7, 640, 360, wgpu::TextureFormat::Bgra8Unorm, 1, 1);
        assert_eq!(descriptor, descriptor);
        assert_ne!(
            descriptor,
            CompositedBaseFrameDescriptor {
                device: 8,
                ..descriptor
            }
        );
        assert_ne!(
            descriptor,
            CompositedBaseFrameDescriptor {
                target_generation: NativeTargetGeneration::from_test_serial(2),
                ..descriptor
            }
        );
        assert_ne!(
            descriptor,
            CompositedBaseFrameDescriptor {
                resource_generation: NativeAdapterGeneration::from_test_serial(2),
                ..descriptor
            }
        );
    }

    #[test]
    fn valid_first_target_creates_and_identical_target_reuses_without_creation() {
        let first = descriptor(7, 640, 360, wgpu::TextureFormat::Rgba8Unorm, 1, 1);
        assert_eq!(
            composited_base_frame_ensure_decision(None, false, context(Some(first), Some(1))),
            CompositedBaseFrameEnsureDecision::Create
        );
        assert_eq!(
            composited_base_frame_ensure_decision(Some(first), false, context(Some(first), None)),
            CompositedBaseFrameEnsureDecision::Reused
        );
    }

    #[test]
    fn changed_target_replaces_only_with_free_retirement_capacity_and_exact_witness() {
        let previous = descriptor(7, 640, 360, wgpu::TextureFormat::Rgba8Unorm, 1, 1);
        let successor = CompositedBaseFrameDescriptor {
            width: 800,
            target_generation: NativeTargetGeneration::from_test_serial(2),
            ..previous
        };
        assert_eq!(
            composited_base_frame_ensure_decision(
                Some(previous),
                false,
                context(Some(successor), Some(1))
            ),
            CompositedBaseFrameEnsureDecision::Replace
        );
        assert_eq!(
            composited_base_frame_ensure_decision(
                Some(previous),
                true,
                context(Some(successor), Some(1))
            ),
            CompositedBaseFrameEnsureDecision::Vetoed
        );
        assert_eq!(
            composited_base_frame_ensure_decision(
                Some(previous),
                false,
                context(Some(successor), None)
            ),
            CompositedBaseFrameEnsureDecision::Vetoed
        );
    }

    #[test]
    fn stale_wrong_unknown_and_exhausted_generation_evidence_is_vetoed() {
        let previous = descriptor(7, 640, 360, wgpu::TextureFormat::Rgba8Unorm, 1, 1);
        let successor = CompositedBaseFrameDescriptor {
            target_generation: NativeTargetGeneration::from_test_serial(2),
            ..previous
        };
        assert_eq!(
            composited_base_frame_ensure_decision(
                Some(previous),
                false,
                context(Some(successor), Some(2))
            ),
            CompositedBaseFrameEnsureDecision::Vetoed
        );
        assert_eq!(
            composited_base_frame_ensure_decision(Some(previous), false, context(None, Some(1))),
            CompositedBaseFrameEnsureDecision::Vetoed
        );

        assert_eq!(
            composited_base_frame_descriptor(CompositedBaseFrameDescriptorRequest {
                device: 7,
                width: successor.width,
                height: successor.height,
                format: successor.format,
                adapter_generation: NativeAdapterGeneration::unknown(),
                resource_generation: successor.resource_generation,
                target_generation: successor.target_generation,
                target_fenced: false,
            }),
            None
        );

        let mut exhausted = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted.advance());
        let exhausted_completion = NativeSubmissionCompletionIdentity::never_submitted(exhausted);
        assert!(!exhausted_completion.is_valid_for_retirement());
    }

    #[test]
    fn requested_backing_footprint_is_checked_and_never_zero() {
        assert_eq!(
            descriptor(7, 640, 360, wgpu::TextureFormat::Rgba8Unorm, 1, 1)
                .requested_backing_bytes(),
            Some(921_600)
        );
        assert_eq!(
            descriptor(7, 5, 5, wgpu::TextureFormat::Bc1RgbaUnorm, 1, 1).requested_backing_bytes(),
            Some(32)
        );
        assert_eq!(
            descriptor(7, 0, 0, wgpu::TextureFormat::Rgba8Unorm, 1, 1).requested_backing_bytes(),
            Some(4)
        );
        assert_eq!(
            descriptor(7, 1, 1, wgpu::TextureFormat::Depth24Plus, 1, 1).requested_backing_bytes(),
            None
        );
        assert_eq!(
            descriptor(7, u32::MAX, u32::MAX, wgpu::TextureFormat::Rgba8Unorm, 1, 1)
                .requested_backing_bytes(),
            None
        );
    }

    #[test]
    fn retirement_identity_binds_predecessor_descriptor_and_completion() {
        let descriptor = descriptor(7, 640, 360, wgpu::TextureFormat::Rgba8Unorm, 1, 1);
        let identity = CompositedBaseFrameRetirementIdentity {
            descriptor,
            completion: NativeSubmissionCompletionIdentity::never_submitted(
                NativeAdapterGeneration::from_test_serial(1),
            ),
        };
        assert_eq!(identity.descriptor, descriptor);
        assert_ne!(
            identity,
            CompositedBaseFrameRetirementIdentity {
                completion: NativeSubmissionCompletionIdentity::never_submitted(
                    NativeAdapterGeneration::from_test_serial(2),
                ),
                ..identity
            }
        );
    }
}
