//! Exact private admission for one native encode/submit/present operation.
//!
//! CPU frame preparation is allowed to observe newer visual work while it is
//! being assembled.  The native packet is not admitted to WGPU until the
//! resulting snapshot is complete.  This module binds that last irreversible
//! boundary to the shared stage owner, the consuming native packet, and every
//! generation that can make a surface target unsafe.

use super::NativeAdapterGeneration;
use super::NativeLifecycle;
use super::frame_stage_admission::{EncodePresentStageTicket, WindowStageOwner};
use super::native_visual_packet::NativeVisualRequestIdentity;
use super::runner_state::NativeTargetGeneration;
use std::num::NonZeroU64;

/// Checked, non-wrapping identity for the complete CPU/native visual snapshot
/// consumed by one encode/present attempt.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NativeFrameSnapshotRevision(NonZeroU64);

impl NativeFrameSnapshotRevision {
    pub(super) const fn initial() -> Self {
        Self(NonZeroU64::MIN)
    }

    pub(super) fn checked_next(&self) -> Option<Self> {
        self.0
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }

    #[cfg(test)]
    pub(super) const fn serial(&self) -> u64 {
        self.0.get()
    }
}

/// Checked allocator owned by one native runner.  The allocator is separate
/// from packet and stage revisions so a retry of the same packet receives a
/// fresh snapshot witness.
#[derive(Debug)]
pub(super) struct NativeFrameSnapshotRevisionAllocator {
    next: Option<NativeFrameSnapshotRevision>,
}

impl Default for NativeFrameSnapshotRevisionAllocator {
    fn default() -> Self {
        Self {
            next: Some(NativeFrameSnapshotRevision::initial()),
        }
    }
}

impl NativeFrameSnapshotRevisionAllocator {
    pub(super) fn allocate(&mut self) -> Option<NativeFrameSnapshotRevision> {
        let revision = self.next.take()?;
        self.next = revision.checked_next();
        Some(revision)
    }
}

/// The finalized native rendering path included in the exact ticket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeEncodePresentPath {
    DirectResize,
    Composited,
}

/// Copyable identity bound to the admitted encode/present operation. Private
/// upload-plan evidence carries this value so it cannot be confused with work
/// from another packet, target, lifecycle, or snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeEncodePresentPlanContext {
    pub(super) packet: NativeVisualRequestIdentity,
    pub(super) adapter_generation: NativeAdapterGeneration,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) lifecycle: NativeLifecycle,
    pub(super) path: NativeEncodePresentPath,
    pub(super) snapshot_revision: NonZeroU64,
}

/// Owned admission evidence captured immediately before ticket creation.
pub(super) struct NativeEncodePresentAdmission {
    pub(super) packet: NativeVisualRequestIdentity,
    pub(super) adapter_generation: NativeAdapterGeneration,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) lifecycle: NativeLifecycle,
    pub(super) path: NativeEncodePresentPath,
    pub(super) snapshot_revision: NativeFrameSnapshotRevision,
}

/// Borrowed currentness evidence used at each irreversible native boundary.
pub(super) struct NativeEncodePresentCurrentEvidence<'a> {
    pub(super) packet: NativeVisualRequestIdentity,
    pub(super) adapter_generation: NativeAdapterGeneration,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) lifecycle: NativeLifecycle,
    pub(super) path: NativeEncodePresentPath,
    pub(super) snapshot_revision: &'a NativeFrameSnapshotRevision,
}

/// Non-`Clone` exact witness for one encode/submit/present operation.
///
/// The ticket deliberately owns no WGPU handles.  It is only an admission
/// witness and can therefore be checked at each irreversible boundary without
/// extending resource lifetimes across a scheduler yield.
#[derive(Debug)]
pub(super) struct NativeEncodePresentTicket {
    stage: EncodePresentStageTicket,
    packet: NativeVisualRequestIdentity,
    adapter_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    lifecycle: NativeLifecycle,
    path: NativeEncodePresentPath,
    snapshot_revision: NativeFrameSnapshotRevision,
}

impl NativeEncodePresentTicket {
    pub(super) fn new(
        stage: EncodePresentStageTicket,
        admission: NativeEncodePresentAdmission,
    ) -> Self {
        let NativeEncodePresentAdmission {
            packet,
            adapter_generation,
            target_generation,
            lifecycle,
            path,
            snapshot_revision,
        } = admission;
        Self {
            stage,
            packet,
            adapter_generation,
            target_generation,
            lifecycle,
            path,
            snapshot_revision,
        }
    }

    pub(super) fn is_current(
        &self,
        owner: &WindowStageOwner,
        evidence: NativeEncodePresentCurrentEvidence<'_>,
    ) -> bool {
        owner.encode_present_ticket_is_current(&self.stage)
            && self.packet == evidence.packet
            && self.adapter_generation == evidence.adapter_generation
            && self.target_generation == evidence.target_generation
            && self.lifecycle == evidence.lifecycle
            && self.path == evidence.path
            && self.snapshot_revision == *evidence.snapshot_revision
    }

    pub(super) fn snapshot_revision(&self) -> &NativeFrameSnapshotRevision {
        &self.snapshot_revision
    }

    pub(super) fn plan_context(&self) -> NativeEncodePresentPlanContext {
        NativeEncodePresentPlanContext {
            packet: self.packet,
            adapter_generation: self.adapter_generation,
            target_generation: self.target_generation,
            lifecycle: self.lifecycle,
            path: self.path,
            snapshot_revision: self.snapshot_revision.0,
        }
    }

    pub(super) fn into_stage_ticket(self) -> EncodePresentStageTicket {
        self.stage
    }
}

/// Complete the exact admitted ticket.  A wrong ticket leaves the actual
/// owner in flight, which makes accidental fallback/replay impossible.
pub(super) fn complete_native_encode_present(
    owner: &mut WindowStageOwner,
    ticket: NativeEncodePresentTicket,
) -> bool {
    owner.complete_encode_present(ticket.into_stage_ticket())
}

/// Veto the exact admitted ticket without recording success evidence.
pub(super) fn veto_native_encode_present(
    owner: &mut WindowStageOwner,
    ticket: NativeEncodePresentTicket,
) -> bool {
    owner.veto_encode_present(ticket.into_stage_ticket())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::frame_scheduler::FrameScheduleKey;
    use crate::gui_runtime::native_vello::generic_runtime::frame_scheduler_policy::SchedulerStage;
    use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
        NativeVisualRequestAdapter, NativeVisualRequestMailbox,
    };
    use winit::window::WindowId;

    fn identity() -> NativeVisualRequestIdentity {
        let mut mailbox = NativeVisualRequestMailbox::new();
        let window_id = WindowId::dummy();
        assert!(mailbox.bind_window(window_id));
        let _ = mailbox
            .enqueue_for_test(crate::gui_runtime::native_vello::generic_runtime::FrameWork::None);
        match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
            super::super::native_visual_packet::NativeVisualRequestBegin::Requested(packet) => {
                packet.identity()
            }
            other => panic!("unexpected packet begin: {other:?}"),
        }
    }

    #[test]
    fn snapshot_revisions_are_checked_and_non_repeating() {
        let mut allocator = NativeFrameSnapshotRevisionAllocator::default();
        let first = allocator.allocate().expect("first snapshot revision");
        let second = allocator.allocate().expect("second snapshot revision");
        assert_eq!(first.serial(), 1);
        assert_eq!(second.serial(), 2);
        assert_ne!(first, second);
    }

    #[test]
    fn ticket_binds_stage_packet_generations_lifecycle_path_and_snapshot() {
        let packet = identity();
        let adapter = NativeAdapterGeneration::from_test_serial(1);
        let target = NativeTargetGeneration::from_test_serial(1);
        for path in [
            NativeEncodePresentPath::DirectResize,
            NativeEncodePresentPath::Composited,
        ] {
            let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
            let stage = owner
                .admit_encode_present(adapter, target)
                .expect("encode/present stage");
            assert_eq!(stage.identity().stage(), SchedulerStage::EncodePresent);
            let stage_identity = stage.identity().clone();
            let revision = NativeFrameSnapshotRevision::initial();
            let ticket = NativeEncodePresentTicket::new(
                stage,
                NativeEncodePresentAdmission {
                    packet,
                    adapter_generation: adapter,
                    target_generation: target,
                    lifecycle: NativeLifecycle::default(),
                    path,
                    snapshot_revision: revision,
                },
            );
            assert!(ticket.is_current(
                &owner,
                NativeEncodePresentCurrentEvidence {
                    packet,
                    adapter_generation: adapter,
                    target_generation: target,
                    lifecycle: NativeLifecycle::default(),
                    path,
                    snapshot_revision: ticket.snapshot_revision(),
                },
            ));
            assert!(!ticket.is_current(
                &owner,
                NativeEncodePresentCurrentEvidence {
                    packet,
                    adapter_generation: adapter,
                    target_generation: target,
                    lifecycle: NativeLifecycle::default(),
                    path: match path {
                        NativeEncodePresentPath::DirectResize => {
                            NativeEncodePresentPath::Composited
                        }
                        NativeEncodePresentPath::Composited => {
                            NativeEncodePresentPath::DirectResize
                        }
                    },
                    snapshot_revision: ticket.snapshot_revision(),
                },
            ));
            assert!(complete_native_encode_present(&mut owner, ticket));
            assert!(!owner.has_in_flight());
            assert!(owner.stale(&stage_identity));
        }
    }

    #[test]
    fn veto_consumes_encode_present_without_recording_success() {
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let stage = owner
            .admit_encode_present(
                NativeAdapterGeneration::from_test_serial(1),
                NativeTargetGeneration::from_test_serial(1),
            )
            .expect("encode/present stage");
        let stage_identity = stage.identity().clone();
        assert!(owner.veto_encode_present(stage));
        assert!(!owner.has_in_flight());
        assert!(!owner.stale(&stage_identity));
    }

    #[test]
    fn primary_and_auxiliary_stage_owners_share_the_same_ticket_kernel() {
        for key in [
            FrameScheduleKey::Primary,
            FrameScheduleKey::Auxiliary(String::from("auxiliary")),
        ] {
            let mut owner = WindowStageOwner::new(key);
            let stage = owner
                .admit_encode_present(
                    NativeAdapterGeneration::from_test_serial(1),
                    NativeTargetGeneration::from_test_serial(1),
                )
                .expect("encode/present stage");
            assert!(owner.encode_present_ticket_is_current(&stage));
            assert!(owner.complete_encode_present(stage));
            assert!(!owner.has_in_flight());
        }
    }

    #[test]
    fn four_thousand_ninety_six_snapshot_revisions_remain_monotonic() {
        let mut allocator = NativeFrameSnapshotRevisionAllocator::default();
        let mut previous = 0;
        for _ in 0..4096 {
            let revision = allocator.allocate().expect("snapshot revision");
            assert!(revision.serial() > previous);
            previous = revision.serial();
        }
        assert_eq!(previous, 4096);
    }

    #[test]
    fn wrong_ticket_preserves_the_real_stage_owner() {
        let packet = identity();
        let adapter = NativeAdapterGeneration::from_test_serial(1);
        let target = NativeTargetGeneration::from_test_serial(1);
        let mut owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let real_stage = owner
            .admit_encode_present(adapter, target)
            .expect("real stage");
        let mut wrong_owner = WindowStageOwner::new(FrameScheduleKey::Primary);
        let wrong_stage = wrong_owner
            .admit_encode_present(adapter, target)
            .expect("wrong stage");
        let wrong_ticket = NativeEncodePresentTicket::new(
            wrong_stage,
            NativeEncodePresentAdmission {
                packet,
                adapter_generation: adapter,
                target_generation: target,
                lifecycle: NativeLifecycle::default(),
                path: NativeEncodePresentPath::Composited,
                snapshot_revision: NativeFrameSnapshotRevision::initial(),
            },
        );
        assert!(!complete_native_encode_present(&mut owner, wrong_ticket));
        assert!(owner.encode_present_ticket_is_current(&real_stage));
        let real_identity = real_stage.identity().clone();
        assert!(owner.complete_encode_present(real_stage));
        assert!(owner.stale(&real_identity));
    }
}
