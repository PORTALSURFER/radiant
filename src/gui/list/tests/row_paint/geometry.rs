use super::fixtures::*;

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
fn dense_row_marker_parts_build_common_edge_markers() {
    assert_eq!(
        DenseRowMarkerParts::leading(3.0)
            .edge_inset(2.0)
            .vertical_inset(4.0)
            .min_height(9.0),
        DenseRowMarkerParts {
            edge: DenseRowMarkerEdge::Leading,
            width: 3.0,
            edge_inset: 2.0,
            vertical_inset: 4.0,
            min_height: 9.0,
        }
    );
    assert_eq!(
        DenseRowMarkerParts::trailing(2.0),
        DenseRowMarkerParts {
            edge: DenseRowMarkerEdge::Trailing,
            width: 2.0,
            edge_inset: 1.0,
            vertical_inset: 3.0,
            min_height: 8.0,
        }
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
