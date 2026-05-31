use super::ExclusiveOpen;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    First,
    Second,
}

#[test]
fn exclusive_open_toggles_one_item_at_a_time() {
    let mut open = ExclusiveOpen::new();

    assert!(!open.any_open());
    open.toggle(Panel::First);
    assert!(open.is_open(&Panel::First));

    open.toggle(Panel::Second);
    assert!(!open.is_open(&Panel::First));
    assert!(open.is_open(&Panel::Second));

    open.toggle(Panel::Second);
    assert!(!open.any_open());
}

#[test]
fn exclusive_open_can_be_created_from_optional_key() {
    let open = ExclusiveOpen::from_open(Some(Panel::First));

    assert_eq!(open.current(), Some(&Panel::First));
    assert_eq!(open.into_option(), Some(Panel::First));
}
