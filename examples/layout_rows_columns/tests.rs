use crate::model::LayoutDemoState;
use crate::view::{grid_is_crowded, grid_spacing};

#[test]
fn dense_grids_use_compact_cards() {
    let state = LayoutDemoState {
        columns: 4,
        rows: 5,
        depth: 1,
        ..LayoutDemoState::default()
    };

    assert!(grid_is_crowded(&state));
    assert_eq!(grid_spacing(&state), 4.0);
}

#[test]
fn three_by_three_nested_grids_use_compact_cards() {
    let state = LayoutDemoState {
        columns: 3,
        rows: 3,
        depth: 2,
        show_nested: true,
        ..LayoutDemoState::default()
    };

    assert!(grid_is_crowded(&state));
    assert_eq!(grid_spacing(&state), 4.0);
}

#[test]
fn three_column_deep_nested_grids_use_compact_cards() {
    let state = LayoutDemoState {
        columns: 3,
        rows: 1,
        depth: 2,
        show_nested: true,
        ..LayoutDemoState::default()
    };

    assert!(grid_is_crowded(&state));
    assert_eq!(grid_spacing(&state), 4.0);
}

#[test]
fn sparse_grids_keep_full_nested_layouts() {
    let state = LayoutDemoState {
        columns: 2,
        rows: 2,
        depth: 2,
        ..LayoutDemoState::default()
    };

    assert!(!grid_is_crowded(&state));
    assert_eq!(grid_spacing(&state), 8.0);
}
