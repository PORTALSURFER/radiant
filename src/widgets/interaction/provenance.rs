use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    widgets::interaction::PointerModifiers,
};

/// Shared category for discrete and continuous interaction input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionSource {
    /// Input from a pointer or other pointing device.
    Pointer,
    /// Input from a keyboard.
    Keyboard,
    /// Input from an accessibility action.
    Accessibility,
    /// Input generated directly by application or framework code.
    Programmatic,
}

/// Optional native evidence associated with one interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionProvenance {
    /// Pointer input with the evidence available at the input boundary.
    Pointer {
        /// Modifier state captured with the pointer sample.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
        /// Optional native sample sequence range.
        sequence_range: Option<InputSequenceRange>,
    },
    /// Keyboard input with its optional native timestamp.
    Keyboard {
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Input initiated through an accessibility action.
    Accessibility,
    /// Input generated directly by application or framework code.
    Programmatic,
}

impl InteractionProvenance {
    /// Return the shared source category for this provenance.
    pub const fn source(self) -> InteractionSource {
        match self {
            Self::Pointer { .. } => InteractionSource::Pointer,
            Self::Keyboard { .. } => InteractionSource::Keyboard,
            Self::Accessibility => InteractionSource::Accessibility,
            Self::Programmatic => InteractionSource::Programmatic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_copy_eq<T: Copy + PartialEq + Eq>() {}

    #[test]
    fn source_maps_every_provenance_variant() {
        assert_eq!(
            InteractionProvenance::Pointer {
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            }
            .source(),
            InteractionSource::Pointer
        );
        assert_eq!(
            InteractionProvenance::Keyboard { timestamp: None }.source(),
            InteractionSource::Keyboard
        );
        assert_eq!(
            InteractionProvenance::Accessibility.source(),
            InteractionSource::Accessibility
        );
        assert_eq!(
            InteractionProvenance::Programmatic.source(),
            InteractionSource::Programmatic
        );
    }

    #[test]
    fn shared_provenance_types_are_copy_and_eq() {
        assert_copy_eq::<InteractionSource>();
        assert_copy_eq::<InteractionProvenance>();

        let source = InteractionSource::Pointer;
        let provenance = InteractionProvenance::Programmatic;
        assert_eq!(source, source);
        assert_eq!(provenance, provenance);
    }
}
