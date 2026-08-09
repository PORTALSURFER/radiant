use super::{input::KeyboardModifiers, numeric_adjustment::NumericStep};

/// A semantic keyboard modifier used to select a numeric step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardModifier {
    /// Select the fine step when Shift is held.
    Shift,
    /// Select the fine or coarse step when the normalized command modifier is held.
    Command,
    /// Select the fine or coarse step when the normalized Control modifier is held.
    Control,
    /// Select the fine or coarse step when Alt/Option is held.
    Alt,
}

/// Stateless numeric step-selection policy for normalized keyboard modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NumericStepModifiers {
    fine: KeyboardModifier,
    coarse: KeyboardModifier,
}

impl NumericStepModifiers {
    /// The conventional macOS selector: Shift for Fine and Command for Coarse.
    pub const MACOS_DEFAULT: Self = Self::new(KeyboardModifier::Shift, KeyboardModifier::Command);

    /// The conventional Windows/Linux selector: Shift for Fine and Control for Coarse.
    pub const WINDOWS_LINUX_DEFAULT: Self =
        Self::new(KeyboardModifier::Shift, KeyboardModifier::Control);

    /// Construct a selector with explicit Fine and Coarse modifiers.
    pub const fn new(fine: KeyboardModifier, coarse: KeyboardModifier) -> Self {
        Self { fine, coarse }
    }

    /// Return the modifier configured for the Fine step.
    pub const fn fine(&self) -> KeyboardModifier {
        self.fine
    }

    /// Return the modifier configured for the Coarse step.
    pub const fn coarse(&self) -> KeyboardModifier {
        self.coarse
    }

    /// Select a step from one normalized modifier sample.
    ///
    /// Fine takes precedence when both configured modifiers are held, including
    /// when Fine and Coarse use the same selector. Each call evaluates only its
    /// supplied sample and does not retain state.
    pub const fn select_step(&self, modifiers: KeyboardModifiers) -> NumericStep {
        if Self::is_held(self.fine, modifiers) {
            NumericStep::Fine
        } else if Self::is_held(self.coarse, modifiers) {
            NumericStep::Coarse
        } else {
            NumericStep::Base
        }
    }

    const fn is_held(modifier: KeyboardModifier, modifiers: KeyboardModifiers) -> bool {
        match modifier {
            KeyboardModifier::Shift => modifiers.shift,
            KeyboardModifier::Command => modifiers.command,
            KeyboardModifier::Control => modifiers.control,
            KeyboardModifier::Alt => modifiers.alt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifiers(command: bool, control: bool, shift: bool, alt: bool) -> KeyboardModifiers {
        KeyboardModifiers {
            command,
            control,
            shift,
            alt,
        }
    }

    #[test]
    fn defaults_and_accessors_are_explicit() {
        assert_eq!(
            NumericStepModifiers::MACOS_DEFAULT,
            NumericStepModifiers::new(KeyboardModifier::Shift, KeyboardModifier::Command)
        );
        assert_eq!(
            NumericStepModifiers::WINDOWS_LINUX_DEFAULT,
            NumericStepModifiers::new(KeyboardModifier::Shift, KeyboardModifier::Control)
        );
        assert_eq!(
            NumericStepModifiers::MACOS_DEFAULT.fine(),
            KeyboardModifier::Shift
        );
        assert_eq!(
            NumericStepModifiers::MACOS_DEFAULT.coarse(),
            KeyboardModifier::Command
        );
        assert_eq!(
            NumericStepModifiers::WINDOWS_LINUX_DEFAULT.coarse(),
            KeyboardModifier::Control
        );
    }

    #[test]
    fn selection_is_deterministic_and_fine_wins_overlaps() {
        let policy = NumericStepModifiers::MACOS_DEFAULT;

        assert_eq!(
            policy.select_step(modifiers(false, false, false, false)),
            NumericStep::Base
        );
        assert_eq!(
            policy.select_step(modifiers(false, false, true, false)),
            NumericStep::Fine
        );
        assert_eq!(
            policy.select_step(modifiers(true, false, false, false)),
            NumericStep::Coarse
        );
        assert_eq!(
            policy.select_step(modifiers(true, false, true, false)),
            NumericStep::Fine
        );
        assert_eq!(
            policy.select_step(modifiers(false, true, false, true)),
            NumericStep::Base
        );

        assert_eq!(
            policy.select_step(modifiers(false, false, false, false)),
            NumericStep::Base
        );
    }

    #[test]
    fn every_modifier_matches_only_its_normalized_field() {
        for (modifier, sample) in [
            (
                KeyboardModifier::Shift,
                modifiers(false, false, true, false),
            ),
            (
                KeyboardModifier::Command,
                modifiers(true, false, false, false),
            ),
            (
                KeyboardModifier::Control,
                modifiers(false, true, false, false),
            ),
            (KeyboardModifier::Alt, modifiers(false, false, false, true)),
        ] {
            let policy = NumericStepModifiers::new(modifier, KeyboardModifier::Shift);
            assert_eq!(policy.select_step(sample), NumericStep::Fine);
        }
    }

    #[test]
    fn same_fine_and_coarse_selector_stays_fine() {
        let policy = NumericStepModifiers::new(KeyboardModifier::Alt, KeyboardModifier::Alt);

        assert_eq!(
            policy.select_step(modifiers(false, false, false, true)),
            NumericStep::Fine
        );
        assert_eq!(
            policy.select_step(modifiers(false, false, false, false)),
            NumericStep::Base
        );
    }
}
