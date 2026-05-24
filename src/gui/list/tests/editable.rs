use super::super::{
    ColumnSummary, ColumnSummaryParts, EditableRowKind, EditableTreeActions, EditableTreeRow,
    EditableTreeRowParts,
};

#[test]
fn column_summary_preserves_title_and_count() {
    let column = ColumnSummary::new("Inbox", 42);

    assert_eq!(column.title, "Inbox");
    assert_eq!(column.item_count, 42);
}

#[test]
fn column_summary_supports_named_parts_construction() {
    let column = ColumnSummary::from_parts(ColumnSummaryParts {
        title: "Inbox".to_owned(),
        item_count: 42,
    });

    assert_eq!(column.title, "Inbox");
    assert_eq!(column.item_count, 42);
}

#[test]
fn editable_row_kind_defaults_to_existing() {
    assert_eq!(EditableRowKind::default(), EditableRowKind::Existing);
}

#[test]
fn editable_tree_actions_default_to_unavailable() {
    let actions = EditableTreeActions::default();

    assert!(!actions.can_create_child);
    assert!(!actions.can_create_root);
    assert!(!actions.can_rename);
    assert!(!actions.can_delete);
    assert!(!actions.can_restore_retained);
    assert!(!actions.can_purge_retained);
    assert!(!actions.can_clear_history);
}

#[test]
fn editable_tree_row_preserves_existing_and_draft_state() {
    let existing = EditableTreeRow::from_parts(EditableTreeRowParts {
        depth: 0,
        selected: true,
        focused: false,
        is_root: true,
        has_children: true,
        expanded: true,
        ..EditableTreeRowParts::new("Root", "3 items")
    })
    .with_backing_index(7);
    let draft = EditableTreeRow::rename_draft(1, "Draft", "Name", None, true);

    assert_eq!(existing.label, "Root");
    assert_eq!(existing.detail, "3 items");
    assert!(existing.flags.selected);
    assert!(existing.flags.is_root);
    assert!(existing.flags.has_children);
    assert!(existing.flags.expanded);
    assert_eq!(existing.kind, EditableRowKind::Existing);
    assert_eq!(existing.backing_index, Some(7));
    assert_eq!(draft.kind, EditableRowKind::RenameDraft);
    assert_eq!(draft.input.value.as_deref(), Some("Draft"));
    assert!(draft.input.focused);
    assert!(draft.input.select_all_on_focus);
}
