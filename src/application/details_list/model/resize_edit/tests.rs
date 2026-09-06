use super::*;
use crate::gui::types::Point;
use crate::widgets::{EditPhase, InteractionSource};

fn apply(
    active: &mut Option<DetailsColumnResizeEdit>,
    message: DragHandleMessage,
    width: f32,
) -> Option<DetailsColumnResizeEditBatch> {
    update_details_column_resize_edit(active, "name", message, Some(width), 80.0, 400.0)
}
fn phases(batch: &DetailsColumnResizeEditBatch) -> Vec<EditPhase> {
    batch.events().iter().map(|event| event.phase).collect()
}
fn x(x: f32) -> Point {
    Point::new(x, 10.0)
}

#[test]
fn threshold_start_final_motion_and_terminal_share_one_width_transaction() {
    let mut active = None;
    let begin = apply(
        &mut active,
        DragHandleMessage::started_from(x(100.0), x(120.0)),
        200.0,
    )
    .unwrap();
    assert_eq!(phases(&begin), [EditPhase::Begin, EditPhase::Update]);
    assert_eq!(begin.width_update().unwrap().width, 220.0);
    let end = apply(&mut active, DragHandleMessage::ended(x(150.0)), 220.0).unwrap();
    assert_eq!(phases(&end), [EditPhase::Update, EditPhase::Commit]);
    assert_eq!(end.width_update().unwrap().width, 250.0);
    assert_eq!(end.transaction(), begin.transaction());
    assert!(active.is_none());
    assert!(apply(&mut active, DragHandleMessage::ended(x(180.0)), 250.0).is_none());
}

#[test]
fn rollback_and_noop_cancel_have_consistent_typed_terminal_but_distinct_value_projection() {
    for moved in [false, true] {
        let mut active = None;
        let begin = apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
        let width = if moved {
            apply(&mut active, DragHandleMessage::moved(x(160.0)), 200.0)
                .unwrap()
                .width_update()
                .unwrap()
                .width
        } else {
            200.0
        };
        let cancel = apply(&mut active, DragHandleMessage::cancelled(x(160.0)), width).unwrap();
        assert_eq!(phases(&cancel), [EditPhase::Cancel]);
        assert_eq!(cancel.transaction(), begin.transaction());
        assert_eq!(
            cancel.width_update().map(|update| update.width),
            moved.then_some(200.0)
        );
        assert!(apply(&mut active, DragHandleMessage::ended(x(200.0)), 200.0).is_none());
    }
}

#[test]
fn no_op_updates_are_suppressed_but_return_to_start_and_commit_are_preserved() {
    let mut active = None;
    apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
    assert!(apply(&mut active, DragHandleMessage::moved(x(100.0)), 200.0).is_none());
    apply(&mut active, DragHandleMessage::moved(x(150.0)), 200.0).unwrap();
    let back = apply(&mut active, DragHandleMessage::moved(x(100.0)), 250.0).unwrap();
    assert_eq!(back.width_update().unwrap().width, 200.0);
    let commit = apply(&mut active, DragHandleMessage::ended(x(100.0)), 200.0).unwrap();
    assert_eq!(phases(&commit), [EditPhase::Commit]);
    assert!(commit.width_update().is_none());
}

#[test]
fn wrong_column_and_duplicate_start_cannot_take_over_an_admitted_resize() {
    let mut active = None;
    let begin = apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
    for message in [
        DragHandleMessage::started(x(150.0)),
        DragHandleMessage::moved(x(150.0)),
        DragHandleMessage::ended(x(150.0)),
        DragHandleMessage::cancelled(x(150.0)),
    ] {
        assert!(
            update_details_column_resize_edit(
                &mut active,
                "other",
                message,
                Some(200.0),
                80.0,
                400.0
            )
            .is_none()
        );
        assert_eq!(active.as_ref().unwrap().transaction(), begin.transaction());
    }
    assert!(apply(&mut active, DragHandleMessage::started(x(300.0)), 200.0).is_none());
    let next = apply(&mut active, DragHandleMessage::moved(x(150.0)), 200.0).unwrap();
    assert_eq!(next.width_update().unwrap().width, 250.0);
}

#[test]
fn removed_replaced_or_rebounded_model_cancels_without_writing_over_the_new_model() {
    for (width, min, max) in [
        (None, 80.0, 400.0),
        (Some(300.0), 80.0, 400.0),
        (Some(250.0), 90.0, 400.0),
        (Some(250.0), f32::NAN, 400.0),
    ] {
        let mut active = None;
        let begin = apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
        apply(&mut active, DragHandleMessage::moved(x(150.0)), 200.0).unwrap();
        let cancel = update_details_column_resize_edit(
            &mut active,
            "name",
            DragHandleMessage::moved(x(180.0)),
            width,
            min,
            max,
        )
        .unwrap();
        assert_eq!(phases(&cancel), [EditPhase::Cancel]);
        assert_eq!(cancel.transaction(), begin.transaction());
        assert!(cancel.width_update().is_none());
        assert!(active.is_none());
        assert!(apply(&mut active, DragHandleMessage::ended(x(200.0)), 300.0).is_none());
    }
}

#[test]
fn invalid_starts_and_nonfinite_motion_are_inert_and_invalid_terminal_cancels() {
    for width in [
        None,
        Some(f32::NAN),
        Some(f32::INFINITY),
        Some(-1.0),
        Some(500.0),
    ] {
        let mut active = None;
        assert!(
            update_details_column_resize_edit(
                &mut active,
                "name",
                DragHandleMessage::started(x(100.0)),
                width,
                80.0,
                400.0
            )
            .is_none()
        );
        assert!(active.is_none());
    }
    let mut active = None;
    apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
    assert!(apply(&mut active, DragHandleMessage::moved(x(f32::NAN)), 200.0).is_none());
    let cancel = apply(
        &mut active,
        DragHandleMessage::ended(x(f32::INFINITY)),
        200.0,
    )
    .unwrap();
    assert_eq!(phases(&cancel), [EditPhase::Cancel]);
    assert!(active.is_none());
}

#[test]
fn finite_extreme_positions_clamp_without_overflow() {
    let mut active = None;
    apply(&mut active, DragHandleMessage::started(x(-f32::MAX)), 200.0).unwrap();
    let update = apply(&mut active, DragHandleMessage::moved(x(f32::MAX)), 200.0).unwrap();
    assert_eq!(update.width_update().unwrap().width, 400.0);
}

#[test]
fn atomic_candidates_preserve_provenance_and_do_not_join_an_active_pointer_owner() {
    for provenance in [
        InteractionProvenance::Keyboard { timestamp: None },
        InteractionProvenance::Accessibility,
        InteractionProvenance::Programmatic,
        InteractionProvenance::Pointer {
            modifiers: Default::default(),
            timestamp: None,
            sequence_range: None,
        },
    ] {
        let mut active = None;
        let batch =
            details_column_width_edit(&active, "name", 200.0, 900.0, 80.0..=400.0, provenance)
                .unwrap();
        assert_eq!(
            phases(&batch),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert!(
            batch
                .events()
                .iter()
                .all(|event| event.provenance == provenance)
        );
        assert_eq!(batch.width_update().unwrap().width, 400.0);
        assert!(
            details_column_width_edit(&active, "name", 400.0, 900.0, 80.0..=400.0, provenance)
                .is_none()
        );
        apply(&mut active, DragHandleMessage::started(x(100.0)), 200.0).unwrap();
        assert!(
            details_column_width_edit(&active, "name", 200.0, 300.0, 80.0..=400.0, provenance)
                .is_none()
        );
        assert_eq!(
            active.as_ref().unwrap().event.provenance.source(),
            InteractionSource::Pointer
        );
    }
}

#[test]
fn pointer_metadata_is_preserved_and_loss_carries_no_fabricated_timestamp() {
    let metadata = DragHandleMetadata {
        timestamp: Some(crate::gui::input::InputTimestamp::capture()),
        modifiers: crate::widgets::PointerModifiers {
            shift: true,
            ..Default::default()
        },
        sequence_range: None,
    };
    let mut active = None;
    let begin = apply(
        &mut active,
        DragHandleMessage::Started {
            origin: x(100.0),
            position: x(120.0),
            metadata,
        },
        200.0,
    )
    .unwrap();
    assert!(
        begin
            .events()
            .iter()
            .all(|event| event.provenance == provenance(metadata))
    );
    let cancel = apply(&mut active, DragHandleMessage::cancelled(x(120.0)), 220.0).unwrap();
    assert_eq!(
        cancel.events()[0].provenance,
        provenance(DragHandleMetadata::empty())
    );
}
