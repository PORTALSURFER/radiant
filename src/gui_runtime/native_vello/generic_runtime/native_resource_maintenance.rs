//! Exact, bounded native-resource maintenance bindings.
//!
//! Normal Running maintenance is admitted against one fixed resource slot.
//! The binding is observational only until the window-stage owner accepts it;
//! callers must re-check the same binding immediately before doing work.

use super::{
    CompositedBaseFrameRetirementIdentity, NativeAdapterGeneration,
    native_render_target::NativeRenderTargetRetirementIdentity,
    submission_completion::NativeSubmissionCompletionIdentity,
};
use std::time::Duration;

pub(super) const NATIVE_RESOURCE_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(16);

/// The bounded slots owned by one native window resource state.
///
/// There is one active bundle and the existing two-entry quarantine.  Keeping
/// this identity explicit prevents a maintenance completion from falling back
/// to a scan after its selected entry has moved or been retired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeResourceMaintenanceSlot {
    Active,
    Quarantine(u8),
}

impl NativeResourceMaintenanceSlot {
    pub(super) const fn ordinal(self) -> u8 {
        match self {
            Self::Quarantine(0) => 0,
            Self::Quarantine(1) => 1,
            Self::Active => 2,
            Self::Quarantine(_) => 3,
        }
    }
}

/// Exact resource and completion-witness evidence captured at admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeResourceMaintenanceBinding {
    slot: NativeResourceMaintenanceSlot,
    generation: NativeAdapterGeneration,
    completion: NativeSubmissionCompletionIdentity,
    composited_base_frame_retirement: Option<CompositedBaseFrameRetirementIdentity>,
    render_target_retirement: Option<NativeRenderTargetRetirementIdentity>,
}

impl NativeResourceMaintenanceBinding {
    pub(super) const fn new(
        slot: NativeResourceMaintenanceSlot,
        generation: NativeAdapterGeneration,
        completion: NativeSubmissionCompletionIdentity,
    ) -> Self {
        Self {
            slot,
            generation,
            completion,
            composited_base_frame_retirement: None,
            render_target_retirement: None,
        }
    }

    pub(super) const fn with_composited_base_frame_retirement(
        mut self,
        retirement: Option<CompositedBaseFrameRetirementIdentity>,
    ) -> Self {
        self.composited_base_frame_retirement = retirement;
        self
    }

    pub(super) const fn slot(self) -> NativeResourceMaintenanceSlot {
        self.slot
    }

    pub(super) const fn generation(self) -> NativeAdapterGeneration {
        self.generation
    }

    pub(super) const fn completion(self) -> NativeSubmissionCompletionIdentity {
        self.completion
    }

    pub(super) const fn composited_base_frame_retirement(
        self,
    ) -> Option<CompositedBaseFrameRetirementIdentity> {
        self.composited_base_frame_retirement
    }

    pub(super) const fn render_target_retirement(
        self,
    ) -> Option<NativeRenderTargetRetirementIdentity> {
        self.render_target_retirement
    }

    pub(super) const fn with_render_target_retirement(
        mut self,
        retirement: Option<NativeRenderTargetRetirementIdentity>,
    ) -> Self {
        self.render_target_retirement = retirement;
        self
    }
}

/// The fixed-slot selection and currentness kernel for one bounded turn.
pub(super) struct NativeResourceMaintenanceKernel;

impl NativeResourceMaintenanceKernel {
    /// Select the first exact actionable slot from the fixed Q0/Q1/active
    /// snapshot, starting at the per-window cursor. The caller supplies at
    /// at most one entry per physical slot.
    pub(super) fn select(
        candidates: [Option<NativeResourceMaintenanceBinding>; 3],
        cursor: u8,
    ) -> Option<NativeResourceMaintenanceBinding> {
        let start = usize::from(cursor % 3);
        (0..3)
            .map(|offset| (start + offset) % 3)
            .find_map(|index| candidates[index])
    }

    pub(super) const fn next_cursor(slot: NativeResourceMaintenanceSlot) -> u8 {
        match slot.ordinal() {
            0 => 1,
            1 => 2,
            2 => 0,
            _ => 0,
        }
    }

    /// A ticket may execute only against the exact binding it observed.
    pub(super) fn is_current(
        admitted: NativeResourceMaintenanceBinding,
        current: NativeResourceMaintenanceBinding,
    ) -> bool {
        admitted == current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;

    fn binding(
        slot: NativeResourceMaintenanceSlot,
        serial: u64,
    ) -> NativeResourceMaintenanceBinding {
        NativeResourceMaintenanceBinding::new(
            slot,
            NativeAdapterGeneration::from_test_serial(serial),
            NativeSubmissionCompletionIdentity::never_submitted(
                NativeAdapterGeneration::from_test_serial(serial),
            ),
        )
    }

    #[test]
    fn selection_is_fixed_slot_and_does_not_fallback() {
        let q0 = binding(NativeResourceMaintenanceSlot::Quarantine(0), 1);
        let q1 = binding(NativeResourceMaintenanceSlot::Quarantine(1), 2);
        let active = binding(NativeResourceMaintenanceSlot::Active, 3);
        assert_eq!(
            NativeResourceMaintenanceKernel::select([Some(q0), Some(q1), Some(active)], 0),
            Some(q0)
        );
        assert_eq!(
            NativeResourceMaintenanceKernel::select([Some(q0), Some(q1), Some(active)], 1),
            Some(q1)
        );
        assert_eq!(
            NativeResourceMaintenanceKernel::select([Some(q0), Some(q1), Some(active)], 2),
            Some(active)
        );
        assert_eq!(
            NativeResourceMaintenanceKernel::select([None, Some(q1), Some(active)], 0),
            Some(q1)
        );
    }

    #[test]
    fn cursor_cycles_continuous_active_and_quarantine_slots() {
        let q0 = binding(NativeResourceMaintenanceSlot::Quarantine(0), 1);
        let q1 = binding(NativeResourceMaintenanceSlot::Quarantine(1), 2);
        let active = binding(NativeResourceMaintenanceSlot::Active, 3);
        let candidates = [Some(q0), Some(q1), Some(active)];
        let mut cursor = 0;
        let mut selected = Vec::new();
        for _ in 0..6 {
            let slot = NativeResourceMaintenanceKernel::select(candidates, cursor)
                .expect("one actionable slot");
            selected.push(slot.slot());
            cursor = NativeResourceMaintenanceKernel::next_cursor(slot.slot());
        }
        assert_eq!(
            selected,
            vec![
                NativeResourceMaintenanceSlot::Quarantine(0),
                NativeResourceMaintenanceSlot::Quarantine(1),
                NativeResourceMaintenanceSlot::Active,
                NativeResourceMaintenanceSlot::Quarantine(0),
                NativeResourceMaintenanceSlot::Quarantine(1),
                NativeResourceMaintenanceSlot::Active,
            ]
        );
    }

    #[test]
    fn currentness_requires_every_binding_field() {
        let exact = binding(NativeResourceMaintenanceSlot::Active, 1);
        assert!(NativeResourceMaintenanceKernel::is_current(exact, exact));
        assert!(!NativeResourceMaintenanceKernel::is_current(
            exact,
            binding(NativeResourceMaintenanceSlot::Quarantine(0), 1)
        ));
        assert!(!NativeResourceMaintenanceKernel::is_current(
            exact,
            binding(NativeResourceMaintenanceSlot::Active, 2)
        ));
    }
}
