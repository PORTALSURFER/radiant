//! Crate-private native visual request handoff.
//!
//! Winit redraw delivery is an asynchronous boundary.  The mailbox keeps the
//! exact request that crossed that boundary separate from the frame work that
//! the runtime observes while the redraw is executing.  It is deliberately a
//! small, per-window state machine: one request is either outstanding or
//! consuming, and only the newest request may wait behind it.

use super::FrameWork;
use std::num::NonZeroU64;
use winit::window::{Window, WindowId};

/// The maximum number of packet owners retained by one window mailbox.
///
/// The requested and consuming states are mutually exclusive.  A consuming
/// packet may retain one newest pending successor, and a same-packet retry may
/// retain that successor beside the reissued requested packet.
pub(super) const NATIVE_VISUAL_MAILBOX_MAX_RETAINED_DEPTH: usize = 2;
const _: [(); NATIVE_VISUAL_MAILBOX_MAX_RETAINED_DEPTH] = [(); 2];

/// A private checked generation for packets owned by one live native window.
///
/// `NonZeroU64` makes zero unrepresentable.  `checked_next` is the only
/// production advancement path, so exhaustion fails closed instead of
/// wrapping or reusing an earlier owner identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeVisualOwnerGeneration(NonZeroU64);

impl NativeVisualOwnerGeneration {
    const fn initial() -> Self {
        Self(NonZeroU64::MIN)
    }

    fn checked_next(self) -> Option<Self> {
        self.0
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }

    #[cfg(test)]
    fn from_test_serial(serial: u64) -> Self {
        let Some(serial) = NonZeroU64::new(serial) else {
            return Self::initial();
        };
        Self(serial)
    }

    #[cfg(test)]
    const fn serial(self) -> u64 {
        self.0.get()
    }
}

/// A private checked monotonic revision for one native visual request stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeVisualRevision(NonZeroU64);

impl NativeVisualRevision {
    const fn initial() -> Self {
        Self(NonZeroU64::MIN)
    }

    fn checked_next(self) -> Option<Self> {
        self.0
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }

    #[cfg(test)]
    fn from_test_serial(serial: u64) -> Self {
        let Some(serial) = NonZeroU64::new(serial) else {
            return Self::initial();
        };
        Self(serial)
    }

    #[cfg(test)]
    const fn serial(self) -> u64 {
        self.0.get()
    }
}

/// Observational provenance for one packet.  It never selects rendering or
/// presentation work; the RedrawRequested kernel remains authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeVisualRequestOrigin {
    ScheduledOrRuntime(FrameWork),
    NativeInvalidationFallback,
}

/// A typed, non-cloneable witness for one native redraw request.
///
/// The packet is not frame state and does not authorize scene work.  Its
/// private origin is observational evidence retained for diagnostics and is
/// intentionally never used as a rendering decision by this module.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NativeVisualRequestPacket {
    window_id: WindowId,
    owner_generation: NativeVisualOwnerGeneration,
    revision: NativeVisualRevision,
    origin: NativeVisualRequestOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeVisualRequestIdentity {
    window_id: WindowId,
    owner_generation: NativeVisualOwnerGeneration,
    revision: NativeVisualRevision,
}

impl NativeVisualRequestPacket {
    pub(super) fn identity(&self) -> NativeVisualRequestIdentity {
        NativeVisualRequestIdentity {
            window_id: self.window_id,
            owner_generation: self.owner_generation,
            revision: self.revision,
        }
    }

    #[cfg(test)]
    const fn observed_frame_work(&self) -> Option<FrameWork> {
        match self.origin {
            NativeVisualRequestOrigin::ScheduledOrRuntime(frame_work) => Some(frame_work),
            NativeVisualRequestOrigin::NativeInvalidationFallback => None,
        }
    }

    #[cfg(test)]
    const fn revision(&self) -> u64 {
        self.revision.serial()
    }

    #[cfg(test)]
    const fn origin(&self) -> NativeVisualRequestOrigin {
        self.origin
    }
}

/// Result of enqueueing one packet into a window mailbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeVisualRequestEnqueue {
    /// The packet became the one outstanding Winit request and was issued.
    Issued,
    /// The packet replaced an outstanding request without another Winit wakeup.
    Replaced,
    /// The packet replaced the newest pending request behind a consuming one.
    Queued,
    /// The mailbox could not admit the packet because its identity allocator
    /// or native fence was unavailable.
    Rejected,
}

/// Result of beginning a `RedrawRequested` event.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum NativeVisualRequestBegin {
    Requested(NativeVisualRequestPacket),
    UnsolicitedFallback(NativeVisualRequestPacket),
    /// A requested packet was present, but the current local/native fences
    /// vetoed beginning it.  The packet and any associated successor were
    /// retired as one ownership transition; callers must not redraw, finish,
    /// or fall back after this result.
    RequestedVetoed,
    WrongWindow,
    Stale,
    Ineligible,
    Exhausted,
}

/// Independent logical eligibility for a requested packet and an unsolicited
/// redraw fallback.  Recovery exceptions may admit the requested packet while
/// an unsolicited wake remains limited to the ordinary presentation fences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NativeVisualRequestEligibility {
    pub(super) requested: bool,
    pub(super) fallback: bool,
}

impl From<bool> for NativeVisualRequestEligibility {
    fn from(eligible: bool) -> Self {
        Self {
            requested: eligible,
            fallback: eligible,
        }
    }
}

/// Result of completing one consuming packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeVisualRequestFinish {
    Completed,
    Reissued,
    Retained,
    WrongWindow,
    Stale,
}

/// The redraw result that determines how the consuming packet is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeVisualRequestDisposition {
    /// A complete native frame reached the existing presentation path.
    Presented,
    /// The frame path was conservatively unable to complete.  Return the same
    /// packet when no newer pending offer exists; otherwise latest-wins
    /// promotes that successor.
    RetrySamePacket,
    /// Retain the consuming packet and any newer pending work without issuing
    /// another Winit wakeup.  The window will reissue the retained request
    /// when its surface becomes visible again.
    RetainUntilUnoccluded,
    /// Drop the consuming packet while retaining any newer pending request.
    DropPacket,
}

/// Per-window fixed-size mailbox for native visual request ownership.
#[derive(Debug)]
pub(super) struct NativeVisualRequestMailbox {
    window_id: Option<WindowId>,
    owner_generation: NativeVisualOwnerGeneration,
    next_revision: Option<NativeVisualRevision>,
    accepting: bool,
    suspended: bool,
    requested: Option<NativeVisualRequestPacket>,
    // The non-Clone packet is owned by the redraw boundary while consuming;
    // retain its exact identity here so stale completion cannot close a newer
    // packet without duplicating the handoff witness.
    consuming: Option<NativeVisualRequestIdentity>,
    pending: Option<NativeVisualRequestPacket>,
}

impl Default for NativeVisualRequestMailbox {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeVisualRequestMailbox {
    pub(super) const fn new() -> Self {
        Self {
            window_id: None,
            owner_generation: NativeVisualOwnerGeneration::initial(),
            next_revision: Some(NativeVisualRevision::initial()),
            accepting: true,
            suspended: false,
            requested: None,
            consuming: None,
            pending: None,
        }
    }

    /// Bind the mailbox to one native window identity.  A replacement first
    /// retires every packet and advances the owner generation.
    pub(super) fn bind_window(&mut self, window_id: WindowId) -> bool {
        match self.window_id {
            None => {
                self.window_id = Some(window_id);
                self.accepting
            }
            Some(current) if current == window_id => self.accepting,
            Some(_) => {
                if !self.invalidate() {
                    return false;
                }
                self.window_id = Some(window_id);
                true
            }
        }
    }

    /// Retire this mailbox permanently.  Close and terminal lifecycle paths
    /// use this instead of allowing a late Winit event to re-enter rendering.
    pub(super) fn retire(&mut self) {
        self.requested = None;
        self.consuming = None;
        self.pending = None;
        if self.accepting {
            let _ = self.advance_owner_generation();
        }
        self.accepting = false;
        self.suspended = true;
    }

    /// Temporarily stop admitting offers and redraw fallbacks while retaining
    /// this mailbox for a later explicit resume.  This is deliberately
    /// reversible and therefore remains distinct from terminal retirement.
    pub(super) fn suspend(&mut self) -> bool {
        self.requested = None;
        self.consuming = None;
        self.pending = None;
        if self.suspended {
            return self.accepting;
        }
        self.suspended = true;
        if !self.accepting {
            return false;
        }
        self.advance_owner_generation()
    }

    /// Resume ordinary packet admission after a reversible suspension.
    pub(super) fn resume(&mut self) -> bool {
        if !self.accepting {
            return false;
        }
        self.suspended = false;
        true
    }

    /// Invalidate all packet ownership and advance the non-wrapping owner
    /// generation.  Once that generation is exhausted, the mailbox stays
    /// closed and all later requests fail closed.
    pub(super) fn invalidate(&mut self) -> bool {
        self.requested = None;
        self.consuming = None;
        self.pending = None;
        if !self.accepting {
            return false;
        }
        self.advance_owner_generation()
    }

    fn advance_owner_generation(&mut self) -> bool {
        let Some(next_generation) = self.owner_generation.checked_next() else {
            self.accepting = false;
            return false;
        };
        self.owner_generation = next_generation;
        true
    }

    fn allocate_packet(
        &mut self,
        origin: NativeVisualRequestOrigin,
    ) -> Option<NativeVisualRequestPacket> {
        if !self.accepting || self.suspended || self.window_id.is_none() {
            return None;
        }
        let revision = self.next_revision?;
        let window_id = self.window_id?;
        self.next_revision = revision.checked_next();
        Some(NativeVisualRequestPacket {
            window_id,
            owner_generation: self.owner_generation,
            revision,
            origin,
        })
    }

    fn enqueue(&mut self, frame_work: FrameWork) -> NativeVisualRequestEnqueue {
        let Some(packet) =
            self.allocate_packet(NativeVisualRequestOrigin::ScheduledOrRuntime(frame_work))
        else {
            return NativeVisualRequestEnqueue::Rejected;
        };
        let result = if self.consuming.is_some() {
            // A currently executing redraw owns the next event slot. Keep only
            // the newest work behind it.
            self.pending = Some(packet);
            NativeVisualRequestEnqueue::Queued
        } else if self.requested.is_some() {
            // The Winit request is not consuming yet. Replace it in place and
            // do not issue another wakeup; the newest offer is authoritative.
            self.requested = Some(packet);
            // A pending successor can exist only after a same-packet retry.
            // A newer requested offer supersedes that older successor too.
            self.pending = None;
            NativeVisualRequestEnqueue::Replaced
        } else {
            self.requested = Some(packet);
            NativeVisualRequestEnqueue::Issued
        };
        self.assert_bounded();
        result
    }

    fn begin<E: Into<NativeVisualRequestEligibility>>(
        &mut self,
        window_id: WindowId,
        eligibility: E,
    ) -> NativeVisualRequestBegin {
        let eligibility = eligibility.into();
        if self.window_id != Some(window_id) {
            return NativeVisualRequestBegin::WrongWindow;
        }
        if !self.accepting || self.suspended {
            return NativeVisualRequestBegin::Ineligible;
        }
        if let Some(packet) = self.requested.take() {
            if !self.packet_is_current(&packet, window_id) {
                self.pending = None;
                let _ = self.invalidate();
                return NativeVisualRequestBegin::Stale;
            }
            if !eligibility.requested {
                self.pending = None;
                let _ = self.advance_owner_generation();
                return NativeVisualRequestBegin::RequestedVetoed;
            }
            let identity = packet.identity();
            self.consuming = Some(identity);
            self.assert_bounded();
            return NativeVisualRequestBegin::Requested(packet);
        }
        if !eligibility.fallback {
            return NativeVisualRequestBegin::Ineligible;
        }
        if self.consuming.is_some() {
            return NativeVisualRequestBegin::Ineligible;
        }
        let Some(packet) =
            self.allocate_packet(NativeVisualRequestOrigin::NativeInvalidationFallback)
        else {
            return NativeVisualRequestBegin::Exhausted;
        };
        let identity = packet.identity();
        self.consuming = Some(identity);
        self.assert_bounded();
        NativeVisualRequestBegin::UnsolicitedFallback(packet)
    }

    fn finish(
        &mut self,
        window_id: WindowId,
        packet: NativeVisualRequestPacket,
        disposition: NativeVisualRequestDisposition,
    ) -> NativeVisualRequestFinish {
        if self.window_id != Some(window_id) {
            return NativeVisualRequestFinish::WrongWindow;
        }
        let Some(consuming) = self.consuming.take() else {
            return NativeVisualRequestFinish::Stale;
        };
        if consuming != packet.identity() {
            self.consuming = Some(consuming);
            return NativeVisualRequestFinish::Stale;
        }
        if let Some(pending) = self.pending.take() {
            // Latest-wins applies to retries too.  A newer pending packet is
            // promoted before the failed consuming packet can be retried.
            self.requested = Some(pending);
            self.assert_bounded();
            if matches!(
                disposition,
                NativeVisualRequestDisposition::RetainUntilUnoccluded
            ) {
                NativeVisualRequestFinish::Retained
            } else {
                NativeVisualRequestFinish::Reissued
            }
        } else if matches!(disposition, NativeVisualRequestDisposition::RetrySamePacket) {
            // With no successor, retry the exact packet without allocating a
            // replacement identity.
            self.requested = Some(packet);
            self.assert_bounded();
            NativeVisualRequestFinish::Reissued
        } else if matches!(
            disposition,
            NativeVisualRequestDisposition::RetainUntilUnoccluded
        ) {
            self.requested = Some(packet);
            self.assert_bounded();
            NativeVisualRequestFinish::Retained
        } else {
            self.assert_bounded();
            NativeVisualRequestFinish::Completed
        }
    }

    fn reissue_requested(&mut self, window_id: WindowId) -> bool {
        if self.window_id != Some(window_id) || !self.accepting {
            return false;
        }
        let Some(packet) = self.requested.as_ref() else {
            return false;
        };
        if self.packet_is_current(packet, window_id) {
            true
        } else {
            self.pending = None;
            let _ = self.invalidate();
            false
        }
    }

    fn packet_is_current(&self, packet: &NativeVisualRequestPacket, window_id: WindowId) -> bool {
        packet.window_id == window_id && packet.owner_generation == self.owner_generation
    }

    pub(super) fn veto_requested(&mut self) -> NativeVisualRequestBegin {
        if !self.has_work() {
            return NativeVisualRequestBegin::Ineligible;
        }
        self.requested = None;
        self.consuming = None;
        self.pending = None;
        let _ = self.advance_owner_generation();
        NativeVisualRequestBegin::RequestedVetoed
    }

    pub(super) fn is_suspended(&self) -> bool {
        self.suspended
    }

    pub(super) fn is_bound_to(&self, window_id: WindowId) -> bool {
        self.window_id == Some(window_id)
    }

    #[cfg(test)]
    fn owner_generation(&self) -> u64 {
        self.owner_generation.serial()
    }

    #[cfg(test)]
    pub(super) fn owner_generation_for_test(&self) -> u64 {
        self.owner_generation()
    }

    #[cfg(test)]
    pub(super) fn enqueue_for_test(&mut self, frame_work: FrameWork) -> NativeVisualRequestEnqueue {
        self.enqueue(frame_work)
    }

    #[cfg(test)]
    fn requested_revision(&self) -> Option<u64> {
        self.requested
            .as_ref()
            .map(|packet| packet.revision.serial())
    }

    #[cfg(test)]
    fn consuming_revision(&self) -> Option<u64> {
        self.consuming
            .as_ref()
            .map(|identity| identity.revision.serial())
    }

    #[cfg(test)]
    fn pending_revision(&self) -> Option<u64> {
        self.pending.as_ref().map(|packet| packet.revision.serial())
    }

    pub(super) fn has_work(&self) -> bool {
        self.requested.is_some() || self.consuming.is_some() || self.pending.is_some()
    }

    fn retained_depth(&self) -> usize {
        usize::from(self.requested.is_some())
            + usize::from(self.consuming.is_some())
            + usize::from(self.pending.is_some())
    }

    fn assert_bounded(&self) {
        debug_assert!(!(self.requested.is_some() && self.consuming.is_some()));
        debug_assert!(self.retained_depth() <= NATIVE_VISUAL_MAILBOX_MAX_RETAINED_DEPTH);
    }

    pub(super) fn has_requested(&self) -> bool {
        self.requested.is_some()
    }
}

/// The one central adapter allowed to call `Window::request_redraw`.
#[derive(Debug)]
pub(super) struct NativeVisualRequestAdapter;

impl NativeVisualRequestAdapter {
    pub(super) fn enqueue(
        mailbox: &mut NativeVisualRequestMailbox,
        window: &Window,
        frame_work: FrameWork,
    ) -> NativeVisualRequestEnqueue {
        let result = mailbox.enqueue(frame_work);
        if matches!(result, NativeVisualRequestEnqueue::Issued) {
            Self::issue(window);
        }
        result
    }

    pub(super) fn enqueue_without_wakeup(
        mailbox: &mut NativeVisualRequestMailbox,
        frame_work: FrameWork,
    ) -> NativeVisualRequestEnqueue {
        mailbox.enqueue(frame_work)
    }

    pub(super) fn reissue(
        mailbox: &mut NativeVisualRequestMailbox,
        window: &Window,
        window_id: WindowId,
    ) -> bool {
        if !mailbox.reissue_requested(window_id) {
            return false;
        }
        Self::issue(window);
        true
    }

    pub(super) fn begin<E: Into<NativeVisualRequestEligibility>>(
        mailbox: &mut NativeVisualRequestMailbox,
        window_id: WindowId,
        eligibility: E,
    ) -> NativeVisualRequestBegin {
        mailbox.begin(window_id, eligibility)
    }

    pub(super) fn finish(
        mailbox: &mut NativeVisualRequestMailbox,
        window: &Window,
        window_id: WindowId,
        packet: NativeVisualRequestPacket,
        disposition: NativeVisualRequestDisposition,
    ) -> NativeVisualRequestFinish {
        let result = mailbox.finish(window_id, packet, disposition);
        if matches!(result, NativeVisualRequestFinish::Reissued) {
            Self::issue(window);
        }
        result
    }

    fn issue(window: &Window) {
        window.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mailbox() -> (NativeVisualRequestMailbox, WindowId) {
        let window_id = WindowId::from(17);
        let mut mailbox = NativeVisualRequestMailbox::new();
        assert!(mailbox.bind_window(window_id));
        (mailbox, window_id)
    }

    #[test]
    fn mailbox_has_fixed_retained_depth() {
        assert_eq!(NATIVE_VISUAL_MAILBOX_MAX_RETAINED_DEPTH, 2);
        let (mailbox, _) = mailbox();
        assert!(!mailbox.has_work());
        assert_eq!(mailbox.retained_depth(), 0);
    }

    #[test]
    fn requested_offer_replaces_without_pending_or_second_wakeup() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        assert_eq!(mailbox.requested_revision(), Some(1));
        assert_eq!(
            mailbox.enqueue(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            }),
            NativeVisualRequestEnqueue::Replaced
        );
        assert_eq!(mailbox.requested_revision(), Some(2));
        assert_eq!(mailbox.pending_revision(), None);
        assert_eq!(mailbox.retained_depth(), 1);
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(packet.revision(), 2);
        assert_eq!(
            packet.origin(),
            NativeVisualRequestOrigin::ScheduledOrRuntime(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            })
        );
        assert_eq!(mailbox.consuming_revision(), Some(2));
        assert_eq!(mailbox.retained_depth(), 1);
    }

    #[test]
    fn consuming_offer_replaces_newest_pending_and_promotes_once() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(
            mailbox.enqueue(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            }),
            NativeVisualRequestEnqueue::Queued
        );
        assert_eq!(mailbox.pending_revision(), Some(2));
        assert_eq!(
            mailbox.enqueue(FrameWork::RebuildScene {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRepaint,
                mode: super::super::SceneRebuildMode::Immediate,
            }),
            NativeVisualRequestEnqueue::Queued
        );
        assert_eq!(mailbox.pending_revision(), Some(3));
        assert_eq!(
            mailbox.finish(window_id, packet, NativeVisualRequestDisposition::Presented),
            NativeVisualRequestFinish::Reissued
        );
        assert_eq!(mailbox.requested_revision(), Some(3));
        assert_eq!(mailbox.pending_revision(), None);
        assert_eq!(mailbox.retained_depth(), 1);
    }

    #[test]
    fn ordinary_resize_target_transition_preserves_claimed_packet() {
        use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;

        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::ResizeSurface {
                reason: super::super::FrameWorkReason::NativeResize,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };

        let mut target_generation = NativeTargetGeneration::from_test_serial(1);
        assert!(target_generation.advance());
        assert_eq!(
            target_generation,
            NativeTargetGeneration::from_test_serial(2)
        );
        assert_eq!(mailbox.consuming_revision(), Some(1));
        assert_eq!(
            packet.observed_frame_work(),
            Some(FrameWork::ResizeSurface {
                reason: super::super::FrameWorkReason::NativeResize,
            })
        );
        assert_eq!(
            mailbox.finish(window_id, packet, NativeVisualRequestDisposition::Presented),
            NativeVisualRequestFinish::Completed
        );
        assert!(!mailbox.has_work());
    }

    #[test]
    fn retry_promotes_newer_pending_work_before_same_packet() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(
            mailbox.enqueue(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            }),
            NativeVisualRequestEnqueue::Queued
        );
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::RetrySamePacket,
            ),
            NativeVisualRequestFinish::Reissued
        );
        assert_eq!(mailbox.requested_revision(), Some(2));
        assert_eq!(mailbox.pending_revision(), None);
        assert_eq!(mailbox.retained_depth(), 1);
        assert_eq!(
            mailbox.enqueue(FrameWork::RebuildScene {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRepaint,
                mode: super::super::SceneRebuildMode::Immediate,
            }),
            NativeVisualRequestEnqueue::Replaced
        );
        assert_eq!(mailbox.requested_revision(), Some(3));
        assert_eq!(mailbox.pending_revision(), None);
        assert_eq!(mailbox.retained_depth(), 1);
    }

    #[test]
    fn retry_keeps_exact_packet_when_no_newer_offer_exists() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::RetrySamePacket,
            ),
            NativeVisualRequestFinish::Reissued
        );
        assert_eq!(mailbox.requested_revision(), Some(1));
        assert_eq!(mailbox.pending_revision(), None);
    }

    #[test]
    fn occluded_finish_retains_and_coalesces_until_one_explicit_reissue() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(
            mailbox.enqueue(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            }),
            NativeVisualRequestEnqueue::Queued
        );
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::RetainUntilUnoccluded,
            ),
            NativeVisualRequestFinish::Retained
        );
        assert_eq!(mailbox.requested_revision(), Some(2));
        assert_eq!(mailbox.pending_revision(), None);

        assert_eq!(
            mailbox.enqueue(FrameWork::RebuildScene {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRepaint,
                mode: super::super::SceneRebuildMode::Immediate,
            }),
            NativeVisualRequestEnqueue::Replaced
        );
        assert_eq!(mailbox.requested_revision(), Some(3));
        assert!(mailbox.has_requested());
        assert!(mailbox.reissue_requested(window_id));
        assert_eq!(mailbox.requested_revision(), Some(3));
    }

    #[test]
    fn newer_offer_revision_is_allocated_before_stale_reissue() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        assert!(mailbox.reissue_requested(window_id));
        assert_eq!(mailbox.requested_revision(), Some(1));
        assert_eq!(
            mailbox.enqueue(FrameWork::RefreshSurface {
                reason: super::super::FrameWorkReason::RuntimeSurfaceRefresh,
            }),
            NativeVisualRequestEnqueue::Replaced
        );
        assert_eq!(mailbox.requested_revision(), Some(2));
        assert!(mailbox.reissue_requested(window_id));
    }

    #[test]
    fn unsolicited_redraw_is_admitted_only_when_eligible() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.begin(window_id, false,),
            NativeVisualRequestBegin::Ineligible
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::UnsolicitedFallback(packet) => packet,
            other => panic!("unexpected fallback result: {other:?}"),
        };
        assert_eq!(packet.observed_frame_work(), None);
        assert_eq!(
            packet.origin(),
            NativeVisualRequestOrigin::NativeInvalidationFallback
        );
        assert_eq!(mailbox.consuming_revision(), Some(1));
        assert_eq!(
            mailbox.begin(window_id, true,),
            NativeVisualRequestBegin::Ineligible
        );
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::DropPacket
            ),
            NativeVisualRequestFinish::Completed
        );
    }

    #[test]
    fn logical_presentation_capability_is_independent_of_host_visibility() {
        let (mut mailbox, window_id) = mailbox();
        let logical_capability = NativeVisualRequestEligibility {
            requested: true,
            fallback: true,
        };

        let packet = match mailbox.begin(window_id, logical_capability) {
            NativeVisualRequestBegin::UnsolicitedFallback(packet) => packet,
            other => panic!("unexpected logical fallback result: {other:?}"),
        };
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::DropPacket,
            ),
            NativeVisualRequestFinish::Completed
        );
    }

    #[test]
    fn requested_recovery_exception_never_admits_unsolicited_fallback() {
        let (mut mailbox, window_id) = mailbox();
        let recovery_only = NativeVisualRequestEligibility {
            requested: true,
            fallback: false,
        };
        assert_eq!(
            mailbox.begin(window_id, recovery_only),
            NativeVisualRequestBegin::Ineligible
        );
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, recovery_only) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected requested recovery begin: {other:?}"),
        };
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::DropPacket,
            ),
            NativeVisualRequestFinish::Completed
        );
    }

    #[test]
    fn requested_veto_clears_packet_and_never_falls_back() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Issued
        );
        let owner = mailbox.owner_generation();

        assert_eq!(
            mailbox.begin(
                window_id,
                NativeVisualRequestEligibility {
                    requested: false,
                    fallback: true,
                },
            ),
            NativeVisualRequestBegin::RequestedVetoed
        );
        assert_eq!(mailbox.owner_generation(), owner + 1);
        assert!(!mailbox.has_work());
        assert!(matches!(
            mailbox.begin(window_id, true),
            NativeVisualRequestBegin::UnsolicitedFallback(_)
        ));
    }

    #[test]
    fn suspension_rejects_offers_and_survives_invalidation_until_resume() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Issued
        );
        assert!(mailbox.suspend());
        assert!(mailbox.is_suspended());
        assert!(!mailbox.has_work());
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Rejected
        );
        assert_eq!(
            mailbox.begin(window_id, true),
            NativeVisualRequestBegin::Ineligible
        );
        assert!(mailbox.invalidate());
        assert!(mailbox.is_suspended());
        assert!(mailbox.resume());
        assert!(!mailbox.is_suspended());
        assert_eq!(
            mailbox.enqueue(FrameWork::None),
            NativeVisualRequestEnqueue::Issued
        );
    }

    #[test]
    fn stale_and_wrong_window_events_do_not_consume_current_work() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        assert_eq!(
            mailbox.begin(WindowId::from(18), true),
            NativeVisualRequestBegin::WrongWindow
        );
        assert_eq!(mailbox.requested_revision(), Some(1));
        let old_owner = mailbox.owner_generation();
        assert_eq!(
            mailbox.begin(window_id, false),
            NativeVisualRequestBegin::RequestedVetoed
        );
        assert_eq!(mailbox.requested_revision(), None);
        assert_eq!(mailbox.pending_revision(), None);
        assert_eq!(mailbox.owner_generation(), old_owner + 1);
        assert!(!mailbox.has_work());
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        let stale_revision = NativeVisualRequestPacket {
            window_id,
            owner_generation: NativeVisualOwnerGeneration::from_test_serial(
                mailbox.owner_generation(),
            ),
            revision: NativeVisualRevision::from_test_serial(99),
            origin: NativeVisualRequestOrigin::ScheduledOrRuntime(FrameWork::None),
        };
        assert_eq!(
            mailbox.finish(
                window_id,
                stale_revision,
                NativeVisualRequestDisposition::DropPacket,
            ),
            NativeVisualRequestFinish::Stale
        );
        assert_eq!(mailbox.consuming_revision(), Some(2));
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::DropPacket
            ),
            NativeVisualRequestFinish::Completed
        );
        assert!(!mailbox.has_work());
    }

    #[test]
    fn stale_owner_event_after_invalidation_cannot_finish_new_work() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let packet = match mailbox.begin(window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet,
            other => panic!("unexpected begin result: {other:?}"),
        };
        assert_eq!(
            mailbox.begin(window_id, true),
            NativeVisualRequestBegin::Ineligible
        );
        assert!(mailbox.invalidate());
        assert_eq!(
            mailbox.finish(
                window_id,
                packet,
                NativeVisualRequestDisposition::RetrySamePacket,
            ),
            NativeVisualRequestFinish::Stale
        );
        assert!(!mailbox.has_work());
    }

    #[test]
    fn invalidation_advances_owner_and_clears_every_slot() {
        let (mut mailbox, window_id) = mailbox();
        assert_eq!(
            mailbox.enqueue(FrameWork::PaintOnly {
                reason: super::super::FrameWorkReason::RuntimePaintOnly,
            }),
            NativeVisualRequestEnqueue::Issued
        );
        let old_owner = mailbox.owner_generation();
        assert!(mailbox.invalidate());
        assert_eq!(mailbox.owner_generation(), old_owner + 1);
        assert!(!mailbox.has_work());
        assert!(matches!(
            mailbox.begin(window_id, true,),
            NativeVisualRequestBegin::UnsolicitedFallback(_)
        ));
    }

    #[test]
    fn retirement_advances_owner_and_rejects_late_events() {
        let (mut mailbox, window_id) = mailbox();
        let old_owner = mailbox.owner_generation();
        assert_eq!(
            mailbox.enqueue(FrameWork::None,),
            NativeVisualRequestEnqueue::Issued
        );
        mailbox.retire();
        assert_eq!(mailbox.owner_generation(), old_owner + 1);
        assert!(!mailbox.has_work());
        assert_eq!(
            mailbox.begin(window_id, true,),
            NativeVisualRequestBegin::Ineligible
        );
    }

    #[test]
    fn owner_and_revision_exhaustion_fail_closed_without_wrap() {
        let window_id = WindowId::from(19);
        let mut revision_exhausted = NativeVisualRequestMailbox {
            window_id: Some(window_id),
            owner_generation: NativeVisualOwnerGeneration::from_test_serial(1),
            next_revision: Some(NativeVisualRevision::from_test_serial(u64::MAX)),
            accepting: true,
            suspended: false,
            requested: None,
            consuming: None,
            pending: None,
        };
        assert_eq!(
            revision_exhausted.enqueue(FrameWork::None,),
            NativeVisualRequestEnqueue::Issued
        );
        assert_eq!(
            revision_exhausted.enqueue(FrameWork::None,),
            NativeVisualRequestEnqueue::Rejected
        );

        let mut generation_exhausted = NativeVisualRequestMailbox {
            window_id: Some(window_id),
            owner_generation: NativeVisualOwnerGeneration::from_test_serial(u64::MAX),
            next_revision: Some(NativeVisualRevision::initial()),
            accepting: true,
            suspended: false,
            requested: None,
            consuming: None,
            pending: None,
        };
        assert!(!generation_exhausted.invalidate());
        assert_eq!(
            generation_exhausted.enqueue(FrameWork::None,),
            NativeVisualRequestEnqueue::Rejected
        );
    }
}
