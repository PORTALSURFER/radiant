use crate::{
    application::{StateDragCallback, StateStringCallback, StateView, column, scroll},
    widgets::DragHandleMessage,
};
use std::sync::Arc;

mod row;

use row::tree_list_row;

/// Named construction inputs for one visible tree-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeListItemParts {
    /// Stable caller-owned item id.
    pub id: String,
    /// Zero-based visual depth.
    pub depth: usize,
    /// Row label.
    pub label: String,
}

/// One visible row in a compact tree list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeListItem {
    /// Stable caller-owned item id.
    pub id: String,
    /// Zero-based visual depth.
    pub depth: usize,
    /// Row label.
    pub label: String,
    /// Whether this row can be expanded or collapsed.
    pub has_children: bool,
    /// Whether this branch is currently expanded.
    pub expanded: bool,
    /// Whether this row is currently selected.
    pub selected: bool,
    /// Whether this row should show a drag handle.
    pub draggable: bool,
    /// Whether this row is the current drop target.
    pub drop_target: bool,
}

impl TreeListItem {
    /// Build one visible tree-list row from named construction inputs.
    pub fn from_parts(parts: TreeListItemParts) -> Self {
        Self {
            id: parts.id,
            depth: parts.depth,
            label: parts.label,
            has_children: false,
            expanded: false,
            selected: false,
            draggable: false,
            drop_target: false,
        }
    }

    /// Build one visible tree-list row.
    pub fn new(id: impl ToString, depth: usize, label: impl Into<String>) -> Self {
        Self::from_parts(TreeListItemParts {
            id: id.to_string(),
            depth,
            label: label.into(),
        })
    }

    /// Mark the row as expandable or collapsible.
    pub fn branch(mut self, expanded: bool) -> Self {
        self.has_children = true;
        self.expanded = expanded;
        self
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Show a compact drag handle before the row label.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Mark this row as the current drop target.
    pub fn drop_target(mut self, drop_target: bool) -> Self {
        self.drop_target = drop_target;
        self
    }
}

/// Build a compact tree list from already-visible caller rows.
pub fn tree_list<State: 'static>(
    items: impl IntoIterator<Item = TreeListItem>,
    on_select: impl Fn(&mut State, String) + Send + Sync + 'static,
    on_toggle: impl Fn(&mut State, String) + Send + Sync + 'static,
) -> StateView<State> {
    tree_list_with_drag(
        items,
        on_select,
        on_toggle,
        None::<fn(&mut State, String)>,
        None::<fn(&mut State, String, DragHandleMessage)>,
    )
}

/// Build a compact tree list with optional per-row drag handles.
pub fn tree_list_with_drag<State: 'static>(
    items: impl IntoIterator<Item = TreeListItem>,
    on_select: impl Fn(&mut State, String) + Send + Sync + 'static,
    on_toggle: impl Fn(&mut State, String) + Send + Sync + 'static,
    on_context: Option<impl Fn(&mut State, String) + Send + Sync + 'static>,
    on_drag: Option<impl Fn(&mut State, String, DragHandleMessage) + Send + Sync + 'static>,
) -> StateView<State> {
    let on_select: StateStringCallback<State> = Arc::new(on_select);
    let on_toggle: StateStringCallback<State> = Arc::new(on_toggle);
    let on_context: Option<StateStringCallback<State>> =
        on_context.map(|on_context| Arc::new(on_context) as StateStringCallback<State>);
    let on_drag: Option<StateDragCallback<State>> =
        on_drag.map(|on_drag| Arc::new(on_drag) as StateDragCallback<State>);

    scroll(
        column(items.into_iter().map(|item| {
            tree_list_row(
                item,
                Arc::clone(&on_select),
                Arc::clone(&on_toggle),
                on_context.as_ref().map(Arc::clone),
                on_drag.as_ref().map(Arc::clone),
            )
        }))
        .fill_width()
        .spacing(1.0),
    )
    .fill_height()
}
