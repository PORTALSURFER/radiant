use super::super::{
    CyclicListSelectionCycle, KeyedListSelection, ListSelectionController, ListSelectionIntent,
    ListSelectionModifiers, cyclic_list_index_after_delta, list_index_after_delta,
    unit_interval_index,
};

#[test]
fn list_index_after_delta_clamps_signed_navigation() {
    assert_eq!(list_index_after_delta(2, 1, 5), Some(3));
    assert_eq!(list_index_after_delta(2, -1, 5), Some(1));
    assert_eq!(list_index_after_delta(2, 20, 5), Some(4));
    assert_eq!(list_index_after_delta(2, -20, 5), Some(0));
    assert_eq!(list_index_after_delta(0, 1, 0), None);
}

#[test]
fn unit_interval_index_maps_unit_coordinates_to_bounded_indices() {
    assert_eq!(unit_interval_index(0.0, 4), Some(0));
    assert_eq!(unit_interval_index(0.24, 4), Some(0));
    assert_eq!(unit_interval_index(0.25, 4), Some(1));
    assert_eq!(unit_interval_index(0.999, 4), Some(3));
    assert_eq!(unit_interval_index(1.0, 4), Some(3));
}

#[test]
fn unit_interval_index_clamps_edges_and_handles_empty_lists() {
    assert_eq!(unit_interval_index(-1.0, 4), Some(0));
    assert_eq!(unit_interval_index(2.0, 4), Some(3));
    assert_eq!(unit_interval_index(f32::NEG_INFINITY, 4), Some(0));
    assert_eq!(unit_interval_index(f32::INFINITY, 4), Some(3));
    assert_eq!(unit_interval_index(f32::NAN, 4), Some(0));
    assert_eq!(unit_interval_index(0.5, 0), None);
}

#[test]
fn cyclic_list_index_after_delta_wraps_signed_navigation() {
    assert_eq!(cyclic_list_index_after_delta(2, 1, 5), Some(3));
    assert_eq!(cyclic_list_index_after_delta(2, -1, 5), Some(1));
    assert_eq!(cyclic_list_index_after_delta(4, 1, 5), Some(0));
    assert_eq!(cyclic_list_index_after_delta(0, -1, 5), Some(4));
    assert_eq!(cyclic_list_index_after_delta(12, 1, 5), Some(3));
    assert_eq!(cyclic_list_index_after_delta(0, 1, 0), None);
}

#[test]
fn cyclic_list_selection_cycle_tracks_query_key_and_wraps_selection() {
    let mut cycle = CyclicListSelectionCycle::new();

    assert_eq!(cycle.selected_index("ki", 4), Some(0));
    assert_eq!(cycle.move_selection("ki", 1, 4), Some(1));
    assert_eq!(cycle.query_key(), Some("ki"));
    assert_eq!(cycle.stored_index(), 1);
    assert_eq!(cycle.selected_index("ki", 4), Some(1));
    assert_eq!(cycle.move_selection("ki", -2, 4), Some(3));
}

#[test]
fn cyclic_list_selection_cycle_resets_display_selection_for_new_query() {
    let mut cycle = CyclicListSelectionCycle::new();
    assert_eq!(cycle.move_selection("kick", 2, 5), Some(2));

    assert_eq!(cycle.selected_index("snare", 5), Some(0));
    assert_eq!(cycle.active_selected_index("snare", 5), None);
    assert_eq!(cycle.move_selection("snare", 1, 5), Some(1));
    assert_eq!(cycle.query_key(), Some("snare"));
    assert_eq!(cycle.stored_index(), 1);
}

#[test]
fn cyclic_list_selection_cycle_clears_on_empty_lists() {
    let mut cycle = CyclicListSelectionCycle::new();
    assert_eq!(cycle.move_selection("kick", 1, 3), Some(1));

    assert_eq!(cycle.selected_index("kick", 0), None);
    assert_eq!(cycle.move_selection("kick", 1, 0), None);
    assert_eq!(cycle.query_key(), None);
    assert_eq!(cycle.stored_index(), 0);
}

#[test]
fn cyclic_list_selection_cycle_selects_explicit_index() {
    let mut cycle = CyclicListSelectionCycle::new();

    assert_eq!(cycle.select("filter", 7, 4), Some(3));
    assert_eq!(cycle.selected_index("filter", 4), Some(3));
    cycle.reset();
    assert_eq!(cycle.selected_index("filter", 4), Some(0));
}

#[test]
fn cyclic_list_selection_cycle_can_start_navigation_from_query_edges() {
    let mut cycle = CyclicListSelectionCycle::new();

    assert_eq!(cycle.move_selection_from_edge("kick", 1, 4), Some(0));
    assert_eq!(cycle.query_key(), Some("kick"));
    assert_eq!(cycle.move_selection_from_edge("kick", 1, 4), Some(1));

    assert_eq!(cycle.move_selection_from_edge("snare", -1, 4), Some(3));
    assert_eq!(cycle.query_key(), Some("snare"));
    assert_eq!(cycle.move_selection_from_edge("snare", -1, 4), Some(2));
}

#[test]
fn cyclic_list_selection_cycle_edge_navigation_clears_empty_lists() {
    let mut cycle = CyclicListSelectionCycle::new();
    assert_eq!(cycle.move_selection_from_edge("kick", 1, 3), Some(0));

    assert_eq!(cycle.move_selection_from_edge("kick", 1, 0), None);
    assert_eq!(cycle.query_key(), None);
    assert_eq!(cycle.stored_index(), 0);
}

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

#[test]
fn list_selection_controller_can_preserve_existing_range_membership() {
    let mut selection = ListSelectionController::new();
    selection.select(0, 8, ListSelectionModifiers::new());
    selection.select(3, 8, ListSelectionModifiers::toggle());

    selection.extend_preserving_existing(6, 8);

    assert_eq!(selection.focused_index(), Some(6));
    assert_eq!(selection.anchor_index(), Some(3));
    assert_eq!(selection.selected_indices(), &[0, 3, 4, 5, 6]);
}

#[test]
fn list_selection_intent_maps_common_extend_toggle_flags() {
    assert_eq!(
        ListSelectionModifiers::from_extend_toggle(false, false),
        ListSelectionModifiers::new()
    );
    assert_eq!(
        ListSelectionModifiers::from_extend_toggle(true, true),
        ListSelectionModifiers::extend()
    );
    assert_eq!(
        ListSelectionIntent::from_extend_toggle(false, false),
        ListSelectionIntent::Replace
    );
    assert_eq!(
        ListSelectionIntent::from_extend_toggle(true, false),
        ListSelectionIntent::Extend
    );
    assert_eq!(
        ListSelectionIntent::from_extend_toggle(false, true),
        ListSelectionIntent::Toggle
    );
    assert_eq!(
        ListSelectionIntent::from_extend_toggle(true, true),
        ListSelectionIntent::ExtendPreservingExisting
    );
}

#[test]
fn list_selection_controller_select_with_intent_supports_additive_range() {
    let mut selection = ListSelectionController::new();
    selection.select_with_intent(0, 8, ListSelectionIntent::Replace);
    selection.select_with_intent(3, 8, ListSelectionIntent::Toggle);

    selection.select_with_intent(6, 8, ListSelectionIntent::ExtendPreservingExisting);

    assert_eq!(selection.focused_index(), Some(6));
    assert_eq!(selection.anchor_index(), Some(3));
    assert_eq!(selection.selected_indices(), &[0, 3, 4, 5, 6]);
}

#[test]
fn keyed_list_selection_tracks_stable_keys_through_range_toggle_and_navigation() {
    let keys = ["hat", "kick", "snare", "tom"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let mut selection = KeyedListSelection::new();

    assert!(selection.select(String::from("kick"), &keys, ListSelectionModifiers::new()));
    assert_eq!(selection.focused_key().map(String::as_str), Some("kick"));
    assert_eq!(selection.anchor_key().map(String::as_str), Some("kick"));
    assert_eq!(selection.selected_keys(), &[String::from("kick")]);

    assert_eq!(
        selection.navigate(1, &keys, true),
        Some(String::from("snare"))
    );
    assert_eq!(selection.focused_key().map(String::as_str), Some("snare"));
    assert_eq!(selection.anchor_key().map(String::as_str), Some("kick"));
    assert_eq!(
        selection.selected_keys(),
        &[String::from("kick"), String::from("snare")]
    );
    assert_eq!(
        selection.navigate_preserving_existing(1, &keys),
        Some(String::from("tom"))
    );
    assert_eq!(
        selection.selected_keys(),
        &[
            String::from("kick"),
            String::from("snare"),
            String::from("tom")
        ]
    );

    assert!(selection.select(String::from("hat"), &keys, ListSelectionModifiers::toggle()));
    assert_eq!(
        selection.selected_keys(),
        &[
            String::from("hat"),
            String::from("kick"),
            String::from("snare"),
            String::from("tom")
        ]
    );

    selection.retain_visible(&[String::from("snare"), String::from("tom")]);
    assert_eq!(
        selection.selected_keys(),
        &[String::from("snare"), String::from("tom")]
    );
}

#[test]
fn keyed_list_selection_supports_additive_range_selection() {
    let keys = ["a", "b", "c", "d", "e"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let mut selection = KeyedListSelection::new();
    selection.select(String::from("a"), &keys, ListSelectionModifiers::new());
    selection.select(String::from("e"), &keys, ListSelectionModifiers::toggle());
    selection.extend_preserving_existing(String::from("c"), &keys);

    assert_eq!(
        selection.selected_keys(),
        &[
            String::from("a"),
            String::from("c"),
            String::from("d"),
            String::from("e")
        ]
    );
}

#[test]
fn keyed_list_selection_select_with_intent_supports_additive_range() {
    let keys = ["a", "b", "c", "d", "e"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let mut selection = KeyedListSelection::new();
    selection.select_with_intent(String::from("a"), &keys, ListSelectionIntent::Replace);
    selection.select_with_intent(String::from("e"), &keys, ListSelectionIntent::Toggle);
    selection.select_with_intent(
        String::from("c"),
        &keys,
        ListSelectionIntent::ExtendPreservingExisting,
    );

    assert_eq!(
        selection.selected_keys(),
        &[
            String::from("a"),
            String::from("c"),
            String::from("d"),
            String::from("e")
        ]
    );
}
