//! Generic selection state primitives.

/// Three-way state for controls representing multiple selected items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TriState {
    /// No selected items currently carry the represented value.
    #[default]
    Off,
    /// Every selected item currently carries the represented value.
    On,
    /// Selected items disagree about the represented value.
    Mixed,
}

#[cfg(test)]
mod tests {
    use super::TriState;

    #[test]
    fn tri_state_defaults_to_off() {
        assert_eq!(TriState::default(), TriState::Off);
    }
}
