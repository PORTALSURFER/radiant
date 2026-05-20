use super::super::{ListSelectionController, ListSelectionModifiers};

#[test]
fn list_selection_controller_tracks_single_toggle_and_range_selection() {
    let mut selection = ListSelectionController::new();

    assert!(selection.select(2, 8, ListSelectionModifiers::new()));
    assert_eq!(selection.focused_index(), Some(2));
    assert_eq!(selection.anchor_index(), Some(2));
    assert_eq!(selection.selected_indices(), &[2]);
    let single_revision = selection.revision();

    assert!(selection.select(5, 8, ListSelectionModifiers::extend()));
    assert_eq!(selection.focused_index(), Some(5));
    assert_eq!(selection.anchor_index(), Some(2));
    assert_eq!(selection.selected_indices(), &[2, 3, 4, 5]);
    assert!(selection.revision() > single_revision);

    assert!(selection.select(3, 8, ListSelectionModifiers::toggle()));
    assert_eq!(selection.focused_index(), Some(3));
    assert_eq!(selection.anchor_index(), Some(3));
    assert_eq!(selection.selected_indices(), &[2, 4, 5]);
    assert!(!selection.is_selected(3));
}

#[test]
fn list_selection_controller_clamps_membership_to_current_item_count() {
    let mut selection = ListSelectionController::new();
    selection.select_all(5);
    assert_eq!(selection.selected_indices(), &[0, 1, 2, 3, 4]);
    let all_revision = selection.revision();

    selection.focus(4, 5);
    selection.clamp_to_len(3);

    assert_eq!(selection.focused_index(), None);
    assert_eq!(selection.anchor_index(), Some(0));
    assert_eq!(selection.selected_indices(), &[0, 1, 2]);
    assert!(selection.revision() > all_revision);
}
