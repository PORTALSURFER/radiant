use super::super::adapter::NativeAdapterGeneration;
use super::super::native_encode_present::NativeEncodePresentPlanContext;
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

/// Candidate work observed immediately before the existing native write
/// predicates execute. A later ticket veto drops this value with the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct GpuSurfaceRenderCanvasUploadPlan {
    pub(super) context: GpuSurfaceRenderCanvasUploadPlanContext,
    pub(super) result: GpuSurfaceRenderCanvasUploadPlanResult,
}

#[derive(Clone, Copy)]
enum GpuSurfaceRenderCanvasUploadClass {
    ImmutablePayload,
    VolatilePayload,
    RendererParameter,
}

impl GpuSurfaceRenderCanvasUploadPlan {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn new(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
    ) -> Self {
        Self {
            context,
            result: GpuSurfaceRenderCanvasUploadPlanResult::NoWork,
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn matches_context(
        self,
        current: GpuSurfaceRenderCanvasUploadPlanContext,
    ) -> bool {
        self.context.accepts_candidate() && current.accepts_candidate() && self.context == current
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) const fn observation(
        self,
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

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_immutable_payload(
        &mut self,
        byte_len: usize,
    ) {
        self.record(
            GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            byte_len,
        );
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn record_volatile_payload(
        &mut self,
        byte_len: usize,
    ) {
        self.record(GpuSurfaceRenderCanvasUploadClass::VolatilePayload, byte_len);
    }

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
}
