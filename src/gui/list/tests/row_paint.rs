use super::super::{
    DenseRowMarkerEdge, DenseRowMarkerParts, DenseRowPalette, DenseRowVisualState,
    dense_row_fill_color, dense_row_inset_rect, dense_row_vertical_marker_rect,
};
use crate::gui::types::{Point, Rect, Rgba8, Vector2};

const SELECTED: Rgba8 = Rgba8 {
    r: 1,
    g: 0,
    b: 0,
    a: 255,
};
const HOVERED: Rgba8 = Rgba8 {
    r: 2,
    g: 0,
    b: 0,
    a: 255,
};
const PRESSED: Rgba8 = Rgba8 {
    r: 3,
    g: 0,
    b: 0,
    a: 255,
};
const ACTIVE: Rgba8 = Rgba8 {
    r: 4,
    g: 0,
    b: 0,
    a: 255,
};
const CANDIDATE: Rgba8 = Rgba8 {
    r: 5,
    g: 0,
    b: 0,
    a: 255,
};

fn palette() -> DenseRowPalette {
    DenseRowPalette::new()
        .selected(SELECTED)
        .hovered(HOVERED)
        .pressed(PRESSED)
        .active_target(ACTIVE)
        .candidate_hovered(CANDIDATE)
}

#[test]
fn dense_row_fill_color_prioritizes_active_interaction_states() {
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                pressed: true,
                active_target: true,
                candidate: true,
            },
            palette(),
        ),
        Some(ACTIVE)
    );
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                candidate: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(CANDIDATE)
    );
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                pressed: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(PRESSED)
    );
}

#[test]
fn dense_row_fill_color_uses_selection_as_base_state() {
    assert_eq!(
        dense_row_fill_color(
            DenseRowVisualState {
                selected: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        ),
        Some(SELECTED)
    );
    assert_eq!(
        dense_row_fill_color(DenseRowVisualState::default(), palette()),
        None
    );
}

#[test]
fn dense_row_vertical_marker_projects_centered_edge_rects() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));

    assert_eq!(
        dense_row_vertical_marker_rect(
            bounds,
            DenseRowMarkerParts {
                edge: DenseRowMarkerEdge::Leading,
                width: 3.0,
                edge_inset: 1.0,
                vertical_inset: 4.0,
                min_height: 8.0,
            },
        ),
        Some(Rect::from_min_size(
            Point::new(11.0, 24.0),
            Vector2::new(3.0, 14.0)
        ))
    );
    assert_eq!(
        dense_row_vertical_marker_rect(
            bounds,
            DenseRowMarkerParts {
                edge: DenseRowMarkerEdge::Trailing,
                width: 2.0,
                edge_inset: 1.0,
                vertical_inset: 3.0,
                min_height: 8.0,
            },
        ),
        Some(Rect::from_min_size(
            Point::new(127.0, 23.0),
            Vector2::new(2.0, 16.0)
        ))
    );
}

#[test]
fn dense_row_inset_rect_rejects_collapsed_geometry() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 6.0));

    assert_eq!(
        dense_row_inset_rect(bounds, 0.5),
        Some(Rect::from_min_max(
            Point::new(0.5, 0.5),
            Point::new(9.5, 5.5)
        ))
    );
    assert_eq!(dense_row_inset_rect(bounds, 4.0), None);
}
