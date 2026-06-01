use super::super::{
    DenseRowMarkerEdge, DenseRowMarkerParts, DenseRowPalette, DenseRowVisualState,
    dense_row_fill_color, dense_row_inset_rect, dense_row_label_font_size,
    dense_row_vertical_marker_rect, push_dense_row_fill, push_dense_row_inset_stroke,
    push_dense_row_vertical_marker,
};
use crate::gui::types::{Point, Rect, Rgba8, Vector2};
use crate::runtime::{PaintPrimitive, PaintStrokeRect};

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
fn dense_row_label_font_size_tracks_compact_row_height() {
    assert_eq!(dense_row_label_font_size(22.0), 13.0);
    assert_eq!(dense_row_label_font_size(28.0), 14.0);
    assert_eq!(dense_row_label_font_size(38.0), 18.0);
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

#[test]
fn push_dense_row_fill_appends_prioritized_fill_when_visible() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 8.0));
    let mut primitives = Vec::new();

    assert!(push_dense_row_fill(
        &mut primitives,
        7,
        bounds,
        DenseRowVisualState {
            selected: true,
            hovered: true,
            ..DenseRowVisualState::default()
        },
        palette(),
    ));

    assert_eq!(primitives.len(), 1);
    match &primitives[0] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(fill.widget_id, 7);
            assert_eq!(fill.rect, bounds);
            assert_eq!(fill.color, HOVERED);
        }
        primitive => panic!("expected fill rect, got {primitive:?}"),
    }
}

#[test]
fn push_dense_row_paint_helpers_skip_invisible_or_missing_geometry() {
    let collapsed = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 8.0));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 8.0));
    let mut primitives = Vec::new();

    assert!(!push_dense_row_fill(
        &mut primitives,
        7,
        collapsed,
        DenseRowVisualState {
            selected: true,
            ..DenseRowVisualState::default()
        },
        palette(),
    ));
    assert!(!push_dense_row_fill(
        &mut primitives,
        7,
        bounds,
        DenseRowVisualState::default(),
        palette(),
    ));
    assert!(!push_dense_row_inset_stroke(
        &mut primitives,
        7,
        bounds,
        20.0,
        ACTIVE,
        1.0,
    ));
    assert!(!push_dense_row_vertical_marker(
        &mut primitives,
        7,
        bounds,
        DenseRowMarkerParts {
            edge: DenseRowMarkerEdge::Leading,
            width: 0.0,
            edge_inset: 1.0,
            vertical_inset: 1.0,
            min_height: 1.0,
        },
        ACTIVE,
    ));

    assert!(primitives.is_empty());
}

#[test]
fn push_dense_row_marker_and_stroke_append_projected_shapes() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let mut primitives = Vec::new();

    assert!(push_dense_row_vertical_marker(
        &mut primitives,
        8,
        bounds,
        DenseRowMarkerParts {
            edge: DenseRowMarkerEdge::Trailing,
            width: 2.0,
            edge_inset: 1.0,
            vertical_inset: 3.0,
            min_height: 8.0,
        },
        CANDIDATE,
    ));
    assert!(push_dense_row_inset_stroke(
        &mut primitives,
        8,
        bounds,
        0.5,
        ACTIVE,
        1.0,
    ));

    assert_eq!(primitives.len(), 2);
    match &primitives[0] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(
                fill.rect,
                Rect::from_min_size(Point::new(127.0, 23.0), Vector2::new(2.0, 16.0))
            );
            assert_eq!(fill.color, CANDIDATE);
        }
        primitive => panic!("expected marker fill rect, got {primitive:?}"),
    }
    assert_eq!(
        primitives[1],
        PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: 8,
            rect: Rect::from_min_max(Point::new(10.5, 20.5), Point::new(129.5, 41.5)),
            color: ACTIVE,
            width: 1.0,
        })
    );
}
