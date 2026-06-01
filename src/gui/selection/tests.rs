use super::{SelectionSet, TriState, TriageTarget};

#[test]
fn selection_set_normalizes_and_supports_sorted_membership() {
    let mut set = SelectionSet::from_items([4, 2, 4, 1]);

    assert_eq!(set.as_slice(), &[1, 2, 4]);
    assert!(set.contains(&2));
    assert!(!set.contains(&3));
    assert!(SelectionSet::slice_contains(set.as_slice(), &4));
    assert!(SelectionSet::slice_is_sorted_unique(set.as_slice()));
    assert!(SelectionSet::slice_is_sorted_unique_by_key(
        &[(1, "a"), (3, "b")],
        |(id, _)| *id
    ));

    assert!(set.insert(3));
    assert!(!set.insert(3));
    assert_eq!(set.as_slice(), &[1, 2, 3, 4]);
    assert!(set.remove(&2));
    assert!(!set.remove(&9));
    assert_eq!(set.as_slice(), &[1, 3, 4]);
    assert!(set.replace_items([8, 7, 8]));
    assert_eq!(set.as_slice(), &[7, 8]);
    assert!(set.extend_items([6, 8]));
    assert_eq!(set.as_slice(), &[6, 7, 8]);
    assert!(set.clear());
    assert!(!set.clear());
}

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
