use super::{TriState, TriageTarget};

#[test]
fn tri_state_defaults_to_off() {
    assert_eq!(TriState::default(), TriState::Off);
}

#[test]
fn triage_target_names_generic_three_way_selection() {
    assert_eq!(TriageTarget::Negative, TriageTarget::Negative);
    assert_eq!(TriageTarget::Neutral, TriageTarget::Neutral);
    assert_eq!(TriageTarget::Positive, TriageTarget::Positive);
}
