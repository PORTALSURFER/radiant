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

#[test]
fn exclusive_open_reports_open_and_close_changes() {
    let mut open = ExclusiveOpen::new();

    assert!(open.open_changed(Panel::First));
    assert!(!open.open_changed(Panel::First));
    assert_eq!(open.current(), Some(&Panel::First));

    assert!(open.open_changed(Panel::Second));
    assert_eq!(open.current(), Some(&Panel::Second));

    assert!(open.close_changed());
    assert!(!open.close_changed());
    assert!(!open.any_open());
}

#[test]
fn exclusive_open_reports_toggle_changes() {
    let mut open = ExclusiveOpen::new();

    assert!(open.toggle_changed(Panel::First));
    assert_eq!(open.current(), Some(&Panel::First));

    assert!(open.toggle_changed(Panel::First));
    assert!(!open.any_open());
}
