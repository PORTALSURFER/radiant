//! Shared crate-private ownership arbitration for numeric interactions.

#![allow(dead_code)]

/// The interaction kinds that may own one numeric-input identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NumericInteractionOwner {
    TextEdit,
    ImeComposition,
    KeyboardAdjustment,
    PointerScrub,
    WheelSequence,
    AccessibilityEdit,
}

/// Allocation-free incumbent-owner gate for one numeric-input identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NumericInteractionGate {
    incumbent: Option<NumericInteractionOwner>,
}

impl NumericInteractionGate {
    /// Create an unowned gate.
    pub(crate) const fn new() -> Self {
        Self { incumbent: None }
    }

    /// Return the current incumbent without changing ownership.
    pub(crate) const fn incumbent(&self) -> Option<NumericInteractionOwner> {
        self.incumbent
    }

    /// Admit an owner only when the gate is currently unowned.
    pub(crate) fn try_admit(&mut self, owner: NumericInteractionOwner) -> bool {
        if self.incumbent.is_some() {
            return false;
        }

        self.incumbent = Some(owner);
        true
    }

    /// Release ownership only when the releasing owner is still incumbent.
    pub(crate) fn release(&mut self, owner: NumericInteractionOwner) {
        if self.incumbent == Some(owner) {
            self.incumbent = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNERS: [NumericInteractionOwner; 6] = [
        NumericInteractionOwner::TextEdit,
        NumericInteractionOwner::ImeComposition,
        NumericInteractionOwner::KeyboardAdjustment,
        NumericInteractionOwner::PointerScrub,
        NumericInteractionOwner::WheelSequence,
        NumericInteractionOwner::AccessibilityEdit,
    ];

    #[test]
    fn every_ordered_distinct_owner_conflict_keeps_the_incumbent() {
        let mut conflict_count = 0;

        for incumbent in OWNERS {
            for contender in OWNERS {
                if incumbent == contender {
                    continue;
                }

                conflict_count += 1;
                let mut gate = NumericInteractionGate::new();
                assert!(gate.try_admit(incumbent));
                assert!(!gate.try_admit(contender));
                assert_eq!(gate.incumbent(), Some(incumbent));
            }
        }

        assert_eq!(conflict_count, 30);
    }

    #[test]
    fn same_owner_readmission_is_denied_without_reacquiring() {
        let owner = NumericInteractionOwner::TextEdit;
        let mut gate = NumericInteractionGate::new();

        assert!(gate.try_admit(owner));
        assert!(!gate.try_admit(owner));
        assert_eq!(gate.incumbent(), Some(owner));
    }

    #[test]
    fn foreign_release_cannot_clear_the_incumbent() {
        let mut gate = NumericInteractionGate::new();
        assert!(gate.try_admit(NumericInteractionOwner::WheelSequence));

        gate.release(NumericInteractionOwner::PointerScrub);

        assert_eq!(
            gate.incumbent(),
            Some(NumericInteractionOwner::WheelSequence)
        );
    }

    #[test]
    fn matching_release_clears_the_gate_for_fresh_admission() {
        let first = NumericInteractionOwner::ImeComposition;
        let second = NumericInteractionOwner::AccessibilityEdit;
        let mut gate = NumericInteractionGate::new();

        assert!(gate.try_admit(first));
        gate.release(first);
        assert_eq!(gate.incumbent(), None);
        assert!(gate.try_admit(second));
        assert_eq!(gate.incumbent(), Some(second));
    }
}
