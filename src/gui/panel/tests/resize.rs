use crate::{
    gui::{
        input::{InputSequence, InputSequenceRange, InputTimestamp},
        panel::*,
        types::Point,
    },
    widgets::{
        DragHandleMessage, DragHandleMetadata, EditPhase, InteractionProvenance, InteractionSource,
        PointerModifiers,
    },
};

#[test]
fn panel_resize_drag_grows_from_trailing_edges() {
    let horizontal = PanelResizeDrag::new(PanelResizeEdge::Right, Point::new(100.0, 0.0), 240.0);
    let vertical = PanelResizeDrag::new(PanelResizeEdge::Bottom, Point::new(0.0, 100.0), 120.0);

    assert_eq!(
        horizontal.size_at(Point::new(140.0, 0.0), 48.0, 420.0),
        280.0
    );
    assert_eq!(vertical.size_at(Point::new(0.0, 140.0), 48.0, 420.0), 160.0);
}

#[test]
fn panel_resize_drag_grows_from_leading_edges() {
    let horizontal = PanelResizeDrag::new(PanelResizeEdge::Left, Point::new(100.0, 0.0), 240.0);
    let vertical = PanelResizeDrag::new(PanelResizeEdge::Top, Point::new(0.0, 100.0), 120.0);

    assert_eq!(
        horizontal.size_at(Point::new(60.0, 0.0), 48.0, 420.0),
        280.0
    );
    assert_eq!(vertical.size_at(Point::new(0.0, 60.0), 48.0, 420.0), 160.0);
}

#[test]
fn panel_resize_drag_clamps_size() {
    let drag = PanelResizeDrag::new(PanelResizeEdge::Right, Point::new(100.0, 0.0), 240.0);

    assert_eq!(drag.size_at(Point::new(-300.0, 0.0), 48.0, 420.0), 48.0);
    assert_eq!(drag.size_at(Point::new(500.0, 0.0), 48.0, 420.0), 420.0);
}

#[test]
fn update_panel_resize_drag_manages_drag_lifecycle() {
    let mut drag = None;

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        None
    );
    assert!(drag.is_some());

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(140.0, 0.0),
                metadata: DragHandleMetadata::empty(),
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        Some(280.0)
    );
    assert!(drag.is_some());

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Ended {
                position: Point::new(200.0, 0.0),
                metadata: DragHandleMetadata::empty(),
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        Some(340.0)
    );
    assert_eq!(drag, None);
}

#[test]
fn update_panel_resize_drag_ignores_orphaned_motion() {
    let mut drag = None;

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(140.0, 0.0),
                metadata: DragHandleMetadata::empty(),
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        None
    );
}

#[test]
fn update_collapsible_panel_resize_drag_collapses_on_double_activate() {
    let mut drag = Some(PanelResizeDrag::new(
        PanelResizeEdge::Top,
        Point::new(0.0, 120.0),
        180.0,
    ));

    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0),
                metadata: DragHandleMetadata::empty(),
            },
            PanelResizeEdge::Top,
            180.0,
            72.0,
            240.0,
            48.0,
        ),
        Some(72.0)
    );
    assert_eq!(drag, None);
}

#[test]
fn update_collapsible_panel_resize_drag_preserves_normal_resize_lifecycle() {
    let mut drag = None;

    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::started(Point::new(0.0, 120.0)),
            PanelResizeEdge::Top,
            148.0,
            72.0,
            240.0,
            72.0,
        ),
        None
    );
    assert!(drag.is_some());
    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(0.0, 80.0),
                metadata: DragHandleMetadata::empty(),
            },
            PanelResizeEdge::Top,
            148.0,
            72.0,
            240.0,
            72.0,
        ),
        Some(188.0)
    );
    assert!(drag.is_some());
}

#[test]
fn panel_resize_state_updates_durable_size_and_drag_lifecycle() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::new(PanelResizeEdge::Right, 48.0, 420.0);

    assert_eq!(
        state.resize(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints,
        ),
        None
    );
    assert_eq!(state.size(), 240.0);
    assert!(state.is_resizing());

    assert_eq!(
        state.resize(
            DragHandleMessage::Moved {
                position: Point::new(160.0, 0.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(300.0)
    );
    assert_eq!(state.size(), 300.0);
    assert!(state.is_resizing());

    assert_eq!(
        state.resize(
            DragHandleMessage::Ended {
                position: Point::new(1_000.0, 0.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(420.0)
    );
    assert_eq!(state.size(), 420.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_constraints_named_edges_preserve_edge_and_normalize_bounds() {
    assert_eq!(
        PanelResizeConstraints::left(100.0, 40.0),
        PanelResizeConstraints {
            edge: PanelResizeEdge::Left,
            min_size: 100.0,
            max_size: 100.0,
        }
    );
    assert_eq!(
        PanelResizeConstraints::right(48.0, 420.0).edge,
        PanelResizeEdge::Right
    );
    assert_eq!(
        PanelResizeConstraints::top(48.0, 420.0).edge,
        PanelResizeEdge::Top
    );
    assert_eq!(
        PanelResizeConstraints::bottom(48.0, 420.0).edge,
        PanelResizeEdge::Bottom
    );
}

#[test]
fn collapsible_panel_resize_constraints_named_edges_preserve_collapse_target() {
    let constraints = CollapsiblePanelResizeConstraints::top(72.0, 240.0, 48.0);

    assert_eq!(constraints.resize.edge, PanelResizeEdge::Top);
    assert_eq!(constraints.resize.min_size, 72.0);
    assert_eq!(constraints.resize.max_size, 240.0);
    assert_eq!(constraints.collapsed_size, 72.0);
    assert_eq!(
        CollapsiblePanelResizeConstraints::right(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Right
    );
    assert_eq!(
        CollapsiblePanelResizeConstraints::left(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Left
    );
    assert_eq!(
        CollapsiblePanelResizeConstraints::bottom(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Bottom
    );
}

#[test]
fn panel_resize_state_toggles_collapsible_size_on_double_activate() {
    let mut state = PanelResizeState::new(180.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 48.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(72.0)
    );
    assert_eq!(state.size(), 72.0);
    assert!(!state.is_resizing());

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(180.0)
    );
    assert_eq!(state.size(), 180.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_state_restores_last_dragged_collapsible_size() {
    let mut state = PanelResizeState::new(180.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 48.0);

    state.resize_collapsible(
        DragHandleMessage::started(Point::new(0.0, 120.0)),
        constraints,
    );
    state.resize_collapsible(
        DragHandleMessage::Ended {
            position: Point::new(0.0, 80.0),
            metadata: DragHandleMetadata::empty(),
        },
        constraints,
    );
    assert_eq!(state.size(), 220.0);

    state.resize_collapsible(
        DragHandleMessage::DoubleActivate {
            position: Point::new(0.0, 120.0),
            metadata: DragHandleMetadata::empty(),
        },
        constraints,
    );
    assert_eq!(state.size(), 72.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(220.0)
    );
    assert_eq!(state.size(), 220.0);
}

#[test]
fn panel_resize_state_expands_to_max_when_no_expanded_size_is_known() {
    let mut state = PanelResizeState::new(72.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 72.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0),
                metadata: DragHandleMetadata::empty(),
            },
            constraints,
        ),
        Some(240.0)
    );
    assert_eq!(state.size(), 240.0);
}

#[test]
fn panel_resize_typed_edit_preserves_identity_start_value_and_provenance() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::right(48.0, 420.0);
    let start_timestamp = InputTimestamp::capture();
    let update_timestamp = InputTimestamp::capture();
    let commit_timestamp = InputTimestamp::capture();
    let mut update_sequence = InputSequenceRange::singleton(InputSequence::from_runtime_value(4));
    update_sequence.extend_end(InputSequence::from_runtime_value(9));
    let start_modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };
    let update_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let commit_modifiers = PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    };

    let begin = state
        .resize_edit(
            DragHandleMessage::Started {
                origin: Point::new(100.0, 0.0),
                position: Point::new(108.0, 0.0),
                metadata: DragHandleMetadata {
                    modifiers: start_modifiers,
                    timestamp: Some(start_timestamp),
                    sequence_range: None,
                },
            },
            constraints,
        )
        .expect("typed resize start should be accepted");
    assert_eq!(begin.phase, EditPhase::Begin);
    assert_eq!(begin.start_value, 240.0);
    assert_eq!(begin.value, 240.0);
    assert_eq!(begin.transaction.source(), InteractionSource::Pointer);
    assert_eq!(
        begin.provenance,
        InteractionProvenance::Pointer {
            modifiers: start_modifiers,
            timestamp: Some(start_timestamp),
            sequence_range: None,
        }
    );

    let update = state
        .resize_edit(
            DragHandleMessage::Moved {
                position: Point::new(160.0, 0.0),
                metadata: DragHandleMetadata {
                    modifiers: update_modifiers,
                    timestamp: Some(update_timestamp),
                    sequence_range: Some(update_sequence),
                },
            },
            constraints,
        )
        .expect("typed resize update should be accepted");
    assert_eq!(update.phase, EditPhase::Update);
    assert_eq!(update.transaction, begin.transaction);
    assert_eq!(update.start_value, begin.start_value);
    assert_eq!(update.value, 300.0);
    assert_eq!(
        update.provenance,
        InteractionProvenance::Pointer {
            modifiers: update_modifiers,
            timestamp: Some(update_timestamp),
            sequence_range: Some(update_sequence),
        }
    );

    let commit = state
        .resize_edit(
            DragHandleMessage::Ended {
                position: Point::new(180.0, 0.0),
                metadata: DragHandleMetadata {
                    modifiers: commit_modifiers,
                    timestamp: Some(commit_timestamp),
                    sequence_range: None,
                },
            },
            constraints,
        )
        .expect("typed resize commit should be accepted");
    assert_eq!(commit.phase, EditPhase::Commit);
    assert_eq!(commit.transaction, begin.transaction);
    assert_eq!(commit.start_value, begin.start_value);
    assert_eq!(commit.value, 320.0);
    assert_eq!(
        commit.provenance,
        InteractionProvenance::Pointer {
            modifiers: commit_modifiers,
            timestamp: Some(commit_timestamp),
            sequence_range: None,
        }
    );
    assert_eq!(state.size(), 320.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_typed_edit_emits_noop_begin_and_commit() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::right(48.0, 420.0);

    let begin = state
        .resize_edit(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints,
        )
        .expect("typed resize start should be accepted");
    let commit = state
        .resize_edit(
            DragHandleMessage::ended(Point::new(100.0, 0.0)),
            constraints,
        )
        .expect("typed resize release should be accepted");

    assert_eq!(begin.phase, EditPhase::Begin);
    assert_eq!(commit.phase, EditPhase::Commit);
    assert_eq!(commit.transaction, begin.transaction);
    assert_eq!(commit.start_value, 240.0);
    assert_eq!(commit.value, 240.0);
    assert_eq!(state.size(), 240.0);
    assert!(!state.is_resizing());

    let begin = state
        .resize_edit(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints,
        )
        .expect("second typed resize start should be accepted");
    let cancel = state
        .resize_edit(
            DragHandleMessage::cancelled(Point::new(100.0, 0.0)),
            constraints,
        )
        .expect("no-op typed cancellation should still be a lifecycle boundary");
    assert_eq!(begin.phase, EditPhase::Begin);
    assert_eq!(cancel.phase, EditPhase::Cancel);
    assert_eq!(cancel.transaction, begin.transaction);
    assert_eq!(cancel.value, 240.0);
}

#[test]
fn panel_resize_typed_cancel_rolls_back_with_pointer_without_native_evidence() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::right(48.0, 420.0);
    let begin = state
        .resize_edit(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints,
        )
        .expect("typed resize start should be accepted");
    state.resize_edit(
        DragHandleMessage::moved(Point::new(160.0, 0.0)),
        constraints,
    );

    let cancel = state
        .resize_edit(
            DragHandleMessage::cancelled(Point::new(160.0, 0.0)),
            constraints,
        )
        .expect("active typed resize cancellation should be accepted");
    assert_eq!(cancel.phase, EditPhase::Cancel);
    assert_eq!(cancel.transaction, begin.transaction);
    assert_eq!(cancel.start_value, 240.0);
    assert_eq!(cancel.value, 240.0);
    assert_eq!(
        cancel.provenance,
        InteractionProvenance::Pointer {
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }
    );
    assert_eq!(state.size(), 240.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_concise_cancel_projects_only_a_changed_rollback() {
    let constraints = PanelResizeConstraints::right(48.0, 420.0);
    let mut changed = PanelResizeState::new(240.0);
    assert_eq!(
        changed.resize(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints
        ),
        None
    );
    assert_eq!(
        changed.resize(
            DragHandleMessage::moved(Point::new(160.0, 0.0)),
            constraints
        ),
        Some(300.0)
    );
    assert_eq!(
        changed.resize(
            DragHandleMessage::cancelled(Point::new(160.0, 0.0)),
            constraints,
        ),
        Some(240.0)
    );
    assert_eq!(changed.size(), 240.0);
    assert!(!changed.is_resizing());

    let mut noop = PanelResizeState::new(240.0);
    noop.resize(
        DragHandleMessage::started(Point::new(100.0, 0.0)),
        constraints,
    );
    assert_eq!(
        noop.resize(
            DragHandleMessage::cancelled(Point::new(100.0, 0.0)),
            constraints,
        ),
        None
    );
    assert_eq!(noop.size(), 240.0);
    assert!(!noop.is_resizing());

    let collapsible_constraints = CollapsiblePanelResizeConstraints::top(72.0, 240.0, 48.0);
    let mut collapsible = PanelResizeState::new(180.0);
    collapsible.resize_collapsible(
        DragHandleMessage::started(Point::new(0.0, 120.0)),
        collapsible_constraints,
    );
    assert_eq!(
        collapsible.resize_collapsible(
            DragHandleMessage::moved(Point::new(0.0, 80.0)),
            collapsible_constraints,
        ),
        Some(220.0)
    );
    assert_eq!(
        collapsible.resize_collapsible(
            DragHandleMessage::cancelled(Point::new(0.0, 80.0)),
            collapsible_constraints,
        ),
        Some(180.0)
    );
    assert_eq!(
        collapsible.resize_collapsible(
            DragHandleMessage::double_activate(Point::new(0.0, 80.0)),
            collapsible_constraints,
        ),
        Some(72.0)
    );
    assert_eq!(
        collapsible.resize_collapsible(
            DragHandleMessage::double_activate(Point::new(0.0, 80.0)),
            collapsible_constraints,
        ),
        Some(180.0)
    );
}

#[test]
fn panel_resize_typed_edit_ignores_orphaned_input() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::right(48.0, 420.0);

    for message in [
        DragHandleMessage::moved(Point::new(160.0, 0.0)),
        DragHandleMessage::ended(Point::new(160.0, 0.0)),
        DragHandleMessage::cancelled(Point::new(160.0, 0.0)),
    ] {
        assert_eq!(state.resize_edit(message, constraints), None);
    }
    assert_eq!(state.size(), 240.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_typed_edit_clamps_every_edge() {
    let cases = [
        (
            PanelResizeEdge::Left,
            Point::new(100.0, 0.0),
            Point::new(1_000.0, 0.0),
        ),
        (
            PanelResizeEdge::Right,
            Point::new(100.0, 0.0),
            Point::new(-1_000.0, 0.0),
        ),
        (
            PanelResizeEdge::Top,
            Point::new(0.0, 100.0),
            Point::new(0.0, 1_000.0),
        ),
        (
            PanelResizeEdge::Bottom,
            Point::new(0.0, 100.0),
            Point::new(0.0, -1_000.0),
        ),
    ];

    for (edge, start, out_of_bounds) in cases {
        let mut state = PanelResizeState::new(240.0);
        let constraints = PanelResizeConstraints::new(edge, 48.0, 420.0);
        state.resize_edit(DragHandleMessage::started(start), constraints);
        let update = state
            .resize_edit(DragHandleMessage::moved(out_of_bounds), constraints)
            .expect("active resize motion should be accepted");
        assert_eq!(update.phase, EditPhase::Update);
        assert_eq!(update.value, 48.0);
        assert!(update.value.is_finite());
        assert!((48.0..=420.0).contains(&state.size()));
    }
}

#[test]
fn panel_resize_collapsible_double_activation_clears_typed_state() {
    let mut state = PanelResizeState::new(180.0);
    let constraints = CollapsiblePanelResizeConstraints::top(72.0, 240.0, 48.0);

    assert_eq!(
        state
            .resize_collapsible_edit(
                DragHandleMessage::started(Point::new(0.0, 120.0)),
                constraints
            )
            .map(|event| event.phase),
        Some(EditPhase::Begin)
    );
    assert_eq!(
        state
            .resize_collapsible_edit(DragHandleMessage::moved(Point::new(0.0, 80.0)), constraints)
            .map(|event| event.phase),
        Some(EditPhase::Update)
    );
    assert_eq!(
        state.resize_collapsible_edit(
            DragHandleMessage::double_activate(Point::new(0.0, 80.0)),
            constraints,
        ),
        None
    );
    assert_eq!(state.size(), 72.0);
    assert!(!state.is_resizing());

    assert_eq!(
        state.resize_collapsible_edit(
            DragHandleMessage::double_activate(Point::new(0.0, 80.0)),
            constraints,
        ),
        None
    );
    assert_eq!(state.size(), 220.0);
    assert!(!state.is_resizing());
}
