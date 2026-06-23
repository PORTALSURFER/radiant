use super::super::DetailsColumnPlacement;
use super::*;
use crate::widgets::DragHandleMessage;

#[test]
fn details_column_resize_drag_clamps_width() {
    let drag = DetailsColumnResizeDrag::new("name", 100.0, 240.0);

    assert_eq!(drag.width_at(130.0, 48.0, 420.0), 270.0);
    assert_eq!(drag.width_at(-500.0, 48.0, 420.0), 48.0);
    assert_eq!(drag.width_at(500.0, 48.0, 420.0), 420.0);
}

#[test]
fn details_column_reorder_drag_uses_estimated_content_left() {
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
    ];

    let content_left = details_column_drag_content_left(&placements, "rating", 300.0, 10.0)
        .expect("rating column should be found");
    let drag = DetailsColumnReorderDrag::new("rating", content_left);

    assert_eq!(content_left, 16.0);
    assert_eq!(drag.pointer, crate::gui::types::Point::new(0.0, 0.0));
    assert_eq!(drag.target_index(&placements, 410.0, 10.0), Some(2));
    assert_eq!(
        drag.target_index(&placements, 560.0, 10.0)
            .map(|target| details_column_reorder_marker_x(
                &placements,
                "rating",
                target,
                content_left,
                10.0
            )),
        Some(398.0)
    );
}

#[test]
fn details_column_reorder_marker_tracks_hovered_column_edge() {
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("collection", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
        DetailsColumnPlacement::new("size", 78.0),
        DetailsColumnPlacement::new("modified", 110.0),
    ];
    let drag = DetailsColumnReorderDrag::new("rating", 0.0);

    let left_side_target = drag
        .target_index(&placements, 560.0, 10.0)
        .expect("drag target on left side of modified");
    assert_eq!(left_side_target, 4);
    assert_eq!(
        details_column_reorder_marker_x(&placements, "rating", left_side_target, 0.0, 10.0),
        558.0
    );

    let right_side_target = drag
        .target_index(&placements, 620.0, 10.0)
        .expect("drag target on right side of modified");
    assert_eq!(right_side_target, 5);
    assert_eq!(
        details_column_reorder_marker_x(&placements, "rating", right_side_target, 0.0, 10.0),
        668.0
    );
}

#[test]
fn details_column_drag_feedback_projects_marker_relative_to_content() {
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
        DetailsColumnPlacement::new("size", 78.0),
    ];
    let drag = DetailsColumnReorderDrag::from_start(
        "rating",
        16.0,
        crate::gui::types::Point::new(410.0, 18.0),
    );

    let feedback =
        details_column_drag_feedback(&drag, &placements, 10.0, 12.0).expect("drag feedback");

    assert_eq!(
        feedback,
        DetailsColumnDragFeedback {
            column_id: String::from("rating"),
            pointer: crate::gui::types::Point::new(410.0, 18.0),
            width: 68.0,
            marker_x: 380.0,
        }
    );
    assert_eq!(
        drag.current_feedback(&placements, 10.0, 12.0),
        Some(feedback)
    );
}

#[test]
fn details_column_drag_feedback_sanitizes_marker_offset_and_missing_columns() {
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
    ];
    let drag = DetailsColumnReorderDrag::from_start(
        "rating",
        16.0,
        crate::gui::types::Point::new(410.0, 18.0),
    );
    let missing = DetailsColumnReorderDrag::new("missing", 16.0);

    assert_eq!(
        details_column_drag_feedback(&drag, &placements, 10.0, f32::NAN)
            .map(|feedback| feedback.marker_x),
        Some(382.0)
    );
    assert_eq!(
        details_column_drag_feedback(&missing, &placements, 10.0, 12.0),
        None
    );
}

#[test]
fn update_details_column_resize_drag_manages_drag_lifecycle() {
    let mut drag = None;

    assert_eq!(
        update_details_column_resize_drag(
            &mut drag,
            "name",
            DragHandleMessage::Started {
                position: crate::gui::types::Point::new(100.0, 0.0)
            },
            Some(240.0),
            48.0,
            420.0,
        ),
        None
    );
    assert!(drag.is_some());

    assert_eq!(
        update_details_column_resize_drag(
            &mut drag,
            "ignored",
            DragHandleMessage::Moved {
                position: crate::gui::types::Point::new(140.0, 0.0)
            },
            None,
            48.0,
            420.0,
        ),
        Some(DetailsColumnWidthUpdate {
            column_id: String::from("name"),
            width: 280.0,
        })
    );

    assert_eq!(
        update_details_column_resize_drag(
            &mut drag,
            "ignored",
            DragHandleMessage::Ended {
                position: crate::gui::types::Point::new(200.0, 0.0)
            },
            None,
            48.0,
            420.0,
        ),
        Some(DetailsColumnWidthUpdate {
            column_id: String::from("name"),
            width: 340.0,
        })
    );
    assert_eq!(drag, None);
}

#[test]
fn update_details_column_resize_drag_ignores_unknown_starts_and_orphaned_motion() {
    let mut drag = None;

    assert_eq!(
        update_details_column_resize_drag(
            &mut drag,
            "missing",
            DragHandleMessage::Started {
                position: crate::gui::types::Point::new(100.0, 0.0)
            },
            None,
            48.0,
            420.0,
        ),
        None
    );
    assert_eq!(drag, None);

    assert_eq!(
        update_details_column_resize_drag(
            &mut drag,
            "name",
            DragHandleMessage::Moved {
                position: crate::gui::types::Point::new(140.0, 0.0)
            },
            Some(240.0),
            48.0,
            420.0,
        ),
        None
    );
}

#[test]
fn update_details_column_reorder_drag_reorders_and_clears_on_end() {
    let mut drag = None;
    let mut columns = vec![
        String::from("name"),
        String::from("rating"),
        String::from("extension"),
        String::from("size"),
    ];
    let placements = || {
        vec![
            DetailsColumnPlacement::new("name", 240.0),
            DetailsColumnPlacement::new("rating", 68.0),
            DetailsColumnPlacement::new("extension", 54.0),
            DetailsColumnPlacement::new("size", 78.0),
        ]
    };

    assert!(!update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "rating",
        DragHandleMessage::Started {
            position: crate::gui::types::Point::new(300.0, 0.0)
        },
        &placements(),
        10.0,
        String::as_str,
    ));
    assert_eq!(
        drag.as_ref().map(|drag| drag.pointer),
        Some(crate::gui::types::Point::new(300.0, 0.0))
    );

    assert!(!update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "ignored",
        DragHandleMessage::Moved {
            position: crate::gui::types::Point::new(410.0, 0.0)
        },
        &placements(),
        10.0,
        String::as_str,
    ));
    assert_eq!(columns, ["name", "rating", "extension", "size"]);
    assert_eq!(
        drag.as_ref().map(|drag| drag.pointer),
        Some(crate::gui::types::Point::new(410.0, 0.0))
    );
    assert_eq!(
        drag.as_ref()
            .and_then(|drag| drag.current_marker_x(&placements(), 10.0)),
        Some(408.0)
    );

    assert!(update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "ignored",
        DragHandleMessage::Ended {
            position: crate::gui::types::Point::new(520.0, 0.0)
        },
        &placements(),
        10.0,
        String::as_str,
    ));
    assert_eq!(columns, ["name", "extension", "size", "rating"]);
    assert_eq!(drag, None);
}

#[test]
fn update_details_column_reorder_drag_cancel_clears_without_reorder() {
    let mut drag = None;
    let mut columns = vec![
        String::from("name"),
        String::from("rating"),
        String::from("extension"),
    ];
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
    ];

    assert!(!update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "rating",
        DragHandleMessage::Started {
            position: crate::gui::types::Point::new(300.0, 0.0)
        },
        &placements,
        10.0,
        String::as_str,
    ));
    assert!(!update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "ignored",
        DragHandleMessage::Moved {
            position: crate::gui::types::Point::new(410.0, 0.0)
        },
        &placements,
        10.0,
        String::as_str,
    ));
    assert!(!update_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "ignored",
        DragHandleMessage::Cancelled {
            position: crate::gui::types::Point::new(410.0, 0.0)
        },
        &placements,
        10.0,
        String::as_str,
    ));

    assert_eq!(columns, ["name", "rating", "extension"]);
    assert_eq!(drag, None);
}

#[test]
fn update_visible_details_column_reorder_drag_preserves_hidden_columns() {
    let mut drag = None;
    let mut columns = vec![
        String::from("name"),
        String::from("source_folder"),
        String::from("rating"),
        String::from("extension"),
        String::from("path"),
    ];
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
    ];
    let visible = |column: &String| column.as_str() != "source_folder" && column.as_str() != "path";

    assert!(!update_visible_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "rating",
        DragHandleMessage::Started {
            position: crate::gui::types::Point::new(300.0, 0.0)
        },
        &placements,
        10.0,
        String::as_str,
        visible,
    ));
    assert_eq!(
        drag.as_ref().map(|drag| drag.pointer),
        Some(crate::gui::types::Point::new(300.0, 0.0))
    );

    assert!(update_visible_details_column_reorder_drag(
        &mut drag,
        &mut columns,
        "ignored",
        DragHandleMessage::Ended {
            position: crate::gui::types::Point::new(8.0, 0.0)
        },
        &placements,
        10.0,
        String::as_str,
        visible,
    ));

    assert_eq!(
        columns,
        ["rating", "name", "source_folder", "extension", "path"]
    );
    assert_eq!(drag, None);
}
